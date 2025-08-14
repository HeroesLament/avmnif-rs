//! Mock implementations for testing
//! 
//! This module contains mock implementations of AtomVM components that allow
//! testing without requiring the full AtomVM runtime environment.
//!
//! # Design Philosophy
//!
//! No global state, no singletons - each test creates its own mock instances.
//! This ensures perfect test isolation and makes the mocks completely generic.

extern crate alloc;

use alloc::{collections::BTreeMap, string::{String, ToString}, vec::Vec, boxed::Box};
use core::cell::RefCell;
use crate::atom::{AtomIndex, AtomTableOps, AtomError, AtomRef, EnsureAtomsOpt};

// ── Mock Atom Table Implementation ─────────────────────────────────────────

/// Mock implementation of AtomTable for testing
/// 
/// This mock provides a pure Rust implementation of atom table operations
/// that maintains the same behavioral contracts as the real AtomVM atom table.
/// 
/// Each instance is completely independent - no shared state between instances.
#[derive(Debug)]
pub struct MockAtomTable {
    atoms: RefCell<BTreeMap<String, u32>>,
    reverse_atoms: RefCell<BTreeMap<u32, String>>,
    next_id: RefCell<u32>,
}

impl MockAtomTable {
    /// Create a new mock atom table with fresh state
    /// 
    /// Each call creates a completely independent table.
    /// Tests should create their own instances for isolation.
    pub fn new() -> Self {
        let table = Self {
            atoms: RefCell::new(BTreeMap::new()),
            reverse_atoms: RefCell::new(BTreeMap::new()),
            next_id: RefCell::new(1), // Reserve 0 for error cases
        };
        
        // Pre-populate with common atoms that AtomVM typically has
        table.pre_populate_common_atoms();
        table
    }

    /// Create a minimal mock table (no pre-populated atoms)
    /// 
    /// Useful for tests that want complete control over what atoms exist.
    pub fn new_empty() -> Self {
        Self {
            atoms: RefCell::new(BTreeMap::new()),
            reverse_atoms: RefCell::new(BTreeMap::new()),
            next_id: RefCell::new(1),
        }
    }

    /// Create a mock table with custom pre-populated atoms
    /// 
    /// Useful for tests that need specific atoms to exist.
    pub fn new_with_atoms(atoms: &[&str]) -> Self {
        let table = Self::new_empty();
        
        for atom_name in atoms {
            let _ = table.ensure_atom_str(atom_name);
        }
        
        table
    }

    fn pre_populate_common_atoms(&self) {
        let common_atoms = [
            "ok", "error", "true", "false", "undefined", "badarg", "nil",
            "atom", "binary", "bitstring", "boolean", "float", "function",
            "integer", "list", "map", "pid", "port", "reference", "tuple"
        ];
        
        for atom_name in &common_atoms {
            let _ = self.ensure_atom_str(atom_name);
        }
    }

    /// Get atom name by index (reverse lookup) - helper method
    pub fn get_atom_name(&self, AtomIndex(idx): AtomIndex) -> Option<String> {
        let reverse_atoms = self.reverse_atoms.borrow();
        reverse_atoms.get(&idx).cloned()
    }

    /// Get all atoms currently in the table (for debugging)
    pub fn list_all_atoms(&self) -> Vec<(AtomIndex, String)> {
        let reverse_atoms = self.reverse_atoms.borrow();
        reverse_atoms.iter()
            .map(|(&idx, name)| (AtomIndex(idx), name.clone()))
            .collect()
    }

    /// Clear all atoms (useful for test setup)
    pub fn clear(&self) {
        self.atoms.borrow_mut().clear();
        self.reverse_atoms.borrow_mut().clear();
        *self.next_id.borrow_mut() = 1;
    }
}

// ── AtomTableOps Implementation ────────────────────────────────────────────

impl AtomTableOps for MockAtomTable {
    fn count(&self) -> usize {
        self.atoms.borrow().len()
    }

