# GScript - Rust Implementation

A Rust implementation of the GScript programming language compiler and runtime. GScript is a dynamically typed, object-oriented scripting language that runs on the GVM (Geert Virtual Machine).

## Features

- **Full compiler pipeline**: parser, AST, bytecode compiler
- **Stack-based virtual machine** (GVM) with garbage collection
- **Dynamic typing** with Numbers (i32), Strings, Booleans, Objects, Arrays, Maps, and Functions
- **Prototype-based OOP** with constructor functions and `this` binding
- **First-class functions** with closures and arrow syntax
- **Tail-call optimization**
- **Concurrency** via `fork()` with shared-heap threading
- **Native module plugin system** for extending the runtime from Rust
- **Bytecode serialization** -- compile once, run on either the Rust or Java runtime
- **Standard library**: System, Net, File, Time, Environment, Http

## Building

```bash
cargo build --release
```

Requires the `gs-core` crate (expected at `../gs-core-rust`).

## Usage

```bash
# Compile and run a .gs source file
gs-lang program.gs

# Compile to portable bytecode
gs-lang --compile program.gs -o program.gsc

# Run from bytecode
gs-lang --run program.gsc

# Debug mode (execution tracing)
gs-lang --debug program.gs

# Print disassembled bytecode
gs-lang --asm program.gs
```

## Hello World

```
module Hello;
import System;

System.print("Hello, world!")
```

## Language Overview

### Variables and Types

```
x = 10
name = "Geert"
flag = true
items = new [1, 2, 3]
config = new ["host" => "localhost", "port" => 8080]
```

### Functions

```
add = (a, b) -> {
    return a + b
}
result = add(2, 3)
```

### Objects

```
Person = (name, age) -> {
    this.name = name
    this.age = age
    greet = () -> {
        return "Hi, I am " + this.name
    }
    return this
}

alice = new Person("Alice", 30)
System.print(alice.greet())
```

### Control Flow

```
for (i = 0; i < 10; i++) {
    if (i % 2 == 0) {
        System.print("" + i)
    }
}
```

### Exception Handling

```
try {
    risky()
} catch (e) {
    System.print("Error: " + e.message + " at line " + e.line)
}
```

### Concurrency

```
child = fork()
if (child == true) {
    System.print("child thread")
} else {
    System.print("parent thread")
}
```

## Standard Library

| Module      | Description                  |
|-------------|------------------------------|
| System      | Console output (`print`)     |
| Net         | TCP sockets, streams         |
| File        | File system operations       |
| Time        | Clock (`now`) and `sleep`    |
| Environment | Environment variables        |
| Http        | HTTP client and server       |

Standard library modules live in `gslib/` as `.gs` wrappers around native Rust modules. The runtime locates them via the `GVM_HOME_RUST` or `GVM_HOME` environment variable, or by searching relative to the source file and executable.

## Project Structure

```
src/
  main.rs          -- CLI entry point
  ast/             -- Abstract syntax tree definitions
  parser/          -- Recursive descent parser
  compiler/        -- AST to GVM bytecode compiler
  runtime/         -- Native method bridge
  lang/            -- GScript type system and value conversion
  stdlib/          -- Native module implementations
    system/        -- Runtime (print), Time, Environment
    net/           -- ServerSocket, Socket, TcpStream
    io/            -- BufferedReader, File
gslib/             -- GScript standard library wrappers (.gs)
examples/          -- Example programs
gscript.md         -- Full language reference
```

## Cross-Runtime Compatibility

Compiled `.gsc` bytecode files are portable between this Rust runtime and the Java runtime. The same `gslib/` standard library files work on both.

## Language Reference

See [gscript.md](gscript.md) for the complete language specification including grammar, operator precedence, native module API, and detailed examples.
