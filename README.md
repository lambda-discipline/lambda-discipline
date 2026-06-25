> Functional Programming Language based on [Lambda Calculus](https://en.wikipedia.org/wiki/Lambda_calculus) highly
> inspired by this video [Programming with Math | The Lambda Calculus - Eyesomorphic](https://youtu.be/ViPNHMSUcog) made
> from scratch, entirely in `C++`.

---

## Getting Started

### Prerequisites

- **CMake** (version 3.20+ recommended)
- A C++20 capable compiler (e.g. `g++`, `clang++`, or MSVC on Windows)

### Setup

```shell
python make.py setup
```

### Run Golden Tests

```shell
python make.py test
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
$ ./cmake-build-debug/lbd run ./examples/conditional_branching.lbd
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
$ ./cmake-build-debug/lbd run ./examples/shapes.lbd
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
$ ./cmake-build-debug/lbd run ./examples/math_demos.lbd
55.000000
```

## Usage

After building, you can run the interpreter with:

```bash
lbd [subcommands] <options>
```

### Options

```
OPTIONS:
  -h,     --help              Print this help message and exit
  -d,     --debug

SUBCOMMANDS:
  run                         Run a Lambda Discipline source file
  docs                        Generate native function signatures for reference
  repl                        Start an interactive Read-Eval-Print Loop (REPL)
```

> Generate the language-reference-document by using the `docs` subcommand, and use it as the source of truth for
> native-function-signatures.

## Editor Plugins

1. [GNU Emacs](./editor-plugins/emacs)

## Linux Releases

Prebuilt Linux releases are distributed as **AppImages**, allowing the interpreter to run on most modern Linux distributions without installation.

After downloading an AppImage, make it executable and run it:

```shell
chmod +x Lambda_Discipline-*.AppImage
./Lambda_Discipline-*.AppImage --help
```

> You can also integrate the AppImage with your desktop environment using tools such as `AppImageLauncher`, or simply keep it as a portable executable.

## Dependencies

1. [fmt](https://github.com/fmtlib/fmt.git): String formatting.
2. [replxx](https://github.com/AmokHuginnsson/replxx.git): For better REPL user experience.
3. [CLI11](https://github.com/CLIUtils/CLI11.git): For better CLI support.
