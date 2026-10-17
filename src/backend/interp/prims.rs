//! First-order builtins: pure functions from forced argument values to a
//! result. Builtins that control evaluation (`if`, folds, `print`, ...) are
//! implemented by the machine in the parent module.

use std::rc::Rc;

use imbl::Vector;

use crate::backend::RuntimeError;
use crate::builtins::Builtin;
use crate::source::Span;
use crate::syntax::number::format_number;

use super::{Thunk, Value};

type Result<T> = std::result::Result<T, RuntimeError>;

/// How much of an argument is evaluated before the builtin runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Strictness {
    /// Passed on unevaluated.
    Lazy,
    /// Evaluated to its outermost form: a list's spine, not its elements.
    Shallow,
    /// A list whose elements are evaluated too, but not the elements of the
    /// lists inside them.
    Elements,
    /// Evaluated completely, as comparing or showing a value needs.
    Deep,
}

/// How much of argument `index` is evaluated before the builtin runs.
pub(super) fn strictness(builtin: Builtin, index: usize) -> Strictness {
    use Builtin as B;
    use Strictness::*;
    match builtin {
        B::If | B::And | B::Or if index > 0 => Lazy,
        // The accumulator is passed on to the folding function unevaluated.
        B::Foldl | B::Foldr if index == 1 => Lazy,
        // The new element is stored unevaluated, and so are the calls of
        // the mapped function.
        B::Cons | B::Push | B::Map | B::ZipWith if index == 0 => Lazy,
        B::Eq | B::Neq | B::Lt | B::Le | B::Gt | B::Ge | B::Compare | B::Sort => Deep,
        B::Show | B::Print | B::Println => Deep,
        B::Join | B::Sum | B::Transpose => Elements,
        _ => Shallow,
    }
}

/// The error for a failed call to `builtin`, pointing at the call if known.
pub(super) fn call_error(builtin: Builtin, span: Option<Span>, message: String) -> RuntimeError {
    let error = RuntimeError::new(message);
    match span {
        Some(span) => error
            .at(span)
            .with_label(format!("in this call to `{builtin}`")),
        None => error,
    }
}

/// The element `head` or `get` selects, still unevaluated.
pub(super) fn element(builtin: Builtin, args: &[Value], span: Option<Span>) -> Result<Thunk> {
    let fail = |message: String| call_error(builtin, span, message);
    match builtin {
        Builtin::Head => list_of(&args[0])?
            .front()
            .cloned()
            .ok_or_else(|| fail("`head` of an empty list".into())),
        Builtin::Get => {
            let i = to_index(number_of(&args[0])?).map_err(fail)?;
            let items = list_of(&args[1])?;
            items
                .get(i)
                .cloned()
                .ok_or_else(|| fail(out_of_bounds(i, items.len())))
        }
        _ => unreachable!("`{builtin}` does not select an element"),
    }
}

