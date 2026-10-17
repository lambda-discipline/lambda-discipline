//! A lazy (call-by-need) abstract machine for Core IR.
//!
//! The machine never recurses on the Rust stack. Like GHC's STG machine it
//! keeps an explicit stack of continuation frames and alternates between
//! two states: *evaluate this expression* and *return this value to the top
//! frame*.
//!
//! * Arguments are passed as shared, memoized [`Thunk`]s. Forcing a thunk
//!   pushes an update frame, which stores the value when it returns, so an
//!   argument is evaluated at most once and only if it is used.
//! * Tail calls, including through `if`, `and` and `or`, push nothing, so
//!   loops run in constant space.
//! * Deep non-tail recursion costs one small heap frame per level instead of
//!   a native stack frame; it is bounded by [`Interpreter::set_max_frames`] and
//!   reported as an error rather than crashing.
//! * Higher-order builtins (`map`, `foldr`, ...) are frames too, so the
//!   functions they call can recurse as deeply as any other code.

mod prims;
pub mod value;

use std::io::Write;
use std::rc::Rc;

use imbl::Vector;

use crate::backend::{Backend, RuntimeError};
use crate::builtins::Builtin;
use crate::core::{Expr, Lambda, Literal, Module};
use crate::hir::GlobalId;
use crate::source::Span;

pub use value::{Closure, Env, Partial, Thunk, ThunkState, Value};

/// Default limit on continuation frames (each is a few dozen bytes).
pub const DEFAULT_MAX_FRAMES: usize = 10_000_000;

pub struct Interpreter {
    /// One thunk per global, indexed by [`GlobalId`]; grows as units load.
    globals: Vec<Option<Thunk>>,
    stack: Vec<Frame>,
    max_frames: usize,
    out: Box<dyn Write>,
}

/// The machine's next move.
enum Control {
    Eval(Rc<Expr>, Rc<Env>),
    Return(Value),
}

/// A continuation: what to do with the value being returned.
enum Frame {
    /// Memoize the value in this thunk.
    Update(Thunk),
    /// Apply the returned function to these arguments.
    Apply { args: Vec<Thunk>, span: Span },
    /// Collect a forced argument of a builtin, then force the next one.
    Strict(BuiltinCall),
    /// Evaluate the remaining elements of a list literal.
    List {
        expr: Rc<Expr>,
        env: Rc<Env>,
        next: usize,
        values: Vector<Value>,
    },
    /// A higher-order builtin waiting for one of its calls to return.
    Iterate(Box<Iteration>),
}

/// A builtin applied to exactly its arity, while its strict arguments are forced.
struct BuiltinCall {
    builtin: Builtin,
    args: Vec<Thunk>,
    /// Forced values of the arguments seen so far (`Unit` for lazy ones).
    values: Vec<Value>,
    span: Option<Span>,
}

/// State of a higher-order builtin between calls of its function argument.
struct Iteration {
    kind: IterationKind,
    func: Value,
    items: Vector<Value>,
    /// Second list, for `zipWith`.
    others: Vector<Value>,
    /// Index of the element whose call is in flight.
    index: usize,
    results: Vector<Value>,
    span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IterationKind {
    Map,
    Filter,
    Foldl,
    Foldr,
    ZipWith,
}

impl Interpreter {
    pub fn new(out: Box<dyn Write>) -> Self {
        Self {
            globals: Vec::new(),
            stack: Vec::new(),
            max_frames: DEFAULT_MAX_FRAMES,
            out,
        }
    }

    /// An interpreter printing to stdout.
    pub fn stdout() -> Self {
        Self::new(Box::new(std::io::stdout()))
    }

    /// Limits how deep non-tail recursion may go before it is reported as an error.
    pub fn set_max_frames(&mut self, max_frames: usize) {
        self.max_frames = max_frames;
    }

    /// Registers a unit's globals without running anything.
    pub fn define(&mut self, module: &Module) {
        for global in &module.globals {
            let index = global.id.index();
            if self.globals.len() <= index {
                self.globals.resize(index + 1, None);
            }
            let env = Env::new(global.body.frame_size);
            self.globals[index] = Some(Thunk::delayed(global.body.expr.clone(), env));
        }
    }

