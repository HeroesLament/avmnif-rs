//! Test utilities for tagged map serialization and ADT conversion

#[cfg(test)]
use alloc::{vec, vec::Vec, string::String, string::ToString};
use crate::atom::AtomTableOps;
use crate::testing::mocks::*;
use crate::term::TermValue;
use crate::tagged::{
    TaggedMap, TaggedError, TaggedResult,
    to_snake_case, get_type_atom, type_field_atom, variant_field_atom,
    get_map_value, extract_string_field, extract_int_field, extract_float_field,
    extract_bool_field, extract_optional_field, validate_type_discriminator
};

#[cfg(test)]
/// Test struct for tagged map serialization
#[derive(Debug, Clone, PartialEq)]
pub struct TestUser {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub active: bool,
}

#[cfg(test)]
impl TaggedMap for TestUser {
    fn to_tagged_map<T: AtomTableOps>(&self, table: &T) -> TaggedResult<TermValue> {
        let type_atom = get_type_atom("test_user", table)?;
        let id_atom = get_type_atom("id", table)?;
        let name_atom = get_type_atom("name", table)?;
        let email_atom = get_type_atom("email", table)?;
        let active_atom = get_type_atom("active", table)?;
        
        let email_value = match &self.email {
            Some(email) => TermValue::Binary(email.as_bytes().to_vec()),
            None => TermValue::Atom(table.ensure_atom_str("nil")?),
        };
        
        let active_value = if self.active {
            TermValue::Atom(table.ensure_atom_str("true")?)
        } else {
            TermValue::Atom(table.ensure_atom_str("false")?)
        };
        
        let pairs = alloc::vec![
            (TermValue::Atom(type_field_atom(table)?), TermValue::Atom(type_atom)),
            (TermValue::Atom(id_atom), TermValue::SmallInt(self.id)),
            (TermValue::Atom(name_atom), TermValue::Binary(self.name.as_bytes().to_vec())),
            (TermValue::Atom(email_atom), email_value),
            (TermValue::Atom(active_atom), active_value),
        ];
        
        Ok(TermValue::Map(pairs))
    }
    
    fn from_tagged_map<T: AtomTableOps>(map: TermValue, table: &T) -> TaggedResult<Self> {
        validate_type_discriminator(&map, "test_user", table)?;
        
        let id = extract_int_field(&map, "id", table)?;
        let name = extract_string_field(&map, "name", table)?;
        let active = extract_bool_field(&map, "active", table)?;
        
        let email = extract_optional_field(&map, "email", table, |value, _table| {
            match value {
                TermValue::Binary(bytes) => {
                    String::from_utf8(bytes.clone()).map_err(|_| TaggedError::InvalidUtf8)
                }
                _ => Err(TaggedError::WrongType { expected: "binary", found: "other" }),
            }
        })?;
        
        Ok(TestUser { id, name, email, active })
    }
    
    fn type_name() -> &'static str {
        "test_user"
    }
}

#[cfg(test)]
/// Test enum for tagged map serialization
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Active,
    Inactive,
    Pending { reason: String },
    Expired { days: i32 },
}

#[cfg(test)]
impl TaggedMap for TestStatus {
    fn to_tagged_map<T: AtomTableOps>(&self, table: &T) -> TaggedResult<TermValue> {
        let type_atom = get_type_atom("test_status", table)?;
        let variant_atom = variant_field_atom(table)?;
        
        let mut pairs = alloc::vec![
            (TermValue::Atom(type_field_atom(table)?), TermValue::Atom(type_atom)),
        ];
        
        match self {
            TestStatus::Active => {
                let active_atom = get_type_atom("active", table)?;
                pairs.push((TermValue::Atom(variant_atom), TermValue::Atom(active_atom)));
            }
            TestStatus::Inactive => {
                let inactive_atom = get_type_atom("inactive", table)?;
                pairs.push((TermValue::Atom(variant_atom), TermValue::Atom(inactive_atom)));
            }
            TestStatus::Pending { reason } => {
                let pending_atom = get_type_atom("pending", table)?;
                let reason_atom = get_type_atom("reason", table)?;
                pairs.push((TermValue::Atom(variant_atom), TermValue::Atom(pending_atom)));
                pairs.push((TermValue::Atom(reason_atom), TermValue::Binary(reason.as_bytes().to_vec())));
            }
            TestStatus::Expired { days } => {
                let expired_atom = get_type_atom("expired", table)?;
                let days_atom = get_type_atom("days", table)?;
                pairs.push((TermValue::Atom(variant_atom), TermValue::Atom(expired_atom)));
                pairs.push((TermValue::Atom(days_atom), TermValue::SmallInt(*days)));
            }
        }
        
        Ok(TermValue::Map(pairs))
    }
    
