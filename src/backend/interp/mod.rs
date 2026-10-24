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
//! * Every few thousand steps the machine checks an interrupt flag (see
//!   [`Interpreter::set_interrupt`]), so a runaway loop can be stopped
//!   without losing the session.
//! * Lists have a strict spine but lazy elements: a list literal suspends
//!   its elements, `map` and `zipWith` build suspended calls, and `foldr`
//!   passes the fold of the rest of the list as an unevaluated argument, so
//!   it stops as soon as its function ignores that argument.
//! * Higher-order builtins that must call their function (`filter`, `foldl`)
//!   are frames too, so the functions they call can recurse as deeply as any
//!   other code.
//! * Builtins that inspect elements (`eq`, `show`, `print`, `sum`, ...)
//!   force them first through walk frames, again without native recursion.
//! * Constructor fields are lazy like list elements. A `case` evaluates its
//!   scrutinee under a case frame, which picks the alternative and binds the
//!   fields, still unevaluated, to frame slots; the alternative runs in tail
//!   position.
//! * Evaluation has no effects. Applying `print`, `readFile`, `bind` and the
//!   other action builtins builds a [`Value::Io`] that describes an action;
//!   only [`Interpreter::perform`] runs one, and [`Backend::run`] performs a
//!   unit's top-level actions in order.

mod prims;
pub mod value;

use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use imbl::Vector;

use crate::backend::{Backend, RuntimeError};
use crate::builtins::Builtin;
use crate::core::{AltPat, Case, Expr, Lambda, Literal, Module};
use crate::hir::{ConId, GlobalId};
use crate::source::Span;

use prims::Strictness;

pub use value::{Action, Closure, Env, Partial, Thunk, ThunkState, Value};

/// Default limit on continuation frames (each is a few dozen bytes).
pub const DEFAULT_MAX_FRAMES: usize = 10_000_000;

/// Machine steps between two checks of the interrupt flag.
const INTERRUPT_INTERVAL: u32 = 4096;

pub struct Interpreter {
    /// One thunk per global, indexed by [`GlobalId`]; grows as units load.
    globals: Vec<Option<Thunk>>,
    stack: Vec<Frame>,
    max_frames: usize,
    out: Box<dyn Write>,
    /// Set from outside (a Ctrl-C handler) to stop the running evaluation.
    interrupt: Arc<AtomicBool>,
    /// Steps until the next check of `interrupt`.
    countdown: u32,
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
    /// Evaluate the elements of the returned list, then return it; with
    /// `deep`, the elements of the lists inside it too.
    Force { deep: bool },
    /// A walk through a list's elements, waiting for one to be evaluated.
    Walk(Box<Walk>),
    /// A higher-order builtin waiting for one of its calls to return.
    Iterate(Box<Iteration>),
    /// Report the returned value, which is evaluated completely, as the
    /// error of a call to `builtin` (`unwrapOk` of an `Err`).
    Fail {
        builtin: Builtin,
        span: Option<Span>,
    },
    /// Choose the alternative of `case` that matches the returned value.
    Case { case: Rc<Case>, env: Rc<Env> },
    /// Perform the returned action; its result goes to the frame below.
    Perform,
    /// Pass the result of the action being performed to `func`, then
    /// perform the action it returns (`bind`).
    Bind { func: Thunk, span: Span },
    /// Discard the result of the action being performed, then perform
    /// `next` (`then`).
    Then { next: Thunk },
    /// Collect the result of the action being performed, then perform the
    /// rest (`sequence`).
    Sequence {
        actions: Vector<Thunk>,
        results: Vector<Thunk>,
    },
}

/// Progress through the elements of `root`, evaluating each of them.
struct Walk {
    root: Value,
    /// Whether to descend into the lists found among the elements.
    deep: bool,
    /// Lists and data being walked, innermost last, with the index of the
    /// next element or field.
    pending: Vec<(Vector<Thunk>, usize)>,
}

impl Walk {
    fn new(root: Value, deep: bool) -> Self {
        let mut walk = Walk {
            root,
            deep,
            pending: Vec::new(),
        };
        walk.enter(&walk.root.clone());
        walk
    }