    /// Evaluates a global to a value.
    pub fn global_value(&mut self, id: GlobalId) -> Result<Value, RuntimeError> {
        let thunk = self.global(id);
        self.force(&thunk)
    }

    /// Forces a thunk, memoizing its value.
    pub fn force(&mut self, thunk: &Thunk) -> Result<Value, RuntimeError> {
        let base = self.stack.len();
        match self.enter(thunk.clone()) {
            Ok(control) => self.run(control),
            Err(error) => {
                self.unwind(base);
                Err(error)
            }
        }
    }

    /// Evaluates an expression in an environment.
    pub fn eval(&mut self, expr: Rc<Expr>, env: Rc<Env>) -> Result<Value, RuntimeError> {
        self.run(Control::Eval(expr, env))
    }

    pub fn flush(&mut self) {
        let _ = self.out.flush();
    }

    /// Gives back the output stream.
    pub fn into_output(mut self) -> Box<dyn Write> {
        self.flush();
        self.out
    }

    fn global(&self, id: GlobalId) -> Thunk {
        self.globals
            .get(id.index())
            .and_then(Clone::clone)
            .expect("globals are defined before they are used")
    }

    // ---- the machine loop ------------------------------------------------

    fn run(&mut self, mut control: Control) -> Result<Value, RuntimeError> {
        let base = self.stack.len();
        loop {
            let step = match control {
                Control::Eval(expr, env) => self.eval_step(expr, env),
                Control::Return(value) => {
                    if self.stack.len() == base {
                        return Ok(value);
                    }
                    let frame = self.stack.pop().expect("the stack is above its base");
                    self.return_step(frame, value)
                }
            };
            control = match step {
                Ok(next) if self.stack.len() <= self.max_frames => next,
                Ok(_) => {
                    self.unwind(base);
                    return Err(RuntimeError::new("recursion is too deep").with_note(format!(
                        "evaluation needed more than {} pending steps; this usually means unbounded recursion",
                        self.max_frames
                    )));
                }
                Err(error) => {
                    self.unwind(base);
                    return Err(error);
                }
            };
        }
    }

    /// Drops frames after an error, making half-forced thunks re-runnable.
    fn unwind(&mut self, base: usize) {
        for frame in self.stack.drain(base..) {
            if let Frame::Update(thunk) = frame {
                let mut state = thunk.0.borrow_mut();
                if let ThunkState::Forcing { expr, env } = &*state {
                    *state = ThunkState::Delayed {
                        expr: expr.clone(),
                        env: env.clone(),
                    };
                }
            }
        }
    }

    fn eval_step(&mut self, expr: Rc<Expr>, env: Rc<Env>) -> Result<Control, RuntimeError> {
        Ok(match expr.as_ref() {
            Expr::Lit(literal) => Control::Return(literal_value(literal)),
            Expr::Local(slot) => return self.enter(env.get(*slot)),
            Expr::Global(id) => return self.enter(self.global(*id)),
            Expr::Builtin(builtin) => {
                if builtin.arity() == 0 {
                    return self.start_builtin(*builtin, Vec::new(), None);
                }
                Control::Return(Value::Builtin(*builtin))
            }
            Expr::Lambda(lambda) => Control::Return(make_closure(lambda, &env)),
            Expr::List { items, .. } => match items.first() {
                None => Control::Return(Value::List(Vector::new())),
                Some(first) => {
                    let first = first.clone();
                    self.stack.push(Frame::List {
                        expr: expr.clone(),
                        env: env.clone(),
                        next: 1,
                        values: Vector::new(),
                    });
                    Control::Eval(first, env)
                }
            },
            Expr::Let { slot, value, body } => {
                // Always delayed: the value may refer to its own slot.
                env.set(*slot, Thunk::delayed(value.clone(), env.clone()));
                Control::Eval(body.clone(), env)
            }
            Expr::App { func, args, span } => {
                let thunks: Vec<Thunk> = args.iter().map(|arg| self.suspend(arg, &env)).collect();
                match self.known_value(func, &env) {
                    Some(callee) => return self.apply(callee, thunks, *span),
                    None => {
                        self.stack.push(Frame::Apply {
                            args: thunks,
                            span: *span,
                        });
                        Control::Eval(func.clone(), env)
                    }
                }
            }
        })
    }

