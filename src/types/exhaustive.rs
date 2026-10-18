//! Exhaustiveness of `match`, by Maranget's usefulness algorithm
//! ("Warnings for pattern matching", JFP 2007).
//!
//! A pattern vector is *useful* with respect to a matrix of rows when some
//! value matches it and no row. The arms of a `match` are exhaustive when
//! the wildcard is not useful against them; when it is, the algorithm also
//! builds a value the arms miss, which the error shows.
//!
//! Only constructors matter: variables, `_` and `()` match anything, and
//! number and string literals never cover their (infinite) types.

use std::rc::Rc;

use crate::hir::{self, ConId, PatternKind};

use super::env::TypeEnv;

#[derive(Debug, Clone, PartialEq)]
enum Pat {
    Wild,
    Con(ConId, Vec<Pat>),
    Lit(Lit),
}

#[derive(Debug, Clone, PartialEq)]
enum Lit {
    Int(i64),
    /// The bits of the float, with `-0.0` folded into `0.0`.
    Float(u64),
    String(Rc<str>),
}

/// A value no arm matches, written as a pattern (`Rect _ _`), or `None`
/// when the arms are exhaustive.
pub fn missing(env: &TypeEnv, arms: &[&hir::Pattern]) -> Option<String> {
    let rows: Vec<Vec<Pat>> = arms.iter().map(|p| vec![simplify(p)]).collect();
    let witness = Checker { env }.useful(&rows, &[Pat::Wild])?;
    let mut out = String::new();
    write_pattern(env, &witness[0], false, &mut out);
    Some(out)
}

fn simplify(pattern: &hir::Pattern) -> Pat {
    match &pattern.kind {
        PatternKind::Con { con, args } => Pat::Con(*con, args.iter().map(simplify).collect()),
        PatternKind::Int(n) => Pat::Lit(Lit::Int(*n)),
        PatternKind::Float(n) => Pat::Lit(Lit::Float(if *n == 0.0 { 0.0 } else { *n }.to_bits())),
        PatternKind::String(s) => Pat::Lit(Lit::String(s.clone())),
        PatternKind::Wild | PatternKind::Bind(_) | PatternKind::Unit | PatternKind::Error => {
            Pat::Wild
        }
    }
}

struct Checker<'a> {
    env: &'a TypeEnv,
}

impl Checker<'_> {
    /// A value matching `q` and none of `rows`, as a pattern vector as long
    /// as `q`; `None` if there is no such value.
    fn useful(&self, rows: &[Vec<Pat>], q: &[Pat]) -> Option<Vec<Pat>> {
        let Some((head, rest)) = q.split_first() else {
            return rows.is_empty().then(Vec::new);
        };
        match head {
            Pat::Con(con, args) => {
                let rows = specialize(rows, &Pat::Con(*con, args.clone()), args.len());
                let mut q: Vec<Pat> = args.clone();
                q.extend_from_slice(rest);
                let witness = self.useful(&rows, &q)?;
                Some(rebuild(*con, args.len(), witness))
            }
            Pat::Lit(_) => {
                let rows = specialize(rows, head, 0);
                let mut witness = self.useful(&rows, rest)?;
                witness.insert(0, head.clone());
                Some(witness)
            }
            Pat::Wild => {
                let heads: Vec<ConId> = rows
                    .iter()
                    .filter_map(|row| match &row[0] {
                        Pat::Con(con, _) => Some(*con),
                        _ => None,
                    })
                    .collect();
                let all = heads.first().map(|&con| self.env.siblings(con));
                if let Some(all) = all.filter(|all| all.iter().all(|c| heads.contains(c))) {
                    // Every constructor appears: some value is missed only
                    // inside one of them.
                    return all.iter().find_map(|&con| {
                        let arity = self.env.constructor(con).arity;
                        let rows = specialize(rows, &Pat::Con(con, wilds(arity)), arity);
                        let mut q = wilds(arity);
                        q.extend_from_slice(rest);
                        let witness = self.useful(&rows, &q)?;
                        Some(rebuild(con, arity, witness))
                    });
                }
                // Some constructor (or literal) never appears: a value built
                // with it is missed if the rest of `q` is.
                let defaults: Vec<Vec<Pat>> = rows
                    .iter()
                    .filter(|row| row[0] == Pat::Wild)
                    .map(|row| row[1..].to_vec())
                    .collect();
                let mut witness = self.useful(&defaults, rest)?;
                let head = match all.and_then(|all| all.iter().find(|c| !heads.contains(c))) {
                    Some(&con) => Pat::Con(con, wilds(self.env.constructor(con).arity)),
                    None => Pat::Wild,
                };
                witness.insert(0, head);
                Some(witness)
            }
        }
    }
}

fn wilds(count: usize) -> Vec<Pat> {
    vec![Pat::Wild; count]
}

