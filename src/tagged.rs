//! Tagged Map serialization for creating Erlang-compatible ADTs
//!
//! This module provides automatic serialization of Rust types into
//! Erlang maps with type discriminators, enabling type-safe communication
//! between Rust ports/NIFs and Erlang processes.
//!
//! # Design Philosophy
//!
//! All operations are generic and work with any AtomTableOps implementation.
//! No global state, no hardcoded dependencies - pure dependency injection.
//!
//! # Examples
//!
//! ```rust,ignore
//! use avmnif_rs::tagged::{TaggedMap, TaggedError};
//! use avmnif_rs::testing::mocks::MockAtomTable;
//!
//! #[derive(TaggedMap)]
//! struct SensorReading {
//!     temperature: f32,
//!     humidity: f32,
//!     timestamp: u64,
//! }
//!
//! // In tests:
//! let table = MockAtomTable::new();
//! let reading = SensorReading { temperature: 23.5, humidity: 45.2, timestamp: 1634567890 };
//! let term = reading.to_tagged_map(&table)?;
//! let parsed = SensorReading::from_tagged_map(term, &table)?;
//!
//! // In production:
//! let table = AtomTable::from_global();
//! let term = reading.to_tagged_map(&table)?;
//! ```

extern crate alloc;

use crate::atom::{AtomTableOps, AtomError, atoms};
use crate::term::{AtomIndex, TermValue};
use alloc::{string::String, string::ToString, vec, vec::Vec, format};
use core::fmt;

// ── Error Handling ──────────────────────────────────────────────────────────

/// Errors that can occur during tagged map operations
#[derive(Debug, Clone, PartialEq)]
pub enum TaggedError {
    /// Atom-related error (atom creation, lookup, etc.)
    AtomError(AtomError),
    /// Wrong type for operation
    WrongType { expected: &'static str, found: &'static str },
    /// Index/key out of bounds  
    OutOfBounds { index: usize, max: usize },
    /// Required field missing from map
    MissingField(String),
    /// Type discriminator doesn't match expected type
    TypeMismatch { expected: String, found: String },
    /// Invalid enum variant
    InvalidVariant { enum_name: String, variant: String },
    /// Memory allocation failed
    OutOfMemory,
    /// Invalid UTF-8 in binary
    InvalidUtf8,
    /// Nested error with path context
    NestedError { path: String, source: alloc::boxed::Box<TaggedError> },
    /// Generic error with message
    Other(String),
}

impl TaggedError {
    /// Create a nested error with path context
    pub fn nested(path: impl Into<String>, source: TaggedError) -> Self {
        TaggedError::NestedError {
            path: path.into(),
            source: alloc::boxed::Box::new(source),
        }
    }
    
    /// Create a type mismatch error
    pub fn type_mismatch(expected: impl Into<String>, found: impl Into<String>) -> Self {
        TaggedError::TypeMismatch {
            expected: expected.into(),
            found: found.into(),
        }
    }
    
    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>) -> Self {
        TaggedError::MissingField(field.into())
    }
    
    /// Create an invalid variant error
    pub fn invalid_variant(enum_name: impl Into<String>, variant: impl Into<String>) -> Self {
        TaggedError::InvalidVariant {
            enum_name: enum_name.into(),
            variant: variant.into(),
        }
    }
}

impl fmt::Display for TaggedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaggedError::AtomError(e) => write!(f, "atom error: {}", e),
            TaggedError::WrongType { expected, found } => 
                write!(f, "wrong type: expected {}, found {}", expected, found),
            TaggedError::OutOfBounds { index, max } => 
                write!(f, "index {} out of bounds (max: {})", index, max),
            TaggedError::MissingField(field) => 
                write!(f, "missing required field: {}", field),
            TaggedError::TypeMismatch { expected, found } => 
                write!(f, "type mismatch: expected {}, found {}", expected, found),
            TaggedError::InvalidVariant { enum_name, variant } => 
                write!(f, "invalid variant '{}' for enum {}", variant, enum_name),
            TaggedError::OutOfMemory => write!(f, "out of memory"),
            TaggedError::InvalidUtf8 => write!(f, "invalid UTF-8"),
            TaggedError::NestedError { path, source } => 
                write!(f, "error at {}: {}", path, source),
            TaggedError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<AtomError> for TaggedError {
    fn from(error: AtomError) -> Self {
        TaggedError::AtomError(error)
    }
}

/// Result type for tagged map operations
pub type TaggedResult<T> = core::result::Result<T, TaggedError>;

// ── Core Trait ──────────────────────────────────────────────────────────────