    fn get_atom_string(&self, AtomIndex(idx): AtomIndex) -> Result<AtomRef<'_>, AtomError> {
        // For the mock, we'll work around the lifetime issue by using a different approach
        let reverse_atoms = self.reverse_atoms.borrow();
        if let Some(atom_str) = reverse_atoms.get(&idx) {
            // Since we can't return a proper AtomRef with borrowed data in a mock,
            // we'll create a static string for the mock. This is safe for testing.
            let leaked_str: &'static str = Box::leak(atom_str.clone().into_boxed_str());
            Ok(AtomRef::new(leaked_str.as_bytes(), AtomIndex(idx)))
        } else {
            Err(AtomError::NotFound)
        }
    }

    fn ensure_atom(&self, name: &[u8]) -> Result<AtomIndex, AtomError> {
        let name_str = core::str::from_utf8(name)
            .map_err(|_| AtomError::InvalidAtomData)?;
        self.ensure_atom_str(name_str)
    }

    fn ensure_atom_str(&self, name: &str) -> Result<AtomIndex, AtomError> {
        if name.len() > 255 {
            return Err(AtomError::InvalidAtomData);
        }
        
        // Check if atom already exists
        {
            let atoms = self.atoms.borrow();
            if let Some(&existing_id) = atoms.get(name) {
                return Ok(AtomIndex(existing_id));
            }
        }
        
        // Create new atom
        let mut next_id = self.next_id.borrow_mut();
        let new_id = *next_id;
        *next_id += 1;
        
        // Insert into both maps
        self.atoms.borrow_mut().insert(name.to_string(), new_id);
        self.reverse_atoms.borrow_mut().insert(new_id, name.to_string());
        
        Ok(AtomIndex(new_id))
    }

    fn find_atom(&self, name: &[u8]) -> Result<AtomIndex, AtomError> {
        let name_str = core::str::from_utf8(name)
            .map_err(|_| AtomError::InvalidAtomData)?;
        
        let atoms = self.atoms.borrow();
        atoms.get(name_str)
            .map(|&id| AtomIndex(id))
            .ok_or(AtomError::NotFound)
    }

    fn atom_equals(&self, AtomIndex(idx): AtomIndex, name: &[u8]) -> bool {
        let name_str = match core::str::from_utf8(name) {
            Ok(s) => s,
            Err(_) => return false,
        };
        self.atom_equals_str(AtomIndex(idx), name_str)
    }

    fn atom_equals_str(&self, AtomIndex(idx): AtomIndex, name: &str) -> bool {
        let reverse_atoms = self.reverse_atoms.borrow();
        if let Some(atom_name) = reverse_atoms.get(&idx) {
            atom_name == name
        } else {
            false
        }
    }

    fn compare_atoms(&self, AtomIndex(idx1): AtomIndex, AtomIndex(idx2): AtomIndex) -> i32 {
        let reverse_atoms = self.reverse_atoms.borrow();
        let name1 = reverse_atoms.get(&idx1);
        let name2 = reverse_atoms.get(&idx2);
        
        match (name1, name2) {
            (Some(n1), Some(n2)) => {
                if n1 < n2 { -1 }
                else if n1 > n2 { 1 }
                else { 0 }
            }
            (Some(_), None) => 1,   // Valid atom > invalid atom
            (None, Some(_)) => -1,  // Invalid atom < valid atom  
            (None, None) => 0,      // Both invalid
        }
    }

    fn ensure_atoms_bulk(
        &self, 
        _data: &[u8], 
        _count: usize, 
        _opt: EnsureAtomsOpt
    ) -> Result<Vec<AtomIndex>, AtomError> {
        // For the mock, we'll just return an error since bulk operations
        // are complex to implement and rarely used in tests
        Err(AtomError::AllocationFailed)
    }
}

// ── Additional Mock Implementations ────────────────────────────────────────

