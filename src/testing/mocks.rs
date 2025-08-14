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
use core::ffi::c_uint;
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

// ── Mock Resource Manager Implementation ───────────────────────────────────

use crate::resource::*;
use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use core::ffi::c_void;

/// Mock resource type for testing
#[derive(Debug, Clone, PartialEq)]
pub struct MockResourceType {
    pub id: usize,
    pub name: String,
    pub has_destructor: bool,
    pub has_stop_callback: bool,
    pub has_down_callback: bool,
}

/// Mock allocated resource for testing
#[derive(Debug, Clone, PartialEq)]
pub struct MockResource {
    pub id: usize,
    pub type_id: usize,
    pub size: u32,
    pub ref_count: usize,
    pub data: Vec<u8>, // Simulated memory content
}

/// Mock monitor for testing
#[derive(Debug, Clone, PartialEq)]
pub struct MockMonitor {
    pub resource_id: usize,
    pub pid: i32,
    pub active: bool,
}

/// Simple no_std state for the mock resource manager
/// 
/// Note: In no_std environment, we can't use Mutex, so this is not thread-safe.
/// For testing purposes, this is acceptable as tests should be single-threaded
/// or use external synchronization.
#[derive(Debug, Default)]
pub struct MockResourceManagerState {
    // Core resource management
    pub resource_types: BTreeMap<String, MockResourceType>,
    pub resources: BTreeMap<usize, MockResource>, // resource_id -> resource
    pub monitors: BTreeMap<usize, MockMonitor>,   // monitor_id -> monitor
    pub term_to_resource: BTreeMap<u64, usize>,   // term -> resource_id
    
    // ID generators
    pub next_type_id: AtomicUsize,
    pub next_resource_id: AtomicUsize,
    pub next_term_id: AtomicUsize,
    pub next_monitor_id: AtomicUsize,
    
    // Call tracking for verification
    pub init_calls: Vec<String>,
    pub alloc_calls: Vec<(usize, u32)>, // (type_id, size)
    pub make_resource_calls: Vec<usize>, // resource_id
    pub get_resource_calls: Vec<(u64, usize)>, // (term, type_id)
    pub keep_resource_calls: Vec<usize>, // resource_id
    pub release_resource_calls: Vec<usize>, // resource_id
    pub select_calls: Vec<(i32, ErlNifSelectFlags, usize)>, // (event, mode, resource_id)
    pub monitor_calls: Vec<(usize, i32)>, // (resource_id, pid)
    pub demonitor_calls: Vec<usize>, // monitor_id
    
    // Destructor simulation
    pub destructor_calls: Vec<usize>, // resource_id
    
    // Behavior control flags for testing edge cases
    pub fail_init: AtomicBool,
    pub fail_alloc: AtomicBool,
    pub fail_make_resource: AtomicBool,
    pub fail_get_resource: AtomicBool,
    pub fail_keep_resource: AtomicBool,
    pub fail_release_resource: AtomicBool,
    pub fail_select: AtomicBool,
    pub fail_monitor: AtomicBool,
    pub fail_demonitor: AtomicBool,
    
    // Resource limits for testing
    pub max_resources: Option<usize>,
    pub max_monitors: Option<usize>,
}

impl MockResourceManagerState {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    
    pub fn generate_type_id(&self) -> usize {
        self.next_type_id.fetch_add(1, Ordering::SeqCst)
    }
    
    pub fn generate_resource_id(&self) -> usize {
        self.next_resource_id.fetch_add(1, Ordering::SeqCst)
    }
    
    pub fn generate_term_id(&self) -> u64 {
        self.next_term_id.fetch_add(1, Ordering::SeqCst) as u64 + 0x12340000
    }
    