/// Trait for types that can be converted to/from tagged Erlang maps
/// 
/// All operations are generic and work with any AtomTableOps implementation.
pub trait TaggedMap: Sized {
    /// Convert this type to a tagged Erlang map using any atom table
    /// 
    /// The resulting map will have a `type` field with the type discriminator
    /// and additional fields for the struct/enum data.
    fn to_tagged_map<T: AtomTableOps>(&self, table: &T) -> TaggedResult<TermValue>;
    
    /// Create this type from a tagged Erlang map using any atom table
    /// 
    /// Validates the `type` field matches the expected type and extracts
    /// the remaining fields to reconstruct the Rust type.
    fn from_tagged_map<T: AtomTableOps>(map: TermValue, table: &T) -> TaggedResult<Self>;
    
    /// Get the type atom name for this type (used for discriminator)
    fn type_name() -> &'static str;
}

// ── Helper Functions ────────────────────────────────────────────────────────

/// Convert Rust identifier to snake_case atom name
/// 
/// Examples:
/// - `SensorReading` -> `"sensor_reading"`
/// - `HTTPClient` -> `"httpclient"`
/// - `XMLParser` -> `"xmlparser"`
pub fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = name.chars().collect();
    
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            // Check if we should add an underscore
            let should_add_underscore = if i == 0 {
                false // Never add underscore at start
            } else {
                let prev_char = chars[i - 1];
                // Add underscore if previous char was lowercase (camelCase boundary)
                prev_char.is_lowercase()
            };
            
            if should_add_underscore {
                result.push('_');
            }
            
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Get atom index for a type name, creating it if necessary
pub fn get_type_atom<T: AtomTableOps>(type_name: &str, table: &T) -> TaggedResult<AtomIndex> {
    let atom_index = table.ensure_atom_str(type_name).map_err(TaggedError::from)?;
    Ok(atom_index)
}

/// Get the standard "type" field atom
pub fn type_field_atom<T: AtomTableOps>(table: &T) -> TaggedResult<AtomIndex> {
    let atom_index = table.ensure_atom_str("type").map_err(TaggedError::from)?;
    Ok(atom_index)
}

/// Get the standard "variant" field atom (for enums)
pub fn variant_field_atom<T: AtomTableOps>(table: &T) -> TaggedResult<AtomIndex> {
    let atom_index = table.ensure_atom_str("variant").map_err(TaggedError::from)?;
    Ok(atom_index)
}

/// Extract map value by atom key
pub fn get_map_value(map: &TermValue, key_atom: AtomIndex) -> TaggedResult<&TermValue> {
    match map {
        TermValue::Map(pairs) => {
            let key = TermValue::Atom(key_atom);
            pairs.iter()
                .find(|(k, _)| k == &key)
                .map(|(_, v)| v)
                .ok_or_else(|| TaggedError::Other(format!("key not found in map")))
        }
        _ => Err(TaggedError::WrongType { expected: "map", found: "other" }),
    }
}

/// Extract required string field from map
pub fn extract_string_field<T: AtomTableOps>(map: &TermValue, field_name: &str, table: &T) -> TaggedResult<String> {
    let field_atom = get_type_atom(field_name, table)?;
    let value = get_map_value(map, field_atom)?;
    
    match value {
        TermValue::Binary(bytes) => {
            String::from_utf8(bytes.clone()).map_err(|_| TaggedError::InvalidUtf8)
        }
        _ => Err(TaggedError::WrongType { expected: "binary/string", found: "other" }),
    }
}

/// Extract required integer field from map
pub fn extract_int_field<T: AtomTableOps>(map: &TermValue, field_name: &str, table: &T) -> TaggedResult<i32> {
    let field_atom = get_type_atom(field_name, table)?;
    let value = get_map_value(map, field_atom)?;
    
    match value {
        TermValue::SmallInt(i) => Ok(*i),
        _ => Err(TaggedError::WrongType { expected: "integer", found: "other" }),
    }
}

/// Extract required float field from map  
pub fn extract_float_field<T: AtomTableOps>(map: &TermValue, field_name: &str, table: &T) -> TaggedResult<f64> {
    let field_atom = get_type_atom(field_name, table)?;
    let value = get_map_value(map, field_atom)?;
    
    match value {
        TermValue::Float(f) => Ok(*f),
        TermValue::SmallInt(i) => Ok(*i as f64), // Allow integer to float conversion
        _ => Err(TaggedError::WrongType { expected: "float", found: "other" }),
    }
}

