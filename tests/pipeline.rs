//! The library API: driving the compiler stages and plugging in a backend.

use lambda_discipline::Session;
use lambda_discipline::backend::{Backend, RuntimeError};
use lambda_discipline::core::{Expr, Module};
use lambda_discipline::loader::load_text;
use lambda_discipline::output::SharedBuffer;
use lambda_discipline::types::TopLevel;

#[test]
fn session_evaluates_and_reports_types() {
    let out = SharedBuffer::new();
    let mut session = Session::new(Box::new(out.clone()));
    let evaluated = session
        .eval(
            "<test>",
            "square = \\x. (mul x x)\n(println (square 4))\n(map square [1, 2])",
        )
        .unwrap();
    assert_eq!(evaluated.definitions[0].0.as_ref(), "square");
    assert_eq!(evaluated.definitions[0].1.to_string(), "Number -> Number");
    assert_eq!(evaluated.values[1].0.show(), "[1, 4]");
    assert_eq!(evaluated.values[1].1.to_string(), "List<Number>");
    assert_eq!(out.contents(), "16\n");
}

#[test]
fn diagnostics_render_against_session_sources() {
    let mut session = Session::new(Box::new(std::io::sink()));
    let diagnostics = session.eval("demo.lbd", "(add True 1)").unwrap_err();
    let mut rendered = Vec::new();
    session.render(&diagnostics, &mut rendered, false);
    let rendered = String::from_utf8(rendered).unwrap();
    assert!(rendered.contains("demo.lbd:1:6"), "{rendered}");
    assert!(
        rendered.contains("expected `Number`, found `Bool`"),
        "{rendered}"
    );
}

#[test]
fn type_of_does_not_evaluate() {
    let mut session = Session::new(Box::new(std::io::sink()));
    let scheme = session.type_of("(error \"never\")").unwrap();
    assert_eq!(scheme.to_string(), "a");
}

/// A backend that only inspects Core, standing in for a future code generator.
struct Inventory {
    globals: Vec<String>,
    lambdas: usize,
}

fn count_lambdas(expr: &Expr) -> usize {
    match expr {
        Expr::Lambda(lambda) => 1 + count_lambdas(&lambda.body),
        Expr::App { func, args, .. } => {
            count_lambdas(func) + args.iter().map(|a| count_lambdas(a)).sum::<usize>()
        }
        Expr::List { items, .. } => items.iter().map(|i| count_lambdas(i)).sum(),
        Expr::Let { value, body, .. } => count_lambdas(value) + count_lambdas(body),
        _ => 0,
    }
}

impl Backend for Inventory {
    type Output = usize;

    fn run(&mut self, module: &Module) -> Result<usize, RuntimeError> {
        for global in &module.globals {
            self.globals.push(global.name.to_string());
            self.lambdas += count_lambdas(&global.body.expr);
        }
        Ok(module.globals.len())
    }
}

#[test]
fn core_ir_can_feed_another_backend() {
    let mut session = Session::new(Box::new(std::io::sink()));
    let loaded = load_text(
        &mut session.sources,
        "<test>",
        "k = \\x. \\y. x\napplyAll = \\fs. (map \\f. (f 1) fs)",
    );
    // Compile with the session's front end, run with a different backend.
    let compiled = session.compile(loaded, TopLevel::Actions).unwrap();
    let mut backend = Inventory {
        globals: Vec::new(),
        lambdas: 0,
    };
    assert_eq!(backend.run(&compiled.module).unwrap(), 2);
    assert_eq!(backend.globals, ["k", "applyAll"]);
    // `\x. \y. x` is uncurried into one lambda; `applyAll` has two.
    assert_eq!(backend.lambdas, 3);
}