    fn from_tagged_map<T: AtomTableOps>(map: TermValue, table: &T) -> TaggedResult<Self> {
        validate_type_discriminator(&map, "test_status", table)?;
        
        let variant_atom = variant_field_atom(table)?;
        let variant_value = get_map_value(&map, variant_atom)?;
        
        let active_atom = get_type_atom("active", table)?;
        let inactive_atom = get_type_atom("inactive", table)?;
        let pending_atom = get_type_atom("pending", table)?;
        let expired_atom = get_type_atom("expired", table)?;
        
        match variant_value {
            TermValue::Atom(atom_idx) if *atom_idx == active_atom => {
                Ok(TestStatus::Active)
            }
            TermValue::Atom(atom_idx) if *atom_idx == inactive_atom => {
                Ok(TestStatus::Inactive)
            }
            TermValue::Atom(atom_idx) if *atom_idx == pending_atom => {
                let reason = extract_string_field(&map, "reason", table)?;
                Ok(TestStatus::Pending { reason })
            }
            TermValue::Atom(atom_idx) if *atom_idx == expired_atom => {
                let days = extract_int_field(&map, "days", table)?;
                Ok(TestStatus::Expired { days })
            }
            _ => Err(TaggedError::invalid_variant("TestStatus", "unknown")),
        }
    }
    
