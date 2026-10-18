//! First-order builtins: pure functions from forced argument values to a
//! result. Builtins that control evaluation (`if`, folds, `print`, ...) are
//! implemented by the machine in the parent module.

use std::rc::Rc;

use imbl::Vector;

use crate::backend::RuntimeError;
use crate::builtins::Builtin;
use crate::source::Span;
use crate::syntax::number::format_float;

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
        B::Min | B::Max => Deep,
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
            let i = to_index(int_of(&args[0])?).map_err(fail)?;
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
    let int = |i: usize| int_of(&args[i]);
    let float = |i: usize| float_of(&args[i]);
    let index = |i: usize| int_of(&args[i]).and_then(|n| to_index(n).map_err(fail));
    let text = |i: usize| string_of(&args[i]);
    let list = |i: usize| list_of(&args[i]);

    use Builtin as B;
    Ok(match builtin {
        B::Add | B::Sub | B::Mul => match (&args[0], &args[1]) {
            (Value::Int(a), Value::Int(b)) => {
                let result = match builtin {
                    B::Add => a.checked_add(*b),
                    B::Sub => a.checked_sub(*b),
                    _ => a.checked_mul(*b),
                };
                Value::Int(result.ok_or_else(|| fail(overflow(builtin, &[*a, *b])))?)
            }
            (Value::Float(a), Value::Float(b)) => Value::Float(match builtin {
                B::Add => a + b,
                B::Sub => a - b,
                _ => a * b,
            }),
            (a, b) => return Err(mixed(a, b)),
        },
        B::Neg | B::Abs => match &args[0] {
            Value::Int(n) => {
                let result = match builtin {
                    B::Neg => n.checked_neg(),
                    _ => n.checked_abs(),
                };
                Value::Int(result.ok_or_else(|| fail(overflow(builtin, &[*n])))?)
            }
            Value::Float(n) => Value::Float(if builtin == B::Neg { -n } else { n.abs() }),
            other => return Err(internal("number", other)),
        },
        B::Pow => {
            let exponent = int(1)?;
            if exponent < 0 {
                return Err(fail(format!(
                    "the exponent given to `pow` must not be negative, but it is {exponent}"
                )));
            }
            match &args[0] {
                Value::Int(base) => Value::Int(
                    int_pow(*base, exponent)
                        .ok_or_else(|| fail(overflow(builtin, &[*base, exponent])))?,
                ),
                Value::Float(base) => Value::Float(
                    i32::try_from(exponent).map_or(base.powf(exponent as f64), |e| base.powi(e)),
                ),
                other => return Err(internal("number", other)),
            }
        }
        B::Div | B::Mod => {
            let (a, b) = (int(0)?, int(1)?);
            if b == 0 {
                return Err(fail("division by zero".into()));
            }
            // Rust's `/` and `%` round toward zero; Haskell's `div` and `mod`
            // round toward negative infinity, so the remainder takes the
            // sign of the divisor. Only `(div i64::MIN -1)` overflows.
            let remainder = a.checked_rem(b).unwrap_or(0);
            let floored = remainder != 0 && (remainder < 0) != (b < 0);
            Value::Int(match builtin {
                B::Div => {
                    let quotient = a
                        .checked_div(b)
                        .ok_or_else(|| fail(overflow(builtin, &[a, b])))?;
                    if floored { quotient - 1 } else { quotient }
                }
                _ if floored => remainder + b,
                _ => remainder,
            })
        }
        B::Fdiv => Value::Float(float(0)? / float(1)?),
        B::Sqrt => Value::Float(float(0)?.sqrt()),
        B::ToFloat => Value::Float(int(0)? as f64),
        B::Floor | B::Ceil | B::Round => {
            let n = float(0)?;
            let whole = match builtin {
                B::Floor => n.floor(),
                B::Ceil => n.ceil(),
                _ => n.round(),
            };
            // `i64::MAX as f64` rounds up to 2^63, which is out of range.
            if !(-(2f64.powi(63))..2f64.powi(63)).contains(&whole) {
                return Err(fail(format!(
                    "{} cannot be converted to an `Int`",
                    format_float(n)
                )));
            }
            Value::Int(whole as i64)
        }
        B::Min | B::Max => {
            let ordering = args[0].compare(&args[1]).map_err(fail)?;
            let first = match builtin {
                B::Min => ordering.is_le(),
                _ => ordering.is_gt(),
            };
            args[if first { 0 } else { 1 }].clone()
        }

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
                _ => Value::Int(ordering as i64),
            }
        }

        B::Not => Value::bool(!bool_of(&args[0])?),

        B::Concat => Value::String(Rc::from(format!("{}{}", text(0)?, text(1)?))),
        B::Show => Value::String(Rc::from(args[0].show())),
        B::ParseInt => {
            let input = text(0)?;
            let number = input
                .trim()
                .parse::<i64>()
                .map_err(|_| fail(format!("cannot parse {:?} as an `Int`", input.as_ref())))?;
            Value::Int(number)
        }
        B::ParseFloat => {
            let input = text(0)?;
            let number = parse_float(input.trim())
                .ok_or_else(|| fail(format!("cannot parse {:?} as a `Float`", input.as_ref())))?;
            Value::Float(number)
        }
        B::StringLength => Value::Int(text(0)?.chars().count() as i64),
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

        B::Length => Value::Int(list(0)?.len() as i64),
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
            let (i, mut items) = (index(0)?, list(1)?);
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
            let (from, to) = (int(0)?, int(1)?);
            let count = (i128::from(to) - i128::from(from)).max(0);
            if count > 100_000_000 {
                return Err(fail(format!("`range` would produce {count} elements")));
            }
            Value::List((from..to).map(|i| Thunk::done(Value::Int(i))).collect())
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
            let mut total: i64 = 0;
            for item in list(0)?.iter() {
                total = total
                    .checked_add(int_of(&element_of(item)?)?)
                    .ok_or_else(|| {
                        fail("integer overflow in `sum`: the total does not fit in an `Int`".into())
                    })?;
            }
            Value::Int(total)
        }
        B::Take => {
            let (n, items) = (index(0)?, list(1)?);
            Value::List(items.take(n.min(items.len())))
        }
        B::Drop => {
            let (n, items) = (index(0)?, list(1)?);
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

/// A list index or count must not be negative.
fn to_index(n: i64) -> std::result::Result<usize, String> {
    usize::try_from(n).map_err(|_| format!("{n} is not a valid index (expected a number >= 0)"))
}

/// `base` to the power of a non-negative `exponent`, by repeated squaring;
/// `None` on overflow.
fn int_pow(mut base: i64, mut exponent: i64) -> Option<i64> {
    let mut result: i64 = 1;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = result.checked_mul(base)?;
        }
        exponent >>= 1;
        if exponent > 0 {
            base = base.checked_mul(base)?;
        }
    }
    Some(result)
}

