//! Mutations that turn a well-typed program into an ill-typed one.

use arbitrary::{Result, Unstructured};

use crate::model::{Expr, Program, Ty};

/// Subterms that are ill-typed wherever they appear, since their own parts
/// disagree.
const ILL_TYPED: &[&str] = &[
    "(add 1 \"one\")",
    "(not 1)",
    "(1 2)",
    "[1, \"two\"]",
    "(Some 1 2)",
    "(length 5)",
    "(if 1 2 3)",
    "(concat \"a\" 1.5)",
];

/// Returns a copy of `program` that the type checker must reject. Either
/// one definition's annotation is replaced by a type that clashes with its
/// body, or a subterm is replaced by one that is ill-typed wherever it
/// appears, such as `(add 1 "one")`.
pub fn mutate(u: &mut Unstructured, program: &Program) -> Result<Program> {
    let mut mutant = program.clone();
    mutant.annotate();
    let candidates: Vec<usize> = (0..mutant.defs.len())
        .filter(|&i| has_known_head(&mutant.defs[i].body))
        .collect();
    if !candidates.is_empty() && u.arbitrary()? {
        let i = *u.choose(&candidates)?;
        let wrong = [
            Ty::Int,
            Ty::String,
            Ty::list(Ty::Bool),
            Ty::fun(Ty::Unit, Ty::Unit),
        ]
        .into_iter()
        .find(|t| t.clashes_with(&mutant.defs[i].ty))
        .expect("no type clashes with all four");
        mutant.defs[i].ann = Some(mutant.ty(&wrong));
        return Ok(mutant);
    }
    let total: usize = mutant.defs.iter().map(|d| size(&d.body)).sum::<usize>()
        + mutant.entries.iter().map(|(e, _)| size(e)).sum::<usize>();
    let mut target = u.int_in_range(0..=total - 1)?;
    let bad = Expr::Global((*u.choose(ILL_TYPED)?).to_owned());
    let roots = mutant
        .defs
        .iter_mut()
        .map(|d| &mut d.body)
        .chain(mutant.entries.iter_mut().map(|(e, _)| e));
    for root in roots {
        if replace(root, &mut target, &bad) {
            break;
        }
    }
    Ok(mutant)
}

/// Whether the outermost type constructor of `e`'s type follows from its
/// form alone, whatever the types of its parts. A variable or an `unwrap`
/// could have any type, so changing their annotation might not clash.
fn has_known_head(e: &Expr) -> bool {
    match e {
        Expr::Int(_)
        | Expr::Float(_)
        | Expr::String(_)
        | Expr::Unit
        | Expr::Lambda { .. }
        | Expr::List(_)
        | Expr::Tuple(_) => true,
        Expr::Global(name) => matches!(name.as_str(), "True" | "False" | "None"),
        Expr::App(func, _) => matches!(
            &**func,
            Expr::Global(name) if matches!(
                name.as_str(),
                "Some" | "Ok" | "Err" | "pure" | "println" | "print" | "cons" | "show"
                    | "concat" | "length" | "not" | "eq"
            )
        ),
        _ => false,
    }
}

fn size(e: &Expr) -> usize {
    1 + match e {
        Expr::App(func, args) => size(func) + args.iter().map(size).sum::<usize>(),
        Expr::Lambda { body, .. } => size(body),
        Expr::Let { value, body, .. }
        | Expr::Bind {
            action: value,
            body,
            ..
        } => size(value) + size(body),
        Expr::List(items) | Expr::Tuple(items) => items.iter().map(size).sum(),
        Expr::Match(scrutinee, arms) => {
            size(scrutinee) + arms.iter().map(|(_, body)| size(body)).sum::<usize>()
        }
        _ => 0,
    }
}

/// Replaces the `target`th node (in preorder) by `bad`; returns whether it
/// was in `e`, counting `target` down by the nodes it passed otherwise.
fn replace(e: &mut Expr, target: &mut usize, bad: &Expr) -> bool {
    if *target == 0 {
        *e = bad.clone();
        return true;
    }
    *target -= 1;
    match e {
        Expr::App(func, args) => {
            replace(func, target, bad) || args.iter_mut().any(|a| replace(a, target, bad))
        }
        Expr::Lambda { body, .. } => replace(body, target, bad),
        Expr::Let { value, body, .. }
        | Expr::Bind {
            action: value,
            body,
            ..
        } => replace(value, target, bad) || replace(body, target, bad),
        Expr::List(items) | Expr::Tuple(items) => items.iter_mut().any(|i| replace(i, target, bad)),
        Expr::Match(scrutinee, arms) => {
            replace(scrutinee, target, bad)
                || arms.iter_mut().any(|(_, body)| replace(body, target, bad))
        }
        _ => false,
    }
}
