//! Test helper functions and utilities
//! 
//! This module provides common utilities and helper functions that make
//! writing tests easier and more consistent across the codebase.
//!
//! # Design Philosophy
//!
//! All helpers are generic and work with any AtomTableOps implementation.
//! No global state, no hardcoded dependencies - pure dependency injection.

use alloc::vec;
use alloc::vec::Vec;
use alloc::format;
use alloc::string::ToString;

use crate::atom::AtomTableOps;
use crate::term::TermValue;

// ── Generic Atom Creation Helpers ──────────────────────────────────────────

/// Create an atom using any atom table
pub fn atom<T: AtomTableOps>(name: &str, table: &T) -> TermValue {
    TermValue::atom(name, table)
}

/// Create multiple atoms at once
pub fn atoms<T: AtomTableOps>(names: &[&str], table: &T) -> Vec<TermValue> {
    names.iter()
        .map(|name| TermValue::atom(name, table))
        .collect()
}

// ── Generic Term Creation Helpers ──────────────────────────────────────────

/// Create a list of integers for testing
pub fn int_list(values: &[i32]) -> TermValue {
    let elements: Vec<TermValue> = values.iter()
        .map(|&v| TermValue::int(v))
        .collect();
    TermValue::list(elements)
}

/// Create a tuple of integers for testing
pub fn int_tuple(values: &[i32]) -> TermValue {
    let elements: Vec<TermValue> = values.iter()
        .map(|&v| TermValue::int(v))
        .collect();
    TermValue::tuple(elements)
}

/// Create a map with atom keys and mixed values
pub fn atom_map<T: AtomTableOps>(
    pairs: &[(&str, TermValue)], 
    table: &T
) -> TermValue {
    let map_pairs: Vec<(TermValue, TermValue)> = pairs.iter()
        .map(|(key_name, value)| (TermValue::atom(key_name, table), value.clone()))
        .collect();
    TermValue::map(map_pairs)
}

/// Create test data for complex nested structures
pub fn create_complex_test_data<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (
            TermValue::atom("user", table),
            TermValue::map(vec![
                (TermValue::atom("name", table), TermValue::atom("alice", table)),
                (TermValue::atom("age", table), TermValue::int(30)),
                (TermValue::atom("active", table), TermValue::atom("true", table)),
                (
                    TermValue::atom("permissions", table),
                    TermValue::list(vec![
                        TermValue::atom("read", table),
                        TermValue::atom("write", table),
                        TermValue::atom("admin", table),
                    ])
                ),
            ])
        ),
        (
            TermValue::atom("session", table),
            TermValue::tuple(vec![
                TermValue::atom("session_id", table),
                TermValue::int(12345),
                TermValue::atom("authenticated", table),
            ])
        ),
        (
            TermValue::atom("metadata", table),
            TermValue::map(vec![
                (TermValue::atom("version", table), TermValue::int(1)),
                (TermValue::atom("created_at", table), TermValue::int(1640995200)), // Unix timestamp
                (
                    TermValue::atom("tags", table),
                    TermValue::list(vec![
                        TermValue::atom("production", table),
                        TermValue::atom("verified", table),
                    ])
                ),
            ])
        ),
    ])
}

// ── Generic Assertion Helpers ──────────────────────────────────────────────

/// Assert that two TermValues are equal with detailed error messages
/// 
/// Provides better debugging information than standard assert_eq!
/// when comparing complex TermValue structures.
pub fn assert_term_eq(left: &TermValue, right: &TermValue) {
    if left != right {
        panic!(
            "TermValue assertion failed:\nLeft:  {:?}\nRight: {:?}",
            left, right
        );
    }
}

/// Assert that a TermValue is an atom with the given name
pub fn assert_atom_str<T: AtomTableOps>(
    term: &TermValue, 
    expected: &str, 
    table: &T
) {
    match term {
        TermValue::Atom(idx) => {
            if !table.atom_equals_str(*idx, expected) {
                let actual = term.as_atom_str(table)
                    .unwrap_or_else(|| format!("unknown({})", idx.0));
                panic!(
                    "Atom assertion failed: expected '{}', got '{}'",
                    expected, actual
                );
            }
        }
        _ => panic!(
            "Expected atom '{}', got non-atom term: {:?}",
            expected, term
        ),
    }
}