/// Runs a first-order builtin. `args` holds the evaluated arguments, with
/// `Unit` in place of lazy ones, which are only in `thunks`.
pub(super) fn pure(
    builtin: Builtin,
    args: &[Value],
    thunks: &[Thunk],
    span: Option<Span>,
) -> Result<Value> {
    let fail = |message: String| call_error(builtin, span, message);
    let num = |i: usize| number_of(&args[i]);
    let text = |i: usize| string_of(&args[i]);
    let list = |i: usize| list_of(&args[i]);

    use Builtin as B;
    Ok(match builtin {
        B::Add => Value::Number(num(0)? + num(1)?),
        B::Sub => Value::Number(num(0)? - num(1)?),
        B::Mul => Value::Number(num(0)? * num(1)?),
        B::Div => Value::Number(num(0)? / num(1)?),
        B::Mod => {
            let (a, b) = (num(0)?, num(1)?);
            Value::Number(a - b * (a / b).floor())
        }
        B::Pow => Value::Number(num(0)?.powf(num(1)?)),
        B::Neg => Value::Number(-num(0)?),
        B::Abs => Value::Number(num(0)?.abs()),
        B::Floor => Value::Number(num(0)?.floor()),
        B::Ceil => Value::Number(num(0)?.ceil()),
        B::Round => Value::Number(num(0)?.round()),
        B::Sqrt => Value::Number(num(0)?.sqrt()),
        B::Min => Value::Number(num(0)?.min(num(1)?)),
        B::Max => Value::Number(num(0)?.max(num(1)?)),

        B::Eq | B::Neq => {
            let equal = args[0].equals(&args[1]).map_err(fail)?;
            Value::bool(equal == (builtin == B::Eq))
        }
        B::Lt | B::Le | B::Gt | B::Ge | B::Compare => {
            let ordering = args[0].compare(&args[1]).map_err(fail)?;
            match builtin {
                B::Lt => Value::bool(ordering.is_lt()),
                B::Le => Value::bool(ordering.is_le()),
                B::Gt => Value::bool(ordering.is_gt()),
                B::Ge => Value::bool(ordering.is_ge()),
                _ => Value::Number(ordering as i8 as f64),
            }
        }

        B::Not => Value::bool(!bool_of(&args[0])?),

        B::Concat => Value::String(Rc::from(format!("{}{}", text(0)?, text(1)?))),
        B::Show => Value::String(Rc::from(args[0].show())),
        B::ParseNumber => {
            let input = text(0)?;
            let number = input
                .trim()
                .parse::<f64>()
                .map_err(|_| fail(format!("cannot parse {:?} as a number", input.as_ref())))?;
            Value::Number(number)
        }
        B::StringLength => Value::Number(text(0)?.chars().count() as f64),
        B::Chars => strings(text(0)?.chars().map(String::from)),
        B::Split => {
            let delimiter = text(0)?;
            if delimiter.is_empty() {
                return Err(fail(
                    "the delimiter given to `split` must not be empty".into(),
                ));
            }
            strings(text(1)?.split(delimiter.as_ref()).map(str::to_owned))
        }
        B::Lines => strings(text(0)?.lines().map(str::to_owned)),
        B::Join => {
            let separator = text(0)?;
            let parts: Vec<String> = list(1)?
                .iter()
                .map(|part| element_of(part).map(|part| part.display()))
                .collect::<Result<_>>()?;
            Value::String(Rc::from(parts.join(&separator)))
        }
        B::Trim => Value::String(Rc::from(text(0)?.trim())),

        B::Length => Value::Number(list(0)?.len() as f64),
        B::IsEmpty => Value::bool(list(0)?.is_empty()),
        B::Tail => {
            let items = list(0)?;
            if items.is_empty() {
                return Err(fail("`tail` of an empty list".into()));
            }
            Value::List(items.skip(1))
        }
        B::Cons => {
            let mut items = list(1)?;
            items.push_front(thunks[0].clone());
            Value::List(items)
        }
        B::Push => {
            let mut items = list(1)?;
            items.push_back(thunks[0].clone());
            Value::List(items)
        }
        B::Remove => {
            let (i, mut items) = (to_index(num(0)?).map_err(fail)?, list(1)?);
            if i >= items.len() {
                return Err(fail(out_of_bounds(i, items.len())));
            }
            items.remove(i);
            Value::List(items)
        }
        B::Append => {
            let mut items = list(0)?;
            items.append(list(1)?);
            Value::List(items)
        }
        B::Reverse => Value::List(list(0)?.into_iter().rev().collect()),
        B::Sort => {
            let mut items: Vec<(Value, Thunk)> = list(0)?
                .into_iter()
                .map(|item| Ok((element_of(&item)?, item)))
                .collect::<Result<_>>()?;
            let mut error = None;
            items.sort_by(|(a, _), (b, _)| {
                a.compare(b).unwrap_or_else(|e| {
                    error.get_or_insert(e);
                    std::cmp::Ordering::Equal
                })
            });
            if let Some(error) = error {
                return Err(fail(error));
            }
            Value::List(items.into_iter().map(|(_, item)| item).collect())
        }
        B::Range => {
            let (from, to) = (num(0)?, num(1)?);
            let count = (to - from).ceil().max(0.0);
            if count > 1e8 {
                return Err(fail(format!(
                    "`range` would produce {} elements",
                    format_number(count)
                )));
            }
            Value::List(
                (0..count as usize)
                    .map(|i| Thunk::done(Value::Number(from + i as f64)))
                    .collect(),
            )
        }
        B::Transpose => {
            let rows: Vec<Vector<Thunk>> = list(0)?
                .iter()
                .map(|row| list_of(&element_of(row)?))
                .collect::<Result<_>>()?;
            let width = rows.iter().map(Vector::len).min().unwrap_or(0);
            Value::List(
                (0..width)
                    .map(|column| {
                        let column = rows.iter().map(|row| row[column].clone()).collect();
                        Thunk::done(Value::List(column))
                    })
                    .collect(),
            )
        }
        B::Sum => {
            let mut total = 0.0;
            for item in list(0)?.iter() {
                total += number_of(&element_of(item)?)?;
            }
            Value::Number(total)
        }
        B::Take => {
            let (n, items) = (to_index(num(0)?).map_err(fail)?, list(1)?);
            Value::List(items.take(n.min(items.len())))
        }
        B::Drop => {
            let (n, items) = (to_index(num(0)?).map_err(fail)?, list(1)?);
            Value::List(items.skip(n.min(items.len())))
        }

        B::ReadFile => {
            let path = text(0)?;
            let contents = std::fs::read_to_string(path.as_ref())
                .map_err(|e| fail(format!("cannot read {:?}: {e}", path.as_ref())))?;
            Value::String(Rc::from(contents))
        }
        B::Error => {
            let error = RuntimeError::new(text(0)?.to_string());
            return Err(match span {
                Some(span) => error.at(span).with_label("raised here"),
                None => error,
            });
        }

        B::Head
        | B::Get
        | B::If
        | B::And
        | B::Or
        | B::Pure
        | B::Bind
        | B::Then
        | B::Sequence
        | B::Print
        | B::Println
        | B::Map
        | B::Filter
        | B::Foldl
        | B::Foldr
        | B::ZipWith => {
            unreachable!("`{builtin}` is implemented by the machine")
        }
        B::File | B::Dir | B::Line => unreachable!("`{builtin}` is lowered to a literal"),
    })
}