    fn return_step(&mut self, frame: Frame, value: Value) -> Result<Control, RuntimeError> {
        match frame {
            Frame::Update(thunk) => {
                *thunk.0.borrow_mut() = ThunkState::Done(value.clone());
                Ok(Control::Return(value))
            }
            Frame::Apply { args, span } => self.apply(value, args, span),
            Frame::Strict(mut call) => {
                call.values.push(value);
                self.continue_builtin(call)
            }
            Frame::List {
                expr,
                env,
                next,
                mut values,
            } => {
                values.push_back(value);
                let Expr::List { items, .. } = expr.as_ref() else {
                    unreachable!("list frames hold list expressions")
                };
                match items.get(next).cloned() {
                    None => Ok(Control::Return(Value::List(values))),
                    Some(item) => {
                        self.stack.push(Frame::List {
                            expr: expr.clone(),
                            env: env.clone(),
                            next: next + 1,
                            values,
                        });
                        Ok(Control::Eval(item, env))
                    }
                }
            }
            Frame::Iterate(iteration) => self.iterate(*iteration, value),
        }
    }

    /// Continues with a thunk's value: immediately if known, otherwise by
    /// evaluating it under an update frame.
    fn enter(&mut self, thunk: Thunk) -> Result<Control, RuntimeError> {
        let mut state = thunk.0.borrow_mut();
        match &*state {
            ThunkState::Done(value) => Ok(Control::Return(value.clone())),
            ThunkState::Forcing { expr, .. } => {
                let error =
                    RuntimeError::new("this value depends on itself, so it can never be computed")
                        .with_note("evaluating it would loop forever");
                Err(match expr_span(expr) {
                    Some(span) => error
                        .at(span)
                        .with_label("evaluating this needs its own value"),
                    None => error,
                })
            }
            ThunkState::Delayed { expr, env } => {
                let (expr, env) = (expr.clone(), env.clone());
                *state = ThunkState::Forcing {
                    expr: expr.clone(),
                    env: env.clone(),
                };
                drop(state);
                self.stack.push(Frame::Update(thunk));
                Ok(Control::Eval(expr, env))
            }
        }
    }

    /// Continues with a thunk in tail position. If nothing else refers to
    /// it, there is no one to share the result with, so it is evaluated
    /// directly without an update frame. This keeps loops in constant space.
    fn tail(&mut self, thunk: Thunk) -> Result<Control, RuntimeError> {
        if Rc::strong_count(&thunk.0) == 1 {
            if let ThunkState::Delayed { expr, env } = &*thunk.0.borrow() {
                return Ok(Control::Eval(expr.clone(), env.clone()));
            }
        }
        self.enter(thunk)
    }

    /// Wraps an argument without evaluating it. Atoms are shared or built
    /// directly instead of allocating a suspended computation.
    fn suspend(&self, expr: &Rc<Expr>, env: &Rc<Env>) -> Thunk {
        match expr.as_ref() {
            Expr::Local(slot) => env.get(*slot),
            Expr::Global(id) => self.global(*id),
            Expr::Lit(literal) => Thunk::done(literal_value(literal)),
            Expr::Lambda(lambda) => Thunk::done(make_closure(lambda, env)),
            Expr::Builtin(builtin) if builtin.arity() > 0 => Thunk::done(Value::Builtin(*builtin)),
            _ => Thunk::delayed(expr.clone(), env.clone()),
        }
    }

    /// The value of a callee that needs no evaluation, if there is one.
    fn known_value(&self, expr: &Rc<Expr>, env: &Rc<Env>) -> Option<Value> {
        match expr.as_ref() {
            Expr::Builtin(builtin) if builtin.arity() > 0 => Some(Value::Builtin(*builtin)),
            Expr::Lambda(lambda) => Some(make_closure(lambda, env)),
            Expr::Global(id) => self.global(*id).value(),
            Expr::Local(slot) => env.get(*slot).value(),
            _ => None,
        }
    }

    // ---- application -------------------------------------------------------

