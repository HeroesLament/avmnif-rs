extern crate alloc;

use core::ffi::c_void;
use alloc::{string::String, vec::Vec, boxed::Box, vec};

// ── Core ADT Definition ──────────────────────────────────────────────────────

/// Clean, functional ADT for AtomVM terms
#[derive(Debug, Clone, PartialEq)]
pub enum TermValue {
    // Immediate values
    SmallInt(i32),
    Atom(AtomIndex),
    Nil,
    
    // Process identifiers  
    Pid(ProcessId),
    Port(PortId),
    Reference(RefId),
    
    // Compound values
    Tuple(Vec<TermValue>),
    List(Box<TermValue>, Box<TermValue>), // Head, Tail (proper cons cell)
    Map(Vec<(TermValue, TermValue)>),     // Key-Value pairs
    Binary(Vec<u8>),
    
    // Special values
    Function(FunctionRef),
    Resource(ResourceRef),
    Float(f64),
    
    // Error case
    Invalid,
}

/// Atom represented by index into atom table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtomIndex(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefId(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionRef {
    pub module: AtomIndex,
    pub function: AtomIndex,
    pub arity: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceRef {
    pub type_name: String,
    pub ptr: *mut c_void,
}

// ── Low-level Term (FFI boundary) ────────────────────────────────────────────

/// Low-level term representation for FFI with AtomVM
/// This handles the bit-level encoding/decoding
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Term(pub usize);

/// AtomVM Context - opaque pointer to runtime context
#[repr(C)]
pub struct Context {
    pub _private: [u8; 0],
}

/// AtomVM GlobalContext - runtime global state
#[repr(C)]
pub struct GlobalContext {
    pub _private: [u8; 0],
}

/// AtomVM Heap for memory allocation
#[repr(C)] 
pub struct Heap {
    pub _private: [u8; 0],
}

// ── AtomVM Constants ─────────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TermType {
    SmallInt,
    Atom,
    Nil,
    Pid,
    Port,
    Reference,
    Tuple,
    List,
    Map,
    Binary,
    Function,
    Resource,
    Float,
    Invalid,
}

impl Term {
    // AtomVM tag constants (from AtomVM source)
    const TERM_PRIMARY_MASK: usize = 0x3;
    const TERM_PRIMARY_IMMED: usize = 0x3;
    const TERM_PRIMARY_LIST: usize = 0x1;
    const TERM_PRIMARY_BOXED: usize = 0x2;
    
    const TERM_IMMED_TAG_MASK: usize = 0xF;
    const TERM_INTEGER_TAG: usize = 0xF;
    const TERM_ATOM_TAG: usize = 0xB;
    const TERM_PID_TAG: usize = 0x3;
    const TERM_PORT_TAG: usize = 0x7;
    
    const TERM_NIL: usize = 0x3B;
    
    const TERM_BOXED_TAG_MASK: usize = 0x3F;
    const TERM_BOXED_TUPLE: usize = 0x00;
    const TERM_BOXED_POSITIVE_INTEGER: usize = 0x08;
    const TERM_BOXED_REF: usize = 0x10;
    const TERM_BOXED_FUN: usize = 0x18;
    const TERM_BOXED_FLOAT: usize = 0x20;
    const TERM_BOXED_REFC_BINARY: usize = 0x28;
    const TERM_BOXED_HEAP_BINARY: usize = 0x30;
    const TERM_BOXED_SUB_BINARY: usize = 0x38;
    const TERM_BOXED_MAP: usize = 0x40;
    const TERM_BOXED_RESOURCE: usize = 0x48;

    /// Get raw term value
    pub fn raw(self) -> usize {
        self.0
    }
    
    /// Create term from raw value
    pub fn from_raw(raw: usize) -> Self {
        Term(raw)
    }

