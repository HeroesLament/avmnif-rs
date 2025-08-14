# AtomVM Atom Table Interface - Safe Rust Bindings for Atom Management

Complete guide to using the AtomVM atom table from Ports and NIFs with thread-safe operations

## Overview

The AtomVM atom table is a global, shared data structure that stores all atoms used by the virtual machine. This module provides safe Rust bindings that allow Ports and NIFs to interact with the VM's atom storage system through thread-safe operations backed by read-write locks.

**Key Benefits:**
- Thread-safe operations across the entire VM
- Zero-copy atom lookups with lifetime management
- Efficient bulk operations for BEAM module loading
- Pre-cached common atoms for performance
- Type-safe error handling

## Core Types and Concepts

### AtomTable - The Global Registry
```rust,ignore
// Get reference to the global atom table
let atom_table = AtomTable::global();

// All operations are thread-safe
let hello_atom = atom_table.ensure_atom(b"hello")?;
```

### AtomIndex - Unique Identifiers
```rust,ignore
pub type AtomIndex = u32;

// AtomIndex values are stable for the VM lifetime
let index: AtomIndex = atom_table.ensure_atom_str("my_atom")?;
```

### AtomRef - Zero-Copy Data Access
```rust,ignore
// Get atom data without copying
let atom_ref = atom_table.get_atom_string(index)?;
assert_eq!(atom_ref.as_bytes(), b"my_atom");
assert_eq!(atom_ref.as_str()?, "my_atom");
```

## Two Primary Usage Patterns

### 1. Atom Creation and Lookup - Dynamic Operations
For runtime atom creation where you don't know the atom names at compile time

### 2. Common Atom Access - High-Performance Cached Access
For frequently used system atoms that benefit from caching

## Pattern 1: Dynamic Atom Creation and Lookup

### When to Use:
- Processing user input or configuration
- Converting strings from external sources
- Runtime atom generation
- Interactive debugging or development tools

### Example: Configuration Parser
```rust,ignore
use avmnif::atom::{AtomTable, AtomIndex, AtomError};

struct ConfigParser {
    table: AtomTable,
    // Cache for frequently accessed config atoms
    config_atoms: std::collections::HashMap<String, AtomIndex>,
}

impl ConfigParser {
    fn new() -> Self {
        Self {
            table: AtomTable::global(),
            config_atoms: std::collections::HashMap::new(),
        }
    }
    
    fn parse_config_option(&mut self, key: &str, value: &str) -> Result<(AtomIndex, AtomIndex), AtomError> {
        // Create or get atom for configuration key
        let key_atom = self.table.ensure_atom_str(key)?;
        
        // Cache frequently used config keys
        self.config_atoms.insert(key.to_string(), key_atom);
        
        // Handle different value types
        let value_atom = match value {
            "true" | "false" => {
                // Use common atoms for booleans
                if value == "true" {
                    crate::atom::atoms::true_atom()?
                } else {
                    crate::atom::atoms::false_atom()?
                }
            }
            "nil" | "null" => crate::atom::atoms::nil()?,
            _ => self.table.ensure_atom_str(value)?,
        };
        
        Ok((key_atom, value_atom))
    }
    
    fn validate_known_option(&self, option_name: &str) -> Result<bool, AtomError> {
        // Try to find atom without creating it
        match self.table.find_atom_str(option_name) {
            Ok(_) => Ok(true),
            Err(AtomError::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }
    
    fn compare_config_priority(&self, option1: AtomIndex, option2: AtomIndex) -> std::cmp::Ordering {
        // Lexicographic comparison for sorting config options
        let cmp_result = self.table.compare_atoms(option1, option2);
        if cmp_result < 0 {
            std::cmp::Ordering::Less
        } else if cmp_result > 0 {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

// Usage in a Port message handler
fn config_port_handler(ctx: &mut Context, message: &Message) -> PortResult {
    let mut parser = ConfigParser::new();
    let (pid, reference, command) = parse_gen_message(message)?;
    
    match command.get_atom_index()? {
        "parse_config" => {
            let config_data = command.get_kv("config")?.to_string()?;
            let mut parsed_options = Vec::new();
            
            for line in config_data.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    let (key_atom, value_atom) = parser.parse_config_option(
                        key.trim(), 
                        value.trim()
                    )?;
                    parsed_options.push((key_atom, value_atom));
                }
            }
            
            // Sort options by atom name for consistent ordering
            parsed_options.sort_by(|a, b| parser.compare_config_priority(a.0, b.0));
            
            send_reply(ctx, pid, reference, tuple!("parsed", parsed_options.len()));
            PortResult::Continue
        }
        
        "check_option" => {
            let option_name = command.get_kv("option")?.to_string()?;
            let exists = parser.validate_known_option(&option_name)?;
            send_reply(ctx, pid, reference, if exists { atom!("exists") } else { atom!("new") });
            PortResult::Continue
        }
        
        _ => PortResult::Continue
    }
}
```

