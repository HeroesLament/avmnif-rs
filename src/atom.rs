//! AtomVM Atom Table Interface
//!
//! This module provides safe Rust bindings to the AtomVM atom table,
//! allowing Ports and NIFs to interact with the VM's atom storage system.
//!
//! # Design Philosophy
//!
//! This module uses dependency injection throughout - no global state.
//! All operations take an `impl AtomTableOps` parameter, making the code
//! generic and testable with any atom table implementation.
//!
//! # Examples
//!
//! ```rust,ignore
//! use avmnif_rs::atom::{AtomTableOps, AtomTable};
//!
//! // In production - use real AtomVM table
//! let atom_table = AtomTable::new(context);
//! let hello_atom = atom_table.ensure_atom_str("hello")?;
//!
//! // In testing - use mock table
//! let atom_table = MockAtomTable::new();
//! let hello_atom = atom_table.ensure_atom_str("hello")?;
//!
//! // Both work the same way!
//! ```

extern crate alloc;

use core::fmt;
use core::str;
use alloc::vec::Vec;

// ── Core Types and Errors ───────────────────────────────────────────────────

/// Index into the atom table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AtomIndex(pub u32);

impl AtomIndex {
    pub const INVALID: AtomIndex = AtomIndex(0);
    
    pub fn new(index: u32) -> Self {
        AtomIndex(index)
    }
    
    pub fn get(self) -> u32 {
        self.0
    }
    
    pub fn is_valid(self) -> bool {
        self.0 != 0
    }
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
    /// Invalid atom data (bad UTF-8, null bytes, etc.)
    InvalidAtomData,
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
            AtomError::InvalidAtomData => write!(f, "invalid atom data or encoding"),
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
    pub fn new(data: &'a [u8], index: AtomIndex) -> Self {
        Self { data, index }
    }

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

// ── Generic Atom Table Operations Trait ────────────────────────────────────

/// Trait for atom table operations - the foundation of our generic design
/// 
/// Any implementation (real AtomVM, mock, in-memory, etc.) can provide
/// these operations, making the entire system generic and testable.
pub trait AtomTableOps {
    /// Get the number of atoms currently in the table
    fn count(&self) -> usize;

    /// Get atom data by index
    fn get_atom_string(&self, index: AtomIndex) -> Result<AtomRef<'_>, AtomError>;

    /// Ensure an atom exists in the table, creating it if necessary
    fn ensure_atom(&self, atom_data: &[u8]) -> Result<AtomIndex, AtomError>;

    /// Ensure an atom exists, but only if it already exists
    fn find_atom(&self, atom_data: &[u8]) -> Result<AtomIndex, AtomError>;

    /// Ensure an atom exists using a string slice
    fn ensure_atom_str(&self, atom_str: &str) -> Result<AtomIndex, AtomError> {
        self.ensure_atom(atom_str.as_bytes())
    }

    /// Find an atom using a string slice
    fn find_atom_str(&self, atom_str: &str) -> Result<AtomIndex, AtomError> {
        self.find_atom(atom_str.as_bytes())
    }

    /// Check if an atom equals the given byte string
    fn atom_equals(&self, atom_index: AtomIndex, data: &[u8]) -> bool;

    /// Check if an atom equals the given string
    fn atom_equals_str(&self, atom_index: AtomIndex, s: &str) -> bool {
        self.atom_equals(atom_index, s.as_bytes())
    }

    /// Compare two atoms lexicographically
    fn compare_atoms(&self, atom1: AtomIndex, atom2: AtomIndex) -> i32;

    /// Bulk insert/lookup atoms from encoded atom data
    fn ensure_atoms_bulk(
        &self,
        atoms_data: &[u8],
        count: usize,
        encoding: EnsureAtomsOpt,
    ) -> Result<Vec<AtomIndex>, AtomError>;
}

// ── AtomVM Implementation ───────────────────────────────────────────────────

use core::ffi::c_void;
use core::slice;

/// Opaque handle to the AtomVM atom table
#[repr(transparent)]
pub struct AtomTable(*mut c_void);

/// Result of atom table operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomTableResult {
    Ok,
    NotFound,
    AllocationFailed,
    InvalidLength,
}

