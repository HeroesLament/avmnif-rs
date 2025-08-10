//! AtomVM Atom Table Interface
//!
//! This module provides safe Rust bindings to the AtomVM atom table,
//! allowing Ports and NIFs to interact with the VM's atom storage system.
//!
//! # Safety
//!
//! The atom table is shared across the entire VM and uses read-write locks
//! for thread safety. All operations through this interface are safe to
//! call from multiple threads.
//!
//! # Examples
//!
//! ```rust
//! use avmnif::atom::{AtomTable, AtomIndex};
//!
//! // Get reference to the global atom table
//! let atom_table = AtomTable::global();
//!
//! // Create or get existing atom
//! let hello_atom = atom_table.ensure_atom(b"hello")?;
//!
//! // Retrieve atom data
//! let atom_data = atom_table.get_atom_string(hello_atom)?;
//! assert_eq!(atom_data, b"hello");
//!
//! // Compare atoms
//! let world_atom = atom_table.ensure_atom(b"world")?;
//! assert!(atom_table.compare_atoms(hello_atom, world_atom) != 0);
//! ```

extern crate alloc;

use core::ffi::c_void;
use core::fmt;
use core::slice;
use core::str;
use alloc::vec::Vec;

/// Opaque handle to the AtomVM atom table
#[repr(transparent)]
pub struct AtomTable(*mut c_void);

/// Index into the atom table
pub type AtomIndex = u32;

/// Result of atom table operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomTableResult {
    Ok,
    NotFound,
    AllocationFailed,
    InvalidLength,
}

/// Copy options for atom insertion
#[repr(u32)]
pub enum AtomCopyOpt {
    /// Reference existing data (caller must ensure lifetime)
    Reference = 0,
    /// Copy atom data into table-owned memory
    Copy = 1,
    /// Check if atom already exists, don't create
    AlreadyExisting = 2,
}

/// Options for bulk atom operations
#[repr(u32)]
pub enum EnsureAtomsOpt {
    /// Standard encoding (length byte + data)
    Standard = 0,
    /// Long encoding (variable-length encoding)
    LongEncoding = 1,
}

/// Errors that can occur during atom operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomError {
    /// Atom not found in table
    NotFound,
    /// Memory allocation failed
    AllocationFailed,
    /// Invalid atom length (too long or encoding error)
    InvalidLength,
    /// Null pointer returned from C API
    NullPointer,
    /// Invalid atom index
    InvalidIndex,
}

impl fmt::Display for AtomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomError::NotFound => write!(f, "atom not found in table"),
            AtomError::AllocationFailed => write!(f, "memory allocation failed"),
            AtomError::InvalidLength => write!(f, "invalid atom length"),
            AtomError::NullPointer => write!(f, "unexpected null pointer from atom table"),
            AtomError::InvalidIndex => write!(f, "invalid atom index"),
        }
    }
}

/// Reference to atom data stored in the table
#[derive(Debug)]
pub struct AtomRef<'a> {
    data: &'a [u8],
    index: AtomIndex,
}

impl<'a> AtomRef<'a> {
    /// Get the atom's index
    pub fn index(&self) -> AtomIndex {
        self.index
    }

    /// Get the atom's data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        self.data
    }

    /// Get the atom's data as a string (if valid UTF-8)
    pub fn as_str(&self) -> Result<&str, str::Utf8Error> {
        str::from_utf8(self.data)
    }

    /// Get the atom's length in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the atom is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<'a> AsRef<[u8]> for AtomRef<'a> {
    fn as_ref(&self) -> &[u8] {
        self.data
    }
}

impl<'a> PartialEq<[u8]> for AtomRef<'a> {
    fn eq(&self, other: &[u8]) -> bool {
        self.data == other
    }
}

impl<'a> PartialEq<&[u8]> for AtomRef<'a> {
    fn eq(&self, other: &&[u8]) -> bool {
        self.data == *other
    }
}

impl<'a> PartialEq<str> for AtomRef<'a> {
    fn eq(&self, other: &str) -> bool {
        self.data == other.as_bytes()
    }
}