/// The error for `Int` arithmetic whose result does not fit in 64 bits.
fn overflow(builtin: Builtin, args: &[i64]) -> String {
    let args: Vec<String> = args.iter().map(i64::to_string).collect();
    format!(
        "integer overflow: `({builtin} {})` does not fit in an `Int`",
        args.join(" ")
    )
}

/// Parses a float literal or an integer, as `parseFloat` accepts them;
/// Rust's parser would also take `inf` and `NaN`.
fn parse_float(text: &str) -> Option<f64> {
    let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
    if digits.starts_with(|c: char| c.is_ascii_digit()) || digits.starts_with('.') {
        text.parse().ok()
    } else {
        None
    }
}

pub(super) fn int_of(value: &Value) -> Result<i64> {
    match value {
        Value::Int(n) => Ok(*n),
        other => Err(internal("Int", other)),
    }
}

pub(super) fn float_of(value: &Value) -> Result<f64> {
    match value {
        Value::Float(n) => Ok(*n),
        other => Err(internal("Float", other)),
    }
}

/// `Int` and `Float` arguments to one `Num` builtin; the type checker
/// rules this out.
fn mixed(a: &Value, b: &Value) -> RuntimeError {
    RuntimeError::new(format!(
        "internal error: arithmetic on `{}` and `{}`",
        a.show(),
        b.show()
    ))
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
    fn indices_must_not_be_negative() {
        assert_eq!(to_index(3), Ok(3));
        assert!(to_index(-1).is_err());
    }

    fn run(builtin: Builtin, args: &[Value]) -> std::result::Result<String, String> {
        pure(builtin, args, &[], None)
            .map(|value| value.show())
            .map_err(|error| error.message)
    }

    #[test]
    fn integer_division_rounds_toward_negative_infinity() {
        let i = Value::Int;
        for (a, b, quotient, remainder) in [
            (7, 2, 3, 1),
            (-7, 2, -4, 1),
            (7, -2, -4, -1),
            (-7, -2, 3, -1),
            (6, -3, -2, 0),
            (i64::MIN, -1, 0, 0),
        ] {
            if a != i64::MIN {
                assert_eq!(run(Builtin::Div, &[i(a), i(b)]), Ok(quotient.to_string()));
            }
            assert_eq!(run(Builtin::Mod, &[i(a), i(b)]), Ok(remainder.to_string()));
        }
        assert_eq!(
            run(Builtin::Div, &[i(1), i(0)]),
            Err("division by zero".into())
        );
        assert_eq!(
            run(Builtin::Mod, &[i(1), i(0)]),
            Err("division by zero".into())
        );
        assert!(
            run(Builtin::Div, &[i(i64::MIN), i(-1)])
                .unwrap_err()
                .starts_with("integer overflow")
        );
    }

    #[test]
    fn int_arithmetic_is_checked() {
        let i = Value::Int;
        assert_eq!(
            run(Builtin::Mul, &[i(i64::MAX), i(2)]),
            Err(format!(
                "integer overflow: `(mul {} 2)` does not fit in an `Int`",
                i64::MAX
            ))
        );
        assert!(run(Builtin::Add, &[i(i64::MAX), i(1)]).is_err());
        assert!(run(Builtin::Sub, &[i(i64::MIN), i(1)]).is_err());
        assert!(run(Builtin::Neg, &[i(i64::MIN)]).is_err());
        assert!(run(Builtin::Abs, &[i(i64::MIN)]).is_err());
        assert_eq!(
            run(Builtin::Neg, &[i(i64::MAX)]),
            Ok(format!("-{}", i64::MAX))
        );
        assert_eq!(
            run(Builtin::Pow, &[i(2), i(62)]),
            Ok("4611686018427387904".into())
        );
        assert!(run(Builtin::Pow, &[i(2), i(63)]).is_err());
        assert!(run(Builtin::Pow, &[i(2), i(1 << 40)]).is_err());
        assert_eq!(run(Builtin::Pow, &[i(1), i(1 << 40)]), Ok("1".into()));
        assert!(run(Builtin::Pow, &[i(2), i(-1)]).is_err());
    }

    #[test]
    fn float_arithmetic_and_conversions() {
        let (i, f) = (Value::Int, Value::Float);
        assert_eq!(run(Builtin::Add, &[f(0.5), f(0.25)]), Ok("0.75".into()));
        assert_eq!(run(Builtin::Pow, &[f(1.5), i(2)]), Ok("2.25".into()));
        assert_eq!(run(Builtin::Fdiv, &[f(1.0), f(0.0)]), Ok("Infinity".into()));
        assert_eq!(run(Builtin::ToFloat, &[i(3)]), Ok("3.0".into()));
        assert_eq!(run(Builtin::Floor, &[f(-2.5)]), Ok("-3".into()));
        assert_eq!(run(Builtin::Ceil, &[f(-2.5)]), Ok("-2".into()));
        assert_eq!(run(Builtin::Round, &[f(-2.5)]), Ok("-3".into()));
        assert_eq!(
            run(Builtin::Floor, &[f(-9.223372036854776e18)]),
            Ok(i64::MIN.to_string())
        );
        for n in [f64::NAN, f64::INFINITY, 9.223372036854776e18] {
            assert!(run(Builtin::Floor, &[f(n)]).is_err(), "{n}");
        }
    }

    #[test]
    fn parsing_numbers() {
        let s = |text: &str| Value::String(text.into());
        assert_eq!(
            run(
                Builtin::ParseInt,
                &[s(" -42
")]
            ),
            Ok("-42".into())
        );
        assert!(run(Builtin::ParseInt, &[s("3.5")]).is_err());
        assert!(run(Builtin::ParseInt, &[s("99999999999999999999")]).is_err());
        assert_eq!(run(Builtin::ParseFloat, &[s("3")]), Ok("3.0".into()));
        assert_eq!(run(Builtin::ParseFloat, &[s("-1e-3")]), Ok("-0.001".into()));
        for text in ["inf", "NaN", "abc", ""] {
            assert!(run(Builtin::ParseFloat, &[s(text)]).is_err(), "{text}");
        }
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
        let n = Value::Int;
        assert_eq!(
            pure(Builtin::Add, &[n(1), n(2)], &[], None).unwrap().show(),
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
        let error = pure(Builtin::Add, &[Value::Unit, n(1)], &[], None).unwrap_err();
        assert!(error.message.starts_with("internal error"));
    }
}