/// Extract required boolean field from map
pub fn extract_bool_field<T: AtomTableOps>(map: &TermValue, field_name: &str, table: &T) -> TaggedResult<bool> {
    let field_atom = get_type_atom(field_name, table)?;
    let value = get_map_value(map, field_atom)?;
    
    let true_atom = atoms::true_atom(table).map_err(TaggedError::from)?;
    let false_atom = atoms::false_atom(table).map_err(TaggedError::from)?;
    
    match value {
        TermValue::Atom(atom_idx) => {
            if *atom_idx == true_atom {
                Ok(true)
            } else if *atom_idx == false_atom {
                Ok(false)
            } else {
                Err(TaggedError::WrongType { expected: "boolean", found: "other atom" })
            }
        }
        _ => Err(TaggedError::WrongType { expected: "boolean", found: "other" }),
    }
}

/// Extract optional field from map
pub fn extract_optional_field<R, F, A>(
    map: &TermValue, 
    field_name: &str, 
    table: &A,
    extractor: F
) -> TaggedResult<Option<R>>
where
    F: FnOnce(&TermValue, &A) -> TaggedResult<R>,
    A: AtomTableOps,
{
    let field_atom = get_type_atom(field_name, table)?;
    
    match get_map_value(map, field_atom) {
        Ok(value) => {
            let nil_atom = atoms::nil(table).map_err(TaggedError::from)?;
            match value {
                TermValue::Atom(atom_idx) if *atom_idx == nil_atom => Ok(None),
                _ => extractor(value, table).map(Some),
            }
        }
        Err(_) => Ok(None), // Field not present
    }
}

/// Validate map has expected type discriminator
pub fn validate_type_discriminator<T: AtomTableOps>(map: &TermValue, expected_type: &str, table: &T) -> TaggedResult<()> {
    let type_atom = type_field_atom(table)?;
    let expected_type_atom = get_type_atom(expected_type, table)?;
    
    let type_value = get_map_value(map, type_atom)?;
    
    match type_value {
        TermValue::Atom(actual_type_atom) => {
            if *actual_type_atom == expected_type_atom {
                Ok(())
            } else {
                // Try to get readable atom name for error
                let actual_name = match table.get_atom_string(*actual_type_atom) {
                    Ok(atom_ref) => atom_ref.as_str().unwrap_or("unknown").to_string(),
                    Err(_) => "unknown".to_string(),
                };
                Err(TaggedError::type_mismatch(expected_type, actual_name))
            }
        }
        _ => Err(TaggedError::WrongType { expected: "atom", found: "other" }),
    }
}

// ── Generic Primitive Type Implementations ─────────────────────────────────

// These allow primitive types to be used directly in tagged structs

impl TaggedMap for i32 {
    fn to_tagged_map<T: AtomTableOps>(&self, table: &T) -> TaggedResult<TermValue> {
        let type_atom = get_type_atom("i32", table)?;
        let value_atom = get_type_atom("value", table)?;
        
        let pairs = alloc::vec![
            (TermValue::Atom(type_field_atom(table)?), TermValue::Atom(type_atom)),
            (TermValue::Atom(value_atom), TermValue::SmallInt(*self)),
        ];
        
        Ok(TermValue::Map(pairs))
    }
    
    fn from_tagged_map<T: AtomTableOps>(map: TermValue, table: &T) -> TaggedResult<Self> {
        validate_type_discriminator(&map, "i32", table)?;
        extract_int_field(&map, "value", table)
    }
    
    fn type_name() -> &'static str {
        "i32"
    }
}

impl TaggedMap for String {
    fn to_tagged_map<T: AtomTableOps>(&self, table: &T) -> TaggedResult<TermValue> {
        let type_atom = get_type_atom("string", table)?;
        let value_atom = get_type_atom("value", table)?;
        
        let pairs = alloc::vec![
            (TermValue::Atom(type_field_atom(table)?), TermValue::Atom(type_atom)),
            (TermValue::Atom(value_atom), TermValue::Binary(self.as_bytes().to_vec())),
        ];
        
        Ok(TermValue::Map(pairs))
    }
    
    fn from_tagged_map<T: AtomTableOps>(map: TermValue, table: &T) -> TaggedResult<Self> {
        validate_type_discriminator(&map, "string", table)?;
        extract_string_field(&map, "value", table)
    }
    
    fn type_name() -> &'static str {
        "string"
    }
}

impl TaggedMap for bool {
    fn to_tagged_map<T: AtomTableOps>(&self, table: &T) -> TaggedResult<TermValue> {
        let type_atom = get_type_atom("bool", table)?;
        let value_atom = get_type_atom("value", table)?;
        let bool_atom = if *self { 
            atoms::true_atom(table).map_err(TaggedError::from)? 
        } else { 
            atoms::false_atom(table).map_err(TaggedError::from)? 
        };
        
        let pairs = alloc::vec![
            (TermValue::Atom(type_field_atom(table)?), TermValue::Atom(type_atom)),
            (TermValue::Atom(value_atom), TermValue::Atom(bool_atom)),
        ];
        
        Ok(TermValue::Map(pairs))
    }
    