// FFI declarations
extern "C" {
    fn atom_table_get_atom_string(
        table: *mut c_void,
        index: AtomIndex,
        out_size: *mut usize,
    ) -> *const u8;

    fn atom_table_ensure_atom(
        table: *mut c_void,
        atom_data: *const u8,
        atom_len: usize,
        opts: u32,
        result: *mut AtomIndex,
    ) -> u32;

    fn atom_table_ensure_atoms(
        table: *mut c_void,
        atoms: *const c_void,
        count: usize,
        translate_table: *mut AtomIndex,
        opt: u32,
    ) -> u32;

    fn atom_table_count(table: *mut c_void) -> usize;

    fn atom_table_is_equal_to_atom_string(
        table: *mut c_void,
        atom_index: AtomIndex,
        string_data: *const u8,
        string_len: usize,
    ) -> bool;

    fn atom_table_cmp_using_atom_index(
        table: *mut c_void,
        atom1: AtomIndex,
        atom2: AtomIndex,
    ) -> i32;

    // Platform-specific function to get global atom table
    fn atomvm_get_global_atom_table() -> *mut c_void;
}

// Helper to convert C result to Rust enum
fn result_from_c(result: u32) -> AtomTableResult {
    match result {
        0 => AtomTableResult::Ok,
        1 => AtomTableResult::NotFound,
        2 => AtomTableResult::AllocationFailed,
        3 => AtomTableResult::InvalidLength,
        _ => AtomTableResult::AllocationFailed, // Unknown error, treat as alloc failure
    }
}

impl AtomTable {
    /// Get a reference to the global atom table
    ///
    /// # Safety
    ///
    /// This assumes the AtomVM is properly initialized and the global
    /// atom table exists. In a properly initialized Port or NIF context,
    /// this should always be safe.
    pub fn global() -> Self {
        let ptr = unsafe { atomvm_get_global_atom_table() };
        AtomTable(ptr)
    }

    /// Create an AtomTable from a raw pointer
    ///
    /// # Safety
    ///
    /// The caller must ensure the pointer is valid and points to a
    /// properly initialized AtomVM atom table.
    pub unsafe fn from_raw(ptr: *mut c_void) -> Self {
        AtomTable(ptr)
    }

    /// Get the raw pointer to the atom table
    ///
    /// # Safety
    ///
    /// The returned pointer should only be used with AtomVM C APIs
    /// and must not outlive this AtomTable instance.
    pub unsafe fn as_raw(&self) -> *mut c_void {
        self.0
    }

    /// Get the number of atoms currently in the table
    pub fn count(&self) -> usize {
        unsafe { atom_table_count(self.0) }
    }

    /// Get atom data by index
    ///
    /// Returns a reference to the atom's data that is valid as long as
    /// the atom table exists (which is typically the VM lifetime).
    pub fn get_atom_string(&self, index: AtomIndex) -> Result<AtomRef<'_>, AtomError> {
        let mut size: usize = 0;
        let ptr = unsafe { atom_table_get_atom_string(self.0, index, &mut size) };
        
        if ptr.is_null() {
            return Err(AtomError::InvalidIndex);
        }

