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
    Io,
    Source,
}

impl Module {
    pub const ALL: [Module; 7] = [
        Module::Number,
        Module::Compare,
        Module::Bool,
        Module::String,
        Module::List,
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
    Add => "add" : "Number -> Number -> Number", Number, "Sum of two numbers.";
    Sub => "sub" : "Number -> Number -> Number", Number, "`(sub a b)` is `a - b`.";
    Mul => "mul" : "Number -> Number -> Number", Number, "Product of two numbers.";
    Div => "div" : "Number -> Number -> Number", Number, "`(div a b)` is `a / b` (IEEE: dividing by zero gives infinity).";
    Mod => "mod" : "Number -> Number -> Number", Number, "Remainder with the sign of the divisor, like Haskell's `mod`.";
    Pow => "pow" : "Number -> Number -> Number", Number, "`(pow base exponent)`.";
    Neg => "neg" : "Number -> Number", Number, "Negation.";
    Abs => "abs" : "Number -> Number", Number, "Absolute value.";
    Floor => "floor" : "Number -> Number", Number, "Largest integer not greater than the argument.";
    Ceil => "ceil" : "Number -> Number", Number, "Smallest integer not less than the argument.";
    Round => "round" : "Number -> Number", Number, "Nearest integer, halves away from zero.";
    Sqrt => "sqrt" : "Number -> Number", Number, "Square root.";
    Min => "min" : "Ord<a> => a -> a -> a", Number, "The smaller of two values; the first if they are equal.";
    Max => "max" : "Ord<a> => a -> a -> a", Number, "The larger of two values; the second if they are equal.";

    Eq => "eq" : "Eq<a> => a -> a -> Bool", Compare, "Structural equality. Every type without functions or `IO` actions in it belongs to `Eq`.";
    Neq => "neq" : "Eq<a> => a -> a -> Bool", Compare, "Structural inequality.";
    Lt => "lt" : "Ord<a> => a -> a -> Bool", Compare, "`(lt a b)` is `a < b`. Every type in `Eq` is also in `Ord`: data is ordered by constructor, in declaration order, then field by field, and lists element by element.";
    Le => "le" : "Ord<a> => a -> a -> Bool", Compare, "`(le a b)` is `a <= b`.";
    Gt => "gt" : "Ord<a> => a -> a -> Bool", Compare, "`(gt a b)` is `a > b`.";
    Ge => "ge" : "Ord<a> => a -> a -> Bool", Compare, "`(ge a b)` is `a >= b`.";
    Compare => "compare" : "Ord<a> => a -> a -> Number", Compare, "`-1`, `0` or `1` as `a` is less than, equal to or greater than `b`.";

    Not => "not" : "Bool -> Bool", Bool, "Boolean negation.";
    And => "and" : "Bool -> Bool -> Bool", Bool, "Conjunction; the second argument is only evaluated when needed.";
    Or => "or" : "Bool -> Bool -> Bool", Bool, "Disjunction; the second argument is only evaluated when needed.";
    If => "if" : "Bool -> a -> a -> a", Bool, "`(if c t e)` evaluates only the chosen branch.";

    Concat => "concat" : "String -> String -> String", String, "Joins two strings.";
    Show => "show" : "Show<a> => a -> String", String, "Renders a value as source-like text (strings are quoted). Every type without functions or `IO` actions in it belongs to `Show`.";
    ParseNumber => "parseNumber" : "String -> Number", String, "Parses a number, ignoring surrounding whitespace.";
    StringLength => "stringLength" : "String -> Number", String, "Number of characters in a string.";
    Chars => "chars" : "String -> List<String>", String, "Splits a string into its characters.";
    Split => "split" : "String -> String -> List<String>", String, "`(split delimiter text)`; the delimiter must not be empty.";
    Lines => "lines" : "String -> List<String>", String, "Splits text into lines, accepting `\\n` and `\\r\\n`.";
    Join => "join" : "String -> List<String> -> String", String, "`(join separator parts)`.";
    Trim => "trim" : "String -> String", String, "Removes leading and trailing whitespace.";

    Length => "length" : "List<a> -> Number", List, "Number of elements.";
    IsEmpty => "isEmpty" : "List<a> -> Bool", List, "Whether the list has no elements.";
    Head => "head" : "List<a> -> a", List, "First element; an error on an empty list.";
    Tail => "tail" : "List<a> -> List<a>", List, "Everything but the first element; an error on an empty list.";
    Get => "get" : "Number -> List<a> -> a", List, "`(get index list)`, zero-based; an error when out of bounds.";
    Cons => "cons" : "a -> List<a> -> List<a>", List, "Prepends an element.";
    Push => "push" : "a -> List<a> -> List<a>", List, "Appends an element at the end.";
    Remove => "remove" : "Number -> List<a> -> List<a>", List, "`(remove index list)` without the element at `index`.";
    Append => "append" : "List<a> -> List<a> -> List<a>", List, "Concatenates two lists.";
    Map => "map" : "(a -> b) -> List<a> -> List<b>", List, "Applies a function to every element.";
    Filter => "filter" : "(a -> Bool) -> List<a> -> List<a>", List, "Keeps the elements satisfying a predicate.";
    Foldl => "foldl" : "(b -> a -> b) -> b -> List<a> -> b", List, "Left fold: `(foldl f z [x, y])` is `(f (f z x) y)`.";
    Foldr => "foldr" : "(a -> b -> b) -> b -> List<a> -> b", List, "Right fold: `(foldr f z [x, y])` is `(f x (f y z))`, where each inner fold is evaluated only if `f` uses it.";
    Reverse => "reverse" : "List<a> -> List<a>", List, "Reverses a list.";
    Sort => "sort" : "Ord<a> => List<a> -> List<a>", List, "Stable ascending sort using the same ordering as `lt`.";
    Range => "range" : "Number -> Number -> List<Number>", List, "`(range from to)` counts from `from` up to, not including, `to`.";
    ZipWith => "zipWith" : "(a -> b -> c) -> List<a> -> List<b> -> List<c>", List, "Combines two lists element-wise, stopping at the shorter.";
    Transpose => "transpose" : "List<List<a>> -> List<List<a>>", List, "Turns rows into columns, truncated to the shortest row.";
    Sum => "sum" : "List<Number> -> Number", List, "Sum of all elements.";
    Take => "take" : "Number -> List<a> -> List<a>", List, "The first `n` elements.";
    Drop => "drop" : "Number -> List<a> -> List<a>", List, "All but the first `n` elements.";

    Pure => "pure" : "a -> IO<a>", Io, "An action that does nothing and produces its argument.";
    Bind => "bind" : "IO<a> -> (a -> IO<b>) -> IO<b>", Io, "`(bind action f)` runs `action`, then the action `f` returns for its result. `let x <- action in body` is `(bind action \\x. body)`.";
    Then => "then" : "IO<a> -> IO<b> -> IO<b>", Io, "`(then first second)` runs `first`, discards its result, then runs `second`.";
    Sequence => "sequence" : "List<IO<a>> -> IO<List<a>>", Io, "Runs a list of actions in order and collects their results.";
    Print => "print" : "Show<a> => a -> IO<Unit>", Io, "An action that prints its arguments without separators. Takes any number of arguments, of any types in `Show`; strings are printed without quotes.";
    Println => "println" : "Show<a> => a -> IO<Unit>", Io, "Like `print`, followed by a newline.";
    ReadFile => "readFile" : "String -> IO<String>", Io, "An action that reads a whole file. A relative path is relative to the working directory, like a path typed on the command line; start it with `__DIR__` to read a file that sits next to the program.";
    Error => "error" : "String -> a", Io, "Stops the program with an error message when it is evaluated. Not an action: it can occur anywhere.";

    File => "__FILE__" : "String", Source, "The path of the source file this name is written in; `<repl>` in the REPL.";
    Dir => "__DIR__" : "String", Source, "The directory of the source file this name is written in, which `use` paths are relative to; `.` in the REPL. `(readFile (concat __DIR__ \"/data.txt\"))` reads the `data.txt` next to the program from any working directory.";
    Line => "__LINE__" : "Number", Source, "The line this name is written on, counting from 1.";
}

impl Builtin {
    pub fn lookup(name: &str) -> Option<Builtin> {
        Builtin::ALL.iter().copied().find(|b| b.name() == name)
    }