    /// Decode the low-level type of this term
    fn decode_type(self) -> TermType {
        if self.0 == Self::TERM_NIL {
            return TermType::Nil;
        }
        
        match self.0 & Self::TERM_PRIMARY_MASK {
            Self::TERM_PRIMARY_IMMED => {
                match self.0 & Self::TERM_IMMED_TAG_MASK {
                    Self::TERM_INTEGER_TAG => TermType::SmallInt,
                    Self::TERM_ATOM_TAG => TermType::Atom,
                    Self::TERM_PID_TAG => TermType::Pid,
                    Self::TERM_PORT_TAG => TermType::Port,
                    _ => TermType::Invalid,
                }
            }
            Self::TERM_PRIMARY_LIST => TermType::List,
            Self::TERM_PRIMARY_BOXED => {
                let boxed_ptr = (self.0 & !Self::TERM_PRIMARY_MASK) as *const usize;
                if boxed_ptr.is_null() {
                    return TermType::Invalid;
                }
                
                let header = unsafe { *boxed_ptr };
                match header & Self::TERM_BOXED_TAG_MASK {
                    Self::TERM_BOXED_TUPLE => TermType::Tuple,
                    Self::TERM_BOXED_POSITIVE_INTEGER => TermType::SmallInt,
                    Self::TERM_BOXED_REF => TermType::Reference,
                    Self::TERM_BOXED_FUN => TermType::Function,
                    Self::TERM_BOXED_FLOAT => TermType::Float,
                    Self::TERM_BOXED_REFC_BINARY |
                    Self::TERM_BOXED_HEAP_BINARY |
                    Self::TERM_BOXED_SUB_BINARY => TermType::Binary,
                    Self::TERM_BOXED_MAP => TermType::Map,
                    Self::TERM_BOXED_RESOURCE => TermType::Resource,
                    _ => TermType::Invalid,
                }
            }
            _ => TermType::Invalid,
        }
    }

    // ── Low-level extraction methods ─────────────────────────────────────────

    fn extract_small_int(self) -> NifResult<i32> {
        match self.decode_type() {
            TermType::SmallInt => {
                let raw_value = (self.0 & !0xF) as i32 >> 4;
                Ok(raw_value)
            }
            _ => Err(NifError::BadArg),
        }
    }

    fn extract_atom_index(self) -> NifResult<u32> {
        match self.decode_type() {
            TermType::Atom => Ok((self.0 >> 4) as u32),
            _ => Err(NifError::BadArg),
        }
    }

    fn extract_tuple_arity(self) -> NifResult<usize> {
        match self.decode_type() {
            TermType::Tuple => {
                let boxed_ptr = (self.0 & !Self::TERM_PRIMARY_MASK) as *const usize;
                let header = unsafe { *boxed_ptr };
                Ok((header >> 6) as usize)
            }
            _ => Err(NifError::BadArg),
        }
    }

    fn extract_tuple_element(self, index: usize) -> NifResult<Term> {
        let arity = self.extract_tuple_arity()?;
        if index >= arity {
            return Err(NifError::BadArg);
        }
        
        let boxed_ptr = (self.0 & !Self::TERM_PRIMARY_MASK) as *const usize;
        let element = unsafe { *boxed_ptr.add(1 + index) };
        Ok(Term(element))
    }

    fn extract_list_head(self) -> NifResult<Term> {
        match self.decode_type() {
            TermType::List => {
                let list_ptr = (self.0 & !Self::TERM_PRIMARY_MASK) as *const usize;
                let head = unsafe { *list_ptr };
                Ok(Term(head))
            }
            _ => Err(NifError::BadArg),
        }
    }

    fn extract_list_tail(self) -> NifResult<Term> {
        match self.decode_type() {
            TermType::List => {
                let list_ptr = (self.0 & !Self::TERM_PRIMARY_MASK) as *const usize;
                let tail = unsafe { *list_ptr.add(1) };
                Ok(Term(tail))
            }
            _ => Err(NifError::BadArg),
        }
    }