    pub fn generate_monitor_id(&self) -> usize {
        self.next_monitor_id.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Convert a type ID to a fake pointer
    fn type_id_to_ptr(&self, type_id: usize) -> *mut ErlNifResourceType {
        (0x1000 + type_id) as *mut ErlNifResourceType
    }
    
    /// Convert a resource ID to a fake pointer
    fn resource_id_to_ptr(&self, resource_id: usize) -> *mut c_void {
        (0x2000 + resource_id) as *mut c_void
    }
    
    /// Convert a fake pointer back to resource ID
    fn ptr_to_resource_id(&self, ptr: *mut c_void) -> Option<usize> {
        let addr = ptr as usize;
        if addr >= 0x2000 && addr < 0x3000 {
            Some(addr - 0x2000)
        } else {
            None
        }
    }
    
    /// Convert a fake pointer back to type ID
    fn ptr_to_type_id(&self, ptr: *mut ErlNifResourceType) -> Option<usize> {
        let addr = ptr as usize;
        if addr >= 0x1000 && addr < 0x2000 {
            Some(addr - 0x1000)
        } else {
            None
        }
    }
}

/// Mock implementation of ResourceManager for testing
/// 
/// Note: This is not thread-safe in no_std. For concurrent testing,
/// external synchronization would be needed.
#[derive(Debug)]
pub struct MockResourceManager {
    pub state: MockResourceManagerState,
}

impl MockResourceManager {
    pub fn new() -> Self {
        Self {
            state: MockResourceManagerState::new(),
        }
    }
    
    pub fn with_max_resources(mut self, max: usize) -> Self {
        self.state.max_resources = Some(max);
        self
    }
    
    pub fn with_max_monitors(mut self, max: usize) -> Self {
        self.state.max_monitors = Some(max);
        self
    }
    
    // Test behavior control methods
    pub fn set_fail_init(&mut self, fail: bool) {
        self.state.fail_init.store(fail, Ordering::SeqCst);
    }
    
    pub fn set_fail_alloc(&mut self, fail: bool) {
        self.state.fail_alloc.store(fail, Ordering::SeqCst);
    }
    
    pub fn set_fail_make_resource(&mut self, fail: bool) {
        self.state.fail_make_resource.store(fail, Ordering::SeqCst);
    }
    
    pub fn set_fail_get_resource(&mut self, fail: bool) {
        self.state.fail_get_resource.store(fail, Ordering::SeqCst);
    }
    
    // State inspection methods
    pub fn get_resource_count(&self) -> usize {
        self.state.resources.len()
    }
    
    pub fn get_resource_type_count(&self) -> usize {
        self.state.resource_types.len()
    }
    
    pub fn get_monitor_count(&self) -> usize {
        self.state.monitors.len()
    }
    
    pub fn verify_init_called(&self, name: &str) -> bool {
        self.state.init_calls.contains(&name.to_string())
    }
    
    pub fn verify_destructor_called(&self, resource_id: usize) -> bool {
        self.state.destructor_calls.contains(&resource_id)
    }
    
    pub fn get_resource_ref_count(&self, ptr: *mut c_void) -> Option<usize> {
        if let Some(resource_id) = self.state.ptr_to_resource_id(ptr) {
            self.state.resources.get(&resource_id).map(|r| r.ref_count)
        } else {
            None
        }
    }
    
    pub fn simulate_destructor_call(&mut self, ptr: *mut c_void) {
        if let Some(resource_id) = self.state.ptr_to_resource_id(ptr) {
            self.state.destructor_calls.push(resource_id);
            self.state.resources.remove(&resource_id);
        }
    }
    
    pub fn reset(&mut self) {
        self.state.reset();
    }
    
    pub fn get_init_call_count(&self) -> usize {
        self.state.init_calls.len()
    }
    
    pub fn get_alloc_call_count(&self) -> usize {
        self.state.alloc_calls.len()
    }
    
    pub fn get_destructor_call_count(&self) -> usize {
        self.state.destructor_calls.len()
    }
    
    // Public getter for state access in tests
    pub fn get_state(&self) -> &MockResourceManagerState {
        &self.state
    }
    
    // Public mutable getter for state access in tests
    pub fn get_state_mut(&mut self) -> &mut MockResourceManagerState {
        &mut self.state
    }
}

impl Default for MockResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManager for MockResourceManager {
    fn init_resource_type(
        &mut self,
        _env: *mut ErlNifEnv,
        name: &str,
        init: &ErlNifResourceTypeInit,
        _flags: ErlNifResourceFlags,
    ) -> Result<*mut ErlNifResourceType, ResourceError> {
        if self.state.fail_init.load(Ordering::SeqCst) {
            return Err(ResourceError::InitializationFailed);
        }
        
        if name.is_empty() || name.len() > 255 {
            return Err(ResourceError::InvalidName);
        }
        
        // Check if resource type already exists
        if self.state.resource_types.contains_key(name) {
            return Err(ResourceError::InitializationFailed);
        }
        
        let type_id = self.state.generate_type_id();
        let resource_type = MockResourceType {
            id: type_id,
            name: name.to_string(),
            has_destructor: init.dtor.is_some(),
            has_stop_callback: init.stop.is_some(),
            has_down_callback: init.down.is_some(),
        };
        
        self.state.init_calls.push(name.to_string());
        self.state.resource_types.insert(name.to_string(), resource_type);
        
        Ok(self.state.type_id_to_ptr(type_id))
    }

