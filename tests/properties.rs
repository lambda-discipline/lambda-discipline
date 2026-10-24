//! Property tests: laws the compiler must obey for every program, checked on
//! random programs from `lbd-smith` and on other random inputs.
//!
//! `arbtest` runs each property for about 100 ms; set `ARBTEST_BUDGET_MS`
//! to run longer. A failure prints a seed: rerun one property with
//! `ARBTEST_SEED=<seed> cargo test --test properties <name>`, and add
//! `.minimize()` to the property to shrink the input.

use std::collections::HashMap;

use arbitrary::{Result, Unstructured};
use arbtest::arbtest;
use lambda_discipline::Session;
use lambda_discipline::core::{self, AltPat};
use lambda_discipline::diagnostics::{Diagnostics, Severity};
use lambda_discipline::loader::{MemoryFiles, load_text_from};
use lambda_discipline::session::Compiled;
use lambda_discipline::source::{SourceMap, Span};
use lambda_discipline::syntax::ast::{ExprKind, Item, Module};
use lambda_discipline::syntax::number::format_float;
use lambda_discipline::syntax::parse_module;
use lambda_discipline::syntax::token::{lex, unescape};
use lambda_discipline::types::TopLevel;
use lbd_smith::{Config, Program};

// ------------------------------------------------------------------ helpers

fn well_typed(u: &mut Unstructured) -> Result<Program> {
    lbd_smith::program(u, &Config::default())
}

/// Compiles `text` without running it; `use` reads no files.
fn compile(text: &str) -> (Session, std::result::Result<Compiled, Diagnostics>) {
    let mut session = Session::new(Box::new(std::io::sink()));
    let loaded = load_text_from(&mut session.sources, &MemoryFiles::new(), "<test>", text);
    let compiled = session.compile(loaded, TopLevel::Any);
    (session, compiled)
}

fn render(session: &Session, diagnostics: &[lambda_discipline::diagnostics::Diagnostic]) -> String {
    let mut out = Vec::new();
    session.render(diagnostics, &mut out, false);
    String::from_utf8_lossy(&out).into_owned()
}

/// Compiles `text`, failing the test with the diagnostics if it is rejected.
fn compile_ok(text: &str) -> (Session, Compiled) {
    match compile(text) {
        (session, Ok(compiled)) => (session, compiled),
        (session, Err(errors)) => panic!(
            "rejected a well-typed program:\n{text}\n{}",
            render(&session, &errors)
        ),
    }
}

/// Each definition's inferred scheme, by name.
fn schemes(compiled: &Compiled) -> HashMap<String, String> {
    compiled
        .unit
        .defs
        .iter()
        .zip(&compiled.def_schemes)
        .map(|(def, scheme)| (def.name.to_string(), scheme.to_string()))
        .collect()
}

fn parse(text: &str) -> (Module, Diagnostics) {
    let mut sources = SourceMap::new();
    let file = sources.add_virtual("<test>", text);
    parse_module(file, text)
}

