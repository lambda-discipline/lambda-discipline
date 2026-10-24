//! The primitive operations every program can use without a `use`.
//!
//! Builtins are declared once here as a backend-agnostic enum, the way GHC
//! declares its primops: the name, type signature, module and documentation
//! live with the declaration, the type checker reads the signatures, and each
//! backend provides its own implementation by matching on [`Builtin`].
//!
//! Naming follows one convention: lowerCamelCase, and the data a function
//! works on comes last, so partial application reads naturally
//! (`(map (add 1))`, `(split ",")`, `(get 0)`). The exception is the source
//! location constants (`__FILE__`, `__DIR__`, `__LINE__`), which are spelled
//! as in C, OCaml and Elixir because their value depends on where they are
//! written.

use std::fmt;

/// Groups builtins for documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Module {
    Number,
    Compare,
    Bool,
    String,
    List,
    Tuple,
    Option,
    Io,
    Source,
}

impl Module {
    pub const ALL: [Module; 9] = [
        Module::Number,
        Module::Compare,
        Module::Bool,
        Module::String,
        Module::List,
        Module::Tuple,
        Module::Option,
        Module::Io,
        Module::Source,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Module::Number => "Numbers",
            Module::Compare => "Comparison",
            Module::Bool => "Booleans",
            Module::String => "Strings",
            Module::List => "Lists",
            Module::Tuple => "Tuples",
            Module::Option => "Options and results",
            Module::Io => "Input/output",
            Module::Source => "Source location",
        }
    }
}

macro_rules! builtins {
    ($( $variant:ident => $name:literal : $sig:literal, $module:ident, $doc:literal; )*) => {
        /// A primitive operation.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Builtin {
            $( $variant, )*
        }

        impl Builtin {
            /// Every builtin, in documentation order.
            pub const ALL: &'static [Builtin] = &[ $( Builtin::$variant, )* ];

            /// The name programs use to refer to it.
            pub fn name(self) -> &'static str {
                match self { $( Builtin::$variant => $name, )* }
            }

            /// The type signature in source syntax, with an optional context
            /// (`Eq<a> => ...`). Lowercase names are type variables,
            /// implicitly quantified.
            pub fn signature(self) -> &'static str {
                match self { $( Builtin::$variant => $sig, )* }
            }

            pub fn module(self) -> Module {
                match self { $( Builtin::$variant => Module::$module, )* }
            }

            pub fn doc(self) -> &'static str {
                match self { $( Builtin::$variant => $doc, )* }
            }
        }
    };
}