/// Assert that a TermValue is an integer with the given value
pub fn assert_int(term: &TermValue, expected: i32) {
    match term {
        TermValue::SmallInt(actual) => {
            if *actual != expected {
                panic!(
                    "Integer assertion failed: expected {}, got {}",
                    expected, actual
                );
            }
        }
        _ => panic!(
            "Expected integer {}, got non-integer term: {:?}",
            expected, term
        ),
    }
}

/// Assert that a TermValue is a list with the given length
pub fn assert_list_length(term: &TermValue, expected_length: usize) {
    let actual_length = term.list_length();
    if actual_length != expected_length {
        panic!(
            "List length assertion failed: expected {}, got {}. Term: {:?}",
            expected_length, actual_length, term
        );
    }
}

/// Assert that a TermValue is a tuple with the given arity
pub fn assert_tuple_arity(term: &TermValue, expected_arity: usize) {
    let actual_arity = term.tuple_arity();
    if actual_arity != expected_arity {
        panic!(
            "Tuple arity assertion failed: expected {}, got {}. Term: {:?}",
            expected_arity, actual_arity, term
        );
    }
}

/// Assert that a map contains a key
pub fn assert_map_has_key<T: AtomTableOps>(
    map: &TermValue,
    key_name: &str,
    table: &T
) {
    let key = TermValue::atom(key_name, table);
    if map.map_get(&key).is_none() {
        panic!(
            "Map assertion failed: expected key '{}' to exist in map: {:?}",
            key_name, map
        );
    }
}

/// Assert that a map has a specific key-value pair
pub fn assert_map_contains<T: AtomTableOps>(
    map: &TermValue,
    key_name: &str,
    expected_value: &TermValue,
    table: &T
) {
    let key = TermValue::atom(key_name, table);
    match map.map_get(&key) {
        Some(actual_value) => {
            if actual_value != expected_value {
                panic!(
                    "Map value assertion failed for key '{}': expected {:?}, got {:?}",
                    key_name, expected_value, actual_value
                );
            }
        }
        None => panic!(
            "Map key assertion failed: key '{}' not found in map: {:?}",
            key_name, map
        ),
    }
}

// ── Generic Testing Utilities ──────────────────────────────────────────────

/// Test that a function correctly handles all common atom types
pub fn test_with_common_atoms<T: AtomTableOps, F>(
    table: &T,
    mut test_fn: F
) 
where 
    F: FnMut(&str, TermValue),
{
    let common_atoms = [
        "ok", "error", "true", "false", "undefined", "badarg", "nil",
        "atom", "binary", "boolean", "float", "function", "integer",
        "list", "map", "pid", "port", "reference", "tuple"
    ];
    
    for &atom_name in &common_atoms {
        let atom_term = TermValue::atom(atom_name, table);
        test_fn(atom_name, atom_term);
    }
}

/// Benchmark helper - measure time for an operation
/// 
/// Note: This is a no-op in no_std environments. 
/// Returns the result and 0 for elapsed time.
pub fn time_operation<F, R>(operation: F) -> (R, u128)
where
    F: FnOnce() -> R,
{
    let result = operation();
    // In no_std, we can't measure time, so return 0
    (result, 0)
}

/// Create a test user fixture
pub fn create_user_fixture<T: AtomTableOps>(
    name: &str, 
    id: i32, 
    role: &str,
    table: &T
) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("name", table), TermValue::atom(name, table)),
        (TermValue::atom("id", table), TermValue::int(id)),
        (TermValue::atom("role", table), TermValue::atom(role, table)),
        (TermValue::atom("active", table), TermValue::atom("true", table)),
    ])
}

/// Create a test config fixture
pub fn create_config_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("host", table), TermValue::atom("localhost", table)),
        (TermValue::atom("port", table), TermValue::int(8080)),
        (TermValue::atom("ssl", table), TermValue::atom("false", table)),
        (
            TermValue::atom("database", table),
            TermValue::map(vec![
                (TermValue::atom("host", table), TermValue::atom("db.example.com", table)),
                (TermValue::atom("port", table), TermValue::int(5432)),
                (TermValue::atom("name", table), TermValue::atom("myapp", table)),
            ])
        ),
    ])
}