    fn alloc_resource(
        &self,
        resource_type: *mut ErlNifResourceType,
        size: c_uint,
    ) -> Result<*mut c_void, ResourceError> {
        if self.state.fail_alloc.load(Ordering::SeqCst) {
            return Err(ResourceError::OutOfMemory);
        }
        
        if resource_type.is_null() {
            return Err(ResourceError::BadResourceType);
        }
        
        if size == 0 {
            return Err(ResourceError::BadArg);
        }
        
        // Check resource limits
        if let Some(max) = self.state.max_resources {
            if self.state.resources.len() >= max {
                return Err(ResourceError::OutOfMemory);
            }
        }
        
        let type_id = match self.state.ptr_to_type_id(resource_type) {
            Some(id) => id,
            None => return Err(ResourceError::BadResourceType),
        };
        
        let resource_id = self.state.generate_resource_id();
        let resource = MockResource {
            id: resource_id,
            type_id,
            size,
            ref_count: 1,
            data: alloc::vec![0u8; size as usize], // Initialize with zeros
        };
        
        // Since we have &self, we need to use unsafe to modify the state
        // This is acceptable for testing purposes
        unsafe {
            let state_ptr = &self.state as *const _ as *mut MockResourceManagerState;
            (*state_ptr).alloc_calls.push((type_id, size));
            (*state_ptr).resources.insert(resource_id, resource);
        }
        
        Ok(self.state.resource_id_to_ptr(resource_id))
    }

    fn make_resource(
        &self,
        _env: *mut ErlNifEnv,
        obj: *mut c_void,
    ) -> Result<ERL_NIF_TERM, ResourceError> {
        if self.state.fail_make_resource.load(Ordering::SeqCst) {
            return Err(ResourceError::BadArg);
        }
        
        if obj.is_null() {
            return Err(ResourceError::BadArg);
        }
        
        let resource_id = match self.state.ptr_to_resource_id(obj) {
            Some(id) => id,
            None => return Err(ResourceError::BadArg),
        };
        
        // Verify resource exists
        if !self.state.resources.contains_key(&resource_id) {
            return Err(ResourceError::ResourceNotFound);
        }
        
        let term = self.state.generate_term_id();
        
        // Since we have &self, use unsafe to modify state
        unsafe {
            let state_ptr = &self.state as *const _ as *mut MockResourceManagerState;
            (*state_ptr).make_resource_calls.push(resource_id);
            (*state_ptr).term_to_resource.insert(term, resource_id);
        }
        
        Ok(term)
    }

    fn get_resource(
        &self,
        _env: *mut ErlNifEnv,
        term: ERL_NIF_TERM,
        resource_type: *mut ErlNifResourceType,
    ) -> Result<*mut c_void, ResourceError> {
        if self.state.fail_get_resource.load(Ordering::SeqCst) {
            return Err(ResourceError::ResourceNotFound);
        }
        
        if resource_type.is_null() {
            return Err(ResourceError::BadArg);
        }
        
        let type_id = match self.state.ptr_to_type_id(resource_type) {
            Some(id) => id,
            None => return Err(ResourceError::BadResourceType),
        };
        
        // Since we have &self, use unsafe to modify state
        unsafe {
            let state_ptr = &self.state as *const _ as *mut MockResourceManagerState;
            (*state_ptr).get_resource_calls.push((term, type_id));
        }
        
        // Look up the resource ID from the term
        let resource_id = match self.state.term_to_resource.get(&term) {
            Some(&id) => id,
            None => return Err(ResourceError::ResourceNotFound),
        };
        
        // Verify resource exists and has correct type
        if let Some(resource) = self.state.resources.get(&resource_id) {
            if resource.type_id == type_id {
                Ok(self.state.resource_id_to_ptr(resource_id))
            } else {
                Err(ResourceError::ResourceNotFound)
            }
        } else {
            Err(ResourceError::ResourceNotFound)
        }
    }