    /// Queues the elements of `value` if it is a list, or its fields if it
    /// is data.
    fn enter(&mut self, value: &Value) {
        if let Value::List(items) | Value::Data(_, items) = value {
            if !items.is_empty() {
                self.pending.push((items.clone(), 0));
            }
        }
    }
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
    items: Vector<Thunk>,
    /// Index of the element whose call is in flight.
    index: usize,
    results: Vector<Thunk>,
    span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IterationKind {
    Filter,
    Foldl,
}

impl Interpreter {
    pub fn new(out: Box<dyn Write>) -> Self {
        Self {
            globals: Vec::new(),
            stack: Vec::new(),
            max_frames: DEFAULT_MAX_FRAMES,
            out,
            interrupt: Arc::new(AtomicBool::new(false)),
            countdown: INTERRUPT_INTERVAL,
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

    /// Uses `flag` to stop evaluation: once it is set, the running
    /// evaluation fails with an `interrupted` error and the flag is cleared.
    pub fn set_interrupt(&mut self, flag: Arc<AtomicBool>) {
        self.interrupt = flag;
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

    /// Evaluates every list element and field inside a value, so it can be shown.
    pub fn normalize(&mut self, value: Value) -> Result<Value, RuntimeError> {
        let base = self.stack.len();
        self.stack.push(Frame::Force { deep: true });
        self.run_from(base, Control::Return(value))
    }

    /// Performs an action, returning its result. Any other value is
    /// returned unchanged.
    pub fn perform(&mut self, value: Value) -> Result<Value, RuntimeError> {
        let action = match value.into_action() {
            Ok(action) => action,
            Err(value) => return Ok(value),
        };
        let base = self.stack.len();
        match self.start_action(action) {
            Ok(control) => self.run_from(base, control),
            Err(error) => {
                self.unwind(base);
                Err(error)
            }
        }
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

    fn run(&mut self, control: Control) -> Result<Value, RuntimeError> {
        self.run_from(self.stack.len(), control)
    }

    /// Runs until a value returns to the frame at `base`.
    fn run_from(&mut self, base: usize, mut control: Control) -> Result<Value, RuntimeError> {
        loop {
            let step = match control {
                Control::Eval(expr, env) => self.eval_step(&expr, env),
                Control::Return(value) => {
                    if self.stack.len() == base {
                        return Ok(value);
                    }
                    let frame = self.stack.pop().expect("the stack is above its base");
                    self.return_step(frame, value)
                }
            };
            control = match step {
                Ok(_) if self.interrupted() => {
                    self.unwind(base);
                    return Err(RuntimeError::interrupted());
                }
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

    /// Counts a step and, every [`INTERRUPT_INTERVAL`] steps, takes the
    /// interrupt flag.
    fn interrupted(&mut self) -> bool {
        self.countdown -= 1;
        if self.countdown > 0 {
            return false;
        }
        self.countdown = INTERRUPT_INTERVAL;
        self.interrupt.swap(false, Ordering::Relaxed)
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

    fn eval_step(&mut self, expr: &Rc<Expr>, env: Rc<Env>) -> Result<Control, RuntimeError> {
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
            Expr::List { items, .. } => Control::Return(Value::List(
                items.iter().map(|item| self.suspend(item, &env)).collect(),
            )),
            Expr::Let { slot, value, body } => {
                // Always delayed: the value may refer to its own slot.
                env.set(*slot, Thunk::delayed(value.clone(), env.clone()));
                Control::Eval(body.clone(), env)
            }
            Expr::Con { con, args, .. } => Control::Return(Value::Data(
                con.clone(),
                args.iter().map(|arg| self.suspend(arg, &env)).collect(),
            )),
            Expr::Case(case) => {
                if let Some(value) = self.known_value(&case.scrutinee, &env) {
                    return Self::select(case, env, &value);
                }
                let scrutinee = case.scrutinee.clone();
                self.stack.push(Frame::Case {
                    case: case.clone(),
                    env: env.clone(),
                });
                Control::Eval(scrutinee, env)
            }
            Expr::App { func, args, span } => {
                let thunks: Vec<Thunk> = args.iter().map(|arg| self.suspend(arg, &env)).collect();
                if let Some(callee) = self.known_value(func, &env) {
                    return self.apply(&callee, thunks, *span);
                }
                self.stack.push(Frame::Apply {
                    args: thunks,
                    span: *span,
                });
                Control::Eval(func.clone(), env)
            }
        })
    }

    fn return_step(&mut self, frame: Frame, value: Value) -> Result<Control, RuntimeError> {
        match frame {
            Frame::Update(thunk) => {
                *thunk.0.borrow_mut() = ThunkState::Done(value.clone());
                Ok(Control::Return(value))
            }
            Frame::Apply { args, span } => self.apply(&value, args, span),
            Frame::Strict(mut call) => {
                call.values.push(value);
                self.continue_builtin(call)
            }
            Frame::Force { deep } => self.walk(Walk::new(value, deep)),
            Frame::Walk(mut walk) => {
                if walk.deep {
                    walk.enter(&value);
                }
                self.walk(*walk)
            }
            Frame::Iterate(iteration) => self.iterate(*iteration, value),
            Frame::Case { case, env } => Self::select(&case, env, &value),
            Frame::Fail { builtin, span } => Err(prims::call_error(
                builtin,
                span,
                format!("`{builtin}` of an `Err`: {}", value.display()),
            )),
            Frame::Perform => match value.into_action() {
                Ok(action) => self.start_action(action),
                Err(other) => Err(RuntimeError::new(format!(
                    "internal error: expected an action, found `{}`",
                    other.show()
                ))),
            },
            Frame::Bind { func, span } => self.bind_next(func, Thunk::done(value), span),
            Frame::Then { next } => self.then_next(next),
            Frame::Sequence {
                actions,
                mut results,
            } => {
                results.push_back(Thunk::done(value));
                self.next_in_sequence(actions, results)
            }
        }
    }

    /// Continues with the alternative of `case` that matches `value`.
    fn select(case: &Rc<Case>, env: Rc<Env>, value: &Value) -> Result<Control, RuntimeError> {
        for alt in &case.alts {
            let matches = match (&alt.pat, value) {
                (AltPat::Con { con, fields }, Value::Data(found, values)) if con.id == found.id => {
                    for (&slot, field) in fields.iter().zip(values.iter()) {
                        env.set(slot, field.clone());
                    }
                    true
                }
                (AltPat::Lit(Literal::Int(n)), Value::Int(m)) => n == m,
                #[expect(clippy::float_cmp, reason = "literal patterns match exactly")]
                (AltPat::Lit(Literal::Float(n)), Value::Float(m)) => n == m,
                (AltPat::Lit(Literal::String(s)), Value::String(t)) => s == t,
                _ => false,
            };
            if matches {
                return Ok(Control::Eval(alt.body.clone(), env));
            }
        }
        match &case.default {
            Some(default) => Ok(Control::Eval(default.clone(), env)),
            None => Err(RuntimeError::new(format!(
                "internal error: no alternative matches `{}`",
                value.show()
            ))
            .at(case.span)),
        }
    }

    /// Evaluates the elements left in a walk, one at a time, then returns
    /// its root.
    fn walk(&mut self, mut walk: Walk) -> Result<Control, RuntimeError> {
        loop {
            let Some((items, index)) = walk.pending.last_mut() else {
                return Ok(Control::Return(walk.root));
            };
            let Some(item) = items.get(*index).cloned() else {
                walk.pending.pop();
                continue;
            };
            *index += 1;
            match item.value() {
                Some(value) if walk.deep => walk.enter(&value),
                Some(_) => {}
                None => {
                    self.stack.push(Frame::Walk(Box::new(walk)));
                    return self.enter(item);
                }
            }
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
        func: &Value,
        mut args: Vec<Thunk>,
        span: Span,
    ) -> Result<Control, RuntimeError> {
        match func {
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
                self.apply(&applied.func, all, span)
            }
            &Value::Builtin(builtin) => {
                let arity = builtin.arity();
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
        if builtin.is_action() {
            // Applying an action builtin only describes the action.
            let action = Action {
                builtin,
                args,
                span,
            };
            return Ok(Control::Return(Value::Io(Rc::new(action))));
        }
        let values = Vec::with_capacity(args.len());
        self.continue_builtin(BuiltinCall {
            builtin,
            args,
            values,
            span,
        })
    }

    /// Forces the builtin's strict arguments one at a time, as far as it
    /// needs them, then runs it.
    fn continue_builtin(&mut self, mut call: BuiltinCall) -> Result<Control, RuntimeError> {
        while call.values.len() < call.args.len() {
            let index = call.values.len();
            let thunk = call.args[index].clone();
            let deep = match prims::strictness(call.builtin, index) {
                Strictness::Lazy => {
                    call.values.push(Value::Unit);
                    continue;
                }
                Strictness::Shallow => {
                    if let Some(value) = thunk.value() {
                        call.values.push(value);
                        continue;
                    }
                    self.stack.push(Frame::Strict(call));
                    return self.enter(thunk);
                }
                Strictness::Elements => false,
                Strictness::Deep => true,
            };
            self.stack.push(Frame::Strict(call));
            self.stack.push(Frame::Force { deep });
            return self.enter(thunk);
        }
        self.run_builtin(call)
    }

    /// Runs a builtin whose strict arguments are all forced.
    fn run_builtin(&mut self, call: BuiltinCall) -> Result<Control, RuntimeError> {
        use Builtin as B;
        let BuiltinCall {
            builtin,
            mut args,
            values,
            span,
        } = call;
        match builtin {
            B::If => {
                let index = if prims::bool_of(&values[0])? { 1 } else { 2 };
                self.tail(args.swap_remove(index))
            }
            B::And => {
                if prims::bool_of(&values[0])? {
                    self.tail(args.swap_remove(1))
                } else {
                    Ok(Control::Return(Value::bool(false)))
                }
            }
            B::Or => {
                if prims::bool_of(&values[0])? {
                    Ok(Control::Return(Value::bool(true)))
                } else {
                    self.tail(args.swap_remove(1))
                }
            }
            B::Print | B::Println => {
                let mut text = prims::string_of(&values[0])?.to_string();
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
            B::WithDefault => match prims::option_of(&values[1])? {
                Some(value) => self.enter(value),
                None => self.tail(args.swap_remove(0)),
            },
            B::Unwrap => match prims::option_of(&values[0])? {
                Some(value) => self.enter(value),
                None => Err(prims::call_error(
                    builtin,
                    span,
                    "`unwrap` of `None`".into(),
                )),
            },
            // The component is entered, so it is evaluated only now.
            B::Fst | B::Snd => {
                let index = usize::from(builtin == B::Snd);
                self.enter(prims::tuple_of(&values[0])?[index].clone())
            }
            B::UnwrapOk => match prims::result_of(&values[0])? {
                Ok(value) => self.enter(value),
                Err(error) => {
                    self.stack.push(Frame::Fail { builtin, span });
                    self.stack.push(Frame::Force { deep: true });
                    self.enter(error)
                }
            },
            B::MapOption | B::MapResult => {
                let span = span.expect("higher-order builtins are applied at a call site");
                let mapped = match &values[1] {
                    // The new field is `(f x)`, evaluated only when used.
                    Value::Data(con, fields) if matches!(con.id, ConId::SOME | ConId::OK) => {
                        let env = Env::with(2, [args[0].clone(), fields[0].clone()]);
                        let call = call_of(Expr::Local(0), 1, 1, span);
                        Value::Data(con.clone(), Vector::unit(Thunk::delayed(call, env)))
                    }
                    other => other.clone(),
                };
                Ok(Control::Return(mapped))
            }
            B::Sequence => self.next_in_sequence(prims::list_of(&values[0])?, Vector::new()),
            B::Map => {
                let span = span.expect("higher-order builtins are applied at a call site");
                // Each element becomes `(f x)`, evaluated only when used.
                let call = call_of(Expr::Local(0), 1, 1, span);
                let results = prims::list_of(&values[1])?
                    .iter()
                    .map(|x| {
                        Thunk::delayed(call.clone(), Env::with(2, [args[0].clone(), x.clone()]))
                    })
                    .collect();
                Ok(Control::Return(Value::List(results)))
            }
            B::ZipWith => {
                let span = span.expect("higher-order builtins are applied at a call site");
                let call = call_of(Expr::Local(0), 1, 2, span);
                let others = prims::list_of(&values[2])?;
                let results = prims::list_of(&values[1])?
                    .iter()
                    .zip(others.iter())
                    .map(|(x, y)| {
                        let env = Env::with(3, [args[0].clone(), x.clone(), y.clone()]);
                        Thunk::delayed(call.clone(), env)
                    })
                    .collect();
                Ok(Control::Return(Value::List(results)))
            }
            B::Filter => {
                let iteration = Iteration {
                    kind: IterationKind::Filter,
                    func: values[0].clone(),
                    items: prims::list_of(&values[1])?,
                    index: 0,
                    results: Vector::new(),
                    span: span.expect("higher-order builtins are applied at a call site"),
                };
                self.next_call(iteration, None)
            }
            B::Foldl => {
                let items = prims::list_of(&values[2])?;
                let initial = args.swap_remove(1);
                if items.is_empty() {
                    return self.tail(initial);
                }
                let iteration = Iteration {
                    kind: IterationKind::Foldl,
                    func: values[0].clone(),
                    index: 0,
                    items,
                    results: Vector::new(),
                    span: span.expect("higher-order builtins are applied at a call site"),
                };
                self.next_call(iteration, Some(initial))
            }
            B::Foldr => {
                // `(foldr f z (cons x rest))` is `(f x (foldr f z rest))`, with
                // the inner fold suspended until `f` uses it.
                let mut items = prims::list_of(&values[2])?;
                let Some(first) = items.pop_front() else {
                    return self.tail(args.swap_remove(1));
                };
                let span = span.expect("higher-order builtins are applied at a call site");
                let env = Env::with(
                    3,
                    [
                        args[0].clone(),
                        args[1].clone(),
                        Thunk::done(Value::List(items)),
                    ],
                );
                let rest = Thunk::delayed(call_of(Expr::Builtin(B::Foldr), 0, 3, span), env);
                self.apply(&values[0], vec![first, rest], span)
            }
            _ => prims::pure(builtin, &values, &args, span).map(Control::Return),
        }
    }

    // ---- actions ----------------------------------------------------------
    //
    // Performing an action returns its result to the frame below. `bind` and
    // `then` push a frame for what comes next, then perform their first
    // action; the next action is performed in place of that frame, so a
    // recursive loop of actions runs in constant space.

    fn start_action(&mut self, action: Rc<Action>) -> Result<Control, RuntimeError> {
        // An action that nobody else holds gives up its arguments, so they
        // are not kept alive (and memoized) while the program runs on.
        let mut action = Rc::try_unwrap(action).unwrap_or_else(|shared| (*shared).clone());
        let mut args = std::mem::take(&mut action.args);
        let span = action.span;
        match action.builtin {
            Builtin::Pure => {
                let result = args.swap_remove(0);
                // Hand the result on unevaluated when something is waiting for it.
                match self.stack.pop() {
                    Some(Frame::Bind { func, span }) => self.bind_next(func, result, span),
                    Some(Frame::Then { next }) => self.then_next(next),
                    frame => {
                        self.stack.extend(frame);
                        self.tail(result)
                    }
                }
            }
            Builtin::Bind | Builtin::Then => {
                let second = args.pop().expect("`bind` and `then` take two arguments");
                let first = args.pop().expect("`bind` and `then` take two arguments");
                self.stack.push(if action.builtin == Builtin::Bind {
                    Frame::Bind {
                        func: second,
                        span: span.expect("`bind` is applied at a call site"),
                    }
                } else {
                    Frame::Then { next: second }
                });
                self.stack.push(Frame::Perform);
                self.tail(first)
            }
            builtin => self.continue_builtin(BuiltinCall {
                builtin,
                values: Vec::with_capacity(args.len()),
                args,
                span,
            }),
        }
    }

    /// Applies `bind`'s function to the result, then performs what it returns.
    fn bind_next(
        &mut self,
        func: Thunk,
        result: Thunk,
        span: Span,
    ) -> Result<Control, RuntimeError> {
        self.stack.push(Frame::Perform);
        if let Some(func) = func.value() {
            self.apply(&func, vec![result], span)
        } else {
            self.stack.push(Frame::Apply {
                args: vec![result],
                span,
            });
            self.tail(func)
        }
    }

    /// Performs the action after a `then`.
    fn then_next(&mut self, next: Thunk) -> Result<Control, RuntimeError> {
        self.stack.push(Frame::Perform);
        self.tail(next)
    }

    /// Performs the first of `actions`, or returns the results when none are left.
    fn next_in_sequence(
        &mut self,
        mut actions: Vector<Thunk>,
        results: Vector<Thunk>,
    ) -> Result<Control, RuntimeError> {
        match actions.pop_front() {
            None => Ok(Control::Return(Value::List(results))),
            Some(action) => {
                self.stack.push(Frame::Sequence { actions, results });
                self.stack.push(Frame::Perform);
                self.enter(action)
            }
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
        let args = match iteration.kind {
            IterationKind::Filter => {
                if iteration.index >= iteration.items.len() {
                    return Ok(Control::Return(Value::List(iteration.results)));
                }
                vec![iteration.items[iteration.index].clone()]
            }
            IterationKind::Foldl => vec![
                acc.expect("folds carry an accumulator"),
                iteration.items[iteration.index].clone(),
            ],
        };
        let (func, span) = (iteration.func.clone(), iteration.span);
        self.stack.push(Frame::Iterate(Box::new(iteration)));
        self.apply(&func, args, span)
    }

    /// Handles the result of one call made by a higher-order builtin.
    fn iterate(&mut self, mut iteration: Iteration, value: Value) -> Result<Control, RuntimeError> {
        let acc = match iteration.kind {
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
        };
        iteration.index += 1;
        self.next_call(iteration, acc)
    }
}

impl Backend for Interpreter {
    type Output = Vec<Value>;

    /// Defines the module's globals, then evaluates its top-level
    /// expressions in order, performing the ones that are actions. Returns
    /// each expression's value, or for an action, its result.
    fn run(&mut self, module: &Module) -> Result<Vec<Value>, RuntimeError> {
        self.define(module);
        let mut values = Vec::with_capacity(module.entries.len());
        for entry in &module.entries {
            let env = Env::new(entry.frame_size);
            let value = self
                .eval(entry.expr.clone(), env)
                .and_then(|value| self.perform(value));
            self.flush();
            values.push(value?);
        }
        Ok(values)
    }
}

fn partial(func: Value, args: Vec<Thunk>) -> Control {
    Control::Return(Value::Partial(Rc::new(Partial { func, args })))
}

/// `func` applied to `count` frame slots starting at `first`: the body of a
/// call the machine suspends, run in a frame holding the arguments.
fn call_of(func: Expr, first: u32, count: u32, span: Span) -> Rc<Expr> {
    Rc::new(Expr::App {
        func: Rc::new(func),
        args: (first..first + count)
            .map(|slot| Rc::new(Expr::Local(slot)))
            .collect(),
        span,
    })
}

fn literal_value(literal: &Literal) -> Value {
    match literal {
        Literal::Int(n) => Value::Int(*n),
        Literal::Float(n) => Value::Float(*n),
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
        Expr::App { span, .. } | Expr::List { span, .. } | Expr::Con { span, .. } => Some(*span),
        Expr::Case(case) => Some(case.span),
        Expr::Lambda(lambda) => Some(lambda.span),
        Expr::Let { body, .. } => expr_span(body),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