    fn type_name() -> &'static str {
        "test_status"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snake_case_conversion() {
        assert_eq!(to_snake_case("SensorReading"), "sensor_reading");
        assert_eq!(to_snake_case("HTTPClient"), "httpclient");
        assert_eq!(to_snake_case("XMLParser"), "xmlparser");
        assert_eq!(to_snake_case("camelCase"), "camel_case");
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
        assert_eq!(to_snake_case("lowercase"), "lowercase");
        assert_eq!(to_snake_case("UPPERCASE"), "uppercase");
        assert_eq!(to_snake_case("A"), "a");
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_primitive_tagged_map_i32() {
        let table = MockAtomTable::new();
        let value = 42i32;
        
        // Test serialization
        let map = value.to_tagged_map(&table).unwrap();
        
        // Verify structure
        if let TermValue::Map(ref pairs) = map {
            assert_eq!(pairs.len(), 2);
            
            // Check type field
            let type_atom = type_field_atom(&table).unwrap();
            let type_value = get_map_value(&map, type_atom).unwrap();
            assert!(table.atom_equals_str(type_value.as_atom().unwrap(), "i32"));
            
            // Check value field
            let value_atom = get_type_atom("value", &table).unwrap();
            let actual_value = get_map_value(&map, value_atom).unwrap();
            assert_eq!(actual_value.as_int().unwrap(), 42);
        } else {
            panic!("Expected map");
        }
        
        // Test deserialization
        let parsed = i32::from_tagged_map(map, &table).unwrap();
        assert_eq!(parsed, 42);
    }

    #[test]
    fn test_primitive_tagged_map_string() {
        let table = MockAtomTable::new();
        let value = "hello world".to_string();
        
        // Test serialization
        let map = value.to_tagged_map(&table).unwrap();
        
        // Verify structure
        let type_atom = type_field_atom(&table).unwrap();
        let type_value = get_map_value(&map, type_atom).unwrap();
        assert!(table.atom_equals_str(type_value.as_atom().unwrap(), "string"));
        
        // Test deserialization
        let parsed = String::from_tagged_map(map, &table).unwrap();
        assert_eq!(parsed, "hello world");
    }

    #[test]
    fn test_primitive_tagged_map_bool() {
        let table = MockAtomTable::new();
        
        // Test true
        let true_val = true;
        let true_map = true_val.to_tagged_map(&table).unwrap();
        let parsed_true = bool::from_tagged_map(true_map, &table).unwrap();
        assert_eq!(parsed_true, true);
        
        // Test false
        let false_val = false;
        let false_map = false_val.to_tagged_map(&table).unwrap();
        let parsed_false = bool::from_tagged_map(false_map, &table).unwrap();
        assert_eq!(parsed_false, false);
    }

    #[test]
    fn test_option_tagged_map() {
        let table = MockAtomTable::new();
        
        // Test Some(value)
        let some_val: Option<i32> = Some(42);
        let some_map = some_val.to_tagged_map(&table).unwrap();
        let parsed_some = Option::<i32>::from_tagged_map(some_map, &table).unwrap();
        assert_eq!(parsed_some, Some(42));
        
        // Test None
        let none_val: Option<i32> = None;
        let none_map = none_val.to_tagged_map(&table).unwrap();
        let parsed_none = Option::<i32>::from_tagged_map(none_map, &table).unwrap();
        assert_eq!(parsed_none, None);
    }

    #[test]
    fn test_vec_tagged_map() {
        let table = MockAtomTable::new();
        
        let vec_val = vec![1i32, 2i32, 3i32];
        let vec_map = vec_val.to_tagged_map(&table).unwrap();
        let parsed_vec = Vec::<i32>::from_tagged_map(vec_map, &table).unwrap();
        assert_eq!(parsed_vec, vec![1, 2, 3]);
        
        // Test empty vec
        let empty_vec: Vec<i32> = vec![];
        let empty_map = empty_vec.to_tagged_map(&table).unwrap();
        let parsed_empty = Vec::<i32>::from_tagged_map(empty_map, &table).unwrap();
        assert_eq!(parsed_empty, vec![]);
    }

    #[test]
    fn test_test_user_struct() {
        let table = MockAtomTable::new();
        
        let user = TestUser {
            id: 123,
            name: "John Doe".to_string(),
            email: Some("john@example.com".to_string()),
            active: true,
        };
        
        // Test serialization
        let map = user.to_tagged_map(&table).unwrap();
        
        // Verify type discriminator
        validate_type_discriminator(&map, "test_user", &table).unwrap();
        
        // Test deserialization
        let parsed = TestUser::from_tagged_map(map, &table).unwrap();
        assert_eq!(parsed, user);
    }

    #[test]
    fn test_test_user_with_none_email() {
        let table = MockAtomTable::new();
        
        let user = TestUser {
            id: 456,
            name: "Jane Doe".to_string(),
            email: None,
            active: false,
        };
        
        let map = user.to_tagged_map(&table).unwrap();
        let parsed = TestUser::from_tagged_map(map, &table).unwrap();
        assert_eq!(parsed, user);
    }

    #[test]
    fn test_test_status_enum_simple_variants() {
        let table = MockAtomTable::new();
        
        // Test Active variant
        let active = TestStatus::Active;
        let active_map = active.to_tagged_map(&table).unwrap();
        let parsed_active = TestStatus::from_tagged_map(active_map, &table).unwrap();
        assert_eq!(parsed_active, TestStatus::Active);
        
        // Test Inactive variant
        let inactive = TestStatus::Inactive;
        let inactive_map = inactive.to_tagged_map(&table).unwrap();
        let parsed_inactive = TestStatus::from_tagged_map(inactive_map, &table).unwrap();
        assert_eq!(parsed_inactive, TestStatus::Inactive);
    }

    #[test]
    fn test_test_status_enum_complex_variants() {
        let table = MockAtomTable::new();
        
        // Test Pending variant with data
        let pending = TestStatus::Pending {
            reason: "Waiting for approval".to_string(),
        };
        let pending_map = pending.to_tagged_map(&table).unwrap();
        let parsed_pending = TestStatus::from_tagged_map(pending_map, &table).unwrap();
        assert_eq!(parsed_pending, pending);
        
        // Test Expired variant with data
        let expired = TestStatus::Expired { days: 30 };
        let expired_map = expired.to_tagged_map(&table).unwrap();
        let parsed_expired = TestStatus::from_tagged_map(expired_map, &table).unwrap();
        assert_eq!(parsed_expired, expired);
    }

    #[test]
    fn test_helper_functions() {
        let table = MockAtomTable::new();
        
        // Test get_type_atom
        let atom_idx = get_type_atom("test_atom", &table).unwrap();
        assert!(table.atom_equals_str(atom_idx, "test_atom"));
        
        // Test type_field_atom
        let type_atom = type_field_atom(&table).unwrap();
        assert!(table.atom_equals_str(type_atom, "type"));
        
        // Test variant_field_atom
        let variant_atom = variant_field_atom(&table).unwrap();
        assert!(table.atom_equals_str(variant_atom, "variant"));
    }

    #[test]
    fn test_map_extraction_functions() {
        let table = MockAtomTable::new();
        
        // Create a test map
        let name_atom = get_type_atom("name", &table).unwrap();
        let age_atom = get_type_atom("age", &table).unwrap();
        let active_atom = get_type_atom("active", &table).unwrap();
        let height_atom = get_type_atom("height", &table).unwrap();
        
        let test_map = TermValue::Map(vec![
            (TermValue::Atom(name_atom), TermValue::Binary(b"Alice".to_vec())),
            (TermValue::Atom(age_atom), TermValue::SmallInt(30)),
            (TermValue::Atom(active_atom), TermValue::Atom(table.ensure_atom_str("true").unwrap())),
            (TermValue::Atom(height_atom), TermValue::Float(5.6)),
        ]);
        
        // Test field extraction
        let name = extract_string_field(&test_map, "name", &table).unwrap();
        assert_eq!(name, "Alice");
        
        let age = extract_int_field(&test_map, "age", &table).unwrap();
        assert_eq!(age, 30);
        
        let active = extract_bool_field(&test_map, "active", &table).unwrap();
        assert_eq!(active, true);
        
        let height = extract_float_field(&test_map, "height", &table).unwrap();
        assert_eq!(height, 5.6);
        
        // Test optional field extraction
        let optional_name = extract_optional_field(&test_map, "name", &table, |value, _table| {
            match value {
                TermValue::Binary(bytes) => {
                    String::from_utf8(bytes.clone()).map_err(|_| TaggedError::InvalidUtf8)
                }
                _ => Err(TaggedError::WrongType { expected: "binary", found: "other" }),
            }
        }).unwrap();
        assert_eq!(optional_name, Some("Alice".to_string()));
        
        // Test missing optional field
        let missing_field = extract_optional_field(&test_map, "missing", &table, |value, _table| {
            match value {
                TermValue::Binary(bytes) => {
                    String::from_utf8(bytes.clone()).map_err(|_| TaggedError::InvalidUtf8)
                }
                _ => Err(TaggedError::WrongType { expected: "binary", found: "other" }),
            }
        }).unwrap();
        assert_eq!(missing_field, None);
    }

    #[test]
    fn test_error_conditions() {
        let table = MockAtomTable::new();
        
        // Test wrong type for map
        let not_a_map = TermValue::SmallInt(42);
        let result = extract_string_field(&not_a_map, "field", &table);
        assert!(matches!(result, Err(TaggedError::WrongType { .. })));
        
        // Test missing field
        let empty_map = TermValue::Map(vec![]);
        let result = extract_string_field(&empty_map, "missing", &table);
        assert!(matches!(result, Err(TaggedError::Other(_))));
        
        // Test type mismatch in validation
        let wrong_type_map = TermValue::Map(vec![
            (TermValue::Atom(type_field_atom(&table).unwrap()), 
             TermValue::Atom(get_type_atom("wrong_type", &table).unwrap())),
        ]);
        let result = validate_type_discriminator(&wrong_type_map, "expected_type", &table);
        assert!(matches!(result, Err(TaggedError::TypeMismatch { .. })));
    }

    #[test]
    fn test_tagged_error_creation() {
        // Test nested error
        let inner_error = TaggedError::OutOfMemory;
        let nested = TaggedError::nested("field.subfield", inner_error.clone());
        if let TaggedError::NestedError { path, source } = nested {
            assert_eq!(path, "field.subfield");
            assert_eq!(*source, inner_error);
        } else {
            panic!("Expected nested error");
        }
        
        // Test type mismatch
        let mismatch = TaggedError::type_mismatch("String", "Integer");
        if let TaggedError::TypeMismatch { expected, found } = mismatch {
            assert_eq!(expected, "String");
            assert_eq!(found, "Integer");
        } else {
            panic!("Expected type mismatch error");
        }
        
        // Test missing field
        let missing = TaggedError::missing_field("email");
        if let TaggedError::MissingField(field) = missing {
            assert_eq!(field, "email");
        } else {
            panic!("Expected missing field error");
        }
        
        // Test invalid variant
        let invalid = TaggedError::invalid_variant("Status", "Unknown");
        if let TaggedError::InvalidVariant { enum_name, variant } = invalid {
            assert_eq!(enum_name, "Status");
            assert_eq!(variant, "Unknown");
        } else {
            panic!("Expected invalid variant error");
        }
    }

    #[test]
    fn test_complex_nested_structure() {
        let table = MockAtomTable::new();
        
        // Create a complex nested structure using primitives
        let user_data: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
        
        // Test serialization
        let map = user_data.to_tagged_map(&table).unwrap();
        
        // Test deserialization
        let parsed = Vec::<Option<i32>>::from_tagged_map(map, &table).unwrap();
        assert_eq!(parsed, vec![Some(1), None, Some(3)]);
    }

    #[test]
    fn test_type_name_methods() {
        assert_eq!(i32::type_name(), "i32");
        assert_eq!(String::type_name(), "string");
        assert_eq!(bool::type_name(), "bool");
        assert_eq!(Option::<i32>::type_name(), "option");
        assert_eq!(Vec::<i32>::type_name(), "vec");
        assert_eq!(TestUser::type_name(), "test_user");
        assert_eq!(TestStatus::type_name(), "test_status");
    }

    #[test]
    fn test_round_trip_serialization() {
        let table = MockAtomTable::new();
        
        // Test multiple round trips to ensure consistency
        let original = TestUser {
            id: 999,
            name: "Round Trip".to_string(),
            email: Some("roundtrip@test.com".to_string()),
            active: true,
        };
        
        for _ in 0..5 {
            let map = original.to_tagged_map(&table).unwrap();
            let parsed = TestUser::from_tagged_map(map, &table).unwrap();
            assert_eq!(parsed, original);
        }
    }

    #[test]
    fn test_float_integer_conversion() {
        let table = MockAtomTable::new();
        
        // Create a map with integer where float is expected
        let field_atom = get_type_atom("test_field", &table).unwrap();
        let test_map = TermValue::Map(vec![
            (TermValue::Atom(field_atom), TermValue::SmallInt(42)),
        ]);
        
        // Should be able to extract as float
        let float_result = extract_float_field(&test_map, "test_field", &table).unwrap();
        assert_eq!(float_result, 42.0);
        
        // Test with actual float
        let float_map = TermValue::Map(vec![
            (TermValue::Atom(field_atom), TermValue::Float(3.14)),
        ]);
        
        let float_result = extract_float_field(&float_map, "test_field", &table).unwrap();
        assert_eq!(float_result, 3.14);
    }
}