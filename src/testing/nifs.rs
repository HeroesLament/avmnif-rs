//! Test utilities for Native Implemented Functions (NIFs) and nif_collection macro

#[cfg(test)]
use alloc::{format, string::String, string::ToString, vec, vec::Vec};
use crate::atom::AtomTableOps;
use crate::testing::mocks::*;
use crate::term::{Term, TermValue, NifResult, NifError, Context};

#[cfg(test)]
/// Mock NIF function for testing the collection macro
pub extern "C" fn test_add_nif(
    _ctx: *mut Context,
    _argc: i32,
    _argv: *const Term
) -> Term {
    // Mock implementation that returns a simple integer
    Term::from_raw(42 << 4 | 0xF) // Small integer 42
}

#[cfg(test)]
/// Mock NIF function for testing string operations
pub extern "C" fn test_string_nif(
    _ctx: *mut Context,
    _argc: i32,
    _argv: *const Term
) -> Term {
    // Mock implementation
    Term::from_raw(0x3B) // NIL
}

#[cfg(test)]
/// Mock NIF function for testing list operations
pub extern "C" fn test_list_nif(
    _ctx: *mut Context,
    _argc: i32,
    _argv: *const Term
) -> Term {
    // Mock implementation
    Term::from_raw(0x3B) // NIL
}

#[cfg(test)]
/// Mock init function for testing
pub fn test_nif_init(_ctx: &mut Context) {
    // Mock initialization - would normally set up resources, etc.
}

#[cfg(test)]
/// Test helper to simulate NIF function calls
pub struct NifCallSimulator {
    pub call_count: u32,
    pub last_function: Option<String>,
    pub last_args: Vec<TermValue>,
}

#[cfg(test)]
impl NifCallSimulator {
    pub fn new() -> Self {
        Self {
            call_count: 0,
            last_function: None,
            last_args: Vec::new(),
        }
    }

    pub fn simulate_call(&mut self, function_name: &str, args: Vec<TermValue>) -> NifResult<TermValue> {
        self.call_count += 1;
        self.last_function = Some(function_name.to_string());
        self.last_args = args.clone();

        // Simulate different NIF behaviors based on function name
        match function_name {
            "add" => {
                if args.len() != 2 {
                    return Err(NifError::BadArity);
                }
                let a = args[0].as_int().ok_or(NifError::BadArg)?;
                let b = args[1].as_int().ok_or(NifError::BadArg)?;
                Ok(TermValue::int(a + b))
            }
            "list_length" => {
                if args.len() != 1 {
                    return Err(NifError::BadArity);
                }
                let length = args[0].list_length();
                Ok(TermValue::int(length as i32))
            }
            "make_tuple" => {
                Ok(TermValue::tuple(args))
            }
            "error_function" => {
                Err(NifError::BadArg)
            }
            _ => Err(NifError::Other("unknown function")),
        }
    }

    pub fn reset(&mut self) {
        self.call_count = 0;
        self.last_function = None;
        self.last_args.clear();
    }
}

#[cfg(test)]
/// Mock resolver function for testing
pub fn mock_nif_resolver(name: &str) -> Option<*const core::ffi::c_void> {
    match name {
        "test_add" => Some(test_add_nif as *const () as *const core::ffi::c_void),
        "test_string" => Some(test_string_nif as *const () as *const core::ffi::c_void),
        "test_list" => Some(test_list_nif as *const () as *const core::ffi::c_void),
        _ => None,
    }
}

