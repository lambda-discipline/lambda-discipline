//! Runtime values, thunks and frames of the interpreter.
//!
//! Values can nest arbitrarily deeply at run time (a list built by polymorphic
//! recursion, a long chain of unevaluated thunks), so nothing here recurses on
//! the native stack: comparison and rendering keep explicit stacks, and
//! dropping goes through `defer_drop`.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

use imbl::Vector;

use crate::builtins::Builtin;
use crate::core::{Expr, Lambda};
use crate::source::Span;

#[derive(Clone)]
pub enum Value {
    Number(f64),
    String(Rc<str>),
    Bool(bool),
    Unit,
    /// Persistent vector: cheap to clone, `O(log n)` push, get and update.
    List(Vector<Value>),
    Closure(Rc<Closure>),
    /// A function applied to fewer arguments than it needs.
    Partial(Rc<Partial>),
    Builtin(Builtin),
    /// An `IO` action, not yet performed.
    Io(Rc<Action>),
}

pub struct Closure {
    pub lambda: Rc<Lambda>,
    pub captured: Box<[Thunk]>,
}

pub struct Partial {
    pub func: Value,
    pub args: Vec<Thunk>,
}

/// An action: an action builtin (`print`, `bind`, ...) applied to its
/// arguments, which stay unevaluated until the action is performed.
#[derive(Clone)]
pub struct Action {
    pub builtin: Builtin,
    pub args: Vec<Thunk>,
    pub span: Option<Span>,
}

impl Value {
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            Value::Closure(_) | Value::Partial(_) | Value::Builtin(_)
        )
    }

    /// The action, if this is one; otherwise the value itself.
    pub fn into_action(self) -> Result<Rc<Action>, Value> {
        match &self {
            Value::Io(action) => {
                let action = action.clone();
                drop(self);
                Ok(action)
            }
            _ => Err(self),
        }
    }

    /// Structural equality; `Err` when functions are compared.
    pub fn equals(&self, other: &Value) -> Result<bool, String> {
        // Pairs of lists whose elements are still being compared.
        let mut pending = Vec::new();
        let (mut a, mut b) = (self, other);
        loop {
            let equal = match (a, b) {
                (Value::Number(a), Value::Number(b)) => a == b,
                (Value::String(a), Value::String(b)) => a == b,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                (Value::Unit, Value::Unit) => true,
                (Value::List(a), Value::List(b)) => {
                    pending.push(a.iter().zip(b));
                    a.len() == b.len()
                }
                (Value::Io(_), _) | (_, Value::Io(_)) => {
                    return Err("actions cannot be compared".to_owned());
                }
                (a, b) if a.is_function() || b.is_function() => {
                    return Err("functions cannot be compared".to_owned());
                }
                _ => false,
            };
            if !equal {
                return Ok(false);
            }
            (a, b) = loop {
                let Some(pairs) = pending.last_mut() else {
                    return Ok(true);
                };
                match pairs.next() {
                    Some(pair) => break pair,
                    None => drop(pending.pop()),
                }
            };
        }
    }

    /// Total order on comparable values; `Err` for functions and `NaN`.
    pub fn compare(&self, other: &Value) -> Result<Ordering, String> {
        // Pairs of lists whose elements are still being compared, with the
        // order of their lengths for when all shared elements are equal.
        let mut pending = Vec::new();
        let (mut a, mut b) = (self, other);
        loop {
            let ordering = match (a, b) {
                (Value::Number(a), Value::Number(b)) => a
                    .partial_cmp(b)
                    .ok_or_else(|| "`NaN` cannot be ordered".to_owned())?,
                (Value::String(a), Value::String(b)) => a.cmp(b),
                (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
                (Value::Unit, Value::Unit) => Ordering::Equal,
                (Value::List(a), Value::List(b)) => {
                    pending.push((a.iter().zip(b), a.len().cmp(&b.len())));
                    Ordering::Equal
                }
                (Value::Io(_), _) | (_, Value::Io(_)) => {
                    return Err("actions cannot be compared".to_owned());
                }
                (a, b) if a.is_function() || b.is_function() => {
                    return Err("functions cannot be compared".to_owned());
                }
                _ => return Err("values of different types cannot be compared".to_owned()),
            };
            if ordering.is_ne() {
                return Ok(ordering);
            }
            (a, b) = loop {
                let Some((pairs, lengths)) = pending.last_mut() else {
                    return Ok(Ordering::Equal);
                };
                match pairs.next() {
                    Some(pair) => break pair,
                    None if lengths.is_ne() => return Ok(*lengths),
                    None => drop(pending.pop()),
                }
            };
        }
    }

    /// How `print` shows a value: strings without quotes.
    pub fn display(&self) -> String {
        match self {
            Value::String(s) => s.to_string(),
            other => other.show(),
        }
    }

    /// Source-like rendering used by `show` and the REPL: strings quoted.
    pub fn show(&self) -> String {
        let mut out = String::new();
        self.write_shown(&mut out);
        out
    }

    fn write_shown(&self, out: &mut String) {
        // Lists being rendered, innermost last: their remaining elements and
        // whether one has been written yet.
        let mut open = Vec::new();
        let mut value = self;
        loop {
            match value {
                Value::Number(n) => out.push_str(&format_number(*n)),
                Value::String(s) => out.push_str(&format!("{:?}", s.as_ref())),
                Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
                Value::Unit => out.push_str("()"),
                Value::List(items) => {
                    out.push('[');
                    open.push((items.iter(), false));
                }
                Value::Builtin(b) => out.push_str(&format!("<builtin {b}>")),
                Value::Closure(_) | Value::Partial(_) => out.push_str("<function>"),
                Value::Io(_) => out.push_str("<action>"),
            }
            value = loop {
                let Some((items, started)) = open.last_mut() else {
                    return;
                };
                match items.next() {
                    Some(item) => {
                        if *started {
                            out.push_str(", ");
                        }
                        *started = true;
                        break item;
                    }
                    None => {
                        out.push(']');
                        open.pop();
                    }
                }
            };
        }
    }
}