## Pattern 2: Common Atom Access - High-Performance Operations

### When to Use:
- Frequently used system atoms (`ok`, `error`, `true`, `false`)
- Port message protocols with known atom sets
- Performance-critical paths that access the same atoms repeatedly
- Error handling and status reporting

### Available Common Atoms:
```rust,ignore
use avmnif::atom::atoms;

// Status atoms
let ok_atom = atoms::ok()?;
let error_atom = atoms::error()?;
let undefined_atom = atoms::undefined()?;

// Boolean atoms  
let true_atom = atoms::true_atom()?;
let false_atom = atoms::false_atom()?;
let nil_atom = atoms::nil()?;

// Process-related atoms
let normal_atom = atoms::normal()?;
let noproc_atom = atoms::noproc()?;
let timeout_atom = atoms::timeout()?;

// Error atoms
let badarg_atom = atoms::badarg()?;
let badarith_atom = atoms::badarith()?;

// Process control atoms
let exit_atom = atoms::exit()?;
let kill_atom = atoms::kill()?;
let monitor_atom = atoms::monitor()?;

// Port-specific atoms
let port_atom = atoms::port()?;
let port_data_atom = atoms::port_data()?;
let port_close_atom = atoms::port_close()?;
```

### Example: High-Performance Port Protocol
```rust,ignore
use avmnif::atom::atoms;

struct FastPortProtocol {
    // Pre-cache all protocol atoms at startup
    cmd_read: AtomIndex,
    cmd_write: AtomIndex,
    cmd_status: AtomIndex,
    
    status_ready: AtomIndex,
    status_busy: AtomIndex,
    status_error: AtomIndex,
    
    // Common atoms cached once
    ok: AtomIndex,
    error: AtomIndex,
}

impl FastPortProtocol {
    fn new() -> Result<Self, AtomError> {
        let table = AtomTable::global();
        
        Ok(Self {
            // Protocol-specific atoms
            cmd_read: table.ensure_atom_str("read")?,
            cmd_write: table.ensure_atom_str("write")?,
            cmd_status: table.ensure_atom_str("status")?,
            
            status_ready: table.ensure_atom_str("ready")?,
            status_busy: table.ensure_atom_str("busy")?,
            status_error: table.ensure_atom_str("error")?,
            
            // Use pre-cached common atoms for maximum performance
            ok: atoms::ok()?,
            error: atoms::error()?,
        })
    }
    
    fn handle_fast_message(&self, command_atom: AtomIndex, data: &[u8]) -> AtomIndex {
        // Fast atom comparison using indices - no string operations
        if command_atom == self.cmd_read {
            // Perform read operation
            if data.len() > 0 {
                self.ok
            } else {
                self.error
            }
        } else if command_atom == self.cmd_write {
            // Perform write operation  
            if data.len() <= MAX_WRITE_SIZE {
                self.status_ready
            } else {
                self.error
            }
        } else if command_atom == self.cmd_status {
            // Return current status
            self.status_ready
        } else {
            // Unknown command
            atoms::badarg().unwrap_or(self.error)
        }
    }
    
    fn create_response_tuple(&self, status: AtomIndex, data: Option<&[u8]>) -> Term {
        match data {
            Some(bytes) => tuple!(status, Term::from_binary(bytes)),
            None => tuple!(status),
        }
    }
}

// Usage in message handler - optimized for speed
fn fast_port_handler(ctx: &mut Context, message: &Message) -> PortResult {
    static mut PROTOCOL: Option<FastPortProtocol> = None;
    
    // Initialize protocol atoms once
    let protocol = unsafe {
        if PROTOCOL.is_none() {
            PROTOCOL = Some(FastPortProtocol::new().unwrap());
        }
        PROTOCOL.as_ref().unwrap()
    };
    
    let (pid, reference, command) = parse_gen_message(message)?;
    let command_atom = command.get_atom_index()?;
    let data = command.get_kv("data").map(|t| t.to_binary()).unwrap_or_default();
    
    // Fast processing using pre-cached atom indices
    let result = protocol.handle_fast_message(command_atom, &data);
    let response = protocol.create_response_tuple(result, Some(&data));
    
    send_reply(ctx, pid, reference, response);
    PortResult::Continue
}
```



## Advanced Usage Patterns

