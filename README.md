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

- TODO: Provide better examples (e.g., Advent of Code snippets).
- TODO: Maybe use rere.py for testing