    fn extract_binary_data(self) -> NifResult<&'static [u8]> {
        match self.decode_type() {
            TermType::Binary => {
                let boxed_ptr = (self.0 & !Self::TERM_PRIMARY_MASK) as *const usize;
                let size = unsafe { *boxed_ptr.add(1) };
                let data_ptr = unsafe { boxed_ptr.add(2) as *const u8 };
                Ok(unsafe { core::slice::from_raw_parts(data_ptr, size) })
            }
            _ => Err(NifError::BadArg),
        }
    }

    fn extract_map_size(self) -> NifResult<usize> {
        match self.decode_type() {
            TermType::Map => {
                let boxed_ptr = (self.0 & !Self::TERM_PRIMARY_MASK) as *const usize;
                let size = unsafe { *boxed_ptr.add(1) };
                Ok(size)
            }
            _ => Err(NifError::BadArg),
        }
    }

    fn extract_map_key(self, _index: usize) -> NifResult<Term> {
        // Placeholder - real implementation would traverse map structure
        Err(NifError::Other("map traversal not implemented"))
    }

    fn extract_map_value(self, _index: usize) -> NifResult<Term> {
        // Placeholder - real implementation would traverse map structure  
        Err(NifError::Other("map traversal not implemented"))
    }

    fn extract_resource_ptr(self) -> NifResult<*mut c_void> {
        match self.decode_type() {
            TermType::Resource => {
                let ptr = (self.0 & !Self::TERM_PRIMARY_MASK) as *mut c_void;
                Ok(ptr)
            }
            _ => Err(NifError::BadArg),
        }
    }

    // ── Low-level encoding methods ───────────────────────────────────────────

    fn encode_small_int(value: i32) -> NifResult<Self> {
        if value >= -(1 << 27) && value < (1 << 27) {
            Ok(Term(((value as usize) << 4) | Self::TERM_INTEGER_TAG))
        } else {
            Err(NifError::Other("integer too large for small int"))
        }
    }

    fn encode_atom(index: u32) -> NifResult<Self> {
        Ok(Term(((index as usize) << 4) | Self::TERM_ATOM_TAG))
    }

    fn encode_nil() -> Self {
        Term(Self::TERM_NIL)
    }

    #[allow(dead_code)]
    fn encode_tuple(_elements: Vec<Term>, _heap: &mut Heap) -> NifResult<Self> {
        // Placeholder - would need actual heap allocation
        Err(NifError::Other("tuple encoding not implemented"))
    }

    #[allow(dead_code)]
    fn encode_list(_head: Term, _tail: Term, _heap: &mut Heap) -> NifResult<Self> {
        // Placeholder - would need actual heap allocation
        Err(NifError::Other("list encoding not implemented"))
    }

    #[allow(dead_code)]
    fn encode_binary(_data: &[u8], _heap: &mut Heap) -> NifResult<Self> {
        // Placeholder - would need actual heap allocation
        Err(NifError::Other("binary encoding not implemented"))
    }

    #[allow(dead_code)]
    fn encode_map(_pairs: Vec<(Term, Term)>, _heap: &mut Heap) -> NifResult<Self> {
        // Placeholder - would need actual heap allocation
        Err(NifError::Other("map encoding not implemented"))
    }
}

// ── Conversion Between ADT and Low-level ─────────────────────────────────────

