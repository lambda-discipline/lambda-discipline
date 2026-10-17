//! Interpreter tests, driven through the whole pipeline.

use crate::diagnostics::Diagnostic;
use crate::output::SharedBuffer;
use crate::session::Session;

fn session() -> (Session, SharedBuffer) {
    let out = SharedBuffer::new();
    (Session::new(Box::new(out.clone())), out)
}

/// Shows the value of the last top-level expression.
fn eval(text: &str) -> String {
    let (mut session, _) = session();
    match session.eval("<test>", text) {
        Ok(evaluated) => evaluated.values.last().expect("an expression").0.show(),
        Err(diagnostics) => panic!("{}", render(&session, &diagnostics)),
    }
}

/// What the program printed.
fn output(text: &str) -> String {
    let (mut session, out) = session();
    if let Err(diagnostics) = session.eval("<test>", text) {
        panic!("{}", render(&session, &diagnostics));
    }
    out.contents()
}

fn runtime_error(text: &str) -> Diagnostic {
    let (mut session, _) = session();
    let mut diagnostics = session
        .eval("<test>", text)
        .expect_err("expected a runtime error");
    assert_eq!(diagnostics.len(), 1);
    diagnostics.remove(0)
}

fn render(session: &Session, diagnostics: &[Diagnostic]) -> String {
    let mut out = Vec::new();
    session.render(diagnostics, &mut out, false);
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn arithmetic() {
    assert_eq!(eval("(add 1 2)"), "3");
    assert_eq!(eval("(sub 1 2.5)"), "-1.5");
    assert_eq!(eval("(mul 3 (div 1 4))"), "0.75");
    assert_eq!(eval("(div 1 0)"), "Infinity");
    assert_eq!(eval("[(mod 7 3), (mod -7 3), (mod 7 -3)]"), "[1, 2, -2]");
    assert_eq!(
        eval("[(pow 2 10), (neg 3), (abs -3), (sqrt 16)]"),
        "[1024, -3, 3, 4]"
    );
    assert_eq!(
        eval("[(floor 1.5), (ceil 1.5), (round 2.5), (round -2.5)]"),
        "[1, 2, 3, -3]"
    );
    assert_eq!(eval("[(min 1 2), (max 1 2)]"), "[1, 2]");
}

#[test]
fn comparison() {
    assert_eq!(
        eval("[(eq 1 1), (neq 1 1), (lt 1 2), (le 2 2), (gt 1 2), (ge 1 2)]"),
        "[true, false, true, true, false, false]"
    );
    assert_eq!(
        eval("[(eq [1, 2] [1, 2]), (lt \"abc\" \"abd\"), (lt [1] [1, 0]), (lt false true)]"),
        "[true, true, true, true]"
    );
    assert_eq!(
        eval("[(compare 1 2), (compare 2 2), (compare 3 2)]"),
        "[-1, 0, 1]"
    );
}

#[test]
fn booleans_are_lazy() {
    assert_eq!(eval("(if true 1 (error \"no\"))"), "1");
    assert_eq!(eval("(and false (error \"no\"))"), "false");
    assert_eq!(eval("(or true (error \"no\"))"), "true");
    assert_eq!(
        eval("[(not true), (and true true), (or false false)]"),
        "[false, true, false]"
    );
}

#[test]
fn strings() {
    assert_eq!(eval("(concat \"ab\" \"cd\")"), "\"abcd\"");
    assert_eq!(eval("(show [\"a\", \"b\"])"), r#""[\"a\", \"b\"]""#);
    assert_eq!(eval("(parseNumber \" 42.5 \")"), "42.5");
    assert_eq!(eval("(stringLength \"λx\")"), "2");
    assert_eq!(eval("(chars \"ab\")"), "[\"a\", \"b\"]");
    assert_eq!(
        eval("(split \",\" \"a,b,,c\")"),
        "[\"a\", \"b\", \"\", \"c\"]"
    );
    assert_eq!(eval("(lines \"a\\r\\nb\\n\")"), "[\"a\", \"b\"]");
    assert_eq!(eval("(join \"-\" [\"a\", \"b\"])"), "\"a-b\"");
    assert_eq!(eval("(trim \"  x \")"), "\"x\"");
}

#[test]
fn lists_are_immutable() {
    assert_eq!(
        eval("let xs = [1, 2] in [(push 3 xs), xs]"),
        "[[1, 2, 3], [1, 2]]"
    );
    assert_eq!(
        eval("let xs = [1, 2, 3] in [(remove 1 xs), xs]"),
        "[[1, 3], [1, 2, 3]]"
    );
}

#[test]
fn list_operations() {
    assert_eq!(eval("[(length [1, 2]), (length [])]"), "[2, 0]");
    assert_eq!(eval("[(isEmpty []), (isEmpty [1])]"), "[true, false]");
    assert_eq!(eval("[(head [1, 2]), (get 1 [1, 2])]"), "[1, 2]");
    assert_eq!(eval("(tail [1, 2, 3])"), "[2, 3]");
    assert_eq!(eval("(cons 0 [1])"), "[0, 1]");
    assert_eq!(eval("(append [1] [2, 3])"), "[1, 2, 3]");
    assert_eq!(eval("(map (mul 2) [1, 2, 3])"), "[2, 4, 6]");
    assert_eq!(eval("(filter (lt 1) [1, 2, 3])"), "[2, 3]");
    assert_eq!(eval("(foldl sub 10 [1, 2])"), "7");
    assert_eq!(eval("(foldr sub 10 [1, 2])"), "9");
    assert_eq!(eval("(foldr cons [] [1, 2])"), "[1, 2]");
    assert_eq!(eval("(reverse [1, 2, 3])"), "[3, 2, 1]");
    assert_eq!(eval("(sort [3, 1, 2])"), "[1, 2, 3]");
    assert_eq!(eval("(sort [\"b\", \"a\"])"), "[\"a\", \"b\"]");
    assert_eq!(eval("(range 0 4)"), "[0, 1, 2, 3]");
    assert_eq!(eval("(range 3 1)"), "[]");
    assert_eq!(eval("(zipWith add [1, 2, 3] [10, 20])"), "[11, 22]");
    assert_eq!(eval("(transpose [[1, 2, 3], [4, 5]])"), "[[1, 4], [2, 5]]");
    assert_eq!(eval("(transpose [])"), "[]");
    assert_eq!(eval("(sum [1, 2, 3.5])"), "6.5");
    assert_eq!(
        eval("[(take 2 [1, 2, 3]), (drop 2 [1, 2, 3]), (take 9 [1])]"),
        "[[1, 2], [3], [1]]"
    );
}

#[test]
fn printing() {
    assert_eq!(
        output(r#"(print "a" 1 [2] "\n") (println "b" true)"#),
        "a1[2]\nbtrue\n"
    );
}

#[test]
fn closures_capture_their_environment() {
    assert_eq!(
        eval(r"adder = \n. \x. (add n x)  (map (adder 10) [1, 2])"),
        "[11, 12]"
    );
    assert_eq!(
        eval(r"compose = \f. \g. \x. (f (g x))  ((compose (mul 2) (add 1)) 3)"),
        "8"
    );
}

#[test]
fn partial_and_over_application() {
    assert_eq!(eval(r"k = \x. \y. x  ((k 1) 2)"), "1");
    assert_eq!(eval(r"id = \x. x  (id add 1 2)"), "3");
    assert_eq!(eval(r"(map (if true) [1]) "), "[<function>]");
    assert_eq!(
        eval(r"pair = \x. \y. \f. (f x y)  ((pair 1 2) \a. \b. b)"),
        "2"
    );
}

#[test]
fn church_encodings_still_work() {
    let text = r"
        cTrue = \x. \y. x
        cFalse = \x. \y. y
        cIf = \c. \t. \e. (c t e)
        (cIf cFalse 1 2)";
    assert_eq!(eval(text), "2");
}

#[test]
fn recursion_and_mutual_recursion() {
    assert_eq!(
        eval(r"fib = \n. (if (lt n 2) n (add (fib (sub n 1)) (fib (sub n 2))))  (fib 20)"),
        "6765"
    );
    assert_eq!(
        eval(
            r"even = \n. (if (eq n 0) true (odd (sub n 1)))  odd = \n. (if (eq n 0) false (even (sub n 1)))  (even 101)"
        ),
        "false"
    );
}

#[test]
fn local_recursive_let() {
    assert_eq!(
        eval(r"let go = \n. \acc. (if (eq n 0) acc (go (sub n 1) (add acc n))) in (go 100 0)"),
        "5050"
    );
}

#[test]
fn deep_tail_recursion_runs_in_constant_stack() {
    assert_eq!(
        eval(r"loop = \n. (if (eq n 0) 0 (loop (sub n 1)))  (loop 1000000)"),
        "0"
    );
}

#[test]
fn deep_non_tail_recursion_does_not_overflow() {
    assert_eq!(
        eval(r"sumTo = \n. (if (eq n 0) 0 (add n (sumTo (sub n 1))))  (sumTo 100000)"),
        "5000050000"
    );
}

#[test]
fn arguments_are_lazy() {
    // The unused argument would fail if evaluated.
    assert_eq!(eval(r#"const = \x. \y. x  (const 1 (error "boom"))"#), "1");
}

#[test]
fn list_elements_are_lazy() {
    // Each unused element would fail if evaluated.
    assert_eq!(eval(r#"(head [1, (error "boom")])"#), "1");
    assert_eq!(eval(r#"(length [(error "a"), (error "b")])"#), "2");
    assert_eq!(eval(r#"(get 1 (cons (error "a") [2]))"#), "2");
    assert_eq!(eval(r#"(head (push (error "a") [1]))"#), "1");
    assert_eq!(eval(r#"(head (reverse [(error "a"), 2]))"#), "2");
    assert_eq!(eval(r#"(length (map (error "a") [1, 2]))"#), "2");
    assert_eq!(eval(r#"(get 1 (map (div 6) [(error "a"), 3]))"#), "2");
    assert_eq!(eval(r#"(head (zipWith add [1, (error "a")] [2, 3]))"#), "3");
    assert_eq!(eval(r#"(length (filter (\x. true) [(error "a")]))"#), "1");
    assert_eq!(
        eval(r#"(head (transpose [[1, (error "a")], [2]]))"#),
        "[1, 2]"
    );
}

#[test]
fn foldr_stops_when_its_function_ignores_the_rest() {
    let program = r#"(foldr \x. \acc. (or (eq x 1) acc) false [1, (error "never needed")])"#;
    assert_eq!(eval(program), "true");
    assert_eq!(output(r#"(println (head [1, (error "boom")]))"#), "1\n");
    // A function that uses the rest still sees every element, in order.
    assert_eq!(
        eval("(foldr (\\x. \\acc. (concat x acc)) \"\" [\"a\", \"b\"])"),
        "\"ab\""
    );
    assert_eq!(eval("(foldr add 0 (range 0 100000))"), "4999950000");
}

#[test]
fn inspecting_a_list_evaluates_its_elements() {
    assert_eq!(runtime_error(r#"[1, (error "shown")]"#).message, "shown");
    assert_eq!(
        runtime_error(r#"(sum [1, (error "added")])"#).message,
        "added"
    );
    assert_eq!(
        runtime_error(r#"(eq [[1]] [[(error "compared")]])"#).message,
        "compared"
    );
    assert_eq!(
        output("(println (map (map (add 1)) [[1, 2], [3]]))"),
        "[[2, 3], [4]]\n"
    );
    assert_eq!(eval("(sort (map neg [1, 3, 2]))"), "[-3, -2, -1]");
}

#[test]
fn effects_run_only_when_performed() {
    // Neither the definition nor the unused binding is ever performed.
    assert_eq!(
        output(
            r#"x = (print "never")
               (let unused = (println "never either") in (println "only this"))"#
        ),
        "only this\n"
    );
    // Building a list of actions performs none of them.
    assert_eq!(eval(r#"(length [(println "a"), (println "b")])"#), "2");
    assert_eq!(
        output(r#"x = (readFile "/definitely/missing")  (println "ok")"#),
        "ok\n"
    );
}

#[test]
fn an_action_runs_each_time_it_is_performed() {
    assert_eq!(
        output(r#"x = (println "x")  y = (println "never")  (then x x)"#),
        "x\nx\n"
    );
}

#[test]
fn let_binds_the_results_of_actions() {
    assert_eq!(
        output(
            r#"let a <- (pure 2) in
               let b = (mul a 10) in
               let _ <- (print "a = " a ", ") in
               (println "b = " b)"#
        ),
        "a = 2, b = 20\n"
    );
    // The action is not in scope of its own binder.
    assert_eq!(
        output(r#"x = 1  (let x <- (pure (add x 1)) in (println x))"#),
        "2\n"
    );
    // The result of `pure` is passed on without being evaluated.
    assert_eq!(
        output(r#"let x <- (pure (error "boom")) in (println "fine")"#),
        "fine\n"
    );
}

#[test]
fn sequence_runs_a_list_of_actions_in_order() {
    assert_eq!(
        output(r#"(sequence [(print "a"), (print "b"), (println "c")])"#),
        "abc\n"
    );
    assert_eq!(
        eval(r#"(sequence [(pure 1), (pure 2), (bind (pure 3) \x. (pure (mul x 10)))])"#),
        "[1, 2, 30]"
    );
    assert_eq!(eval("(sequence [])"), "[]");
}

#[test]
fn performed_results_are_returned_with_their_types() {
    let (mut session, _) = session();
    let evaluated = session
        .eval("<test>", r"(bind (pure 20) \x. (pure (add x 1)))")
        .unwrap();
    assert_eq!(evaluated.values[0].0.show(), "21");
    assert_eq!(evaluated.values[0].1.to_string(), "Number");
    assert_eq!(eval("[(pure 1)]"), "[<action>]");
}

#[test]
fn recursive_action_loops_run_in_constant_space() {
    let (mut session, out) = session();
    session.set_max_frames(100);
    let program = r#"
        loop = \n. (if (eq n 0) (pure 0) (then (print "") (loop (sub n 1))))
        count = \n. (if (eq n 0) (pure ()) (let x <- (pure n) in (count (sub x 1))))
        (then (loop 100000) (count 100000))
        (sequence (map \x. (print "") (range 0 100000)))
        (println "done")
    "#;
    if let Err(diagnostics) = session.eval("<test>", program) {
        panic!("{}", render(&session, &diagnostics));
    }
    assert_eq!(out.contents(), "done\n");
}

#[test]
fn runtime_errors_point_at_the_call() {
    let error = runtime_error("x = (head [])\nx");
    assert_eq!(error.message, "`head` of an empty list");
    assert_eq!(error.span.unwrap().range(), 4..13);

    assert_eq!(
        runtime_error("(get 5 [1])").message,
        "index 5 is out of bounds for a list of length 1"
    );
    assert_eq!(
        runtime_error("(get -1 [1])").message,
        "-1 is not a valid index (expected a whole number >= 0)"
    );
    assert_eq!(
        runtime_error("(remove 1 [1])").message,
        "index 1 is out of bounds for a list of length 1"
    );
    assert_eq!(
        runtime_error("(parseNumber \"abc\")").message,
        "cannot parse \"abc\" as a number"
    );
    assert_eq!(
        runtime_error("(split \"\" \"abc\")").message,
        "the delimiter given to `split` must not be empty"
    );
    assert_eq!(runtime_error("(error \"custom\")").message, "custom");
    assert_eq!(
        runtime_error(r"(eq \x. x \y. y)").message,
        "functions cannot be compared"
    );
    assert_eq!(
        runtime_error("(eq (pure 1) (pure 1))").message,
        "actions cannot be compared"
    );
    assert!(
        runtime_error("(readFile \"/definitely/missing\")")
            .message
            .starts_with("cannot read")
    );
}

#[test]
fn self_dependent_values_are_detected() {
    let error = runtime_error("x = (add x 1)\nx");
    assert_eq!(
        error.message,
        "this value depends on itself, so it can never be computed"
    );
}

#[test]
fn definitions_persist_across_inputs() {
    let (mut session, out) = session();
    session.eval("<1>", r"double = \x. (mul x 2)").unwrap();
    let evaluated = session.eval("<2>", "(double 21)").unwrap();
    assert_eq!(evaluated.values[0].0.show(), "42");
    assert_eq!(evaluated.values[0].1.to_string(), "Number");
    // Redefinition shadows; earlier users keep the old definition.
    session.eval("<3>", "useDouble = (double 1)").unwrap();
    session.eval("<4>", r"double = \x. x").unwrap();
    session
        .eval("<5>", r"(println useDouble (double 1))")
        .unwrap();
    assert_eq!(out.contents(), "21\n");
}

#[test]
fn errors_leave_the_session_usable() {
    let (mut session, _) = session();
    session.eval("<1>", "x = 1").unwrap();
    assert!(session.eval("<2>", "(head [])").is_err());
    assert!(session.eval("<3>", "y = (add x \"s\")").is_err());
    assert_eq!(session.eval("<4>", "x").unwrap().values[0].0.show(), "1");
    assert!(
        session.eval("<5>", "y").is_err(),
        "a failed definition is not added"
    );
}