    /// Variadic builtins accept any number of arguments of independent types
    /// and must be called directly; they cannot be passed around as values.
    pub fn is_variadic(self) -> bool {
        matches!(self, Builtin::Print | Builtin::Println)
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

    /// How the signature is shown to users.
    pub fn display_signature(self) -> String {
        if self.is_variadic() {
            "Show<a> => a... -> IO<Unit>".to_owned()
        } else {
            self.signature().to_owned()
        }
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
                b')' => depth -= 1,
                b'>' if i > 0 && bytes[i - 1] == b'-' => {
                    if depth == 0 {
                        arrows += 1;
                    }
                }
                b'>' => depth -= 1,
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
        for builtin in Builtin::ALL.iter().filter(|b| b.module() == module) {
            out.push_str(&format!(
                "{} : {}\n    {}\n",
                builtin.name(),
                builtin.display_signature(),
                builtin.doc()
            ));
        }
        out.push('\n');
    }
    out
}

/// Renders the reference as JSON for tools such as the website generator:
/// `{"version", "modules": [{"name", "title", "builtins": [...]}]}`, where
/// each builtin has `name`, `signature`, `arity`, `variadic` and `doc`.
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
            "{{\"name\": {}, \"signature\": {}, \"arity\": {}, \"variadic\": {}, \"doc\": {}}}",
            json_string(self.name()),
            json_string(&self.display_signature()),
            self.arity(),
            self.is_variadic(),
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
        assert!(text.contains("print : Show<a> => a... -> IO<Unit>"));
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
        assert!(json.contains(
            r#""signature": "Show<a> => a... -> IO<Unit>", "arity": 1, "variadic": true"#
        ));
    }
}