// ---- dropping ----------------------------------------------------------------
//
// Dropping a value drops what it owns, which can be another value of the same
// kind, and so on to any depth. The `Drop` impls below move the owned parts
// out and hand them to `defer_drop`, which drops them in a loop instead.

/// The owned parts of a value, waiting to be dropped.
#[allow(dead_code, reason = "the fields are only ever dropped")]
enum Garbage {
    List(Vector<Value>),
    Value(Value),
    Thunks(Vec<Thunk>),
    Captured(Box<[Thunk]>),
    Slots(Box<[RefCell<Option<Thunk>>]>),
}

thread_local! {
    static DRAINING: Cell<bool> = const { Cell::new(false) };
    static GARBAGE: RefCell<Vec<Garbage>> = const { RefCell::new(Vec::new()) };
}

/// Drops `garbage` without recursing: drops that happen meanwhile only queue
/// their parts, and the outermost call empties the queue in a loop.
fn defer_drop(garbage: Garbage) {
    if DRAINING.try_with(Cell::get).unwrap_or(true) {
        // During thread teardown the queue may be gone; the garbage is then
        // dropped in place, with the closure.
        let _ = GARBAGE.try_with(|queue| queue.borrow_mut().push(garbage));
        return;
    }
    DRAINING.set(true);
    drop(garbage);
    while let Some(garbage) = GARBAGE.with(|queue| queue.borrow_mut().pop()) {
        drop(garbage);
    }
    DRAINING.set(false);
}

impl Drop for Value {
    fn drop(&mut self) {
        match self {
            Value::List(items) if !items.is_empty() => {
                defer_drop(Garbage::List(std::mem::take(items)));
            }
            _ => {}
        }
    }
}

impl Drop for Closure {
    fn drop(&mut self) {
        if !self.captured.is_empty() {
            defer_drop(Garbage::Captured(std::mem::take(&mut self.captured)));
        }
    }
}

