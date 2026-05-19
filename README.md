# Kria Programming Language

A custom programming language written in Rust, featuring a flat bytecode VM with combined loop instructions for high-performance execution.

## Features

- File extension: `.krx`
- Flat bytecode VM with constant pool and combined instructions
- Dynamic + strong typing
- Variable assignment with `set`
- Arithmetic operations (+, -, *, /)
- Comparison operators: `==`, `!=`, `>`, `<`, `>=`, `<=`
- Logical operators: `and`, `or`, `not`
- Conditional branches: `if`, `elseif`, `else`
- Loops: `while`
- **Functions** (named and anonymous/lambda)
- **Return statements** with values
- **Function parameters** and local variable scoping
- **Input operations**: read strings, integers, and floats from stdin
- Block scope with `{}`
- Line comments using `//`
- Print function
- Null handling for undefined variables
- Newline-based statement termination (no semicolons)

## Architecture

Kria compiles source into flat bytecode (u8 opcodes + constant pool) before execution on a stack-based VM with call frame support for functions. The compiler emits combined instructions for hot paths like `while (var < N) { var = var + 1 }`, reducing dispatch from 6 to 1 per iteration.

```text
Source → Lexer → Parser → Compiler → Flat Bytecode + Function Metadata → Stack-based VM with Call Frames
```

**Key Components:**
- **Lexer**: Tokenizes input, recognizes keywords (`fn`, `return`), operators, literals
- **Parser**: Recursive descent parser with operator precedence, builds AST
- **Compiler**: Generates bytecode with local variable scoping for function parameters
- **VM**: Stack-based execution with call frame stack for function calls and returns

### Bytecode Optimizations

1. **Combined Loop Instructions**: `OP_LOOP_INC_LESS` - single opcode for `while (var < N) { var++ }` pattern
2. **Specialized Global Ops**: `OP_INC_GLOBAL`, `OP_ADD_GLOBAL` for common patterns
3. **Function Metadata**: Functions store bytecode offset and parameter count for efficient calls
4. **Local Variable Access**: `OP_LOAD_LOCAL`, `OP_STORE_LOCAL` for parameter and local variable access

### Performance

| Test | Avg Execution Time |
|------|-------------------|
| `perf_test.krx` (1M loop) | ~12ms |
| `test.krx` (general features) | ~5ms |
| `func_comprehensive_test.krx` (7 function tests) | ~1ms |

*Measured with `cargo build --release` on warm cache.*

## Building

Ensure you have Rust installed. Then:

```bash
# Debug build (for development)
cargo build

# Release build (for performance — recommended)
cargo build --release
```

## Running

```bash
# Development
cargo run -- test.krx

# Release (recommended for benchmarking)
cargo run --release -- test.krx

# Or run the binary directly after release build
./target/release/kria test.krx
```

Example `test.krx`:
```kria
set x = 5
set x = 4
set y = 3
set text = "test"
print(x + y)
print(text)

set mynum = true
if (mynum == true) {
   print("Mynum is true")
} elseif (mynum == false) {
   print("Mynum is false")
} else {
   print("mynum must be a boolean")
}

set counter = 0
while (counter < 5) {
    print(counter)
    set counter = counter + 1
}

// Line comments start with // and continue to the end of the line.
```

## Functions

Kria supports both named functions and anonymous (lambda) functions:

### Named Functions
```kria
fn add(x, y) {
    return x + y
}

print(add(5, 3))  // Output: 8
```

### Anonymous Functions (Lambda)
```kria
set multiply = fn(a, b) {
    return a * b
}

print(multiply(4, 5))  // Output: 20
```

### Function with Control Flow
```kria
fn max_value(a, b) {
    if (a > b) {
        return a
    } else {
        return b
    }
}

print(max_value(10, 7))  // Output: 10
```

### Function with Loops
```kria
fn sum_to(n) {
    set total = 0
    set i = 0
    while (i < n) {
        set total = total + i
        set i = i + 1
    }
    return total
}

print(sum_to(5))  // Output: 10 (0+1+2+3+4)
```

## Input Operations

Kria provides three ways to read input from stdin:

### Reading Strings
```kria
set name = input
print(name)
```

Use `input` as a bare identifier to read a string from stdin.

### Reading Integers
```kria
set age = int()
print(age)
```

Call `int()` to read an integer. Non-numeric input will cause an error.

### Reading Floats
```kria
set height = float()
print(height)
```

Call `float()` to read a floating-point number. The VM stores numbers as 64-bit integers, so floats are truncated.

### Example: Interactive Program
```kria
print("What is your name?")
set name = input

print("How old are you?")
set age = int()

print("Hello, ")
print(name)
print("! You are ")
print(age)
print(" years old.")
```

## Installation

### Windows (creates kria-setup.exe)
```powershell
# Requires NSIS: https://nsis.sourceforge.io/
powershell scripts/build-windows.ps1
# Run kria-setup.exe to install
```

### Linux/macOS (creates kria-setup.sh)
```bash
./scripts/build-linux.sh
sudo ./dist/kria-setup.sh /usr/local
```

After installation, use:
```bash
kria test.krx
```

## License
MIT