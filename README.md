# avmnif-rs

Safe Rust bindings and utilities for building AtomVM Native Implemented Functions (NIFs) and ports.

## Overview

`avmnif-rs` provides a type-safe, memory-safe foundation for integrating Rust code with AtomVM. It handles the low-level details of AtomVM's term format, atom management, and FFI boundaries, allowing you to focus on your application logic.

## Features

- **Memory-safe term conversion** between Rust types and AtomVM terms
- **Generic atom table operations** that work with any AtomVM context
- **Tagged ADT serialization** for type-safe Erlang-Rust data exchange
- **NIF collection macros** for easy function registration
- **Port communication utilities** for building custom port drivers
- **Comprehensive error handling** with proper Erlang error propagation
- **No-std compatible** core functionality

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
avmnif-rs = "0.1"

[lib]
crate-type = ["cdylib"]
```

### Basic NIF Example

```rust
use avmnif_rs::{nif_collection, term::*, atom::AtomTableOps};

fn add_numbers(ctx: &mut Context, args: &[Term]) -> NifResult<Term> {
    if args.len() != 2 {
        return Err(NifError::BadArity);
    }
    
    let a = args[0].to_value()?.as_int().ok_or(NifError::BadArg)?;
    let b = args[1].to_value()?.as_int().ok_or(NifError::BadArg)?;
    
    let result = TermValue::int(a + b);
    Ok(Term::from_value(result, &mut ctx.heap)?)
}

fn nif_init(_ctx: &mut Context) {
    // Initialize any resources here
}

nif_collection!(
    math_nifs,
    init = nif_init,
    nifs = [
        ("add", 2, add_numbers),
    ]
);
```

### Tagged ADT Example

```rust
use avmnif_rs::{tagged::TaggedMap, testing::mocks::MockAtomTable};

#[derive(TaggedMap)]
struct SensorReading {
    temperature: f32,
    humidity: f32,
    timestamp: u64,
}

fn main() {
    let table = MockAtomTable::new();
    let reading = SensorReading {
        temperature: 23.5,
        humidity: 45.2,
        timestamp: 1634567890,
    };
    
    // Serialize to Erlang map
    let term = reading.to_tagged_map(&table).unwrap();
    
    // Deserialize back to Rust
    let parsed = SensorReading::from_tagged_map(term, &table).unwrap();
    assert_eq!(parsed.temperature, 23.5);
}
```

## Architecture

### Core Components

- **`term`** - AtomVM term representation and conversion utilities
- **`atom`** - Generic atom table operations and management
- **`tagged`** - Type-safe ADT serialization with discriminator atoms
- **`ports`** - Port communication and lifecycle management
- **`resource`** - Resource type registration and management

### Design Principles

1. **Generic by design** - All operations work with any `AtomTableOps` implementation
2. **Memory safety** - No unsafe code in the public API surface
3. **Error transparency** - All failure modes are explicit and recoverable
4. **Zero-cost abstractions** - Minimal runtime overhead
5. **Testing-first** - Comprehensive test coverage with mock implementations

## Error Handling

All operations return explicit `Result` types with detailed error information:

```rust
pub enum NifError {
    BadArg,           // Invalid argument type or value
    BadArity,         // Wrong number of arguments
    OutOfMemory,      // Memory allocation failed
    SystemLimit,      // System resource limit exceeded
    InvalidTerm,      // Malformed term structure
    Other(&'static str), // Custom error message
}
```

Errors automatically convert to appropriate Erlang error atoms when crossing the FFI boundary.

## Building

The library compiles to a static library (`.a`) that gets linked into AtomVM:

```bash
cargo build --release
```

This produces `target/release/libmy_nifs.a` which can be linked into AtomVM builds.

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

Tests include:
- Mock atom table implementations for isolated testing
- Round-trip serialization validation
- Error condition coverage
- Memory safety verification
- Integration scenarios

## Platform Support

- **Primary**: Linux x86_64, ARM64
- **Secondary**: macOS, Windows (via WSL)
- **Embedded**: Any target supported by AtomVM with `no_std`

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## Minimum Supported Rust Version (MSRV)

Rust 1.70 or later.