impl Drop for Partial {
    fn drop(&mut self) {
        defer_drop(Garbage::Value(std::mem::replace(
            &mut self.func,
            Value::Unit,
        )));
        defer_drop(Garbage::Thunks(std::mem::take(&mut self.args)));
    }
}

impl Drop for Action {
    fn drop(&mut self) {
        if !self.args.is_empty() {
            defer_drop(Garbage::Thunks(std::mem::take(&mut self.args)));
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        if !self.slots.is_empty() {
            defer_drop(Garbage::Slots(std::mem::take(&mut self.slots)));
        }
    }
}

/// Numbers print the way most languages print them: `42`, `3.14`, `0.1`.
pub fn format_number(n: f64) -> String {
    if n.is_infinite() {
        return if n > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    format!("{n}")
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.show())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}

/// A frame of slots: captured variables, then parameters, then `let`s.
pub struct Env {
    slots: Box<[RefCell<Option<Thunk>>]>,
}

impl Env {
    pub fn new(size: u32) -> Rc<Env> {
        Rc::new(Env {
            slots: (0..size).map(|_| RefCell::new(None)).collect(),
        })
    }

    pub fn with(size: u32, initial: impl IntoIterator<Item = Thunk>) -> Rc<Env> {
        let env = Env::new(size);
        for (slot, thunk) in initial.into_iter().enumerate() {
            env.set(slot as u32, thunk);
        }
        env
    }

    pub fn get(&self, slot: u32) -> Thunk {
        self.slots[slot as usize]
            .borrow()
            .clone()
            .expect("slots are initialized before use")
    }

    pub fn set(&self, slot: u32, thunk: Thunk) {
        *self.slots[slot as usize].borrow_mut() = Some(thunk);
    }
}

pub enum ThunkState {
    /// Not yet evaluated.
    Delayed {
        expr: Rc<Expr>,
        env: Rc<Env>,
    },
    /// Being evaluated; forcing it again means the value depends on itself.
    Forcing {
        expr: Rc<Expr>,
        env: Rc<Env>,
    },
    Done(Value),
}

/// A shared, memoized suspended computation (call-by-need).
#[derive(Clone)]
pub struct Thunk(pub Rc<RefCell<ThunkState>>);

impl Thunk {
    pub fn delayed(expr: Rc<Expr>, env: Rc<Env>) -> Thunk {
        Thunk(Rc::new(RefCell::new(ThunkState::Delayed { expr, env })))
    }

    pub fn done(value: Value) -> Thunk {
        Thunk(Rc::new(RefCell::new(ThunkState::Done(value))))
    }