/// The rows that can match `head` in their first column, with that column
/// replaced by the `arity` sub-patterns of `head`.
fn specialize(rows: &[Vec<Pat>], head: &Pat, arity: usize) -> Vec<Vec<Pat>> {
    rows.iter()
        .filter_map(|row| {
            let mut out = match (&row[0], head) {
                (Pat::Wild, _) => wilds(arity),
                (Pat::Con(c, args), Pat::Con(d, _)) if c == d => args.clone(),
                (Pat::Lit(a), Pat::Lit(b)) if a == b => Vec::new(),
                _ => return None,
            };
            out.extend_from_slice(&row[1..]);
            Some(out)
        })
        .collect()
}

/// Folds the first `arity` patterns of a witness back into `con`.
fn rebuild(con: ConId, arity: usize, mut witness: Vec<Pat>) -> Vec<Pat> {
    let rest = witness.split_off(arity);
    let mut out = vec![Pat::Con(con, witness)];
    out.extend(rest);
    out
}

fn write_pattern(env: &TypeEnv, pattern: &Pat, nested: bool, out: &mut String) {
    match pattern {
        Pat::Wild | Pat::Lit(_) => out.push('_'),
        Pat::Con(con, args) => {
            let parens = nested && !args.is_empty();
            if parens {
                out.push('(');
            }
            out.push_str(&env.constructor(*con).name);
            for arg in args {
                out.push(' ');
                write_pattern(env, arg, true, out);
            }
            if parens {
                out.push(')');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_text;
    use crate::resolve::{EmptyScope, resolve};
    use crate::source::SourceMap;
    use crate::types::{TopLevel, check_unit};

    /// The witness for the first `match` in `text`, as the type checker
    /// reports it.
    fn missing_in(text: &str) -> Option<String> {
        let mut sources = SourceMap::new();
        let loaded = load_text(&mut sources, "<test>", text);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        let resolved = resolve(loaded.items, &EmptyScope);
        assert!(
            resolved.diagnostics.is_empty(),
            "{:?}",
            resolved.diagnostics
        );
        let typed = check_unit(&resolved.unit, &[], &TypeEnv::new(), TopLevel::Any);
        match typed.diagnostics.as_slice() {
            [] => None,
            [diagnostic] => {
                let label = diagnostic.label.as_deref().unwrap();
                Some(
                    label
                        .trim_end_matches("` is not covered")
                        .trim_start_matches('`')
                        .to_owned(),
                )
            }
            more => panic!("{more:#?}"),
        }
    }

    const SHAPES: &str = "type Shape = Circle Int | Rect Int Int | Dot\n";
    const OPTION: &str = "type Option<a> = None | Some a\n";

    #[test]
    fn complete_matches() {
        assert_eq!(
            missing_in(&format!(
                "{SHAPES}f = \\s. match s | Circle r -> r | Rect w h -> w | Dot -> 0"
            )),
            None
        );
        assert_eq!(
            missing_in(&format!("{SHAPES}f = \\s. match s | Dot -> 0 | _ -> 1")),
            None
        );
        assert_eq!(missing_in("f = \\x. match x | y -> y"), None);
        assert_eq!(missing_in("f = \\x. match x | () -> 1"), None);
        assert_eq!(
            missing_in("f = \\b. match b | True -> 1 | False -> 0"),
            None
        );
    }

    #[test]
    fn a_missing_constructor_is_named_with_wildcard_fields() {
        assert_eq!(
            missing_in(&format!(
                "{SHAPES}f = \\s. match s | Circle r -> r | Dot -> 0"
            )),
            Some("Rect _ _".into())
        );
        assert_eq!(
            missing_in("f = \\b. match b | True -> 1"),
            Some("False".into())
        );
    }

    #[test]
    fn missing_values_inside_constructors() {
        assert_eq!(
            missing_in(&format!(
                "{OPTION}f = \\o. match o | None -> 0 | Some None -> 1"
            )),
            Some("Some (Some _)".into())
        );
        assert_eq!(
            missing_in(&format!(
                "{OPTION}type Pair<a, b> = Pair a b\nf = \\p. match p | Pair True _ -> 0 | Pair _ False -> 1"
            )),
            Some("Pair False True".into())
        );
    }

    #[test]
    fn literals_never_cover_their_type() {
        assert_eq!(
            missing_in("f = \\n. match n | 0 -> 1 | 1 -> 2"),
            Some("_".into())
        );
        assert_eq!(missing_in("f = \\s. match s | \"a\" -> 1 | _ -> 2"), None);
        assert_eq!(
            missing_in(&format!(
                "{OPTION}f = \\o. match o | Some 1 -> 1 | None -> 0"
            )),
            Some("Some _".into())
        );
    }
}
