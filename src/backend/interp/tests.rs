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
    assert_eq!(eval("(sub 1.0 2.5)"), "-1.5");
    assert_eq!(eval("(mul 3.0 (fdiv 1.0 4.0))"), "0.75");
    assert_eq!(eval("(fdiv 1.0 0.0)"), "Infinity");
    assert_eq!(eval("[(div 7 2), (div -7 2), (div 7 -2)]"), "[3, -4, -4]");
    assert_eq!(eval("[(mod 7 3), (mod -7 3), (mod 7 -3)]"), "[1, 2, -2]");
    assert_eq!(
        eval("[(pow 2 10), (neg 3), (abs -3), (pow -3 3)]"),
        "[1024, -3, 3, -27]"
    );
    assert_eq!(
        eval("[(sqrt 16.0), (abs -0.5), (toFloat 3)]"),
        "[4.0, 0.5, 3.0]"
    );
    assert_eq!(
        eval("[9007199254740993, -9223372036854775808]"),
        "[9007199254740993, -9223372036854775808]"
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
        "[True, False, True, True, False, False]"
    );
    assert_eq!(
        eval("[(eq [1, 2] [1, 2]), (lt \"abc\" \"abd\"), (lt [1] [1, 0]), (lt False True)]"),
        "[True, True, True, True]"
    );
    assert_eq!(
        eval("[(compare 1 2), (compare 2 2), (compare 3 2)]"),
        "[-1, 0, 1]"
    );
}

#[test]
fn booleans_are_lazy() {
    assert_eq!(eval("(if True 1 (error \"no\"))"), "1");
    assert_eq!(eval("(and False (error \"no\"))"), "False");
    assert_eq!(eval("(or True (error \"no\"))"), "True");
    assert_eq!(
        eval("[(not True), (and True True), (or False False)]"),
        "[False, True, False]"
    );
}