    /// The value, if already computed.
    pub fn value(&self) -> Option<Value> {
        match &*self.0.borrow() {
            ThunkState::Done(value) => Some(value.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(items: Vec<Value>) -> Value {
        Value::List(items.into_iter().collect())
    }

    #[test]
    fn numbers_print_like_other_languages() {
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(2.25), "2.25");
        assert_eq!(format_number(-0.5), "-0.5");
        assert_eq!(format_number(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(format_number(f64::INFINITY), "Infinity");
        assert_eq!(format_number(f64::NAN), "NaN");
    }

    #[test]
    fn display_and_show() {
        let value = list(vec![Value::String("a\"b".into()), Value::Number(1.0)]);
        assert_eq!(value.show(), r#"["a\"b", 1]"#);
        assert_eq!(Value::String("hi\n".into()).display(), "hi\n");
        assert_eq!(Value::String("hi\n".into()).show(), r#""hi\n""#);
        assert_eq!(Value::Bool(true).show(), "true");
        assert_eq!(Value::Unit.show(), "()");
        assert_eq!(Value::Builtin(Builtin::Add).show(), "<builtin add>");
    }

    #[test]
    fn structural_equality() {
        assert!(
            list(vec![Value::Number(1.0)])
                .equals(&list(vec![Value::Number(1.0)]))
                .unwrap()
        );
        assert!(
            !list(vec![Value::Number(1.0)])
                .equals(&list(vec![]))
                .unwrap()
        );
        assert!(
            !Value::Number(f64::NAN)
                .equals(&Value::Number(f64::NAN))
                .unwrap()
        );
        assert!(
            Value::Builtin(Builtin::Add)
                .equals(&Value::Builtin(Builtin::Add))
                .is_err()
        );
    }

    #[test]
    fn ordering() {
        use Ordering::*;
        let n = Value::Number;
        assert_eq!(n(1.0).compare(&n(2.0)), Ok(Less));
        assert_eq!(
            Value::String("b".into()).compare(&Value::String("a".into())),
            Ok(Greater)
        );
        assert_eq!(
            list(vec![n(1.0)]).compare(&list(vec![n(1.0), n(0.0)])),
            Ok(Less)
        );
        assert_eq!(
            list(vec![n(2.0)]).compare(&list(vec![n(1.0), n(0.0)])),
            Ok(Greater)
        );
        assert!(n(f64::NAN).compare(&n(1.0)).is_err());
    }

    /// `depth` lists, each holding the next, around `[1]`.
    fn nested_list(depth: usize, innermost: f64) -> Value {
        let mut value = list(vec![Value::Number(innermost)]);
        for _ in 0..depth {
            value = list(vec![value]);
        }
        value
    }

    // These run on the test harness's small stack: deep values must not use it.
    const DEEP: usize = 100_000;

    #[test]
    fn deep_lists_compare_show_and_drop_without_overflow() {
        let a = nested_list(DEEP, 1.0);
        let b = nested_list(DEEP, 2.0);
        assert_eq!(a.equals(&a.clone()), Ok(true));
        assert_eq!(a.equals(&b), Ok(false));
        assert_eq!(a.compare(&b), Ok(Ordering::Less));
        assert_eq!(b.compare(&a), Ok(Ordering::Greater));
        let expected = format!("{}[1]{}", "[".repeat(DEEP), "]".repeat(DEEP));
        assert_eq!(a.show(), expected);
    }

    #[test]
    fn nested_lists_keep_their_rendering_and_order() {
        let n = Value::Number;
        let value = list(vec![list(vec![n(1.0), list(vec![])]), n(2.0), list(vec![])]);
        assert_eq!(value.show(), "[[1, []], 2, []]");
        assert_eq!(
            list(vec![list(vec![n(1.0)]), n(2.0)])
                .compare(&list(vec![list(vec![n(1.0), n(0.0)]), n(0.0)])),
            Ok(Ordering::Less)
        );
        assert_eq!(
            list(vec![list(vec![n(1.0)]), n(3.0)]).compare(&list(vec![list(vec![n(1.0)]), n(2.0)])),
            Ok(Ordering::Greater)
        );
        assert!(
            list(vec![list(vec![Value::Builtin(Builtin::Add)])])
                .equals(&list(vec![list(vec![Value::Builtin(Builtin::Add)])]))
                .is_err()
        );
    }

    #[test]
    fn deep_thunk_and_partial_chains_drop_without_overflow() {
        let expr = Rc::new(Expr::Lit(crate::core::Literal::Unit));
        let mut thunk = Thunk::done(Value::Unit);
        for _ in 0..DEEP {
            thunk = Thunk::delayed(expr.clone(), Env::with(1, [thunk]));
        }
        drop(thunk);

        let mut value = Value::Builtin(Builtin::Add);
        for _ in 0..DEEP {
            value = Value::Partial(Rc::new(Partial {
                func: value,
                args: Vec::new(),
            }));
        }
        drop(value);
    }

    #[test]
    fn env_slots() {
        let env = Env::with(3, [Thunk::done(Value::Unit)]);
        env.set(2, Thunk::done(Value::Number(5.0)));
        assert_eq!(env.get(2).value().unwrap().show(), "5");
        assert_eq!(env.get(0).value().unwrap().show(), "()");
    }
}