/// Create test statistics fixture
pub fn create_stats_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("requests_total", table), TermValue::int(1000)),
        (TermValue::atom("errors_total", table), TermValue::int(5)),
        (TermValue::atom("uptime_seconds", table), TermValue::int(86400)),
        (TermValue::atom("memory_mb", table), TermValue::int(512)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mocks::MockAtomTable;

    #[test]
    fn test_generic_atom_creation() {
        let table = MockAtomTable::new();
        
        let hello_atom = atom("hello", &table);
        let world_atom = atom("world", &table);
        
        assert_atom_str(&hello_atom, "hello", &table);
        assert_atom_str(&world_atom, "world", &table);
    }

    #[test]
    fn test_multiple_atoms() {
        let table = MockAtomTable::new();
        
        let atom_terms = atoms(&["red", "green", "blue"], &table);
        assert_eq!(atom_terms.len(), 3);
        
        assert_atom_str(&atom_terms[0], "red", &table);
        assert_atom_str(&atom_terms[1], "green", &table);
        assert_atom_str(&atom_terms[2], "blue", &table);
    }

    #[test]
    fn test_helper_assertions() {
        let table = MockAtomTable::new();
        
        let int_term = TermValue::int(42);
        let atom_term = atom("test", &table);
        let list_term = int_list(&[1, 2, 3]);
        let tuple_term = int_tuple(&[10, 20]);
        
        assert_int(&int_term, 42);
        assert_atom_str(&atom_term, "test", &table);
        assert_list_length(&list_term, 3);
        assert_tuple_arity(&tuple_term, 2);
    }

    #[test]
    fn test_complex_test_data() {
        let table = MockAtomTable::new();
        let data = create_complex_test_data(&table);
        
        // Should be a map with user, session, and metadata
        let user_key = TermValue::atom("user", &table);
        let session_key = TermValue::atom("session", &table);
        let metadata_key = TermValue::atom("metadata", &table);
        
        let user = data.map_get(&user_key).unwrap();
        let session = data.map_get(&session_key).unwrap();
        let metadata = data.map_get(&metadata_key).unwrap();
        
        // Verify structure
        let name_key = TermValue::atom("name", &table);
        let version_key = TermValue::atom("version", &table);
        assert!(user.map_get(&name_key).is_some());
        assert_tuple_arity(session, 3);
        assert!(metadata.map_get(&version_key).is_some());
    }

    #[test]
    fn test_time_operation() {
        let (result, _time) = time_operation(|| {
            // In no_std, we can't actually sleep or measure time
            42
        });
        
        assert_eq!(result, 42);
        // Don't assert on time in no_std environment
    }

    #[test] 
    fn test_common_atoms() {
        let table = MockAtomTable::new();
        let mut count = 0;
        
        test_with_common_atoms(&table, |name, term| {
            assert_atom_str(&term, name, &table);
            count += 1;
        });
        
        assert!(count > 10); // Should test many atoms
    }

    #[test]
    fn test_map_assertions() {
        let table = MockAtomTable::new();
        
        let test_map = TermValue::map(vec![
            (TermValue::atom("name", &table), TermValue::atom("alice", &table)),
            (TermValue::atom("age", &table), TermValue::int(30)),
        ]);
        
        // Test map has key
        assert_map_has_key(&test_map, "name", &table);
        assert_map_has_key(&test_map, "age", &table);
        
        // Test map contains specific values
        assert_map_contains(&test_map, "name", &TermValue::atom("alice", &table), &table);
        assert_map_contains(&test_map, "age", &TermValue::int(30), &table);
    }

    #[test]
    fn test_fixture_creation() {
        let table = MockAtomTable::new();
        
        // Test user fixture
        let user = create_user_fixture("bob", 123, "admin", &table);
        assert_map_has_key(&user, "name", &table);
        assert_map_has_key(&user, "id", &table);
        assert_map_has_key(&user, "role", &table);
        
        // Test config fixture
        let config = create_config_fixture(&table);
        assert_map_has_key(&config, "host", &table);
        assert_map_has_key(&config, "port", &table);
        assert_map_has_key(&config, "database", &table);
        
        // Test stats fixture
        let stats = create_stats_fixture(&table);
        assert_map_has_key(&stats, "requests_total", &table);
        assert_map_contains(&stats, "requests_total", &TermValue::int(1000), &table);
    }
}