        let data = unsafe { slice::from_raw_parts(ptr, size) };
        Ok(AtomRef { data, index })
    }

    /// Ensure an atom exists in the table, creating it if necessary
    ///
    /// This function will copy the atom data into table-managed memory,
    /// so the input data doesn't need to persist after this call.
    ///
    /// Returns the atom index, which may be for an existing atom if
    /// the same data was already in the table.
    pub fn ensure_atom(&self, atom_data: &[u8]) -> Result<AtomIndex, AtomError> {
        let mut result: AtomIndex = 0;
        let status = unsafe {
            atom_table_ensure_atom(
                self.0,
                atom_data.as_ptr(),
                atom_data.len(),
                AtomCopyOpt::Copy as u32,
                &mut result,
            )
        };

        match result_from_c(status) {
            AtomTableResult::Ok => Ok(result),
            AtomTableResult::NotFound => Err(AtomError::NotFound),
            AtomTableResult::AllocationFailed => Err(AtomError::AllocationFailed),
            AtomTableResult::InvalidLength => Err(AtomError::InvalidLength),
        }
    }

    /// Ensure an atom exists, but only if it already exists
    ///
    /// This is useful for looking up atoms without creating new ones.
    pub fn find_atom(&self, atom_data: &[u8]) -> Result<AtomIndex, AtomError> {
        let mut result: AtomIndex = 0;
        let status = unsafe {
            atom_table_ensure_atom(
                self.0,
                atom_data.as_ptr(),
                atom_data.len(),
                AtomCopyOpt::AlreadyExisting as u32,
                &mut result,
            )
        };

        match result_from_c(status) {
            AtomTableResult::Ok => Ok(result),
            AtomTableResult::NotFound => Err(AtomError::NotFound),
            AtomTableResult::AllocationFailed => Err(AtomError::AllocationFailed),
            AtomTableResult::InvalidLength => Err(AtomError::InvalidLength),
        }
    }

    /// Ensure an atom exists using a string slice
    pub fn ensure_atom_str(&self, atom_str: &str) -> Result<AtomIndex, AtomError> {
        self.ensure_atom(atom_str.as_bytes())
    }

    /// Find an atom using a string slice
    pub fn find_atom_str(&self, atom_str: &str) -> Result<AtomIndex, AtomError> {
        self.find_atom(atom_str.as_bytes())
    }

    /// Check if an atom equals the given byte string
    pub fn atom_equals(&self, atom_index: AtomIndex, data: &[u8]) -> bool {
        unsafe {
            atom_table_is_equal_to_atom_string(
                self.0,
                atom_index,
                data.as_ptr(),
                data.len(),
            )
        }
    }

    /// Check if an atom equals the given string
    pub fn atom_equals_str(&self, atom_index: AtomIndex, s: &str) -> bool {
        self.atom_equals(atom_index, s.as_bytes())
    }

    /// Compare two atoms lexicographically
    ///
    /// Returns:
    /// - negative value if atom1 < atom2
    /// - 0 if atom1 == atom2  
    /// - positive value if atom1 > atom2
    pub fn compare_atoms(&self, atom1: AtomIndex, atom2: AtomIndex) -> i32 {
        unsafe { atom_table_cmp_using_atom_index(self.0, atom1, atom2) }
    }

    /// Bulk insert/lookup atoms from encoded atom data
    ///
    /// This is primarily used when loading BEAM modules that contain
    /// atom tables in the standard BEAM format.
    ///
    /// Returns a translation table mapping from the input atom indices
    /// to the global atom table indices.
    pub fn ensure_atoms_bulk(
        &self,
        atoms_data: &[u8],
        count: usize,
        encoding: EnsureAtomsOpt,
    ) -> Result<Vec<AtomIndex>, AtomError> {
        let mut translate_table = Vec::with_capacity(count);
        translate_table.resize(count, 0u32);
        
        let status = unsafe {
            atom_table_ensure_atoms(
                self.0,
                atoms_data.as_ptr() as *const c_void,
                count,
                translate_table.as_mut_ptr(),
                encoding as u32,
            )
        };

        match result_from_c(status) {
            AtomTableResult::Ok => Ok(translate_table),
            AtomTableResult::NotFound => Err(AtomError::NotFound),
            AtomTableResult::AllocationFailed => Err(AtomError::AllocationFailed),
            AtomTableResult::InvalidLength => Err(AtomError::InvalidLength),
        }
    }
}

// Safety: AtomTable operations are thread-safe due to internal locking
unsafe impl Send for AtomTable {}
unsafe impl Sync for AtomTable {}

/// Common atom constants
///
/// These represent frequently used atoms in Erlang/Elixir systems
pub mod atoms {
    use super::{AtomIndex, AtomTable, AtomError};
    use alloc::sync::Arc;
    use core::sync::atomic::{AtomicU32, Ordering};

    // Simple atomic-based lazy initialization for no_std
    static OK_ATOM: AtomicU32 = AtomicU32::new(0);
    static ERROR_ATOM: AtomicU32 = AtomicU32::new(0);
    static TRUE_ATOM: AtomicU32 = AtomicU32::new(0);
    static FALSE_ATOM: AtomicU32 = AtomicU32::new(0);
    static NIL_ATOM: AtomicU32 = AtomicU32::new(0);
    static UNDEFINED_ATOM: AtomicU32 = AtomicU32::new(0);
    static NOPROC_ATOM: AtomicU32 = AtomicU32::new(0);
    static NORMAL_ATOM: AtomicU32 = AtomicU32::new(0);
    static TIMEOUT_ATOM: AtomicU32 = AtomicU32::new(0);
    static BADARG_ATOM: AtomicU32 = AtomicU32::new(0);
    static BADARITH_ATOM: AtomicU32 = AtomicU32::new(0);
    static EXIT_ATOM: AtomicU32 = AtomicU32::new(0);
    static KILL_ATOM: AtomicU32 = AtomicU32::new(0);
    static MONITOR_ATOM: AtomicU32 = AtomicU32::new(0);
    static PORT_ATOM: AtomicU32 = AtomicU32::new(0);
    static PORT_DATA_ATOM: AtomicU32 = AtomicU32::new(0);
    static PORT_CLOSE_ATOM: AtomicU32 = AtomicU32::new(0);