impl Term {
    /// Convert low-level term to high-level ADT
    pub fn to_value(self) -> NifResult<TermValue> {
        match self.decode_type() {
            TermType::SmallInt => {
                let val = self.extract_small_int()?;
                Ok(TermValue::SmallInt(val))
            }
            TermType::Atom => {
                let index = self.extract_atom_index()?;
                Ok(TermValue::Atom(AtomIndex(index)))
            }
            TermType::Nil => Ok(TermValue::Nil),
            TermType::Tuple => {
                let arity = self.extract_tuple_arity()?;
                let mut elements = Vec::with_capacity(arity);
                for i in 0..arity {
                    let elem_term = self.extract_tuple_element(i)?;
                    elements.push(elem_term.to_value()?);
                }
                Ok(TermValue::Tuple(elements))
            }
            TermType::List => {
                let head_term = self.extract_list_head()?;
                let tail_term = self.extract_list_tail()?;
                Ok(TermValue::List(
                    Box::new(head_term.to_value()?),
                    Box::new(tail_term.to_value()?)
                ))
            }
            TermType::Binary => {
                let data = self.extract_binary_data()?;
                Ok(TermValue::Binary(data.to_vec()))
            }
            TermType::Map => {
                let size = self.extract_map_size()?;
                let mut pairs = Vec::with_capacity(size);
                for i in 0..size {
                    let key_term = self.extract_map_key(i)?;
                    let val_term = self.extract_map_value(i)?;
                    pairs.push((key_term.to_value()?, val_term.to_value()?));
                }
                Ok(TermValue::Map(pairs))
            }
            TermType::Resource => {
                let ptr = self.extract_resource_ptr()?;
                Ok(TermValue::Resource(ResourceRef {
                    type_name: "unknown".into(),
                    ptr,
                }))
            }
            TermType::Pid => {
                let id = (self.0 >> 4) as u32; // Simplified
                Ok(TermValue::Pid(ProcessId(id)))
            }
            TermType::Port => {
                let id = (self.0 >> 4) as u32; // Simplified
                Ok(TermValue::Port(PortId(id)))
            }
            _ => Ok(TermValue::Invalid),
        }
    }
    
    /// Convert high-level ADT to low-level term
    #[allow(dead_code)]
    pub fn from_value(value: TermValue, heap: &mut Heap) -> NifResult<Self> {
        match value {
            TermValue::SmallInt(i) => Self::encode_small_int(i),
            TermValue::Atom(AtomIndex(idx)) => Self::encode_atom(idx),
            TermValue::Nil => Ok(Self::encode_nil()),
            
            TermValue::Tuple(elements) => {
                let term_elements: Result<Vec<Term>, NifError> = elements
                    .into_iter()
                    .map(|elem| Self::from_value(elem, heap))
                    .collect();
                Self::encode_tuple(term_elements?, heap)
            }
            
            TermValue::List(head, tail) => {
                let head_term = Self::from_value(*head, heap)?;
                let tail_term = Self::from_value(*tail, heap)?;
                Self::encode_list(head_term, tail_term, heap)
            }
            
            TermValue::Binary(data) => {
                Self::encode_binary(&data, heap)
            }
            
            TermValue::Map(pairs) => {
                let term_pairs: Result<Vec<(Term, Term)>, NifError> = pairs
                    .into_iter()
                    .map(|(k, v)| Ok((Self::from_value(k, heap)?, Self::from_value(v, heap)?)))
                    .collect();
                Self::encode_map(term_pairs?, heap)
            }
            
            _ => Err(NifError::Other("unsupported term type for encoding")),
        }
    }
}

// ── Functional Operations on TermValue (ADT Methods) ─────────────────────────

impl TermValue {
    /// Pattern match on integers
    pub fn as_int(&self) -> Option<i32> {
        match self {
            TermValue::SmallInt(i) => Some(*i),
            _ => None,
        }
    }
    
    /// Pattern match on atoms
    pub fn as_atom(&self) -> Option<AtomIndex> {
        match self {
            TermValue::Atom(idx) => Some(*idx),
            _ => None,
        }
    }
    
    /// Pattern match on tuples
    pub fn as_tuple(&self) -> Option<&[TermValue]> {
        match self {
            TermValue::Tuple(elements) => Some(elements),
            _ => None,
        }
    }
    
    /// Pattern match on lists (functional style)
    pub fn as_list(&self) -> Option<(&TermValue, &TermValue)> {
        match self {
            TermValue::List(head, tail) => Some((head, tail)),
            _ => None,
        }
    }

    /// Check if this is nil
    pub fn is_nil(&self) -> bool {
        matches!(self, TermValue::Nil)
    }

    /// Check if this is an empty list
    pub fn is_empty_list(&self) -> bool {
        self.is_nil()
    }
    
