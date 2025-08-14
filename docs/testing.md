# Testing Guide

## Architecture

The testing system uses dependency injection with the `AtomTableOps` trait:

- **Production**: `AtomTable::from_global()` - real AtomVM FFI
- **Testing**: `MockAtomTable::new()` - pure Rust implementation

No global state. Each test gets an isolated atom table instance.

## Core Pattern

```rust
#[test]
fn test_example() {
    let table = MockAtomTable::new();
    
    let user = user_fixture(&table);
    let atom = TermValue::atom("test", &table);
    
    assert_atom_str(&atom, "test", &table);
    assert_map_has_key(&user, "name", &table);
}
```

## Testing Modules

### `testing/mocks.rs`

Mock implementations for testing without AtomVM.

- `MockAtomTable::new()` - Standard table with common atoms
- `MockAtomTable::new_empty()` - Empty table
- `MockAtomTable::new_with_atoms(&["custom"])` - Pre-populated with specific atoms

### `testing/helpers.rs`

Generic test utilities that work with any atom table.

```rust
// Atom creation
let atom = atom("hello", &table);
let atoms = atoms(&["red", "green", "blue"], &table);

// Assertions
assert_atom_str(&term, "expected", &table);
assert_int(&term, 42);
assert_map_has_key(&map, "key", &table);

// Test data
let list = int_list(&[1, 2, 3]);
let map = atom_map(&[("key", value)], &table);
```

### `testing/fixtures.rs`

Pre-built realistic test data.

```rust
let user = user_fixture(&table);              // User with id, name, email
let admin = admin_user_fixture(&table);       // Admin with permissions
let config = config_fixture(&table);          // App configuration
let nested = nested_structure_fixture(&table); // Complex nested data

// Scenarios
let session = scenarios::user_session_scenario(&table);
let error = scenarios::error_scenario(&table);
```

## Key Functions

All functions that need atoms take a table parameter:

```rust
// Generic function
fn process_data<T: AtomTableOps>(data: &TermValue, table: &T) -> TermValue {
    if data.is_atom_str("ok", table) {
        TermValue::atom("success", table)
    } else {
        TermValue::atom("error", table)
    }
}

// Tagged serialization
let tagged = data.to_tagged_map(&table)?;
let restored = MyStruct::from_tagged_map(tagged, &table)?;
```

## Test Isolation

Each `MockAtomTable::new()` creates a completely independent instance. Tests cannot interfere with each other.

```rust
#[test] 
fn test_a() {
    let table = MockAtomTable::new(); // Isolated
    // ... test code
}

#[test]
fn test_b() {
    let table = MockAtomTable::new(); // Isolated
    // ... test code  
}
```