    fn from_tagged_map<T: AtomTableOps>(map: TermValue, table: &T) -> TaggedResult<Self> {
        validate_type_discriminator(&map, "bool", table)?;
        extract_bool_field(&map, "value", table)
    }
    
    fn type_name() -> &'static str {
        "bool"
    }
}

impl<U: TaggedMap> TaggedMap for Option<U> {
    fn to_tagged_map<T: AtomTableOps>(&self, table: &T) -> TaggedResult<TermValue> {
        match self {
            Some(value) => {
                let inner_map = value.to_tagged_map(table)?;
                let type_atom = get_type_atom("option", table)?;
                let variant_atom = variant_field_atom(table)?;
                let some_atom = get_type_atom("some", table)?;
                let value_atom = get_type_atom("value", table)?;
                
                let pairs = alloc::vec![
                    (TermValue::Atom(type_field_atom(table)?), TermValue::Atom(type_atom)),
                    (TermValue::Atom(variant_atom), TermValue::Atom(some_atom)),
                    (TermValue::Atom(value_atom), inner_map),
                ];
                
                Ok(TermValue::Map(pairs))
            }
            None => {
                let type_atom = get_type_atom("option", table)?;
                let variant_atom = variant_field_atom(table)?;
                let none_atom = atoms::nil(table).map_err(TaggedError::from)?;
                
                let pairs = alloc::vec![
                    (TermValue::Atom(type_field_atom(table)?), TermValue::Atom(type_atom)),
                    (TermValue::Atom(variant_atom), TermValue::Atom(none_atom)),
                ];
                
                Ok(TermValue::Map(pairs))
            }
        }
    }
    
    fn from_tagged_map<T: AtomTableOps>(map: TermValue, table: &T) -> TaggedResult<Self> {
        validate_type_discriminator(&map, "option", table)?;
        
        let variant_atom = variant_field_atom(table)?;
        let variant_value = get_map_value(&map, variant_atom)?;
        
        let some_atom = get_type_atom("some", table)?;
        let none_atom = atoms::nil(table).map_err(TaggedError::from)?;
        
        match variant_value {
            TermValue::Atom(atom_idx) if *atom_idx == some_atom => {
                let value_atom = get_type_atom("value", table)?;
                let inner_map = get_map_value(&map, value_atom)?;
                let inner_value = U::from_tagged_map(inner_map.clone(), table)?;
                Ok(Some(inner_value))
            }
            TermValue::Atom(atom_idx) if *atom_idx == none_atom => {
                Ok(None)
            }
            _ => Err(TaggedError::invalid_variant("Option", "unknown")),
        }
    }
    
    fn type_name() -> &'static str {
        "option"
    }
}

impl<U: TaggedMap> TaggedMap for Vec<U> {
    fn to_tagged_map<T: AtomTableOps>(&self, table: &T) -> TaggedResult<TermValue> {
        let type_atom = get_type_atom("vec", table)?;
        let elements_atom = get_type_atom("elements", table)?;
        
        // Convert each element to tagged map
        let mut element_maps = Vec::new();
        for item in self {
            element_maps.push(item.to_tagged_map(table)?);
        }
        
        let elements_list = TermValue::from_vec(element_maps);
        
        let pairs = alloc::vec![
            (TermValue::Atom(type_field_atom(table)?), TermValue::Atom(type_atom)),
            (TermValue::Atom(elements_atom), elements_list),
        ];
        
        Ok(TermValue::Map(pairs))
    }
    
    fn from_tagged_map<T: AtomTableOps>(map: TermValue, table: &T) -> TaggedResult<Self> {
        validate_type_discriminator(&map, "vec", table)?;
        
        let elements_atom = get_type_atom("elements", table)?;
        let elements_value = get_map_value(&map, elements_atom)?;
        
        let elements_vec = elements_value.list_to_vec();
        let mut result = Vec::new();
        
        for element_map in elements_vec {
            let item = U::from_tagged_map(element_map, table)?;
            result.push(item);
        }
        
        Ok(result)
    }
    
    fn type_name() -> &'static str {
        "vec"
    }
}

// ── Re-exports ──────────────────────────────────────────────────────────────

// Re-export the derive macro when available
#[cfg(feature = "derive")]
pub use avmnif_derive::TaggedMap;