    /// Fold over list elements (functional programming!)
    pub fn fold_list<T, F>(&self, init: T, f: F) -> T 
    where 
        F: Fn(T, &TermValue) -> T,
    {
        match self {
            TermValue::Nil => init,
            TermValue::List(head, tail) => {
                let acc = f(init, head);
                tail.fold_list(acc, f)
            }
            _ => init, // Not a list
        }
    }
    
    /// Map over list elements  
    pub fn map_list<F>(&self, f: F) -> TermValue
    where
        F: Fn(&TermValue) -> TermValue + Clone,
    {
        match self {
            TermValue::Nil => TermValue::Nil,
            TermValue::List(head, tail) => {
                TermValue::List(
                    Box::new(f(head)),
                    Box::new(tail.map_list(f))
                )
            }
            _ => self.clone(), // Not a list
        }
    }

    /// Filter list elements
    pub fn filter_list<F>(&self, predicate: F) -> TermValue
    where
        F: Fn(&TermValue) -> bool + Clone,
    {
        match self {
            TermValue::Nil => TermValue::Nil,
            TermValue::List(head, tail) => {
                let filtered_tail = tail.filter_list(predicate.clone());
                if predicate(head) {
                    TermValue::List(head.clone(), Box::new(filtered_tail))
                } else {
                    filtered_tail
                }
            }
            _ => self.clone(),
        }
    }

    /// Get list length
    pub fn list_length(&self) -> usize {
        self.fold_list(0, |acc, _| acc + 1)
    }

    /// Convert list to Vec
    pub fn list_to_vec(&self) -> Vec<TermValue> {
        let mut result = Vec::new();
        let mut current = self;
        
        loop {
            match current {
                TermValue::Nil => break,
                TermValue::List(head, tail) => {
                    result.push((**head).clone());
                    current = tail;
                }
                _ => break,
            }
        }
        
        result
    }
    
    /// Get map value by key (functional lookup)
    pub fn map_get(&self, key: &TermValue) -> Option<&TermValue> {
        match self {
            TermValue::Map(pairs) => {
                pairs.iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v)
            }
            _ => None,
        }
    }

    /// Set map value (returns new map)
    pub fn map_set(&self, key: TermValue, value: TermValue) -> TermValue {
        match self {
            TermValue::Map(pairs) => {
                let mut new_pairs = pairs.clone();
                
                // Update existing key or add new one
                if let Some(pos) = new_pairs.iter().position(|(k, _)| k == &key) {
                    new_pairs[pos] = (key, value);
                } else {
                    new_pairs.push((key, value));
                }
                
                TermValue::Map(new_pairs)
            }
            _ => self.clone(),
        }
    }
    
    /// Construct list from iterator (functional construction)
    pub fn from_iter<I>(iter: I) -> TermValue 
    where 
        I: IntoIterator<Item = TermValue>,
        I::IntoIter: DoubleEndedIterator,
    {
        iter.into_iter()
            .rev()
            .fold(TermValue::Nil, |acc, elem| {
                TermValue::List(Box::new(elem), Box::new(acc))
            })
    }

    /// Construct proper list from Vec
    pub fn from_vec(elements: Vec<TermValue>) -> TermValue {
        Self::from_iter(elements)
    }
}

// ── Smart Constructors (ADT-friendly) ────────────────────────────────────────

impl TermValue {
    pub fn int(value: i32) -> Self {
        TermValue::SmallInt(value)
    }
    
    pub fn atom(name: &str) -> Self {
        // Simple atom table lookup - in real implementation would use global atom table
        let index = match name {
            "ok" => 1,
            "error" => 2,
            "true" => 3,
            "false" => 4,
            "undefined" => 5,
            "badarg" => 6,
            "nil" => 7,
            _ => 0,
        };
        TermValue::Atom(AtomIndex(index))
    }
    
    pub fn tuple(elements: Vec<TermValue>) -> Self {
        TermValue::Tuple(elements)
    }
    
    pub fn list(elements: Vec<TermValue>) -> Self {
        Self::from_vec(elements)
    }
    
    pub fn binary(data: Vec<u8>) -> Self {
        TermValue::Binary(data)
    }
    