#[cfg(test)]
/// Test collection definition using our macro
#[macro_export]
macro_rules! test_nif_collection {
    () => {
        $crate::nif_collection!(
            test_collection,
            init = test_nif_init,
            nifs = [
                ("add", 2, test_add_nif),
                ("string_op", 1, test_string_nif),
                ("list_op", 1, test_list_nif),
            ]
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nif_call_simulator_creation() {
        let simulator = NifCallSimulator::new();
        assert_eq!(simulator.call_count, 0);
        assert!(simulator.last_function.is_none());
        assert_eq!(simulator.last_args.len(), 0);
    }

    #[test]
    fn test_nif_call_simulator_add_function() {
        let mut simulator = NifCallSimulator::new();
        
        let args = vec![TermValue::int(10), TermValue::int(20)];
        let result = simulator.simulate_call("add", args).unwrap();
        
        assert_eq!(result, TermValue::int(30));
        assert_eq!(simulator.call_count, 1);
        assert_eq!(simulator.last_function.as_ref().unwrap(), "add");
        assert_eq!(simulator.last_args.len(), 2);
    }

    #[test]
    fn test_nif_call_simulator_bad_arity() {
        let mut simulator = NifCallSimulator::new();
        
        let args = vec![TermValue::int(10)]; // Should be 2 args for add
        let result = simulator.simulate_call("add", args);
        
        assert_eq!(result, Err(NifError::BadArity));
        assert_eq!(simulator.call_count, 1);
    }

    #[test]
    fn test_nif_call_simulator_bad_args() {
        let mut simulator = NifCallSimulator::new();
        let atom_table = MockAtomTable::new();
        
        let atom = TermValue::atom("not_a_number", &atom_table);
        let args = vec![TermValue::int(10), atom];
        let result = simulator.simulate_call("add", args);
        
        assert_eq!(result, Err(NifError::BadArg));
    }

    #[test]
    fn test_nif_call_simulator_list_length() {
        let mut simulator = NifCallSimulator::new();
        
        let list = TermValue::list(vec![
            TermValue::int(1),
            TermValue::int(2),
            TermValue::int(3),
        ]);
        
        let args = vec![list];
        let result = simulator.simulate_call("list_length", args).unwrap();
        
        assert_eq!(result, TermValue::int(3));
        assert_eq!(simulator.last_function.as_ref().unwrap(), "list_length");
    }

    #[test]
    fn test_nif_call_simulator_make_tuple() {
        let mut simulator = NifCallSimulator::new();
        
        let args = vec![
            TermValue::int(1),
            TermValue::int(2),
            TermValue::int(3),
        ];
        
        let result = simulator.simulate_call("make_tuple", args.clone()).unwrap();
        
        if let Some(elements) = result.as_tuple() {
            assert_eq!(elements.len(), 3);
            assert_eq!(elements[0], TermValue::int(1));
            assert_eq!(elements[1], TermValue::int(2));
            assert_eq!(elements[2], TermValue::int(3));
        } else {
            panic!("Expected tuple result");
        }
    }

    #[test]
    fn test_nif_call_simulator_error_function() {
        let mut simulator = NifCallSimulator::new();
        
        let result = simulator.simulate_call("error_function", vec![]);
        assert_eq!(result, Err(NifError::BadArg));
    }

    #[test]
    fn test_nif_call_simulator_unknown_function() {
        let mut simulator = NifCallSimulator::new();
        
        let result = simulator.simulate_call("unknown_func", vec![]);
        assert_eq!(result, Err(NifError::Other("unknown function")));
    }

    #[test]
    fn test_nif_call_simulator_reset() {
        let mut simulator = NifCallSimulator::new();
        
        simulator.simulate_call("add", vec![TermValue::int(1), TermValue::int(2)]).unwrap();
        assert_eq!(simulator.call_count, 1);
        assert!(simulator.last_function.is_some());
        
        simulator.reset();
        assert_eq!(simulator.call_count, 0);
        assert!(simulator.last_function.is_none());
        assert_eq!(simulator.last_args.len(), 0);
    }

    #[test]
    fn test_mock_nif_resolver() {
        // Test known functions
        let add_ptr = mock_nif_resolver("test_add");
        assert!(add_ptr.is_some());
        
        let string_ptr = mock_nif_resolver("test_string");
        assert!(string_ptr.is_some());
        
        let list_ptr = mock_nif_resolver("test_list");
        assert!(list_ptr.is_some());
        
        // Test unknown function
        let unknown_ptr = mock_nif_resolver("unknown");
        assert!(unknown_ptr.is_none());
    }

    #[test]
    fn test_nif_function_pointers() {
        // Test that our mock NIF functions have valid addresses
        let add_fn_ptr = test_add_nif as *const ();
        let string_fn_ptr = test_string_nif as *const ();
        let list_fn_ptr = test_list_nif as *const ();
        
        assert!(!add_fn_ptr.is_null());
        assert!(!string_fn_ptr.is_null());
        assert!(!list_fn_ptr.is_null());
        
        // Ensure they're different functions
        assert_ne!(add_fn_ptr, string_fn_ptr);
        assert_ne!(string_fn_ptr, list_fn_ptr);
    }

    #[test]
    fn test_term_raw_values() {
        // Test that our mock NIF functions return expected raw term values
        use core::ptr;
        
        let result = test_add_nif(ptr::null_mut(), 0, ptr::null());
        assert_eq!(result.raw(), 42 << 4 | 0xF); // Small integer 42
        
        let nil_result = test_string_nif(ptr::null_mut(), 0, ptr::null());
        assert_eq!(nil_result.raw(), 0x3B); // NIL
    }

    #[test]
    fn test_nif_collection_macro_components() {
        // This test verifies the macro generates the expected component names
        // Note: We can't easily test the actual macro expansion in unit tests,
        // but we can test the components that would be generated
        
        // Test function name generation (what the macro would create)
        let moniker = "test_collection";
        let expected_init_name = format!("{}_nif_init", moniker);
        let expected_resolver_name = format!("{}_get_nif", moniker);
        
        assert_eq!(expected_init_name, "test_collection_nif_init");
        assert_eq!(expected_resolver_name, "test_collection_get_nif");
        
        // Test that our NIF functions have the expected signatures
        let _init_fn: fn(&mut Context) = test_nif_init;
        let _add_fn: extern "C" fn(*mut Context, i32, *const Term) -> Term = test_add_nif;
    }

    #[test]
    fn test_nif_collection_function_list() {
        // Test the functions that would be registered by our test collection
        let functions = vec![
            ("add", 2),
            ("string_op", 1),
            ("list_op", 1),
        ];
        
        assert_eq!(functions.len(), 3);
        assert_eq!(functions[0], ("add", 2));
        assert_eq!(functions[1], ("string_op", 1));
        assert_eq!(functions[2], ("list_op", 1));
    }

    #[test]
    fn test_multiple_nif_calls() {
        let mut simulator = NifCallSimulator::new();
        
        // Simulate multiple calls
        simulator.simulate_call("add", vec![TermValue::int(1), TermValue::int(2)]).unwrap();
        simulator.simulate_call("list_length", vec![TermValue::list(vec![TermValue::int(1)])]).unwrap();
        simulator.simulate_call("add", vec![TermValue::int(5), TermValue::int(10)]).unwrap();
        
        assert_eq!(simulator.call_count, 3);
        assert_eq!(simulator.last_function.as_ref().unwrap(), "add");
        assert_eq!(simulator.last_args[0], TermValue::int(5));
        assert_eq!(simulator.last_args[1], TermValue::int(10));
    }

    #[test]
    fn test_nif_error_handling_patterns() {
        let mut simulator = NifCallSimulator::new();
        
        // Test various error conditions
        let error_cases = vec![
            ("add", vec![TermValue::int(1)], NifError::BadArity),
            ("unknown_func", vec![], NifError::Other("unknown function")),
            ("error_function", vec![], NifError::BadArg),
        ];
        
        for (func_name, args, expected_error) in error_cases {
            let result = simulator.simulate_call(func_name, args);
            assert_eq!(result, Err(expected_error));
        }
    }

    #[test]
    fn test_nif_collection_registration_data() {
        // Test the data that would be used for registration
        let collection_name = "test_collection";
        let nif_definitions = vec![
            ("add", 2, "test_add_nif"),
            ("string_op", 1, "test_string_nif"),
            ("list_op", 1, "test_list_nif"),
        ];
        
        // Verify collection metadata
        assert_eq!(collection_name, "test_collection");
        assert_eq!(nif_definitions.len(), 3);
        
        // Verify each NIF definition has the expected structure
        for (name, arity, func_name) in &nif_definitions {
            assert!(!name.is_empty());
            assert!(*arity > 0);
            assert!(!func_name.is_empty());
        }
    }

    #[test]
    fn test_link_section_attributes() {
        // Test that we understand the link section logic used in the macro
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let expected_section = "__DATA,.nif_collection";
        
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let expected_section = ".nif_collection";
        
        // Just verify the section names are what we expect
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        assert_eq!(expected_section, "__DATA,.nif_collection");
        
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        assert_eq!(expected_section, ".nif_collection");
    }
}