    fn get_or_create_atom(atomic: &AtomicU32, name: &str) -> Result<AtomIndex, AtomError> {
        let current = atomic.load(Ordering::Relaxed);
        if current != 0 {
            return Ok(current);
        }

        let table = AtomTable::global();
        let index = table.ensure_atom_str(name)?;
        
        // Try to set it, but if someone else beat us to it, use their value
        match atomic.compare_exchange_weak(0, index, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => Ok(index),
            Err(existing) => Ok(existing),
        }
    }

    /// Get the atom index for `ok`
    pub fn ok() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&OK_ATOM, "ok")
    }

    /// Get the atom index for `error`
    pub fn error() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&ERROR_ATOM, "error")
    }

    /// Get the atom index for `true`
    pub fn true_atom() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&TRUE_ATOM, "true")
    }

    /// Get the atom index for `false`
    pub fn false_atom() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&FALSE_ATOM, "false")
    }

    /// Get the atom index for `nil`
    pub fn nil() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&NIL_ATOM, "nil")
    }

    /// Get the atom index for `undefined`
    pub fn undefined() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&UNDEFINED_ATOM, "undefined")
    }

    /// Get the atom index for `noproc`
    pub fn noproc() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&NOPROC_ATOM, "noproc")
    }

    /// Get the atom index for `normal`
    pub fn normal() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&NORMAL_ATOM, "normal")
    }

    /// Get the atom index for `timeout`
    pub fn timeout() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&TIMEOUT_ATOM, "timeout")
    }

    /// Get the atom index for `badarg`
    pub fn badarg() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&BADARG_ATOM, "badarg")
    }

    /// Get the atom index for `badarith`
    pub fn badarith() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&BADARITH_ATOM, "badarith")
    }

    /// Get the atom index for `exit`
    pub fn exit() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&EXIT_ATOM, "exit")
    }

    /// Get the atom index for `kill`
    pub fn kill() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&KILL_ATOM, "kill")
    }

    /// Get the atom index for `monitor`
    pub fn monitor() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&MONITOR_ATOM, "monitor")
    }

    /// Get the atom index for `port`
    pub fn port() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&PORT_ATOM, "port")
    }

    /// Get the atom index for `port_data`
    pub fn port_data() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&PORT_DATA_ATOM, "port_data")
    }

    /// Get the atom index for `port_close`
    pub fn port_close() -> Result<AtomIndex, AtomError> {
        get_or_create_atom(&PORT_CLOSE_ATOM, "port_close")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_creation_and_retrieval() {
        let table = AtomTable::global();
        
        // Create an atom
        let index = table.ensure_atom(b"test_atom").expect("Failed to create atom");
        
        // Retrieve it
        let atom_ref = table.get_atom_string(index).expect("Failed to get atom");
        assert_eq!(atom_ref.as_bytes(), b"test_atom");
        
        // Ensure same atom returns same index
        let index2 = table.ensure_atom(b"test_atom").expect("Failed to get existing atom");
        assert_eq!(index, index2);
    }

    #[test]
    fn test_atom_comparison() {
        let table = AtomTable::global();
        
        let atom1 = table.ensure_atom(b"abc").unwrap();
        let atom2 = table.ensure_atom(b"def").unwrap();
        let atom3 = table.ensure_atom(b"abc").unwrap();
        
        assert_eq!(atom1, atom3);
        assert_ne!(atom1, atom2);
        
        assert!(table.compare_atoms(atom1, atom2) < 0);
        assert_eq!(table.compare_atoms(atom1, atom3), 0);
        assert!(table.compare_atoms(atom2, atom1) > 0);
    }

    #[test]
    fn test_atom_equality() {
        let table = AtomTable::global();
        
        let atom_index = table.ensure_atom_str("hello").unwrap();
        
        assert!(table.atom_equals_str(atom_index, "hello"));
        assert!(!table.atom_equals_str(atom_index, "world"));
        assert!(table.atom_equals(atom_index, b"hello"));
        assert!(!table.atom_equals(atom_index, b"world"));
    }

    #[test]
    fn test_common_atoms() {
        use crate::atom::atoms;
        
        let ok_atom = atoms::ok().unwrap();
        let error_atom = atoms::error().unwrap();
        
        let table = AtomTable::global();
        assert!(table.atom_equals_str(ok_atom, "ok"));
        assert!(table.atom_equals_str(error_atom, "error"));
        assert_ne!(ok_atom, error_atom);
    }
}