### Atom Comparison and Sorting
```rust,ignore
fn sort_atoms_by_name(atom_indices: &mut [AtomIndex]) {
    let table = AtomTable::global();
    
    atom_indices.sort_by(|&a, &b| {
        let cmp_result = table.compare_atoms(a, b);
        if cmp_result < 0 {
            std::cmp::Ordering::Less
        } else if cmp_result > 0 {
            std::cmp::Ordering::Greater  
        } else {
            std::cmp::Ordering::Equal
        }
    });
}

fn find_atom_with_prefix(prefix: &str) -> Vec<AtomIndex> {
    let table = AtomTable::global();
    let mut matching_atoms = Vec::new();
    
    // Note: This is a simplified example. Real implementation would
    // need to iterate through the atom table more efficiently
    for i in 1..table.count() {
        if let Ok(atom_ref) = table.get_atom_string(i as u32) {
            if let Ok(atom_str) = atom_ref.as_str() {
                if atom_str.starts_with(prefix) {
                    matching_atoms.push(i as u32);
                }
            }
        }
    }
    
    matching_atoms
}
```

### Thread-Safe Atom Caching
```rust,ignore
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

struct ThreadSafeAtomCache {
    cache: Arc<Mutex<HashMap<String, AtomIndex>>>,
    table: AtomTable,
}

impl ThreadSafeAtomCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            table: AtomTable::global(),
        }
    }
    
    fn get_or_create_atom(&self, name: &str) -> Result<AtomIndex, AtomError> {
        // Try cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(&index) = cache.get(name) {
                return Ok(index);
            }
        }
        
        // Create atom and cache it
        let index = self.table.ensure_atom_str(name)?;
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(name.to_string(), index);
        }
        
        Ok(index)
    }
    
    fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}
```

## Error Handling Best Practices

### Comprehensive Error Management
```rust,ignore
fn handle_atom_operation_with_recovery(atom_name: &str) -> Result<AtomIndex, String> {
    let table = AtomTable::global();
    
    match table.ensure_atom_str(atom_name) {
        Ok(index) => Ok(index),
        Err(AtomError::NotFound) => {
            // This shouldn't happen with ensure_atom, but handle it
            Err(format!("Atom '{}' could not be found or created", atom_name))
        }
        Err(AtomError::AllocationFailed) => {
            // System is likely out of memory
            Err("System out of memory - cannot create new atoms".to_string())
        }
        Err(AtomError::InvalidLength) => {
            // Atom name is too long or has encoding issues
            Err(format!("Atom name '{}' is invalid (too long or encoding issue)", atom_name))
        }
        Err(AtomError::NullPointer) => {
            // Critical system error
            Err("Critical system error - atom table is corrupted".to_string())
        }
        Err(AtomError::InvalidIndex) => {
            // This shouldn't happen with ensure_atom
            Err("Internal error - invalid atom index".to_string())
        }
    }
}

fn safe_atom_string_access(index: AtomIndex) -> Result<String, String> {
    let table = AtomTable::global();
    
    match table.get_atom_string(index) {
        Ok(atom_ref) => {
            match atom_ref.as_str() {
                Ok(s) => Ok(s.to_string()),
                Err(_) => {
                    // Atom contains non-UTF8 data, return as hex
                    let hex_string = atom_ref.as_bytes()
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>();
                    Ok(format!("binary_atom({})", hex_string))
                }
            }
        }
        Err(e) => Err(format!("Failed to access atom {}: {}", index, e))
    }
}
```

## Performance Considerations

### Optimization Guidelines:

**Use Common Atoms Cache:**
- Pre-cache frequently used atoms at startup
- Avoid repeated `ensure_atom` calls for known atoms
- Use the `atoms` module for system atoms

**Minimize String Operations:**
- Compare atoms by index when possible
- Cache `AtomIndex` values rather than strings
- Use `atom_equals` for string comparison without allocation

**Bulk Operations:**
- Use `ensure_atoms_bulk` for loading many atoms at once
- Batch atom creation when loading modules or configurations
- Consider memory locality for frequently accessed atoms

**Thread Safety:**
- AtomTable operations are thread-safe but not lock-free
- Minimize contention by reducing concurrent atom creation
- Cache atom indices in thread-local storage when appropriate

### Memory Usage:
- Atoms are never garbage collected - they live for VM lifetime
- Be cautious about creating atoms from user input
- Consider using a whitelist for dynamic atom creation
- Monitor atom table growth in long-running systems

## Safety and Thread Considerations

### Thread Safety Guarantees:
- All `AtomTable` operations are thread-safe
- Multiple threads can safely access the same atom table
- Internal locking ensures consistency across operations
- `AtomRef` lifetimes are managed safely

### Memory Safety:
- No manual memory management required for atoms
- `AtomRef` provides safe access to atom data
- Automatic cleanup of internal resources
- No dangling pointer risks with proper lifetime management

This documentation covers the complete AtomVM atom table interface, providing patterns for efficient, safe, and thread-aware atom management in your Rust-based Ports and NIFs.