// Future: Add MockContext, MockHeap, etc. here as needed

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_atom_table_basic_operations() {
        let table = MockAtomTable::new();
        
        // Test atom creation
        let ok_atom = table.ensure_atom_str("ok").unwrap();
        let error_atom = table.ensure_atom_str("error").unwrap();
        
        // Test that same name returns same index
        let ok_atom2 = table.ensure_atom_str("ok").unwrap();
        assert_eq!(ok_atom, ok_atom2);
        
        // Test that different names return different indices
        assert_ne!(ok_atom, error_atom);
        
        // Test atom string comparison
        assert!(table.atom_equals_str(ok_atom, "ok"));
        assert!(!table.atom_equals_str(ok_atom, "error"));
        assert!(table.atom_equals_str(error_atom, "error"));
        assert!(!table.atom_equals_str(error_atom, "ok"));
    }

    #[test]
    fn test_mock_atom_table_reverse_lookup() {
        let table = MockAtomTable::new();
        
        let hello_atom = table.ensure_atom_str("hello").unwrap();
        let world_atom = table.ensure_atom_str("world").unwrap();
        
        // Test reverse lookup
        assert_eq!(table.get_atom_name(hello_atom), Some("hello".to_string()));
        assert_eq!(table.get_atom_name(world_atom), Some("world".to_string()));
        
        // Test non-existent atom
        assert_eq!(table.get_atom_name(AtomIndex(9999)), None);
    }

    #[test]
    fn test_mock_atom_table_byte_operations() {
        let table = MockAtomTable::new();
        
        // Test ensure_atom with bytes
        let test_atom = table.ensure_atom(b"test").unwrap();
        assert!(table.atom_equals(test_atom, b"test"));
        assert!(!table.atom_equals(test_atom, b"other"));
        
        // Test find_atom
        let found = table.find_atom(b"test").unwrap();
        assert_eq!(found, test_atom);
        
        // Test find non-existent
        assert!(table.find_atom(b"nonexistent").is_err());
    }

    #[test]
    fn test_mock_atom_table_compare() {
        let table = MockAtomTable::new();
        
        let atom_a = table.ensure_atom_str("aaa").unwrap();
        let atom_b = table.ensure_atom_str("bbb").unwrap();
        let atom_a2 = table.ensure_atom_str("aaa").unwrap();
        
        // Test comparison
        assert!(table.compare_atoms(atom_a, atom_b) < 0);  // "aaa" < "bbb"
        assert!(table.compare_atoms(atom_b, atom_a) > 0);  // "bbb" > "aaa"
        assert_eq!(table.compare_atoms(atom_a, atom_a2), 0); // "aaa" == "aaa"
    }

    #[test]
    fn test_mock_atom_table_count() {
        let table = MockAtomTable::new();
        
        // Should start with pre-populated atoms
        let initial_count = table.count();
        assert!(initial_count > 0);
        
        // Add a new atom
        let _ = table.ensure_atom_str("new_atom").unwrap();
        assert_eq!(table.count(), initial_count + 1);
        
        // Adding same atom shouldn't increase count
        let _ = table.ensure_atom_str("new_atom").unwrap();
        assert_eq!(table.count(), initial_count + 1);
    }

    #[test]
    fn test_mock_atom_table_isolation() {
        // Test that new() creates isolated instances
        let table1 = MockAtomTable::new();
        let table2 = MockAtomTable::new();
        
        let atom1 = table1.ensure_atom_str("isolated").unwrap();
        
        // table2 shouldn't know about atoms from table1
        assert!(!table2.atom_equals_str(atom1, "isolated"));
        
        // But it can create its own
        let atom2 = table2.ensure_atom_str("isolated").unwrap();
        assert!(table2.atom_equals_str(atom2, "isolated"));
        
        // Both tables have the same pre-populated atoms, so "isolated" gets index 22 in both
        // This is actually correct behavior - the tables are isolated but deterministic
        assert_eq!(atom1, atom2); // Same index because same pre-population
        
        // Verify true isolation: table1 shouldn't accept table2's atoms for different strings
        let table1_unique = table1.ensure_atom_str("table1_only").unwrap();
        assert!(!table2.atom_equals_str(table1_unique, "table1_only"));
        
        let table2_unique = table2.ensure_atom_str("table2_only").unwrap(); 
        assert!(!table1.atom_equals_str(table2_unique, "table2_only"));
        
        // These unique atoms will have the same index (23) because they're the first unique atom
        // created in each table after "isolated", but they're in different tables
        assert_eq!(table1_unique, table2_unique); // Same index, different tables (correct behavior)
    }

    #[test]
    fn test_mock_atom_table_empty() {
        let table = MockAtomTable::new_empty();
        
        // Should start with no atoms
        assert_eq!(table.count(), 0);
        
        // Add an atom
        let hello_atom = table.ensure_atom_str("hello").unwrap();
        assert_eq!(table.count(), 1);
        assert!(table.atom_equals_str(hello_atom, "hello"));
    }

    #[test]
    fn test_mock_atom_table_with_custom_atoms() {
        let custom_atoms = ["red", "green", "blue"];
        let table = MockAtomTable::new_with_atoms(&custom_atoms);
        
        // Should have exactly the custom atoms
        assert_eq!(table.count(), 3);
        
        // All custom atoms should exist
        for atom_name in &custom_atoms {
            let atom_idx = table.find_atom_str(atom_name).unwrap();
            assert!(table.atom_equals_str(atom_idx, atom_name));
        }
        
        // Other atoms should not exist
        assert!(table.find_atom_str("yellow").is_err());
    }

    #[test]
    fn test_mock_atom_table_clear() {
        let table = MockAtomTable::new();
        
        // Should start with pre-populated atoms
        assert!(table.count() > 0);
        
        // Clear all atoms
        table.clear();
        assert_eq!(table.count(), 0);
        
        // Can add new atoms after clearing
        let hello_atom = table.ensure_atom_str("hello").unwrap();
        assert_eq!(table.count(), 1);
        assert!(table.atom_equals_str(hello_atom, "hello"));
    }

    #[test]
    fn test_mock_atom_table_list_all() {
        let table = MockAtomTable::new_with_atoms(&["a", "b", "c"]);
        
        let all_atoms = table.list_all_atoms();
        assert_eq!(all_atoms.len(), 3);
        
        // Should contain all our atoms
        let atom_names: Vec<String> = all_atoms.into_iter()
            .map(|(_, name)| name)
            .collect();
        assert!(atom_names.contains(&"a".to_string()));
        assert!(atom_names.contains(&"b".to_string()));
        assert!(atom_names.contains(&"c".to_string()));
    }

    #[test]
    fn test_mock_atom_table_error_conditions() {
        let table = MockAtomTable::new();
        
        // Test name too long
        let long_name = "a".repeat(256);
        assert!(table.ensure_atom_str(&long_name).is_err());
        
        // Test reverse lookup of non-existent atom
        assert_eq!(table.get_atom_name(AtomIndex(99999)), None);
        
        // Test bulk operations return error
        assert!(table.ensure_atoms_bulk(&[], 0, EnsureAtomsOpt::Standard).is_err());
    }
}