builtins! {
    Add => "add" : "Num<a> => a -> a -> a", Number, "Sum of two numbers. `Int` arithmetic that overflows 64 bits is an error.";
    Sub => "sub" : "Num<a> => a -> a -> a", Number, "`(sub a b)` is `a - b`.";
    Mul => "mul" : "Num<a> => a -> a -> a", Number, "Product of two numbers.";
    Neg => "neg" : "Num<a> => a -> a", Number, "Negation.";
    Abs => "abs" : "Num<a> => a -> a", Number, "Absolute value.";
    Pow => "pow" : "Num<a> => a -> Int -> a", Number, "`(pow base exponent)`; the exponent must not be negative.";
    Div => "div" : "Int -> Int -> Int", Number, "`(div a b)` is `a / b` rounded toward negative infinity, like Haskell's `div`; dividing by zero is an error.";
    Mod => "mod" : "Int -> Int -> Int", Number, "Remainder of `div`, with the sign of the divisor, like Haskell's `mod`.";
    Fdiv => "fdiv" : "Float -> Float -> Float", Number, "`(fdiv a b)` is `a / b` (IEEE: dividing by zero gives infinity).";
    Sqrt => "sqrt" : "Float -> Float", Number, "Square root.";
    ToFloat => "toFloat" : "Int -> Float", Number, "The `Float` nearest to an `Int`.";
    Floor => "floor" : "Float -> Int", Number, "Largest integer not greater than the argument; an error for `NaN`, infinities and values beyond `Int`.";
    Ceil => "ceil" : "Float -> Int", Number, "Smallest integer not less than the argument.";
    Round => "round" : "Float -> Int", Number, "Nearest integer, halves away from zero.";
    Min => "min" : "Ord<a> => a -> a -> a", Number, "The smaller of two values; the first if they are equal.";
    Max => "max" : "Ord<a> => a -> a -> a", Number, "The larger of two values; the second if they are equal.";

    Eq => "eq" : "Eq<a> => a -> a -> Bool", Compare, "Structural equality. Every type without functions or `IO` actions in it belongs to `Eq`.";
    Neq => "neq" : "Eq<a> => a -> a -> Bool", Compare, "Structural inequality.";
    Lt => "lt" : "Ord<a> => a -> a -> Bool", Compare, "`(lt a b)` is `a < b`. Every type in `Eq` is also in `Ord`: data is ordered by constructor, in declaration order, then field by field, and lists element by element.";
    Le => "le" : "Ord<a> => a -> a -> Bool", Compare, "`(le a b)` is `a <= b`.";
    Gt => "gt" : "Ord<a> => a -> a -> Bool", Compare, "`(gt a b)` is `a > b`.";
    Ge => "ge" : "Ord<a> => a -> a -> Bool", Compare, "`(ge a b)` is `a >= b`.";
    Compare => "compare" : "Ord<a> => a -> a -> Int", Compare, "`-1`, `0` or `1` as `a` is less than, equal to or greater than `b`.";

    Not => "not" : "Bool -> Bool", Bool, "Boolean negation.";
    And => "and" : "Bool -> Bool -> Bool", Bool, "Conjunction; the second argument is only evaluated when needed.";
    Or => "or" : "Bool -> Bool -> Bool", Bool, "Disjunction; the second argument is only evaluated when needed.";
    If => "if" : "Bool -> a -> a -> a", Bool, "`(if c t e)` evaluates only the chosen branch.";

    Concat => "concat" : "String -> String -> String", String, "Joins two strings.";
    Show => "show" : "Show<a> => a -> String", String, "Renders a value as source-like text (strings are quoted). Every type without functions or `IO` actions in it belongs to `Show`.";
    ParseInt => "parseInt" : "String -> Result<String, Int>", String, "Parses an integer such as `-42`, ignoring surrounding whitespace; `Err` with a message when the text is not one.";
    ParseFloat => "parseFloat" : "String -> Result<String, Float>", String, "Parses a number such as `3.5`, `2` or `1e-3`, ignoring surrounding whitespace; `Err` with a message when the text is not one.";
    StringLength => "stringLength" : "String -> Int", String, "Number of characters in a string.";
    Chars => "chars" : "String -> List<String>", String, "Splits a string into its characters.";
    Split => "split" : "String -> String -> List<String>", String, "`(split delimiter text)`; the delimiter must not be empty.";
    Lines => "lines" : "String -> List<String>", String, "Splits text into lines, accepting `\\n` and `\\r\\n`.";
    Join => "join" : "String -> List<String> -> String", String, "`(join separator parts)`.";
    Trim => "trim" : "String -> String", String, "Removes leading and trailing whitespace.";

    Length => "length" : "List<a> -> Int", List, "Number of elements.";
    IsEmpty => "isEmpty" : "List<a> -> Bool", List, "Whether the list has no elements.";
    Head => "head" : "List<a> -> Option<a>", List, "First element, or `None` for an empty list.";
    Tail => "tail" : "List<a> -> Option<List<a>>", List, "Everything but the first element, or `None` for an empty list. `(drop 1 xs)` gives the empty list instead.";
    Get => "get" : "Int -> List<a> -> Option<a>", List, "`(get index list)`, zero-based, or `None` when the index is out of bounds.";
    Cons => "cons" : "a -> List<a> -> List<a>", List, "Prepends an element.";
    Push => "push" : "a -> List<a> -> List<a>", List, "Appends an element at the end.";
    Remove => "remove" : "Int -> List<a> -> List<a>", List, "`(remove index list)` without the element at `index`; the list unchanged when the index is out of bounds, like `take` and `drop`.";
    Append => "append" : "List<a> -> List<a> -> List<a>", List, "Concatenates two lists.";
    Map => "map" : "(a -> b) -> List<a> -> List<b>", List, "Applies a function to every element.";
    Filter => "filter" : "(a -> Bool) -> List<a> -> List<a>", List, "Keeps the elements satisfying a predicate.";
    Foldl => "foldl" : "(b -> a -> b) -> b -> List<a> -> b", List, "Left fold: `(foldl f z [x, y])` is `(f (f z x) y)`.";
    Foldr => "foldr" : "(a -> b -> b) -> b -> List<a> -> b", List, "Right fold: `(foldr f z [x, y])` is `(f x (f y z))`, where each inner fold is evaluated only if `f` uses it.";
    Reverse => "reverse" : "List<a> -> List<a>", List, "Reverses a list.";
    Sort => "sort" : "Ord<a> => List<a> -> List<a>", List, "Stable ascending sort using the same ordering as `lt`.";
    Range => "range" : "Int -> Int -> List<Int>", List, "`(range from to)` counts from `from` up to, not including, `to`.";
    ZipWith => "zipWith" : "(a -> b -> c) -> List<a> -> List<b> -> List<c>", List, "Combines two lists element-wise, stopping at the shorter.";
    Transpose => "transpose" : "List<List<a>> -> List<List<a>>", List, "Turns rows into columns, truncated to the shortest row.";
    Sum => "sum" : "List<Int> -> Int", List, "Sum of all elements. For floats, use `(foldl add 0.0 xs)`.";
    Take => "take" : "Int -> List<a> -> List<a>", List, "The first `n` elements.";
    Drop => "drop" : "Int -> List<a> -> List<a>", List, "All but the first `n` elements.";

    Fst => "fst" : "(a, b) -> a", Tuple, "The first component of a pair.";
    Snd => "snd" : "(a, b) -> b", Tuple, "The second component of a pair.";

    WithDefault => "withDefault" : "a -> Option<a> -> a", Option, "`(withDefault fallback option)` is the value in `Some`, or `fallback` for `None`; `fallback` is only evaluated when needed.";
    MapOption => "mapOption" : "(a -> b) -> Option<a> -> Option<b>", Option, "Applies a function to the value in `Some`; `None` stays `None`.";
    Unwrap => "unwrap" : "Option<a> -> a", Option, "The value in `Some`; an error for `None`. For scripts that know the value is there.";
    MapResult => "mapResult" : "(a -> b) -> Result<e, a> -> Result<e, b>", Option, "Applies a function to the value in `Ok`; an `Err` stays as it is.";
    ToOption => "toOption" : "Result<e, a> -> Option<a>", Option, "`Some` of the value in `Ok`, or `None` for an `Err`, dropping the error.";
    UnwrapOk => "unwrapOk" : "Show<e> => Result<e, a> -> a", Option, "The value in `Ok`; for an `Err`, an error that shows it (a string without quotes). For scripts that know the value is there.";

    Pure => "pure" : "a -> IO<a>", Io, "An action that does nothing and produces its argument.";
    Bind => "bind" : "IO<a> -> (a -> IO<b>) -> IO<b>", Io, "`(bind action f)` runs `action`, then the action `f` returns for its result. `let x <- action in body` is `(bind action \\x. body)`.";
    Then => "then" : "IO<a> -> IO<b> -> IO<b>", Io, "`(then first second)` runs `first`, discards its result, then runs `second`.";
    Sequence => "sequence" : "List<IO<a>> -> IO<List<a>>", Io, "Runs a list of actions in order and collects their results.";
    Print => "print" : "String -> IO<Unit>", Io, "An action that prints a string. Build other output with `show`, `concat` and `join`: `(println (concat \"total: \" (show n)))`.";
    Println => "println" : "String -> IO<Unit>", Io, "Like `print`, followed by a newline.";
    ReadFile => "readFile" : "String -> IO<String>", Io, "An action that reads a whole file. A relative path is relative to the working directory, like a path typed on the command line; start it with `__DIR__` to read a file that sits next to the program.";
    Error => "error" : "String -> a", Io, "Stops the program with an error message when it is evaluated. Not an action: it can occur anywhere.";

    File => "__FILE__" : "String", Source, "The path of the source file this name is written in; `<repl>` in the REPL.";
    Dir => "__DIR__" : "String", Source, "The directory of the source file this name is written in, which `use` paths are relative to; `.` in the REPL. `(readFile (concat __DIR__ \"/data.txt\"))` reads the `data.txt` next to the program from any working directory.";
    Line => "__LINE__" : "Int", Source, "The line this name is written on, counting from 1.";
}