/// The module's `Debug` form without its spans, so that two parses of
/// differently laid out text compare equal.
fn shape(module: &Module) -> String {
    let debug = format!("{module:?}");
    let mut out = String::with_capacity(debug.len());
    let mut rest = debug.as_str();
    while let Some(start) = rest.find("Span {") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find('}').expect("a span ends") + start + 1;
        out.push('_');
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

// --------------------------------------------------------------- properties

/// P1: printing a parsed program and parsing it again gives the same tree.
#[test]
fn printed_ast_parses_to_the_same_ast() {
    arbtest(|u| {
        let config = Config {
            syntactic: u.arbitrary()?,
            ..Config::default()
        };
        let text = lbd_smith::program(u, &config)?.to_string();
        let (module, errors) = parse(&text);
        assert!(
            errors.is_empty(),
            "generated text does not parse:\n{text}\n{errors:?}"
        );
        let printed = module.to_string();
        let (reparsed, errors) = parse(&printed);
        assert!(
            errors.is_empty(),
            "printed AST does not parse:\n{printed}\n{errors:?}"
        );
        assert_eq!(
            shape(&module),
            shape(&reparsed),
            "\n{text}\nprinted as\n{printed}"
        );
        Ok(())
    });
}

/// P2: number literals read back as exactly the number that was printed.
#[test]
fn number_literals_round_trip() {
    arbtest(|u| {
        let x: f64 = u.arbitrary()?;
        let n: i64 = u.arbitrary()?;
        let float = if x.is_finite() { x } else { -0.0 };
        let text = format!("x = {}\ny = {n}\n", format_float(float));
        let (module, errors) = parse(&text);
        assert!(errors.is_empty(), "{text}\n{errors:?}");
        let values: Vec<&ExprKind> = module
            .items
            .iter()
            .map(|item| match item {
                Item::Def(def) => &def.value.kind,
                other => panic!("unexpected item {other:?}"),
            })
            .collect();
        match values[..] {
            [ExprKind::Float(read), ExprKind::Int(m)] => {
                assert_eq!(read.to_bits(), float.to_bits(), "{text}");
                assert_eq!(*m, n, "{text}");
            }
            _ => panic!("unexpected values {values:?} for\n{text}"),
        }
        Ok(())
    });
}

/// P3: programs that are well-typed by construction type-check, both with
/// every definition annotated with the type the generator meant and with
/// no annotations at all.
#[test]
fn well_typed_programs_check() {
    arbtest(|u| {
        let mut program = well_typed(u)?;
        compile_ok(&program.to_string());
        program.annotate();
        compile_ok(&program.to_string());
        Ok(())
    });
}

/// P4: writing each inferred scheme back as an annotation is accepted and
/// infers the same schemes.
#[test]
fn inferred_schemes_check_as_annotations() {
    arbtest(|u| {
        let mut program = well_typed(u)?;
        let (_, compiled) = compile_ok(&program.to_string());
        let inferred = schemes(&compiled);
        for def in &mut program.defs {
            def.ann = Some(inferred[&def.name].clone());
        }
        let text = program.to_string();
        let (_, compiled) = compile_ok(&text);
        assert_eq!(schemes(&compiled), inferred, "\n{text}");
        Ok(())
    });
}

/// P5: consistently renaming variables and reordering definitions does not
/// change any inferred type.
#[test]
fn types_do_not_depend_on_names_or_order() {
    arbtest(|u| {
        let program = well_typed(u)?;
        let (_, compiled) = compile_ok(&program.to_string());
        let original = schemes(&compiled);

        let mut changed = program.clone();
        // `x1` becomes `v1` and `f1` becomes `g1`.
        changed.rename(|name| {
            let (head, digits) = name.split_at(1);
            match head {
                "x" => format!("v{digits}"),
                "f" => format!("g{digits}"),
                _ => name.to_owned(),
            }
        });
        for i in (1..changed.defs.len()).rev() {
            let j = u.int_in_range(0..=i)?;
            changed.defs.swap(i, j);
        }
        let text = changed.to_string();
        let (_, compiled) = compile_ok(&text);
        let renamed: HashMap<String, String> = schemes(&compiled)
            .into_iter()
            .map(|(name, scheme)| (name.replacen('g', "f", 1), scheme))
            .collect();
        assert_eq!(renamed, original, "\n{program}\nchanged to\n{text}");
        Ok(())
    });
}

/// P6: ill-typed mutants of well-typed programs are rejected, with errors
/// that point inside the program.
#[test]
fn ill_typed_mutants_are_rejected() {
    arbtest(|u| {
        let program = well_typed(u)?;
        let mutant = lbd_smith::mutate(u, &program)?;
        let text = mutant.to_string();
        let (session, result) = compile(&text);
        let Err(errors) = result else {
            panic!("accepted an ill-typed program:\n{text}");
        };
        assert!(errors.iter().any(|e| e.severity == Severity::Error));
        for error in &errors {
            for span in error
                .span
                .iter()
                .chain(error.secondary.iter().map(|(s, _)| s))
            {
                assert_span_inside(&session.sources, *span);
            }
        }
        Ok(())
    });
}

fn assert_span_inside(sources: &SourceMap, span: Span) {
    let text = sources.get(span.file).text();
    let range = span.range();
    assert!(
        range.start <= range.end && range.end <= text.len(),
        "{range:?}"
    );
    assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
}

/// P7: Core is well-scoped: every slot a body reads was bound before, every
/// capture exists in the enclosing frame and every global exists.
#[test]
fn core_is_well_scoped() {
    arbtest(|u| {
        let program = well_typed(u)?;
        let text = program.to_string();
        let (_, compiled) = compile_ok(&text);
        let globals = compiled.module.globals.len();
        let bodies = compiled
            .module
            .globals
            .iter()
            .map(|g| &g.body)
            .chain(&compiled.module.entries);
        for body in bodies {
            let mut scope = Scope {
                bound: vec![false; body.frame_size as usize],
                globals,
            };
            scope
                .check(&body.expr)
                .unwrap_or_else(|e| panic!("{e}\n{text}"));
        }
        Ok(())
    });
}

/// Which slots of the current frame hold a value.
struct Scope {
    bound: Vec<bool>,
    globals: usize,
}

impl Scope {
    fn bind(&mut self, slot: u32) -> std::result::Result<(), String> {
        let slot = slot as usize;
        if slot >= self.bound.len() {
            return Err(format!(
                "slot {slot} outside a frame of {}",
                self.bound.len()
            ));
        }
        self.bound[slot] = true;
        Ok(())
    }

    /// Checks `e` without changing which slots are bound afterwards.
    fn check(&mut self, e: &core::Expr) -> std::result::Result<(), String> {
        let saved = self.bound.clone();
        let result = self.check_inner(e);
        self.bound = saved;
        result
    }

    fn check_inner(&mut self, e: &core::Expr) -> std::result::Result<(), String> {
        match e {
            core::Expr::Lit(_) | core::Expr::Builtin(_) => Ok(()),
            core::Expr::Local(slot) => match self.bound.get(*slot as usize) {
                Some(true) => Ok(()),
                _ => Err(format!("reads slot {slot}, which is not bound")),
            },
            core::Expr::Global(id) if id.index() < self.globals => Ok(()),
            core::Expr::Global(id) => Err(format!("unknown global {id:?}")),
            core::Expr::App { func, args, .. } => {
                self.check(func)?;
                args.iter().try_for_each(|a| self.check(a))
            }
            core::Expr::List { items, .. } => items.iter().try_for_each(|i| self.check(i)),
            core::Expr::Con { con, args, .. } => {
                if con.arity as usize != args.len() {
                    return Err(format!("{} applied to {} fields", con.name, args.len()));
                }
                args.iter().try_for_each(|a| self.check(a))
            }
            core::Expr::Let { slot, value, body } => {
                self.bind(*slot)?;
                self.check(value)?;
                self.check(body)
            }
            core::Expr::Lambda(lambda) => {
                for capture in &lambda.captures {
                    if self.bound.get(*capture as usize) != Some(&true) {
                        return Err(format!("captures slot {capture}, which is not bound"));
                    }
                }
                let own = lambda.captures.len() + lambda.arity as usize;
                if own > lambda.frame_size as usize {
                    return Err(format!(
                        "{own} captures and parameters in a frame of {}",
                        lambda.frame_size
                    ));
                }
                let mut inner = Scope {
                    bound: (0..lambda.frame_size as usize).map(|i| i < own).collect(),
                    globals: self.globals,
                };
                inner.check(&lambda.body)
            }
            core::Expr::Case(case) => {
                self.check(&case.scrutinee)?;
                for alt in &case.alts {
                    let saved = self.bound.clone();
                    if let AltPat::Con { con, fields } = &alt.pat {
                        if con.arity as usize != fields.len() {
                            return Err(format!(
                                "{} matched with {} fields",
                                con.name,
                                fields.len()
                            ));
                        }
                        for field in fields {
                            self.bind(*field)?;
                        }
                    }
                    self.check(&alt.body)?;
                    self.bound = saved;
                }
                case.default.as_ref().map_or(Ok(()), |d| self.check(d))
            }
        }
    }
}

/// P8: a `match` is accepted exactly when enumerating the values of its
/// scrutinee's type finds none that no arm matches.
#[test]
fn exhaustiveness_agrees_with_brute_force() {
    arbtest(|u| {
        let ty = small_ty(u, 2)?;
        let arms: Vec<SmallPat> = (0..u.int_in_range(1..=4)?)
            .map(|_| small_pat(u, &ty, 2))
            .collect::<Result<_>>()?;
        let expected = values(&ty, 3)
            .iter()
            .all(|v| arms.iter().any(|p| p.matches(v)));
        let mut text = format!("f : {} -> Int = \\x. match x", ty.source());
        for (i, arm) in arms.iter().enumerate() {
            text.push_str(&format!("\n    | {} -> {i}", arm.source(false)));
        }
        let (session, result) = compile(&text);
        match (expected, result) {
            (true, Err(errors)) => panic!(
                "rejected an exhaustive match:\n{text}\n{}",
                render(&session, &errors)
            ),
            (false, Ok(_)) => panic!("accepted a non-exhaustive match:\n{text}"),
            (false, Err(errors)) => assert!(
                errors.iter().any(|e| e.message.contains("does not cover")),
                "rejected for another reason:\n{text}\n{}",
                render(&session, &errors)
            ),
            (true, Ok(_)) => {}
        }
        Ok(())
    });
}

#[derive(Debug, Clone)]
enum SmallTy {
    Bool,
    Int,
    Unit,
    Option(Box<SmallTy>),
    Pair(Box<SmallTy>, Box<SmallTy>),
    Result(Box<SmallTy>, Box<SmallTy>),
}

/// A value whose parts beyond some depth are unknown (`Any`), standing for
/// every value of their type. An Int other than the ones patterns mention
/// is `Any` too.
#[derive(Debug, Clone)]
enum SmallValue {
    Con(&'static str, Vec<SmallValue>),
    Int(i64),
    Any,
}

#[derive(Debug, Clone)]
enum SmallPat {
    Wild,
    Con(&'static str, Vec<SmallPat>),
    Int(i64),
    Unit,
}

const INTS: [i64; 3] = [-1, 0, 7];

fn small_ty(u: &mut Unstructured, depth: u32) -> Result<SmallTy> {
    if depth == 0 {
        return Ok(u
            .choose(&[SmallTy::Bool, SmallTy::Int, SmallTy::Unit])?
            .clone());
    }
    let d = depth - 1;
    Ok(match u.int_in_range(0..=5)? {
        0 => SmallTy::Bool,
        1 => SmallTy::Int,
        2 => SmallTy::Option(Box::new(small_ty(u, d)?)),
        3 => SmallTy::Pair(Box::new(small_ty(u, d)?), Box::new(small_ty(u, d)?)),
        4 => SmallTy::Result(Box::new(small_ty(u, d)?), Box::new(small_ty(u, d)?)),
        _ => SmallTy::Unit,
    })
}

impl SmallTy {
    fn source(&self) -> String {
        match self {
            SmallTy::Bool => "Bool".into(),
            SmallTy::Int => "Int".into(),
            SmallTy::Unit => "Unit".into(),
            SmallTy::Option(t) => format!("Option<{}>", t.source()),
            SmallTy::Pair(a, b) => format!("({}, {})", a.source(), b.source()),
            SmallTy::Result(e, a) => format!("Result<{}, {}>", e.source(), a.source()),
        }
    }
}

fn small_pat(u: &mut Unstructured, ty: &SmallTy, depth: u32) -> Result<SmallPat> {
    if depth == 0 || u.ratio(1, 3)? {
        return Ok(SmallPat::Wild);
    }
    let d = depth - 1;
    Ok(match ty {
        SmallTy::Bool => SmallPat::Con(u.choose(&["True", "False"])?, vec![]),
        SmallTy::Int => SmallPat::Int(*u.choose(&INTS)?),
        SmallTy::Unit => SmallPat::Unit,
        SmallTy::Option(t) => {
            if u.arbitrary()? {
                SmallPat::Con("None", vec![])
            } else {
                SmallPat::Con("Some", vec![small_pat(u, t, d)?])
            }
        }
        SmallTy::Pair(a, b) => SmallPat::Con("", vec![small_pat(u, a, d)?, small_pat(u, b, d)?]),
        SmallTy::Result(e, a) => {
            if u.arbitrary()? {
                SmallPat::Con("Err", vec![small_pat(u, e, d)?])
            } else {
                SmallPat::Con("Ok", vec![small_pat(u, a, d)?])
            }
        }
    })
}

impl SmallPat {
    fn source(&self, atomic: bool) -> String {
        match self {
            SmallPat::Wild => "_".into(),
            SmallPat::Int(n) => n.to_string(),
            SmallPat::Unit => "()".into(),
            SmallPat::Con("", items) => {
                let items: Vec<String> = items.iter().map(|p| p.source(false)).collect();
                format!("({})", items.join(", "))
            }
            SmallPat::Con(name, fields) if fields.is_empty() => (*name).into(),
            SmallPat::Con(name, fields) => {
                let fields: Vec<String> = fields.iter().map(|p| p.source(true)).collect();
                let text = format!("{name} {}", fields.join(" "));
                if atomic { format!("({text})") } else { text }
            }
        }
    }

    fn matches(&self, value: &SmallValue) -> bool {
        match (self, value) {
            (SmallPat::Wild | SmallPat::Unit, _) => true,
            (SmallPat::Int(n), SmallValue::Int(m)) => n == m,
            (SmallPat::Con(name, pats), SmallValue::Con(con, fields)) => {
                name == con && pats.iter().zip(fields).all(|(p, v)| p.matches(v))
            }
            _ => false,
        }
    }
}

/// Every value of `ty` down to `depth`, with `Any` below that.
fn values(ty: &SmallTy, depth: u32) -> Vec<SmallValue> {
    if depth == 0 {
        return vec![SmallValue::Any];
    }
    let d = depth - 1;
    let wrap = |name: &'static str, t: &SmallTy| {
        values(t, d)
            .into_iter()
            .map(move |v| SmallValue::Con(name, vec![v]))
    };
    match ty {
        SmallTy::Bool => vec![
            SmallValue::Con("True", vec![]),
            SmallValue::Con("False", vec![]),
        ],
        SmallTy::Int => INTS
            .iter()
            .map(|&n| SmallValue::Int(n))
            .chain([SmallValue::Any])
            .collect(),
        SmallTy::Unit => vec![SmallValue::Any],
        SmallTy::Option(t) => std::iter::once(SmallValue::Con("None", vec![]))
            .chain(wrap("Some", t))
            .collect(),
        SmallTy::Result(e, a) => wrap("Err", e).chain(wrap("Ok", a)).collect(),
        SmallTy::Pair(a, b) => {
            let bs = values(b, d);
            values(a, d)
                .into_iter()
                .flat_map(|x| {
                    bs.iter()
                        .map(move |y| SmallValue::Con("", vec![x.clone(), y.clone()]))
                })
                .collect()
        }
    }
}

/// P9: `eq` and `compare` obey the laws of equality and of a total order on
/// generated values.
#[test]
fn eq_and_ord_obey_their_laws() {
    arbtest(|u| {
        let ty = small_value_ty(u, 2)?;
        let [a, b, c] = [
            value_source(u, &ty, 2)?,
            value_source(u, &ty, 2)?,
            value_source(u, &ty, 2)?,
        ];
        let laws = format!(
            "[(eq {a} {a}), \
              (eq (eq {a} {b}) (eq {b} {a})), \
              (eq (compare {a} {b}) (neg (compare {b} {a}))), \
              (eq (eq {a} {b}) (eq (compare {a} {b}) 0)), \
              (eq (lt {a} {b}) (eq (compare {a} {b}) -1)), \
              (eq (le {a} {b}) (not (gt {a} {b}))), \
              (or (not (and (le {a} {b}) (le {b} {c}))) (le {a} {c})), \
              (or (not (and (eq {a} {b}) (eq {b} {c}))) (eq {a} {c}))]"
        );
        let mut session = Session::new(Box::new(std::io::sink()));
        let evaluated = session
            .eval("<laws>", &laws)
            .unwrap_or_else(|e| panic!("{laws}\n{}", render(&session, &e)));
        let shown = evaluated.values[0].0.show();
        assert!(
            !shown.contains("False"),
            "a law fails: {shown}\nfor\n{laws}"
        );
        Ok(())
    });
}

/// A type whose values are all comparable: `ty` in `value_source`.
#[derive(Debug, Clone)]
enum ValueTy {
    Int,
    Float,
    String,
    Bool,
    List(Box<ValueTy>),
    Option(Box<ValueTy>),
    Pair(Box<ValueTy>, Box<ValueTy>),
}

fn small_value_ty(u: &mut Unstructured, depth: u32) -> Result<ValueTy> {
    let d = depth.saturating_sub(1);
    Ok(match u.int_in_range(0..=6)? {
        _ if depth == 0 => ValueTy::Int,
        0 => ValueTy::Float,
        1 => ValueTy::String,
        2 => ValueTy::Bool,
        3 => ValueTy::List(Box::new(small_value_ty(u, d)?)),
        4 => ValueTy::Option(Box::new(small_value_ty(u, d)?)),
        5 => ValueTy::Pair(
            Box::new(small_value_ty(u, d)?),
            Box::new(small_value_ty(u, d)?),
        ),
        _ => ValueTy::Int,
    })
}

/// Source text of a value of `ty`, from a small set so that equal values
/// are common.
fn value_source(u: &mut Unstructured, ty: &ValueTy, depth: u32) -> Result<String> {
    Ok(match ty {
        ValueTy::Int => u
            .choose(&["-1", "0", "2", "9223372036854775807"])?
            .to_string(),
        ValueTy::Float => u
            .choose(&["-0.0", "0.0", "1.5", "-2.5", "1e+300"])?
            .to_string(),
        ValueTy::String => u.choose(&["\"\"", "\"a\"", "\"ab\"", "\"é\""])?.to_string(),
        ValueTy::Bool => u.choose(&["True", "False"])?.to_string(),
        ValueTy::List(t) => {
            let n = if depth == 0 {
                0
            } else {
                u.int_in_range(0..=2)?
            };
            let items = (0..n)
                .map(|_| value_source(u, t, depth - 1))
                .collect::<Result<Vec<_>>>()?;
            format!("[{}]", items.join(", "))
        }
        ValueTy::Option(t) => {
            if u.arbitrary()? {
                "None".to_owned()
            } else {
                format!("(Some {})", value_source(u, t, depth.saturating_sub(1))?)
            }
        }
        ValueTy::Pair(a, b) => format!(
            "({}, {})",
            value_source(u, a, depth.saturating_sub(1))?,
            value_source(u, b, depth.saturating_sub(1))?
        ),
    })
}

/// P10: string escapes and tokens.
#[test]
fn escaped_strings_unescape_to_themselves() {
    arbtest(|u| {
        let s: String = u.arbitrary()?;
        // The AST printer writes strings with Rust's escapes.
        assert_eq!(unescape(&format!("{s:?}")).as_deref(), Ok(s.as_str()));
        Ok(())
    });
}

#[test]
fn tokens_tile_valid_programs() {
    arbtest(|u| {
        let config = Config {
            syntactic: true,
            ..Config::default()
        };
        let text = lbd_smith::program(u, &config)?.to_string();
        let mut sources = SourceMap::new();
        let file = sources.add_virtual("<test>", text.as_str());
        let (tokens, errors) = lex(file, &text);
        assert!(errors.is_empty(), "{text}\n{errors:?}");
        let mut end = 0;
        for token in &tokens {
            let range = token.span.range();
            for line in text[end..range.start].split('\n') {
                let line = line.trim();
                assert!(
                    line.is_empty() || line.starts_with("--"),
                    "{line:?} between tokens in\n{text}"
                );
            }
            end = range.end;
        }
        assert_eq!(end, text.len());
        Ok(())
    });
}