fn strings(items: impl Iterator<Item = String>) -> Value {
    Value::List(
        items
            .map(|s| Thunk::done(Value::String(Rc::from(s))))
            .collect(),
    )
}

fn out_of_bounds(index: usize, len: usize) -> String {
    format!("index {index} is out of bounds for a list of length {len}")
}

/// A list index must be a whole, non-negative number.
fn to_index(n: f64) -> std::result::Result<usize, String> {
    if n.fract() == 0.0 && n >= 0.0 && n <= usize::MAX as f64 {
        Ok(n as usize)
    } else {
        Err(format!(
            "{} is not a valid index (expected a whole number >= 0)",
            format_number(n)
        ))
    }
}

pub(super) fn number_of(value: &Value) -> Result<f64> {
    match value {
        Value::Number(n) => Ok(*n),
        other => Err(internal("Number", other)),
    }
}

pub(super) fn bool_of(value: &Value) -> Result<bool> {
    value.as_bool().ok_or_else(|| internal("Bool", value))
}

pub(super) fn string_of(value: &Value) -> Result<Rc<str>> {
    match value {
        Value::String(s) => Ok(s.clone()),
        other => Err(internal("String", other)),
    }
}

pub(super) fn list_of(value: &Value) -> Result<Vector<Thunk>> {
    match value {
        Value::List(items) => Ok(items.clone()),
        other => Err(internal("List", other)),
    }
}

/// The value of a list element the machine has already evaluated.
fn element_of(thunk: &Thunk) -> Result<Value> {
    thunk
        .value()
        .ok_or_else(|| RuntimeError::new("internal error: a list element was not evaluated"))
}

/// Type-checked programs never hit this; it guards hand-built Core IR.
fn internal(expected: &str, found: &Value) -> RuntimeError {
    RuntimeError::new(format!(
        "internal error: expected a `{expected}`, found `{}`",
        found.show()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_must_be_whole_and_non_negative() {
        assert_eq!(to_index(3.0), Ok(3));
        assert!(to_index(-1.0).is_err());
        assert!(to_index(1.5).is_err());
        assert!(to_index(f64::NAN).is_err());
    }

    #[test]
    fn strictness_of_arguments() {
        use Strictness::*;
        assert_eq!(strictness(Builtin::If, 0), Shallow);
        assert_eq!(strictness(Builtin::If, 1), Lazy);
        assert_eq!(strictness(Builtin::Or, 1), Lazy);
        assert_eq!(strictness(Builtin::Foldr, 1), Lazy);
        assert_eq!(strictness(Builtin::Foldr, 2), Shallow);
        assert_eq!(strictness(Builtin::Cons, 0), Lazy);
        assert_eq!(strictness(Builtin::Head, 0), Shallow);
        assert_eq!(strictness(Builtin::Sum, 0), Elements);
        assert_eq!(strictness(Builtin::Eq, 1), Deep);
        assert_eq!(strictness(Builtin::Add, 1), Shallow);
    }

    #[test]
    fn pure_builtins_on_values() {
        let n = Value::Number;
        assert_eq!(
            pure(Builtin::Add, &[n(1.0), n(2.0)], &[], None)
                .unwrap()
                .show(),
            "3"
        );
        assert_eq!(
            pure(Builtin::Not, &[Value::bool(false)], &[], None)
                .unwrap()
                .show(),
            "True"
        );
        let error = element(Builtin::Head, &[Value::List(Vector::new())], None).unwrap_err();
        assert_eq!(error.message, "`head` of an empty list");
        let error = pure(Builtin::Add, &[Value::Unit, n(1.0)], &[], None).unwrap_err();
        assert!(error.message.starts_with("internal error"));
    }
}
