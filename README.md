> Functional Programming Language based on [Lambda Calculus](https://en.wikipedia.org/wiki/Lambda_calculus) highly
> inspired by this video [Programming with Math | The Lambda Calculus - Eyesomorphic](https://youtu.be/ViPNHMSUcog) made
> from scratch, entirely in `C++`.

## Getting Started

### Prerequisites

- **CMake** (version 3.16+ recommended)
- **Python 3.8+** (for development environment and tooling)
- A C++17 capable compiler (e.g. `g++`, `clang++`, or MSVC on Windows)

### Setup

#### GNU/Linux

```bash
bash ./scripts/setup.sh
```

#### Windows (PowerShell)

```powershell
. .\scripts\setup.ps1
```

---

## Samples

Booleans and If-Then construct built entirely using Lambda Expressions.

```haskell
true: Bool = \x: Any. \y: Any. x
false: Bool = \x: Any. \y: Any. y

if_then: Any = \condition: Any.
    \then_clause: Any. \else_clause: Any.
        (condition then_clause else_clause)

(print (if_then true "True" "False"))
(print (if_then false "True" "False"))
```

```console
$ ./cmake-build-debug/lbd.exe -f ./examples/if_then.lbd
True
False
```

Shapes Demo

```haskell
-- Define Pi as constant
pi: Float = 3.14

-- Define function to calculate square
square: Float -> Float = \x: Float. (mul x x)

-- Define function to calculate area of circle
area_of_circle: Float -> Float = \r: Float.
    (mul pi (square r))

-- Define function to calculate volume of cylinder
volume_of_cylinder: Float -> Float -> Float = \r: Float.
	\h: Float. (mul (area_of_circle r) h)

-- Calculate the volume of a cylinder
(print "Volume of cylinder is: ")
(print (volume_of_cylinder 5.0 10.0))
```

```console
$ ./cmake-build-debug/lbd -f ./examples/shapes.lbd
Volume of cylinder is:
785.000000
```

Fibonacci

```haskell
fibonacci: Any = \num: Float.
    (if_zero (cmp 0 num) 0
        (if_zero (cmp 1 num) 1
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
-r, --repl              Run in interactive REPL mode
```

---

## Editor Plugins

1. [GNU Emacs](./editor-plugins/emacs)

---

## TODO

- TODO: Add better examples (e.g., Advent of Code snippets).
- TODO: Maybe use rere.py for testing