    fn keep_resource(&self, obj: *mut c_void) -> Result<(), ResourceError> {
        if self.state.fail_keep_resource.load(Ordering::SeqCst) {
            return Err(ResourceError::BadArg);
        }
        
        if obj.is_null() {
            return Err(ResourceError::BadArg);
        }
        
        let resource_id = match self.state.ptr_to_resource_id(obj) {
            Some(id) => id,
            None => return Err(ResourceError::BadArg),
        };
        
        // Since we have &self, use unsafe to modify state
        unsafe {
            let state_ptr = &self.state as *const _ as *mut MockResourceManagerState;
            if let Some(resource) = (*state_ptr).resources.get_mut(&resource_id) {
                resource.ref_count += 1;
                (*state_ptr).keep_resource_calls.push(resource_id);
                Ok(())
            } else {
                Err(ResourceError::ResourceNotFound)
            }
        }
    }

    fn release_resource(&self, obj: *mut c_void) -> Result<(), ResourceError> {
        if self.state.fail_release_resource.load(Ordering::SeqCst) {
            return Err(ResourceError::BadArg);
        }
        
        if obj.is_null() {
            return Err(ResourceError::BadArg);
        }
        
        let resource_id = match self.state.ptr_to_resource_id(obj) {
            Some(id) => id,
            None => return Err(ResourceError::BadArg),
        };
        
        // Since we have &self, use unsafe to modify state
        unsafe {
            let state_ptr = &self.state as *const _ as *mut MockResourceManagerState;
            if let Some(resource) = (*state_ptr).resources.get_mut(&resource_id) {
                (*state_ptr).release_resource_calls.push(resource_id);
                
                if resource.ref_count > 0 {
                    resource.ref_count -= 1;
                    
                    // If ref count reaches 0, simulate destructor call
                    if resource.ref_count == 0 {
                        (*state_ptr).destructor_calls.push(resource_id);
                        (*state_ptr).resources.remove(&resource_id);
                    }
                }
                Ok(())
            } else {
                Err(ResourceError::ResourceNotFound)
            }
        }
    }

    fn select(
        &self,
        _env: *mut ErlNifEnv,
        event: ErlNifEvent,
        mode: ErlNifSelectFlags,
        obj: *mut c_void,
        _pid: *const ErlNifPid,
        _reference: ERL_NIF_TERM,
    ) -> Result<(), ResourceError> {
        if self.state.fail_select.load(Ordering::SeqCst) {
            return Err(ResourceError::BadArg);
        }
        
        if obj.is_null() {
            return Err(ResourceError::BadArg);
        }
        
        let resource_id = match self.state.ptr_to_resource_id(obj) {
            Some(id) => id,
            None => return Err(ResourceError::BadArg),
        };
        
        if !self.state.resources.contains_key(&resource_id) {
            return Err(ResourceError::ResourceNotFound);
        }
        
        // Since we have &self, use unsafe to modify state
        unsafe {
            let state_ptr = &self.state as *const _ as *mut MockResourceManagerState;
            (*state_ptr).select_calls.push((event, mode, resource_id));
        }
        
        Ok(())
    }

    fn monitor_process(
        &self,
        _env: *mut ErlNifEnv,
        obj: *mut c_void,
        target_pid: *const ErlNifPid,
        _mon: *mut ErlNifMonitor,
    ) -> Result<(), ResourceError> {
        if self.state.fail_monitor.load(Ordering::SeqCst) {
            return Err(ResourceError::BadArg);
        }
        
        if obj.is_null() || target_pid.is_null() {
            return Err(ResourceError::BadArg);
        }
        
        let resource_id = match self.state.ptr_to_resource_id(obj) {
            Some(id) => id,
            None => return Err(ResourceError::BadArg),
        };
        
        if !self.state.resources.contains_key(&resource_id) {
            return Err(ResourceError::ResourceNotFound);
        }
        
        // Check monitor limits
        if let Some(max) = self.state.max_monitors {
            if self.state.monitors.len() >= max {
                return Err(ResourceError::BadArg);
            }
        }
        
        let pid = unsafe { *target_pid };
        
        // Since we have &self, use unsafe to modify state
        unsafe {
            let state_ptr = &self.state as *const _ as *mut MockResourceManagerState;
            let monitor_id = (*state_ptr).generate_monitor_id();
            let monitor = MockMonitor {
                resource_id,
                pid,
                active: true,
            };
            
            (*state_ptr).monitor_calls.push((resource_id, pid));
            (*state_ptr).monitors.insert(monitor_id, monitor);
        }
        
        Ok(())
    }