impl Builtin {
    pub fn lookup(name: &str) -> Option<Builtin> {
        Builtin::ALL.iter().copied().find(|b| b.name() == name)
    }

    /// Builtins that build an `IO` action. Applying one only describes the
    /// action; it runs when the program performs it.
    pub fn is_action(self) -> bool {
        matches!(
            self,
            Builtin::Pure
                | Builtin::Bind
                | Builtin::Then
                | Builtin::Sequence
                | Builtin::Print
                | Builtin::Println
                | Builtin::ReadFile
        )
    }

    /// Number of arguments the primitive consumes before it runs. Constants
    /// such as `true` have arity 0.
    pub fn arity(self) -> usize {
        static ARITIES: std::sync::OnceLock<Vec<usize>> = std::sync::OnceLock::new();
        ARITIES.get_or_init(|| Builtin::ALL.iter().map(|b| b.count_arrows()).collect())
            [self as usize]
    }

    fn count_arrows(self) -> usize {
        // Count top-level arrows in the signature after its context,
        // ignoring parenthesized ones.
        let signature = self.signature();
        let ty = signature.split_once("=>").map_or(signature, |(_, ty)| ty);
        let mut depth = 0i32;
        let mut arrows = 0;
        let bytes = ty.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            match b {
                b'(' | b'<' => depth += 1,
                b'>' if i > 0 && bytes[i - 1] == b'-' => {
                    if depth == 0 {
                        arrows += 1;
                    }
                }
                b')' | b'>' => depth -= 1,
                _ => {}
            }
        }
        arrows
    }
}

impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Renders the reference documentation printed by `lbd docs`.
pub fn reference() -> String {
    let mut out = String::new();
    for module in Module::ALL {
        out.push_str(&format!("-- {}\n", module.title()));
        if module == Module::Bool {
            out.push_str("type Bool = False | True\n    Booleans are data: `match` on `True` and `False`, or use `if`.\n");
        }
        if module == Module::Tuple {
            out.push_str("type (a, b)\n    Tuples group 2 to 7 values of any types: `(1, \"one\")` has type `(Int, String)`. Take one apart with a pattern: `\\(x, y). x`, `let (x, y) = p in x` or `match p | (x, y) -> x`.\n");
        }
        if module == Module::Option {
            out.push_str("type Option<a> = None | Some a\n    A value that may be missing, such as the `head` of a list.\n");
            out.push_str("type Result<e, a> = Ok a | Err e\n    The value of a computation that can fail with an error of type `e`, such as `parseInt`.\n");
        }
        for builtin in Builtin::ALL.iter().filter(|b| b.module() == module) {
            out.push_str(&format!(
                "{} : {}\n    {}\n",
                builtin.name(),
                builtin.signature(),
                builtin.doc()
            ));
        }
        out.push('\n');
    }
    out
}

/// Renders the reference as JSON for tools such as the website generator:
/// `{"version", "modules": [{"name", "title", "builtins": [...]}]}`, where
/// each builtin has `name`, `signature`, `arity` and `doc`.
pub fn reference_json() -> String {
    let modules: Vec<String> = Module::ALL
        .iter()
        .map(|&module| {
            let builtins: Vec<String> = Builtin::ALL
                .iter()
                .filter(|b| b.module() == module)
                .map(|b| format!("        {}", b.to_json()))
                .collect();
            format!(
                "    {{\n      \"name\": {},\n      \"title\": {},\n      \"builtins\": [\n{}\n      ]\n    }}",
                json_string(&format!("{module:?}")),
                json_string(module.title()),
                builtins.join(",\n")
            )
        })
        .collect();
    format!(
        "{{\n  \"version\": {},\n  \"modules\": [\n{}\n  ]\n}}\n",
        json_string(env!("CARGO_PKG_VERSION")),
        modules.join(",\n")
    )
}