    fn apply(
        &mut self,
        func: Value,
        mut args: Vec<Thunk>,
        span: Span,
    ) -> Result<Control, RuntimeError> {
        match &func {
            Value::Closure(closure) => {
                let arity = closure.lambda.arity as usize;
                if args.len() < arity {
                    return Ok(partial(func.clone(), args));
                }
                let rest = args.split_off(arity);
                if !rest.is_empty() {
                    self.stack.push(Frame::Apply { args: rest, span });
                }
                let env = Env::with(
                    closure.lambda.frame_size,
                    closure.captured.iter().cloned().chain(args),
                );
                Ok(Control::Eval(closure.lambda.body.clone(), env))
            }
            Value::Partial(applied) => {
                let mut all = applied.args.clone();
                all.append(&mut args);
                self.apply(applied.func.clone(), all, span)
            }
            &Value::Builtin(builtin) => {
                let arity = if builtin.is_variadic() {
                    args.len()
                } else {
                    builtin.arity()
                };
                if args.len() < arity {
                    return Ok(partial(Value::Builtin(builtin), args));
                }
                let rest = args.split_off(arity);
                if !rest.is_empty() {
                    self.stack.push(Frame::Apply { args: rest, span });
                }
                self.start_builtin(builtin, args, Some(span))
            }
            other => {
                Err(RuntimeError::new(format!("`{}` is not a function", other.show())).at(span))
            }
        }
    }

    fn start_builtin(
        &mut self,
        builtin: Builtin,
        args: Vec<Thunk>,
        span: Option<Span>,
    ) -> Result<Control, RuntimeError> {
        let values = Vec::with_capacity(args.len());
        self.continue_builtin(BuiltinCall {
            builtin,
            args,
            values,
            span,
        })
    }

    /// Forces the builtin's strict arguments one at a time, then runs it.
    fn continue_builtin(&mut self, mut call: BuiltinCall) -> Result<Control, RuntimeError> {
        while call.values.len() < call.args.len() {
            let index = call.values.len();
            if !prims::is_strict(call.builtin, index) {
                call.values.push(Value::Unit);
                continue;
            }
            let thunk = call.args[index].clone();
            if let Some(value) = thunk.value() {
                call.values.push(value);
                continue;
            }
            self.stack.push(Frame::Strict(call));
            return self.enter(thunk);
        }
        self.run_builtin(call)
    }

    /// Runs a builtin whose strict arguments are all forced.
    fn run_builtin(&mut self, call: BuiltinCall) -> Result<Control, RuntimeError> {
        let BuiltinCall {
            builtin,
            mut args,
            values,
            span,
        } = call;
        use Builtin as B;
        match builtin {
            B::If => {
                let index = if prims::bool_of(&values[0])? { 1 } else { 2 };
                self.tail(args.swap_remove(index))
            }
            B::And => match prims::bool_of(&values[0])? {
                true => self.tail(args.swap_remove(1)),
                false => Ok(Control::Return(Value::Bool(false))),
            },
            B::Or => match prims::bool_of(&values[0])? {
                true => Ok(Control::Return(Value::Bool(true))),
                false => self.tail(args.swap_remove(1)),
            },
            B::Print | B::Println => {
                let mut text: String = values.iter().map(Value::display).collect();
                if builtin == B::Println {
                    text.push('\n');
                }
                self.out.write_all(text.as_bytes()).map_err(|e| {
                    let error = RuntimeError::new(format!("cannot write output: {e}"));
                    match span {
                        Some(span) => error.at(span),
                        None => error,
                    }
                })?;
                Ok(Control::Return(Value::Unit))
            }
            B::Map | B::Filter | B::ZipWith => {
                let kind = match builtin {
                    B::Map => IterationKind::Map,
                    B::Filter => IterationKind::Filter,
                    _ => IterationKind::ZipWith,
                };
                let others = match kind {
                    IterationKind::ZipWith => prims::list_of(&values[2])?,
                    _ => Vector::new(),
                };
                let iteration = Iteration {
                    kind,
                    func: values[0].clone(),
                    items: prims::list_of(&values[1])?,
                    others,
                    index: 0,
                    results: Vector::new(),
                    span: span.expect("higher-order builtins are applied at a call site"),
                };
                self.next_call(iteration, None)
            }
            B::Foldl | B::Foldr => {
                let items = prims::list_of(&values[2])?;
                let initial = args.swap_remove(1);
                if items.is_empty() {
                    return self.tail(initial);
                }
                let kind = if builtin == B::Foldl {
                    IterationKind::Foldl
                } else {
                    IterationKind::Foldr
                };
                let iteration = Iteration {
                    kind,
                    func: values[0].clone(),
                    index: if kind == IterationKind::Foldl {
                        0
                    } else {
                        items.len() - 1
                    },
                    items,
                    others: Vector::new(),
                    results: Vector::new(),
                    span: span.expect("higher-order builtins are applied at a call site"),
                };
                self.next_call(iteration, Some(initial))
            }
            _ => prims::pure(builtin, &values, span).map(Control::Return),
        }
    }

