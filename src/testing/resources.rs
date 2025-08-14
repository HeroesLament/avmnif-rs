//! Resource management testing suite
//! 
//! This module provides comprehensive tests for the resource management system
//! without requiring a running AtomVM instance. All mocks are in testing/mocks.rs.

use crate::resource::*;
use crate::term::NifError;
use crate::testing::mocks::MockResourceManager;

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    // Test data structures
    #[repr(C)]
    #[derive(Debug, PartialEq)]
    struct TestResource {
        id: u32,
        name: [u8; 16],
        active: bool,
    }

    impl Default for TestResource {
        fn default() -> Self {
            Self {
                id: 42,
                name: *b"test_resource\0\0\0",
                active: true,
            }
        }
    }

    #[test]
    fn test_mock_resource_manager_creation() {
        let manager = MockResourceManager::new();
        assert_eq!(manager.get_resource_count(), 0);
        assert_eq!(manager.get_resource_type_count(), 0);
        assert_eq!(manager.get_monitor_count(), 0);
    }

    #[test]
    fn test_resource_type_initialization() {
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        // Test successful initialization
        let result = manager.init_resource_type(
            env,
            "test_resource",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        );
        
        assert!(result.is_ok());
        assert!(manager.verify_init_called("test_resource"));
        assert_eq!(manager.get_resource_type_count(), 1);
    }

    #[test]
    fn test_invalid_resource_name() {
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        // Test empty name
        let result = manager.init_resource_type(
            env,
            "",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        );
        
        assert_eq!(result.unwrap_err(), ResourceError::InvalidName);
        
        // Test too long name
        let long_name = "a".repeat(256);
        let result = manager.init_resource_type(
            env,
            &long_name,
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        );
        
        assert_eq!(result.unwrap_err(), ResourceError::InvalidName);
    }

    #[test]
    fn test_resource_allocation() {
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        // First create a resource type
        let resource_type = manager.init_resource_type(
            env,
            "test_type",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        // Test successful allocation
        let result = manager.alloc_resource(resource_type, 1024);
        assert!(result.is_ok());
        assert_eq!(manager.get_resource_count(), 1);
        
        // Test null resource type
        let result = manager.alloc_resource(core::ptr::null_mut(), 1024);
        assert_eq!(result.unwrap_err(), ResourceError::BadResourceType);
        
        // Test zero size
        let result = manager.alloc_resource(resource_type, 0);
        assert_eq!(result.unwrap_err(), ResourceError::BadArg);
    }

    #[test]
    fn test_resource_reference_counting() {
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        // Create resource type and allocate resource
        let resource_type = manager.init_resource_type(
            env,
            "test_type",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        let resource_ptr = manager.alloc_resource(resource_type, 1024).unwrap();
        
        // Initial ref count should be 1
        assert_eq!(manager.get_resource_ref_count(resource_ptr), Some(1));
        
        // Test keep resource
        let result = manager.keep_resource(resource_ptr);
        assert!(result.is_ok());
        assert_eq!(manager.get_resource_ref_count(resource_ptr), Some(2));
        
        // Test release resource
        let result = manager.release_resource(resource_ptr);
        assert!(result.is_ok());
        assert_eq!(manager.get_resource_ref_count(resource_ptr), Some(1));
        
        // Release again to trigger destructor
        let result = manager.release_resource(resource_ptr);
        assert!(result.is_ok());
        assert_eq!(manager.get_resource_count(), 0); // Resource should be destroyed
        
        // Test null pointer
        let result = manager.keep_resource(core::ptr::null_mut());
        assert_eq!(result.unwrap_err(), ResourceError::BadArg);
    }

    #[test]
    fn test_make_and_get_resource() {
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        // Create resource type and allocate resource
        let resource_type = manager.init_resource_type(
            env,
            "test_type",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        let resource_ptr = manager.alloc_resource(resource_type, 1024).unwrap();
        
        // Test make resource
        let term = manager.make_resource(env, resource_ptr).unwrap();
        assert!(term != 0);
        
        // Test get resource - using term % 1000 to map back to resource
        let retrieved_ptr = manager.get_resource(env, term, resource_type).unwrap();
        assert_eq!(retrieved_ptr, resource_ptr);
        
        // Test with wrong type
        let other_type = manager.init_resource_type(
            env,
            "other_type",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        let result = manager.get_resource(env, term, other_type);
        assert_eq!(result.unwrap_err(), ResourceError::ResourceNotFound);
    }

    #[test]
    fn test_process_monitoring() {
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        // Create resource
        let resource_type = manager.init_resource_type(
            env,
            "test_type",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        let resource_ptr = manager.alloc_resource(resource_type, 1024).unwrap();
        let pid = 12345i32;
        let mut monitor = ErlNifMonitor {
            resource_type: core::ptr::null_mut(),
            ref_ticks: 0,
        };
        
        // Test monitor process
        let result = manager.monitor_process(env, resource_ptr, &pid, &mut monitor);
        assert!(result.is_ok());
        assert_eq!(manager.get_monitor_count(), 1);
        
        // Test demonitor process
        let result = manager.demonitor_process(env, resource_ptr, &monitor);
        assert!(result.is_ok());
        assert_eq!(manager.get_monitor_count(), 0);
    }

    #[test]
    fn test_select_operations() {
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        // Create resource
        let resource_type = manager.init_resource_type(
            env,
            "test_type",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        let resource_ptr = manager.alloc_resource(resource_type, 1024).unwrap();
        let pid = 12345i32;
        let reference = 0x98765432u64;
        
        // Test select
        let result = manager.select(
            env,
            5, // file descriptor
            ErlNifSelectFlags::ERL_NIF_SELECT_READ,
            resource_ptr,
            &pid,
            reference,
        );
        assert!(result.is_ok());
        
        // Verify the call was tracked
        let state = manager.get_state();
        assert_eq!(state.select_calls.len(), 1);
        assert_eq!(state.select_calls[0].0, 5); // event
        assert_eq!(state.select_calls[0].1, ErlNifSelectFlags::ERL_NIF_SELECT_READ); // mode
    }

    #[test]
    fn test_error_injection() {
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        // Test init failure
        manager.set_fail_init(true);
        let result = manager.init_resource_type(
            env,
            "test_resource",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        );
        assert_eq!(result.unwrap_err(), ResourceError::InitializationFailed);
        
        // Reset and test alloc failure
        manager.set_fail_init(false);
        let resource_type = manager.init_resource_type(
            env,
            "test_resource",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        manager.set_fail_alloc(true);
        let result = manager.alloc_resource(resource_type, 1024);
        assert_eq!(result.unwrap_err(), ResourceError::OutOfMemory);
    }

    #[test]
    fn test_resource_limits() {
        let mut manager = MockResourceManager::new().with_max_resources(1);
        let env = core::ptr::null_mut();
        
        let resource_type = manager.init_resource_type(
            env,
            "test_type",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        // First allocation should succeed
        let result = manager.alloc_resource(resource_type, 1024);
        assert!(result.is_ok());
        
        // Second allocation should fail due to limit
        let result = manager.alloc_resource(resource_type, 1024);
        assert_eq!(result.unwrap_err(), ResourceError::OutOfMemory);
    }

    #[test]
    fn test_error_conversion() {
        // Test ResourceError to NifError conversion
        assert_eq!(NifError::from(ResourceError::OutOfMemory), NifError::OutOfMemory);
        assert_eq!(NifError::from(ResourceError::BadArg), NifError::BadArg);
        assert_eq!(NifError::from(ResourceError::InvalidName), NifError::BadArg);
    }

    #[test]
    fn test_global_manager_integration() {
        // Test that we can initialize and use the global manager
        let manager = MockResourceManager::new();
        init_resource_manager(manager);
        
        // Test that we can get the manager
        let _global_manager = get_resource_manager();
        
        // Note: We cannot test the convenience functions keep_resource() and release_resource()
        // in tests because they would try to link to FFI functions that don't exist in test builds.
        // The convenience functions are only meant for production use when the global manager
        // is initialized with AtomVMResourceManager.
    }

    #[test]
    fn test_helper_functions() {
        // Test resource type init helpers
        let init = resource_type_init();
        assert_eq!(init.members, 0);
        assert!(init.dtor.is_none());
        assert!(init.stop.is_none());
        assert!(init.down.is_none());
        
        // Test with destructor
        unsafe extern "C" fn test_dtor(_env: *mut ErlNifEnv, _obj: *mut core::ffi::c_void) {}
        let init_with_dtor = resource_type_init_with_dtor(test_dtor);
        assert_eq!(init_with_dtor.members, 1);
        assert!(init_with_dtor.dtor.is_some());
        
        // Test full init
        let init_full = resource_type_init_full(Some(test_dtor), None, None);
        assert_eq!(init_full.members, 1);
        assert!(init_full.dtor.is_some());
        assert!(init_full.stop.is_none());
        assert!(init_full.down.is_none());
    }

    #[test]
    fn test_resource_type_flags() {
        // Test that our enums have correct values
        assert_eq!(ErlNifResourceFlags::ERL_NIF_RT_CREATE as i32, 1);
        
        assert_eq!(ErlNifSelectFlags::ERL_NIF_SELECT_READ as i32, 1);
        assert_eq!(ErlNifSelectFlags::ERL_NIF_SELECT_WRITE as i32, 2);
        assert_eq!(ErlNifSelectFlags::ERL_NIF_SELECT_STOP as i32, 4);
        
        // Test that flags implement required traits
        let flag1 = ErlNifResourceFlags::ERL_NIF_RT_CREATE;
        let flag2 = ErlNifResourceFlags::ERL_NIF_RT_CREATE;
        assert_eq!(flag1, flag2); // PartialEq
        
        let _flag3 = flag1; // Copy
        assert_eq!(flag1, flag2); // Original still usable after copy
    }

    #[test] 
    fn test_concurrent_access_simulation() {
        // Since we're in no_std, we can't actually test concurrency,
        // but we can simulate concurrent-like access patterns
        let mut manager = MockResourceManager::new();
        let env = core::ptr::null_mut();
        
        let resource_type = manager.init_resource_type(
            env,
            "concurrent_test",
            &resource_type_init(),
            ErlNifResourceFlags::ERL_NIF_RT_CREATE,
        ).unwrap();
        
        // Simulate multiple "threads" allocating resources
        let mut resources = alloc::vec::Vec::new();
        for i in 0..10 {
            let resource = manager.alloc_resource(resource_type, 100 + i).unwrap();
            resources.push(resource);
        }
        
        assert_eq!(manager.get_resource_count(), 10);
        
        // Simulate concurrent reference counting
        for resource in &resources {
            assert!(manager.keep_resource(*resource).is_ok());
        }
        
        // All resources should have ref count 2 now
        for resource in &resources {
            assert_eq!(manager.get_resource_ref_count(*resource), Some(2));
        }
        
        // Release all references
        for resource in &resources {
            assert!(manager.release_resource(*resource).is_ok());
            assert!(manager.release_resource(*resource).is_ok());
        }
        
        // All resources should be destroyed
        assert_eq!(manager.get_resource_count(), 0);
    }
}