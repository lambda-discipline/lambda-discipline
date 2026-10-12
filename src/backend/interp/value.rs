//! Runtime values, thunks and frames of the interpreter.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

use imbl::Vector;

use crate::builtins::Builtin;
use crate::core::{Expr, Lambda};

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
}

pub struct Closure {
    pub lambda: Rc<Lambda>,
    pub captured: Box<[Thunk]>,
}

pub struct Partial {
    pub func: Value,
    pub args: Vec<Thunk>,
}

impl Value {
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            Value::Closure(_) | Value::Partial(_) | Value::Builtin(_)
        )
    }

    /// Structural equality; `Err` when functions are compared.
    pub fn equals(&self, other: &Value) -> Result<bool, String> {
        Ok(match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::List(a), Value::List(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (x, y) in a.iter().zip(b) {
                    if !x.equals(y)? {
                        return Ok(false);
                    }
                }
                true
            }
            (a, b) if a.is_function() || b.is_function() => {
                return Err("functions cannot be compared".to_owned());
            }
            _ => false,
        })
    }

    /// Total order on comparable values; `Err` for functions and `NaN`.
    pub fn compare(&self, other: &Value) -> Result<Ordering, String> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a
                .partial_cmp(b)
                .ok_or_else(|| "`NaN` cannot be ordered".to_owned()),
            (Value::String(a), Value::String(b)) => Ok(a.cmp(b)),
            (Value::Bool(a), Value::Bool(b)) => Ok(a.cmp(b)),
            (Value::Unit, Value::Unit) => Ok(Ordering::Equal),
            (Value::List(a), Value::List(b)) => {
                for (x, y) in a.iter().zip(b) {
                    match x.compare(y)? {
                        Ordering::Equal => {}
                        other => return Ok(other),
                    }
                }
                Ok(a.len().cmp(&b.len()))
            }
            (a, b) if a.is_function() || b.is_function() => {
                Err("functions cannot be compared".to_owned())
            }
            _ => Err("values of different types cannot be compared".to_owned()),
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
        match self {
            Value::Number(n) => out.push_str(&format_number(*n)),
            Value::String(s) => out.push_str(&format!("{:?}", s.as_ref())),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Unit => out.push_str("()"),
            Value::List(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    item.write_shown(out);
                }
                out.push(']');
            }
            Value::Builtin(b) => out.push_str(&format!("<builtin {b}>")),
            Value::Closure(_) | Value::Partial(_) => out.push_str("<function>"),
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

    #[test]
    fn env_slots() {
        let env = Env::with(3, [Thunk::done(Value::Unit)]);
        env.set(2, Thunk::done(Value::Number(5.0)));
        assert_eq!(env.get(2).value().unwrap().show(), "5");
        assert_eq!(env.get(0).value().unwrap().show(), "()");
    }
}