#[test]
fn strings() {
    assert_eq!(eval("(concat \"ab\" \"cd\")"), "\"abcd\"");
    assert_eq!(eval("(show [\"a\", \"b\"])"), r#""[\"a\", \"b\"]""#);
    assert_eq!(eval("(parseInt \" 42 \")"), "(Ok 42)");
    assert_eq!(eval("(parseFloat \" 42.5 \")"), "(Ok 42.5)");
    assert_eq!(
        eval("(parseInt \"abc\")"),
        r#"(Err "cannot parse \"abc\" as an `Int`")"#
    );
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
    assert_eq!(eval("[(isEmpty []), (isEmpty [1])]"), "[True, False]");
    assert_eq!(
        eval("[(head [1, 2]), (get 1 [1, 2]), (head []), (get 2 [1, 2])]"),
        "[(Some 1), (Some 2), None, None]"
    );
    assert_eq!(
        eval("[(tail [1, 2, 3]), (tail [])]"),
        "[(Some [2, 3]), None]"
    );
    assert_eq!(
        eval("[(remove 0 [1, 2]), (remove -1 [1]), (remove 1 [1])]"),
        "[[2], [1], [1]]"
    );
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
    assert_eq!(eval("(sum [1, 2, 3])"), "6");
    assert_eq!(eval("(foldl add 0.0 [1.0, 2.5])"), "3.5");
    assert_eq!(
        eval("[(take 2 [1, 2, 3]), (drop 2 [1, 2, 3]), (take 9 [1])]"),
        "[[1, 2], [3], [1]]"
    );
}

#[test]
fn printing() {
    assert_eq!(
        output(r#"(print "a" 1 [2] "\n") (println "b" True)"#),
        "a1[2]\nbTrue\n"
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
    assert_eq!(eval(r"(map (if True) [1]) "), "[<function>]");
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
            r"even = \n. (if (eq n 0) True (odd (sub n 1)))  odd = \n. (if (eq n 0) False (even (sub n 1)))  (even 101)"
        ),
        "False"
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
    assert_eq!(eval(r#"(unwrap (head [1, (error "boom")]))"#), "1");
    assert_eq!(eval(r#"(length [(error "a"), (error "b")])"#), "2");
    assert_eq!(eval(r#"(unwrap (get 1 (cons (error "a") [2])))"#), "2");
    assert_eq!(eval(r#"(unwrap (head (push (error "a") [1])))"#), "1");
    assert_eq!(eval(r#"(unwrap (head (reverse [(error "a"), 2])))"#), "2");
    assert_eq!(eval(r#"(length (map (error "a") [1, 2]))"#), "2");
    assert_eq!(
        eval(r#"(unwrap (get 1 (map (div 6) [(error "a"), 3])))"#),
        "2"
    );
    assert_eq!(
        eval(r#"(unwrap (head (zipWith add [1, (error "a")] [2, 3])))"#),
        "3"
    );
    assert_eq!(eval(r#"(length (filter (\x. True) [(error "a")]))"#), "1");
    assert_eq!(
        eval(r#"(unwrap (head (transpose [[1, (error "a")], [2]])))"#),
        "[1, 2]"
    );
}

#[test]
fn foldr_stops_when_its_function_ignores_the_rest() {
    let program = r#"(foldr \x. \acc. (or (eq x 1) acc) False [1, (error "never needed")])"#;
    assert_eq!(eval(program), "True");
    assert_eq!(
        output(r#"(println (unwrap (head [1, (error "boom")])))"#),
        "1\n"
    );
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
    assert_eq!(evaluated.values[0].1.to_string(), "Int");
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
    let error = runtime_error("x = (unwrap (head []))\nx");
    assert_eq!(error.message, "`unwrap` of `None`");
    assert_eq!(error.span.unwrap().range(), 4..22);

    assert_eq!(
        runtime_error("(take -1 [1])").message,
        "-1 is not a valid index (expected a number >= 0)"
    );
    assert_eq!(
        runtime_error("(unwrapOk (parseInt \"abc\"))").message,
        "`unwrapOk` of an `Err`: cannot parse \"abc\" as an `Int`"
    );
    assert_eq!(
        runtime_error("(unwrapOk (Err [\"a\", (concat \"b\" \"c\")]))").message,
        "`unwrapOk` of an `Err`: [\"a\", \"bc\"]"
    );
    assert_eq!(
        runtime_error("(split \"\" \"abc\")").message,
        "the delimiter given to `split` must not be empty"
    );
    assert_eq!(runtime_error("(error \"custom\")").message, "custom");
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
    assert_eq!(evaluated.values[0].1.to_string(), "Int");
    // Redefinition shadows; earlier users keep the old definition.
    session.eval("<3>", "useDouble = (double 1)").unwrap();
    session.eval("<4>", r"double = \x. x").unwrap();
    session
        .eval("<5>", r"(println useDouble (double 1))")
        .unwrap();
    assert_eq!(out.contents(), "21\n");
}

#[test]
fn redefined_types_are_new_types() {
    let (mut session, out) = session();
    session.eval("<1>", "type T = A | B\nold = A").unwrap();
    session.eval("<2>", "type T = C Int\nnew = (C 1)").unwrap();
    // The old value still has the old type, so it cannot pass for the new one.
    let errors = session.eval("<3>", "same = (eq old new)").unwrap_err();
    assert_eq!(errors[0].message, "mismatched types");
    session.eval("<4>", "(println old \" \" new)").unwrap();
    assert_eq!(out.contents(), "A (C 1)\n");
    assert!(
        session.eval("<5>", "x = A").is_ok(),
        "old constructors stay usable"
    );
}

#[test]
fn errors_leave_the_session_usable() {
    let (mut session, _) = session();
    session.eval("<1>", "x = 1").unwrap();
    assert!(session.eval("<2>", "(unwrap (head []))").is_err());
    assert!(session.eval("<3>", "y = (add x \"s\")").is_err());
    assert_eq!(session.eval("<4>", "x").unwrap().values[0].0.show(), "1");
    assert!(
        session.eval("<5>", "y").is_err(),
        "a failed definition is not added"
    );
}

const SHAPES: &str = "type Shape = Circle Int | Rect Int Int\n";

#[test]
fn constructors_build_data_and_match_takes_it_apart() {
    let program = format!(
        "{SHAPES}area = \\s. match s | Circle r -> (mul 3 (mul r r)) | Rect w h -> (mul w h)\n\
         (map area [(Circle 1), (Rect 2 3)])"
    );
    assert_eq!(eval(&program), "[3, 6]");
    assert_eq!(
        eval(&format!("{SHAPES}(map Circle [1, 2])")),
        "[(Circle 1), (Circle 2)]"
    );
    assert_eq!(eval(&format!("{SHAPES}((Rect 2) 3)")), "(Rect 2 3)");
    assert_eq!(
        eval("[None, (Some (Some \"x\"))]"),
        r#"[None, (Some (Some "x"))]"#
    );
}

#[test]
fn nested_patterns_and_literals() {
    let program = "f = \\o. match o\n\
         | Some (Some 0) -> \"zero\"\n\
         | Some (Some n) -> (show n)\n\
         | Some None -> \"inner none\"\n\
         | None -> \"none\"\n\
         [(f (Some (Some 0))), (f (Some (Some 7))), (f (Some None)), (f None)]";
    assert_eq!(eval(program), r#"["zero", "7", "inner none", "none"]"#);
    let strings = "f = \\s. match s | \"a\" -> 1 | \"b\" -> 2 | other -> (stringLength other)\n\
                   [(f \"a\"), (f \"b\"), (f \"cde\")]";
    assert_eq!(eval(strings), "[1, 2, 3]");
    assert_eq!(eval("match () | () -> 1"), "1");
}

#[test]
fn the_first_matching_arm_wins() {
    let program = "f = \\o. match o | Some 1 -> \"one\" | Some _ -> \"some\" | _ -> \"other\"\n\
         [(f (Some 1)), (f (Some 2)), (f None)]";
    assert_eq!(eval(program), r#"["one", "some", "other"]"#);
}

#[test]
fn fields_are_lazy() {
    let program = "type Pair<a, b> = Pair a b\n\
         first = \\p. match p | Pair x _ -> x\n\
         (first (Pair 1 (error \"never needed\")))";
    assert_eq!(eval(program), "1");
    let program = "match (Some (error \"boom\")) | Some _ -> \"fine\" | None -> \"no\"";
    assert_eq!(eval(program), r#""fine""#);
}

#[test]
fn matching_forces_only_what_the_first_possible_arm_needs() {
    // The first arm only needs the first field; it matches, so the second
    // field is never evaluated even though a later arm tests it.
    let program = "type Pair<a, b> = Pair a b\n\
                   f = \\p. match p | Pair True _ -> 1 | Pair _ True -> 2 | Pair _ _ -> 3\n\
                   (f (Pair True (error \"not needed\")))";
    assert_eq!(eval(program), "1");
}

#[test]
fn data_compares_by_constructor_then_fields() {
    let program =
        "[(eq (Some 1) (Some 1)), (eq (Some 1) None), (lt None (Some 0)), (lt (Some 1) (Some 2))]";
    assert_eq!(eval(program), "[True, False, True, True]");
    assert_eq!(
        eval("(sort [(Some 2), None, (Some 1)])"),
        "[None, (Some 1), (Some 2)]"
    );
    assert_eq!(eval("(compare False True)"), "-1");
}

#[test]
fn recursive_data_and_tail_calls_through_match() {
    let program = "type Tree<a> = Leaf | Node Tree<a> a Tree<a>\n\
                   insert = \\x. \\t. match t\n\
                   | Leaf -> (Node Leaf x Leaf)\n\
                   | Node l v r -> (if (lt x v) (Node (insert x l) v r) (Node l v (insert x r)))\n\
                   toList = \\t. match t | Leaf -> [] | Node l v r -> (append (toList l) (cons v (toList r)))\n\
                   (toList (foldl \\t. \\x. (insert x t) Leaf [5, 3, 8, 1, 4]))";
    assert_eq!(eval(program), "[1, 3, 4, 5, 8]");
    // A loop written with `match` runs in constant space.
    let program = "type Nat = Zero | Succ Nat\n\
                   count = \\n. \\acc. match n | Zero -> acc | Succ m -> (count m (add acc 1))\n\
                   build = \\k. \\acc. (if (eq k 0) acc (build (sub k 1) (Succ acc)))\n\
                   (count (build 1000000 Zero) 0)";
    assert_eq!(eval(program), "1000000");
}

#[test]
fn deep_data_shows_compares_and_drops_without_overflow() {
    let program = "type Nat = Zero | Succ Nat\n\
                   build = \\k. \\acc. (if (eq k 0) acc (build (sub k 1) (Succ acc)))\n\
                   n = (build 200000 Zero)\n\
                   [(eq n n), (gt (Succ n) n), (eq (stringLength (show n)) 1400004)]";
    assert_eq!(eval(program), "[True, True, True]");
}

#[test]
fn options_and_results() {
    assert_eq!(
        eval("[(withDefault 0 (head [1])), (withDefault 0 (head []))]"),
        "[1, 0]"
    );
    // The fallback is only evaluated for `None`, and the mapped function
    // only when the new value is used.
    assert_eq!(eval(r#"(withDefault (error "boom") (Some 1))"#), "1");
    assert_eq!(
        eval(r#"[(mapOption (add 1) (Some 1)), (mapOption (add 1) None)]"#),
        "[(Some 2), None]"
    );
    assert_eq!(
        eval(r#"match (mapOption (error "boom") (Some 1)) | Some _ -> 1 | None -> 0"#),
        "1"
    );
    assert_eq!(
        eval(r#"[(mapResult (mul 2) (parseInt "21")), (mapResult (mul 2) (parseInt "x"))]"#),
        r#"[(Ok 42), (Err "cannot parse \"x\" as an `Int`")]"#
    );
    assert_eq!(
        eval(r#"[(toOption (parseInt "1")), (toOption (parseInt "x"))]"#),
        "[(Some 1), None]"
    );
    assert_eq!(eval(r#"(unwrapOk (parseFloat "2.5"))"#), "2.5");
    assert_eq!(eval("match (head [1, 2]) | Some x -> x | None -> 0"), "1");
    // Built-in data compares like declared data, constructor by constructor.
    assert_eq!(
        eval("[(lt None (Some 0)), (lt (Ok 1) (Err 0)), (eq (Some [1]) (head [[1]]))]"),
        "[True, True, True]"
    );
}