// FFI declarations - Note: These expect raw u32 values, not AtomIndex structs
extern "C" {
    fn atom_table_get_atom_string(
        table: *mut c_void,
        index: u32,  // Raw u32, not AtomIndex
        out_size: *mut usize,
    ) -> *const u8;

    fn atom_table_ensure_atom(
        table: *mut c_void,
        atom_data: *const u8,
        atom_len: usize,
        opts: u32,
        result: *mut u32,  // Raw u32, not AtomIndex
    ) -> u32;

    fn atom_table_ensure_atoms(
        table: *mut c_void,
        atoms: *const c_void,
        count: usize,
        translate_table: *mut u32,  // Raw u32, not AtomIndex
        opt: u32,
    ) -> u32;

    fn atom_table_count(table: *mut c_void) -> usize;

    fn atom_table_is_equal_to_atom_string(
        table: *mut c_void,
        atom_index: u32,  // Raw u32, not AtomIndex
        string_data: *const u8,
        string_len: usize,
    ) -> bool;

    fn atom_table_cmp_using_atom_index(
        table: *mut c_void,
        atom1: u32,  // Raw u32, not AtomIndex
        atom2: u32,  // Raw u32, not AtomIndex
    ) -> i32;

    fn atomvm_get_global_atom_table() -> *mut c_void;
}

// Helper to convert C result to Rust enum
fn result_from_c(result: u32) -> AtomTableResult {
    match result {
        0 => AtomTableResult::Ok,
        1 => AtomTableResult::NotFound,
        2 => AtomTableResult::AllocationFailed,
        3 => AtomTableResult::InvalidLength,
        _ => AtomTableResult::AllocationFailed,
    }
}

impl AtomTable {
    /// Create an AtomTable from a raw pointer
    /// 
    /// # Safety
    /// The pointer must be valid and point to a real AtomVM atom table
    pub unsafe fn from_raw(ptr: *mut c_void) -> Self {
        AtomTable(ptr)
    }

    /// Create an AtomTable from the global AtomVM instance
    /// 
    /// This should only be used in production with a running AtomVM.
    /// For testing, use MockAtomTable instead.
    pub fn from_global() -> Self {
        let ptr = unsafe { atomvm_get_global_atom_table() };
        AtomTable(ptr)
    }

    /// Get the raw pointer to the atom table
    /// 
    /// # Safety
    /// The returned pointer should only be used with AtomVM C functions
    pub unsafe fn as_raw(&self) -> *mut c_void {
        self.0
    }
}

impl AtomTableOps for AtomTable {
    fn count(&self) -> usize {
        unsafe { atom_table_count(self.0) }
    }

    fn get_atom_string(&self, index: AtomIndex) -> Result<AtomRef<'_>, AtomError> {
        let mut size: usize = 0;
        let ptr = unsafe { atom_table_get_atom_string(self.0, index.0, &mut size) };
        
        if ptr.is_null() {
            return Err(AtomError::InvalidIndex);
        }

