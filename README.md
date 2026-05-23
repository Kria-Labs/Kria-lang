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
- Conditional branches: `if`, `else`
- Loops: `while`, `for-in`
- **Loop control**: `break`, `continue`
- **Functions** (named and anonymous/lambda)
- **Closures** — nested functions capture outer parameters and upvalues (copy-on-create)
- **Arrays** — mutable `[...]` and immutable `#[...]`, indexing, `length`, `push`/`pop`, iterate with `for-in`
- **Objects** — `{ key: value }` literals, dot/bracket access, property assign, `rmv()`, deep equality, `for key, value in obj`
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

Run the full benchmark suite (warmup + multiple runs, median/min/max/mean):

```bash
cargo build --release
./benchmarks/benchmarks.sh
# Results: benchmarks/benchmark_results.txt
```

Optional: install [hyperfine](https://github.com/sharkdp/hyperfine) for more stable timing (`cargo install hyperfine` or your package manager). The script uses hyperfine when available, otherwise falls back to bash + `/usr/bin/time`.

Override run count: `BENCH_WARMUP=3 BENCH_RUNS=10 ./benchmarks/benchmarks.sh`

| Category | Example benchmark | Typical median (release, warm cache) |
|----------|-------------------|-------------------------------------|
| Loops | `benchmarks/bench_loop_optimized.krx` (1M iter) | ~10–15ms |
| Functions | `benchmarks/bench_functions.krx` | ~1–3ms |
| Arrays / objects | `benchmarks/bench_arrays.krx`, `bench_objects.krx` | varies |
| Closures | `benchmarks/bench_closures.krx` | varies |

*Numbers depend on hardware; re-run `./benchmarks/benchmarks.sh` for your machine.*

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

## Loop Control: Break and Continue

Kria supports `break` to exit a loop early and `continue` to skip to the next iteration:

### Break
```kria
set i = 0
while (i < 10) {
    if (i == 5) {
        break  // Exit loop immediately
    }
    print(i)
    set i = i + 1
}
// Output: 0, 1, 2, 3, 4
```

### Continue
```kria
set i = 0
while (i < 5) {
    set i = i + 1
    if (i == 3) {
        continue  // Skip to next iteration
    }
    print(i)
}
// Output: 1, 2, 4, 5
```

### With For-In Loops
```kria
set arr = [1, 2, 3, 4, 5]
for item in arr {
    if (item == 2) {
        continue
    }
    if (item == 4) {
        break
    }
    print(item)
}
// Output: 1, 3
```

**Note:** In nested loops, `break` and `continue` affect only the innermost loop they appear in.

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

### Closures

Nested functions can use variables from enclosing functions. Values are **copied when the inner function is created** (not shared afterward).

```kria
set create_multiplier = fn(factor) {
    return fn(x) {
        return x * factor
    }
}

set times_three = create_multiplier(3)
print(times_three(7))  // Output: 21
```

Captured variables include:

- Parameters of enclosing functions (e.g. `factor` above)
- Variables already captured by an enclosing closure (nested closures chain captures)

**Notes:**

- Capture happens at creation time: if you `set x = 1`, then `set f = fn() { return x }`, then `set x = 2`, calling `f()` still returns `1`.
- `set` inside a function body (other than parameters) still uses **global** storage; only parameters and captured names use closure locals/upvalues.

## Arrays

Kria has two array kinds:

| Syntax | Mutable | `push` / `pop` | `arr[i] = v` |
|--------|---------|----------------|--------------|
| `[1, 2, 3]` | Yes | Yes | Yes (no `set`) |
| `#[1, 2, 3]` | No | Runtime error | Runtime error |

### Literals and indexing

```kria
set arr = [10, 20, 30]
set frozen = #[1, 2]

print(arr[0])       // 10
set x = arr[1]      // read into variable

arr[0] = 99         // element assign (mutable only)
print(arr.length)   // 3
```

Nested arrays are supported: `[[1, 2], [3, 4]]`.

### push and pop

```kria
push(arr, 40)       // append in place (mutable only)
set last = pop(arr) // remove last element; empty array → error
```

### Equality

Arrays compare with **deep equality**: `[1, 2] == [1, 2]` is `true`.

### for-in loops

```kria
for item in arr {
    print(item)
}
```

### Member access (`.length`)

Dot syntax on arrays reads `.length` as the element count. On objects, unknown dot members evaluate to `null`.

### Notes

- Array capture in closures follows the same copy-on-create rules as other values.
- `set` inside a function body still targets globals; only parameters use locals.

## Objects

Object literals use `key: value` pairs inside `{ }`:

```kria
set user = { name: "Arda", age: 17 }
print(user.name)      // dot access
print(user["age"])    // bracket access (key must be a string at runtime)
```

Missing properties return `null` (no error). Objects compare with **order-independent deep equality**: `{ x: 1, y: 2 } == { y: 2, x: 1 }` is `true`.

### Property assignment

Assign without `set` on the left-hand path:

```kria
user.age = 18
user["name"] = "Arda K"
user.profile.city = "Ankara"   // auto-creates missing intermediate objects
```

### rmv

Remove a property (no error if the key is missing):

```kria
rmv(user.name)
rmv(user[key])
```

### for-in on objects

Objects require both key and value names; arrays keep the single-variable form:

```kria
for key, value in user {
    print(key)
    print(value)
}

for item in arr {
    print(item)
}
```

Using `for key, value in arr` or `for item in obj` is a **compile-time error**.

### Notes

- Bracket keys must be strings at runtime (numbers are not coerced).
- `rmv` on a missing key is a silent no-op.
- `set` inside a function body still targets globals; only parameters and upvalues are local.

## Input Operations

Kria provides a unified input system with type specifications:

### Reading Strings
```kria
set name = input<str>("What is your name?")
print(name)
```

Use `input<str>("prompt")` to read a string from stdin with a prompt message.

### Reading Integers
```kria
set age = input<int>("How old are you?")
print(age)
```

Use `input<int>("prompt")` to read an integer. Non-numeric input will cause an error and retry.

### Reading Floats
```kria
set height = input<float>("What is your height?")
print(height)
```

Use `input<float>("prompt")` to read a floating-point number. The VM stores numbers as 64-bit integers, so floats are truncated.

### Example: Interactive Program
```kria
set name = input<str>("What is your name?")
set age = input<int>("How old are you?")

print("Hello, ")
print(name)
print("! You are ")
print(age)
print(" years old.")
```

## Installation

### Windows (creates installer in `release/`)
```powershell
# Requires NSIS: https://nsis.sourceforge.io/
cargo build --release
cd release
makensis kria-setup.nsi
# Run release\kria-1.0.0-windows-x86_64-setup.exe to install
```

### Linux/macOS
```bash
./release/build.sh install
# Binary is installed to ~/.kria/bin — add that directory to PATH if needed
```

To create a distributable archive instead:
```bash
./release/build.sh package
# Creates release/kria-<version>-<os>-<arch>.tar.gz
```

After installation, use:
```bash
kria test.krx
```

## License
MIT