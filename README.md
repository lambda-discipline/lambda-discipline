> Functional Programming Language based on [Lambda Calculus](https://en.wikipedia.org/wiki/Lambda_calculus) highly
> inspired by this video [Programming with Math | The Lambda Calculus - Eyesomorphic](https://youtu.be/ViPNHMSUcog) made
> from scratch, entirely in `C++`.

---

## Getting Started

### Prerequisites

- **CMake** (version 3.20+ recommended)
- A C++23 capable compiler (e.g. `g++`, `clang++`, or MSVC on Windows)

### Setup

```bash
bash ./scripts/build.sh
```

### Run Golden Tests

```bash
bash ./tests/run_tests.sh
```

## Samples

Booleans and If-Then construct built entirely using Lambda Expressions.

```haskell
true: Bool = \x: Any. \y: Any. x
false: Bool = \x: Any. \y: Any. y

if: Any = \condition: Any.
    \thenClause: Any. \elseClause: Any.
        (condition thenClause elseClause)

boolToString: Bool = \bool: Bool. (if bool "true" "false")

(print (boolToString true) "\n")
(print (boolToString false) "\n")
```

```console
$ ./cmake-build-debug/lbd -f  ./examples/conditional_branching.lbd
true
false
```

Shapes Demo

```haskell
-- Define Pi as constant
pi: Float = 3.14

-- Define function to calculate square
square: Float -> Float = \x: Float. (mul x x)

-- Define function to calculate area of circle
areaOfCircle: Float -> Float = \r: Float.
    (mul pi (square r))

-- Define function to calculate volume of cylinder
volumeOfCylinder: Float -> Float -> Float = \r: Float.
	  \h: Float. (mul (areaOfCircle r) h)

-- Calculate the volume of a cylinder
(print "Volume of cylinder is: " (volumeOfCylinder 5.0 10.0) "\n")
```

```console
$ ./cmake-build-debug/lbd -f ./examples/shapes.lbd
Volume of cylinder is: 785.000000
```

Fibonacci

```haskell
fibonacci: Any = \num: Float.
    (null (cmp 0 num) 0
        (null (cmp 1 num) 1
            (add (fibonacci (sub num 1.0)) (fibonacci (sub num 2.0)))))

(print (fibonacci 10))
```

```console
$ ./cmake-build-debug/lbd -f ./examples/math_demos.lbd
55.000000
```

## Usage

After building, you can run the interpreter with:

```bash
lbd [options]
```

### Options

```
-f, --file <filepath>   Specify input source filepath to run
-h, --help              Show this help message and exit
-d, --debug             Enable debug mode
-r, --repl              Run in interactive REPL node
--docs                  Generate language-reference-document
```

> Generate the language-reference-document by using the `--docs` flag, and use it as the source of truth for
> native-function-signatures.

## Editor Plugins

1. [GNU Emacs](./editor-plugins/emacs)

## Dependencies

1. [readline](https://man7.org/linux/man-pages/man3/readline.3.html): For better REPL user experience.
2. [fmt](https://github.com/fmtlib/fmt): For formatting strings.