        let data = unsafe { slice::from_raw_parts(ptr, size) };
        Ok(AtomRef::new(data, index))
    }

    fn ensure_atom(&self, atom_data: &[u8]) -> Result<AtomIndex, AtomError> {
        let mut result: u32 = 0;  // Raw u32 for FFI
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
            AtomTableResult::Ok => Ok(AtomIndex(result)),
            AtomTableResult::NotFound => Err(AtomError::NotFound),
            AtomTableResult::AllocationFailed => Err(AtomError::AllocationFailed),
            AtomTableResult::InvalidLength => Err(AtomError::InvalidLength),
        }
    }

    fn find_atom(&self, atom_data: &[u8]) -> Result<AtomIndex, AtomError> {
        let mut result: u32 = 0;  // Raw u32 for FFI
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
            AtomTableResult::Ok => Ok(AtomIndex(result)),
            AtomTableResult::NotFound => Err(AtomError::NotFound),
            AtomTableResult::AllocationFailed => Err(AtomError::AllocationFailed),
            AtomTableResult::InvalidLength => Err(AtomError::InvalidLength),
        }
    }

    fn atom_equals(&self, atom_index: AtomIndex, data: &[u8]) -> bool {
        unsafe {
            atom_table_is_equal_to_atom_string(
                self.0,
                atom_index.0,  // Extract raw u32
                data.as_ptr(),
                data.len(),
            )
        }
    }

    fn compare_atoms(&self, atom1: AtomIndex, atom2: AtomIndex) -> i32 {
        unsafe { atom_table_cmp_using_atom_index(self.0, atom1.0, atom2.0) }
    }

    fn ensure_atoms_bulk(
        &self,
        atoms_data: &[u8],
        count: usize,
        encoding: EnsureAtomsOpt,
    ) -> Result<Vec<AtomIndex>, AtomError> {
        let mut translate_table: Vec<u32> = Vec::with_capacity(count);  // Raw u32 for FFI
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
            AtomTableResult::Ok => {
                // Convert Vec<u32> to Vec<AtomIndex>
                let result: Vec<AtomIndex> = translate_table.into_iter().map(AtomIndex).collect();
                Ok(result)
            }
            AtomTableResult::NotFound => Err(AtomError::NotFound),
            AtomTableResult::AllocationFailed => Err(AtomError::AllocationFailed),
            AtomTableResult::InvalidLength => Err(AtomError::InvalidLength),
        }
    }
}

// Safety: AtomTable operations are thread-safe due to internal locking
unsafe impl Send for AtomTable {}
unsafe impl Sync for AtomTable {}

// ── Common Atom Utilities ───────────────────────────────────────────────────

/// Utilities for working with common atoms
/// 
/// These functions work with any atom table implementation.
pub mod atoms {
    use super::*;

    /// Ensure common atoms exist in a table
    /// 
    /// This is useful for initializing any atom table (real or mock)
    /// with the standard atoms that AtomVM typically provides.
    pub fn ensure_common_atoms<T: AtomTableOps>(table: &T) -> Result<(), AtomError> {
        let common_atoms = [
            "ok", "error", "true", "false", "undefined", "badarg", "nil",
            "atom", "binary", "bitstring", "boolean", "float", "function",
            "integer", "list", "map", "pid", "port", "reference", "tuple"
        ];
        
        for atom_name in &common_atoms {
            table.ensure_atom_str(atom_name)?;
        }
        
        Ok(())
    }

    /// Get an "ok" atom from any table
    pub fn ok<T: AtomTableOps>(table: &T) -> Result<AtomIndex, AtomError> {
        table.ensure_atom_str("ok")
    }

    /// Get an "error" atom from any table
    pub fn error<T: AtomTableOps>(table: &T) -> Result<AtomIndex, AtomError> {
        table.ensure_atom_str("error")
    }

    /// Get a "true" atom from any table
    pub fn true_atom<T: AtomTableOps>(table: &T) -> Result<AtomIndex, AtomError> {
        table.ensure_atom_str("true")
    }

    /// Get a "false" atom from any table
    pub fn false_atom<T: AtomTableOps>(table: &T) -> Result<AtomIndex, AtomError> {
        table.ensure_atom_str("false")
    }

    /// Get a "nil" atom from any table
    pub fn nil<T: AtomTableOps>(table: &T) -> Result<AtomIndex, AtomError> {
        table.ensure_atom_str("nil")
    }

    /// Get an "undefined" atom from any table
    pub fn undefined<T: AtomTableOps>(table: &T) -> Result<AtomIndex, AtomError> {
        table.ensure_atom_str("undefined")
    }

    /// Get a "badarg" atom from any table
    pub fn badarg<T: AtomTableOps>(table: &T) -> Result<AtomIndex, AtomError> {
        table.ensure_atom_str("badarg")
    }
}