    // ---- higher-order builtins --------------------------------------------

    /// Starts the call for `iteration.index`, or finishes. `acc` is the
    /// accumulator for folds.
    fn next_call(
        &mut self,
        iteration: Iteration,
        acc: Option<Thunk>,
    ) -> Result<Control, RuntimeError> {
        let item = |i: usize| Thunk::done(iteration.items[i].clone());
        let args = match iteration.kind {
            IterationKind::Map | IterationKind::Filter => {
                if iteration.index >= iteration.items.len() {
                    return Ok(Control::Return(Value::List(iteration.results)));
                }
                vec![item(iteration.index)]
            }
            IterationKind::ZipWith => {
                if iteration.index >= iteration.items.len().min(iteration.others.len()) {
                    return Ok(Control::Return(Value::List(iteration.results)));
                }
                vec![
                    item(iteration.index),
                    Thunk::done(iteration.others[iteration.index].clone()),
                ]
            }
            IterationKind::Foldl => vec![
                acc.expect("folds carry an accumulator"),
                item(iteration.index),
            ],
            IterationKind::Foldr => vec![
                item(iteration.index),
                acc.expect("folds carry an accumulator"),
            ],
        };
        let (func, span) = (iteration.func.clone(), iteration.span);
        self.stack.push(Frame::Iterate(Box::new(iteration)));
        self.apply(func, args, span)
    }

    /// Handles the result of one call made by a higher-order builtin.
    fn iterate(&mut self, mut iteration: Iteration, value: Value) -> Result<Control, RuntimeError> {
        let acc = match iteration.kind {
            IterationKind::Map | IterationKind::ZipWith => {
                iteration.results.push_back(value);
                None
            }
            IterationKind::Filter => {
                if prims::bool_of(&value)? {
                    iteration
                        .results
                        .push_back(iteration.items[iteration.index].clone());
                }
                None
            }
            IterationKind::Foldl => {
                if iteration.index + 1 == iteration.items.len() {
                    return Ok(Control::Return(value));
                }
                Some(Thunk::done(value))
            }
            IterationKind::Foldr => {
                if iteration.index == 0 {
                    return Ok(Control::Return(value));
                }
                iteration.index -= 1;
                return self.next_call(iteration, Some(Thunk::done(value)));
            }
        };
        iteration.index += 1;
        self.next_call(iteration, acc)
    }
}

impl Backend for Interpreter {
    type Output = Vec<Value>;

    /// Defines the module's globals, then evaluates its top-level
    /// expressions in order, returning their values.
    fn run(&mut self, module: &Module) -> Result<Vec<Value>, RuntimeError> {
        self.define(module);
        let mut values = Vec::with_capacity(module.entries.len());
        for entry in &module.entries {
            let env = Env::new(entry.frame_size);
            let value = self.eval(entry.expr.clone(), env);
            self.flush();
            values.push(value?);
        }
        Ok(values)
    }
}

fn partial(func: Value, args: Vec<Thunk>) -> Control {
    Control::Return(Value::Partial(Rc::new(Partial { func, args })))
}

fn literal_value(literal: &Literal) -> Value {
    match literal {
        Literal::Number(n) => Value::Number(*n),
        Literal::String(s) => Value::String(s.clone()),
        Literal::Unit => Value::Unit,
    }
}

fn make_closure(lambda: &Rc<Lambda>, env: &Rc<Env>) -> Value {
    Value::Closure(Rc::new(Closure {
        lambda: lambda.clone(),
        captured: lambda.captures.iter().map(|&slot| env.get(slot)).collect(),
    }))
}

/// Source location of an expression, when it has one.
fn expr_span(expr: &Expr) -> Option<Span> {
    match expr {
        Expr::App { span, .. } | Expr::List { span, .. } => Some(*span),
        Expr::Lambda(lambda) => Some(lambda.span),
        Expr::Let { body, .. } => expr_span(body),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