impl Builtin {
    /// This builtin as a one-line JSON object.
    pub fn to_json(self) -> String {
        format!(
            "{{\"name\": {}, \"signature\": {}, \"arity\": {}, \"doc\": {}}}",
            json_string(self.name()),
            json_string(self.signature()),
            self.arity(),
            json_string(self.doc())
        )
    }
}

fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_lookup_works() {
        let mut names: Vec<_> = Builtin::ALL.iter().map(|b| b.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Builtin::ALL.len());
        for builtin in Builtin::ALL {
            assert_eq!(Builtin::lookup(builtin.name()), Some(*builtin));
        }
        assert_eq!(Builtin::lookup("nope"), None);
    }

    #[test]
    fn arity_counts_top_level_arrows_only() {
        assert_eq!(Builtin::Line.arity(), 0);
        assert_eq!(Builtin::Neg.arity(), 1);
        assert_eq!(Builtin::Add.arity(), 2);
        assert_eq!(Builtin::If.arity(), 3);
        assert_eq!(Builtin::Map.arity(), 2);
        assert_eq!(Builtin::Eq.arity(), 2);
        assert_eq!(Builtin::Show.arity(), 1);
        assert_eq!(Builtin::Foldr.arity(), 3);
        assert_eq!(Builtin::ZipWith.arity(), 3);
        assert_eq!(Builtin::Transpose.arity(), 1);
        assert_eq!(Builtin::Bind.arity(), 2);
        assert_eq!(Builtin::ReadFile.arity(), 1);
        assert_eq!(Builtin::Head.arity(), 1);
        assert_eq!(Builtin::MapResult.arity(), 2);
        assert_eq!(Builtin::Fst.arity(), 1);
    }

    #[test]
    fn every_signature_parses() {
        use crate::source::SourceMap;
        let mut map = SourceMap::new();
        let file = map.add_virtual("<builtins>", "");
        for builtin in Builtin::ALL {
            assert!(
                crate::syntax::parse_scheme(file, builtin.signature()).is_ok(),
                "{builtin}: {}",
                builtin.signature()
            );
        }
    }

    #[test]
    fn reference_lists_every_builtin_under_its_module() {
        let text = reference();
        for builtin in Builtin::ALL {
            assert!(
                text.contains(&format!("{} : ", builtin.name())),
                "{builtin}"
            );
        }
        assert!(text.contains("-- Lists\n"));
        assert!(text.contains("type Result<e, a> = Ok a | Err e\n"));
        assert!(text.contains("println : String -> IO<Unit>"));
        assert!(text.contains("eq : Eq<a> => a -> a -> Bool"));
    }

    #[test]
    fn json_reference_escapes_and_lists_every_builtin() {
        assert_eq!(json_string("a\"b\\c\nd\u{1}"), r#""a\"b\\c\nd\u0001""#);
        let json = reference_json();
        for builtin in Builtin::ALL {
            assert!(json.contains(&builtin.to_json()), "{builtin}");
        }
        assert!(json.contains(r#""title": "Lists""#));
        assert!(
            json.contains(r#""name": "println", "signature": "String -> IO<Unit>", "arity": 1"#)
        );
    }
}
