//! The primitive operations every program can use without a `use`.
//!
//! Builtins are declared once here as a backend-agnostic enum, the way GHC
//! declares its primops: the name, type signature, module and documentation
//! live with the declaration, the type checker reads the signatures, and each
//! backend provides its own implementation by matching on [`Builtin`].
//!
//! Naming follows one convention: lowerCamelCase, and the data a function
//! works on comes last, so partial application reads naturally
//! (`(map (add 1))`, `(split ",")`, `(get 0)`).

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
}

impl Module {
    pub const ALL: [Module; 6] = [
        Module::Number,
        Module::Compare,
        Module::Bool,
        Module::String,
        Module::List,
        Module::Io,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Module::Number => "Numbers",
            Module::Compare => "Comparison",
            Module::Bool => "Booleans",
            Module::String => "Strings",
            Module::List => "Lists",
            Module::Io => "Input/output",
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

            /// The type signature in source syntax. Lowercase names are
            /// type variables, implicitly quantified.
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
    Min => "min" : "Number -> Number -> Number", Number, "Smaller of two numbers.";
    Max => "max" : "Number -> Number -> Number", Number, "Larger of two numbers.";

    Eq => "eq" : "a -> a -> Bool", Compare, "Structural equality. Comparing functions is a runtime error.";
    Neq => "neq" : "a -> a -> Bool", Compare, "Structural inequality.";
    Lt => "lt" : "a -> a -> Bool", Compare, "`(lt a b)` is `a < b`. Numbers, strings, booleans and lists are ordered.";
    Le => "le" : "a -> a -> Bool", Compare, "`(le a b)` is `a <= b`.";
    Gt => "gt" : "a -> a -> Bool", Compare, "`(gt a b)` is `a > b`.";
    Ge => "ge" : "a -> a -> Bool", Compare, "`(ge a b)` is `a >= b`.";
    Compare => "compare" : "a -> a -> Number", Compare, "`-1`, `0` or `1` as `a` is less than, equal to or greater than `b`.";

    True => "true" : "Bool", Bool, "Boolean truth.";
    False => "false" : "Bool", Bool, "Boolean falsehood.";
    Not => "not" : "Bool -> Bool", Bool, "Boolean negation.";
    And => "and" : "Bool -> Bool -> Bool", Bool, "Conjunction; the second argument is only evaluated when needed.";
    Or => "or" : "Bool -> Bool -> Bool", Bool, "Disjunction; the second argument is only evaluated when needed.";
    If => "if" : "Bool -> a -> a -> a", Bool, "`(if c t e)` evaluates only the chosen branch.";

    Concat => "concat" : "String -> String -> String", String, "Joins two strings.";
    Show => "show" : "a -> String", String, "Renders any value as source-like text (strings are quoted).";
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
    Foldr => "foldr" : "(a -> b -> b) -> b -> List<a> -> b", List, "Right fold: `(foldr f z [x, y])` is `(f x (f y z))`.";
    Reverse => "reverse" : "List<a> -> List<a>", List, "Reverses a list.";
    Sort => "sort" : "List<a> -> List<a>", List, "Stable ascending sort using the same ordering as `lt`.";
    Range => "range" : "Number -> Number -> List<Number>", List, "`(range from to)` counts from `from` up to, not including, `to`.";
    ZipWith => "zipWith" : "(a -> b -> c) -> List<a> -> List<b> -> List<c>", List, "Combines two lists element-wise, stopping at the shorter.";
    Transpose => "transpose" : "List<List<a>> -> List<List<a>>", List, "Turns rows into columns, truncated to the shortest row.";
    Sum => "sum" : "List<Number> -> Number", List, "Sum of all elements.";
    Take => "take" : "Number -> List<a> -> List<a>", List, "The first `n` elements.";
    Drop => "drop" : "Number -> List<a> -> List<a>", List, "All but the first `n` elements.";

    Print => "print" : "a -> Unit", Io, "Prints its arguments without separators. Takes any number of arguments of any types.";
    Println => "println" : "a -> Unit", Io, "Like `print`, followed by a newline.";
    ReadFile => "readFile" : "String -> String", Io, "Reads a whole file, relative to the working directory.";
    Error => "error" : "String -> a", Io, "Stops the program with an error message.";
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

    /// How the signature is shown to users.
    pub fn display_signature(self) -> String {
        if self.is_variadic() {
            "a... -> Unit".to_owned()
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
        // Count top-level arrows in the signature, ignoring parenthesized ones.
        let mut depth = 0i32;
        let mut arrows = 0;
        let bytes = self.signature().as_bytes();
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
        assert_eq!(Builtin::True.arity(), 0);
        assert_eq!(Builtin::Neg.arity(), 1);
        assert_eq!(Builtin::Add.arity(), 2);
        assert_eq!(Builtin::If.arity(), 3);
        assert_eq!(Builtin::Map.arity(), 2);
        assert_eq!(Builtin::Foldr.arity(), 3);
        assert_eq!(Builtin::ZipWith.arity(), 3);
        assert_eq!(Builtin::Transpose.arity(), 1);
    }

    #[test]
    fn every_signature_parses() {
        use crate::source::SourceMap;
        let mut map = SourceMap::new();
        let file = map.add_virtual("<builtins>", "");
        for builtin in Builtin::ALL {
            assert!(
                crate::syntax::parse_type(file, builtin.signature()).is_ok(),
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
        assert!(text.contains("print : a... -> Unit"));
    }
}