    fn demonitor_process(
        &self,
        _env: *mut ErlNifEnv,
        obj: *mut c_void,
        _mon: *const ErlNifMonitor,
    ) -> Result<(), ResourceError> {
        if self.state.fail_demonitor.load(Ordering::SeqCst) {
            return Err(ResourceError::BadArg);
        }
        
        if obj.is_null() {
            return Err(ResourceError::BadArg);
        }
        
        let resource_id = match self.state.ptr_to_resource_id(obj) {
            Some(id) => id,
            None => return Err(ResourceError::BadArg),
        };
        
        // Since we have &self, use unsafe to modify state
        unsafe {
            let state_ptr = &self.state as *const _ as *mut MockResourceManagerState;
            
            // Find and remove monitor for this resource
            let monitor_ids: Vec<_> = (*state_ptr).monitors.iter()
                .filter(|(_, monitor)| monitor.resource_id == resource_id)
                .map(|(id, _)| *id)
                .collect();
            
            if monitor_ids.is_empty() {
                return Err(ResourceError::ResourceNotFound);
            }
            
            for monitor_id in monitor_ids {
                (*state_ptr).demonitor_calls.push(monitor_id);
                (*state_ptr).monitors.remove(&monitor_id);
            }
        }
        
        Ok(())
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

    // Resource Manager Tests
    #[test]
    fn test_mock_resource_manager_creation() {
        let manager = MockResourceManager::new();
        assert_eq!(manager.get_resource_count(), 0);
        assert_eq!(manager.get_resource_type_count(), 0);
        assert_eq!(manager.get_monitor_count(), 0);
    }

    #[test]
    fn test_mock_resource_manager_builder_pattern() {
        let manager = MockResourceManager::new()
            .with_max_resources(5)
            .with_max_monitors(3);
        
        assert_eq!(manager.state.max_resources, Some(5));
        assert_eq!(manager.state.max_monitors, Some(3));
    }

    #[test]
    fn test_mock_resource_manager_error_injection() {
        let mut manager = MockResourceManager::new();
        
        // Test that failure flags work
        manager.set_fail_init(true);
        assert!(manager.state.fail_init.load(Ordering::SeqCst));
        
        manager.set_fail_alloc(true);
        assert!(manager.state.fail_alloc.load(Ordering::SeqCst));
        
        manager.set_fail_make_resource(true);
        assert!(manager.state.fail_make_resource.load(Ordering::SeqCst));
        
        manager.set_fail_get_resource(true);
        assert!(manager.state.fail_get_resource.load(Ordering::SeqCst));
    }

    #[test]
    fn test_mock_resource_manager_state_tracking() {
        let manager = MockResourceManager::new();
        
        // Test initial counts
        assert_eq!(manager.get_init_call_count(), 0);
        assert_eq!(manager.get_alloc_call_count(), 0);
        assert_eq!(manager.get_destructor_call_count(), 0);
        
        // Test that state can be reset
        let mut manager = manager;
        manager.state.init_calls.push("test".to_string());
        assert_eq!(manager.get_init_call_count(), 1);
        
        manager.reset();
        assert_eq!(manager.get_init_call_count(), 0);
    }

    #[test]
    fn test_mock_resource_manager_pointer_conversion() {
        let state = MockResourceManagerState::new();
        
        // Test type ID to pointer conversion
        let type_ptr = state.type_id_to_ptr(42);
        assert_eq!(type_ptr as usize, 0x1000 + 42);
        
        let recovered_type_id = state.ptr_to_type_id(type_ptr);
        assert_eq!(recovered_type_id, Some(42));
        
        // Test resource ID to pointer conversion
        let resource_ptr = state.resource_id_to_ptr(123);
        assert_eq!(resource_ptr as usize, 0x2000 + 123);
        
        let recovered_resource_id = state.ptr_to_resource_id(resource_ptr);
        assert_eq!(recovered_resource_id, Some(123));
        
        // Test invalid pointers
        let invalid_ptr = 0x5000 as *mut c_void;
        assert_eq!(state.ptr_to_resource_id(invalid_ptr), None);
        
        let invalid_type_ptr = 0x5000 as *mut ErlNifResourceType;
        assert_eq!(state.ptr_to_type_id(invalid_type_ptr), None);
    }
}