    pub fn map(pairs: Vec<(TermValue, TermValue)>) -> Self {
        TermValue::Map(pairs)
    }

    pub fn pid(id: u32) -> Self {
        TermValue::Pid(ProcessId(id))
    }

    pub fn port(id: u32) -> Self {
        TermValue::Port(PortId(id))
    }

    pub fn reference(id: u64) -> Self {
        TermValue::Reference(RefId(id))
    }

    pub fn float(value: f64) -> Self {
        TermValue::Float(value)
    }
}

// ── Convenience Methods for Common Operations ────────────────────────────────

impl TermValue {
    /// Extract integer with default
    pub fn to_int_or(&self, default: i32) -> i32 {
        self.as_int().unwrap_or(default)
    }

    /// Extract tuple element by index
    pub fn tuple_get(&self, index: usize) -> Option<&TermValue> {
        self.as_tuple()?.get(index)
    }

    /// Extract tuple arity
    pub fn tuple_arity(&self) -> usize {
        self.as_tuple().map(|t| t.len()).unwrap_or(0)
    }

    /// Example: Sum all integers in a list
    pub fn sum_list(&self) -> i32 {
        self.fold_list(0, |acc, elem| {
            acc + elem.as_int().unwrap_or(0)
        })
    }
    
    /// Example: Convert list of integers to list of their doubles
    pub fn double_ints(&self) -> TermValue {
        self.map_list(|elem| {
            match elem.as_int() {
                Some(i) => TermValue::int(i * 2),
                None => elem.clone(),
            }
        })
    }

    /// Check if atom matches string
    pub fn is_atom_str(&self, name: &str) -> bool {
        match self.as_atom() {
            Some(AtomIndex(idx)) => {
                // Simple lookup - real implementation would use atom table
                match idx {
                    1 => name == "ok",
                    2 => name == "error", 
                    3 => name == "true",
                    4 => name == "false",
                    _ => false,
                }
            }
            None => false,
        }
    }
}

// ── Error Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NifError {
    BadArg,
    BadArity, 
    OutOfMemory,
    SystemLimit,
    InvalidTerm,
    Other(&'static str),
}

impl From<&'static str> for NifError {
    fn from(s: &'static str) -> Self {
        NifError::Other(s)
    }
}

pub type NifResult<T> = core::result::Result<T, NifError>;

// ── Quick Constructor Macros ─────────────────────────────────────────────────

#[macro_export]
macro_rules! atom {
    ($name:literal) => {
        TermValue::atom($name)
    };
}

#[macro_export]
macro_rules! tuple {
    ($($elem:expr),* $(,)?) => {
        TermValue::tuple(alloc::vec![$($elem),*])
    };
}

#[macro_export]
macro_rules! list {
    ($($elem:expr),* $(,)?) => {
        TermValue::list(alloc::vec![$($elem),*])
    };
}

#[macro_export]
macro_rules! map {
    ($($key:expr => $val:expr),* $(,)?) => {
        TermValue::map(alloc::vec![$(($key, $val)),*])
    };
}

// ── Usage Examples ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_adt_operations() {
        // Create terms using smart constructors
        let numbers = list![
            TermValue::int(1),
            TermValue::int(2), 
            TermValue::int(3)
        ];

        // Functional operations
        let doubled = numbers.double_ints();
        let sum = numbers.sum_list();
        
        assert_eq!(sum, 6);

        // Pattern matching
        match numbers {
            TermValue::List(head, _tail) => {
                assert_eq!(head.as_int(), Some(1));
            }
            _ => panic!("Expected list"),
        }

        // Tuple operations
        let point = tuple![TermValue::int(10), TermValue::int(20)];
        assert_eq!(point.tuple_arity(), 2);
        assert_eq!(point.tuple_get(0).unwrap().as_int(), Some(10));

        // Map operations
        let config = map![
            atom!("width") => TermValue::int(320),
            atom!("height") => TermValue::int(240)
        ];
        
        let width = config.map_get(&atom!("width"));
        assert_eq!(width.unwrap().as_int(), Some(320));
    }
}