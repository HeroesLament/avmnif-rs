//! Resource management macros for AtomVM NIFs
//! 
//! Provides safe Rust wrappers around AtomVM's resource NIF API

use crate::term::{NifError, NifResult, Term};
use core::ffi::{c_void, c_char, c_int, c_uint};

#[cfg(feature = "alloc")]
use alloc::string::{String, ToString};

// Suppress naming warnings for FFI compatibility
#[allow(non_camel_case_types)]
pub type ERL_NIF_TERM = u64; // typedef term ERL_NIF_TERM (assuming 64-bit term)

pub type ErlNifEnv = c_void; // Opaque struct
pub type ErlNifResourceType = c_void; // Opaque struct  
pub type ErlNifPid = i32;
pub type ErlNifEvent = c_int;

/// Resource destructor callback type
pub type ErlNifResourceDtor = unsafe extern "C" fn(caller_env: *mut ErlNifEnv, obj: *mut c_void);

/// Select stop callback type  
pub type ErlNifResourceStop = unsafe extern "C" fn(
    caller_env: *mut ErlNifEnv, 
    obj: *mut c_void, 
    event: ErlNifEvent, 
    is_direct_call: c_int
);

/// Resource monitor callback type
pub type ErlNifResourceDown = unsafe extern "C" fn(
    caller_env: *mut ErlNifEnv, 
    obj: *mut c_void, 
    pid: *mut ErlNifPid, 
    mon: *mut ErlNifMonitor
);

/// Monitor type
#[repr(C)]
pub struct ErlNifMonitor {
    pub resource_type: *mut ErlNifResourceType,
    pub ref_ticks: u64,
}

/// Resource type initialization callbacks
#[repr(C)]
pub struct ErlNifResourceTypeInit {
    pub members: c_int,
    pub dtor: Option<ErlNifResourceDtor>,
    pub stop: Option<ErlNifResourceStop>, 
    pub down: Option<ErlNifResourceDown>,
}

/// Resource creation flags
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum ErlNifResourceFlags {
    ERL_NIF_RT_CREATE = 1,
    // ERL_NIF_RT_TAKEOVER not supported yet
}

/// Select mode flags
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum ErlNifSelectFlags {
    ERL_NIF_SELECT_READ = 1,
    ERL_NIF_SELECT_WRITE = 2,
    ERL_NIF_SELECT_STOP = 4,
}

// AtomVM Resource NIF FFI declarations (exact signatures from erl_nif.h)
extern "C" {
    /// Create or take over a resource type
    pub fn enif_init_resource_type(
        env: *mut ErlNifEnv,
        name: *const c_char,
        init: *const ErlNifResourceTypeInit,
        flags: ErlNifResourceFlags,
        tried: *mut ErlNifResourceFlags,
    ) -> *mut ErlNifResourceType;

    /// Allocate a new resource of the specified type and size
    pub fn enif_alloc_resource(
        resource_type: *mut ErlNifResourceType,
        size: c_uint,
    ) -> *mut c_void;

    /// Create an Erlang term from a resource pointer
    pub fn enif_make_resource(
        env: *mut ErlNifEnv,
        obj: *mut c_void,
    ) -> ERL_NIF_TERM;

    /// Extract a resource from an Erlang term
    pub fn enif_get_resource(
        env: *mut ErlNifEnv,
        t: ERL_NIF_TERM,
        resource_type: *mut ErlNifResourceType,
        objp: *mut *mut c_void,
    ) -> c_int;

    /// Increment resource reference count
    pub fn enif_keep_resource(obj: *mut c_void) -> c_int;

    /// Decrement resource reference count
    pub fn enif_release_resource(obj: *mut c_void) -> c_int;

    /// Select on file descriptors  
    pub fn enif_select(
        env: *mut ErlNifEnv,
        event: ErlNifEvent,
        mode: ErlNifSelectFlags,
        obj: *mut c_void,
        pid: *const ErlNifPid,
        reference: ERL_NIF_TERM,
    ) -> c_int;

    /// Monitor a process using a resource
    pub fn enif_monitor_process(
        env: *mut ErlNifEnv,
        obj: *mut c_void,
        target_pid: *const ErlNifPid,
        mon: *mut ErlNifMonitor,
    ) -> c_int;

    /// Remove a process monitor
    pub fn enif_demonitor_process(
        caller_env: *mut ErlNifEnv,
        obj: *mut c_void,
        mon: *const ErlNifMonitor,
    ) -> c_int;
}

/// Register a new resource type with AtomVM
/// 
/// # Usage
/// ```rust
/// resource_type!(DISPLAY_TYPE, DisplayContext, display_destructor);
/// ```
#[macro_export]
macro_rules! resource_type {
    ($resource_name:ident, $rust_type:ty, $destructor_fn:ident) => {
        // Create global static to hold the resource type pointer
        static mut $resource_name: *mut $crate::resource::ErlNifResourceType = core::ptr::null_mut();
        
        // Create a module init function that registers this resource type
        paste::paste! {
            #[no_mangle]
            pub extern "C" fn [<init_ $resource_name:lower>](env: *mut $crate::resource::ErlNifEnv) -> bool {
                let resource_name_cstr = concat!(stringify!($resource_name), "\0");
                let init_callbacks = $crate::resource::resource_type_init_with_dtor($destructor_fn);
                let mut tried_flags = $crate::resource::ErlNifResourceFlags::ERL_NIF_RT_CREATE;
                
                unsafe {
                    $resource_name = $crate::resource::enif_init_resource_type(
                        env,
                        resource_name_cstr.as_ptr() as *const core::ffi::c_char,
                        &init_callbacks,
                        $crate::resource::ErlNifResourceFlags::ERL_NIF_RT_CREATE,
                        &mut tried_flags,
                    );
                    
                    !$resource_name.is_null()
                }
            }
            
            // Provide a getter function for the resource type
            #[no_mangle]
            pub extern "C" fn [<get_ $resource_name:lower>]() -> *mut $crate::resource::ErlNifResourceType {
                unsafe { $resource_name }
            }
        }
    };
    
    // Version without destructor
    ($resource_name:ident, $rust_type:ty) => {
        // Create global static to hold the resource type pointer
        static mut $resource_name: *mut $crate::resource::ErlNifResourceType = core::ptr::null_mut();
        
        paste::paste! {
            #[no_mangle]
            pub extern "C" fn [<init_ $resource_name:lower>](env: *mut $crate::resource::ErlNifEnv) -> bool {
                let resource_name_cstr = concat!(stringify!($resource_name), "\0");
                let init_callbacks = $crate::resource::resource_type_init();
                let mut tried_flags = $crate::resource::ErlNifResourceFlags::ERL_NIF_RT_CREATE;
                
                unsafe {
                    $resource_name = $crate::resource::enif_init_resource_type(
                        env,
                        resource_name_cstr.as_ptr() as *const core::ffi::c_char,
                        &init_callbacks,
                        $crate::resource::ErlNifResourceFlags::ERL_NIF_RT_CREATE,
                        &mut tried_flags,
                    );
                    
                    !$resource_name.is_null()
                }
            }
            
            #[no_mangle]
            pub extern "C" fn [<get_ $resource_name:lower>]() -> *mut $crate::resource::ErlNifResourceType {
                unsafe { $resource_name }
            }
        }
    };
}

/// Create a new resource instance
/// 
/// # Usage
/// ```rust
/// let display_ptr = create_resource!(display_type, DisplayContext {
///     width: 240,
///     height: 320,
///     initialized: true,
/// })?;
/// ```
#[macro_export]
macro_rules! create_resource {
    ($type_var:ident, $data:expr) => {{
        let data = $data;
        let size = core::mem::size_of_val(&data) as core::ffi::c_uint;
        let ptr = unsafe {
            paste::paste! {
                extern "C" {
                    fn [<get_ $type_var:lower>]() -> *mut $crate::resource::ErlNifResourceType;
                }
                let resource_type = [<get_ $type_var:lower>]();
                $crate::resource::enif_alloc_resource(resource_type, size)
            }
        };
        if ptr.is_null() {
            Err($crate::term::NifError::OutOfMemory)
        } else {
            // Write the data to the allocated resource
            unsafe {
                core::ptr::write(ptr as *mut _, data);
            }
            Ok(ptr)
        }
    }};
}

/// Extract a resource from an Erlang term
/// 
/// # Usage
/// ```rust
/// let display = get_resource!(env, args[0], display_type)?;
/// display.width = 320;
/// ```
#[macro_export]
macro_rules! get_resource {
    ($env:expr, $term:expr, $type_var:ident) => {{
        let mut ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let success = unsafe {
            paste::paste! {
                extern "C" {
                    fn [<get_ $type_var:lower>]() -> *mut $crate::resource::ErlNifResourceType;
                }
                let resource_type = [<get_ $type_var:lower>]();
                $crate::resource::enif_get_resource(
                    $env.as_c_ptr(),
                    $term.as_raw(),
                    resource_type,
                    &mut ptr as *mut *mut core::ffi::c_void,
                )
            }
        };
        if success != 0 && !ptr.is_null() {
            // SAFETY: Resource type system ensures this cast is valid
            Ok(unsafe { &mut *(ptr as *mut _) })
        } else {
            Err($crate::term::NifError::BadArg)
        }
    }};
}

/// Convert a resource pointer to an Erlang term
/// 
/// # Usage
/// ```rust
/// let term = make_resource_term!(env, display_ptr);
/// ```
#[macro_export]
macro_rules! make_resource_term {
    ($env:expr, $resource_ptr:expr) => {{
        let raw_term = unsafe {
            $crate::resource::enif_make_resource(
                $env.as_c_ptr(),
                $resource_ptr,
            )
        };
        $crate::term::Term::from_raw(raw_term)
    }};
}

/// Manually increment resource reference count
/// 
/// Most users won't need this - automatic reference counting
/// happens when resources are created/passed to Erlang
pub fn keep_resource(resource: *mut c_void) -> NifResult<()> {
    let result = unsafe { enif_keep_resource(resource) };
    if result != 0 {
        Ok(())
    } else {
        Err(NifError::BadArg)
    }
}

/// Manually decrement resource reference count
/// 
/// Most users won't need this - automatic cleanup happens
/// when Erlang GC removes the resource term
pub fn release_resource(resource: *mut c_void) -> NifResult<()> {
    let result = unsafe { enif_release_resource(resource) };
    if result != 0 {
        Ok(())
    } else {
        Err(NifError::BadArg)
    }
}

/// Helper for creating resource type initialization structs
pub const fn resource_type_init() -> ErlNifResourceTypeInit {
    ErlNifResourceTypeInit {
        members: 0,
        dtor: None,
        stop: None,
        down: None,
    }
}

/// Helper for creating resource type initialization with destructor
pub const fn resource_type_init_with_dtor(dtor: ErlNifResourceDtor) -> ErlNifResourceTypeInit {
    ErlNifResourceTypeInit {
        members: 1,
        dtor: Some(dtor),
        stop: None,
        down: None,
    }
}

/// Helper for creating resource type initialization with all callbacks
pub const fn resource_type_init_full(
    dtor: Option<ErlNifResourceDtor>,
    stop: Option<ErlNifResourceStop>,
    down: Option<ErlNifResourceDown>,
) -> ErlNifResourceTypeInit {
    let mut members = 0;
    if dtor.is_some() { members += 1; }
    if stop.is_some() { members += 1; }
    if down.is_some() { members += 1; }
    
    ErlNifResourceTypeInit {
        members,
        dtor,
        stop,
        down,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestResource {
        id: u32,
        #[cfg(feature = "alloc")]
        name: String,
        active: bool,
    }

    #[test]
    fn test_resource_type_init_helpers() {
        let init = resource_type_init();
        assert_eq!(init.members, 0);
        assert!(init.dtor.is_none());

        unsafe extern "C" fn test_dtor(_env: *mut ErlNifEnv, _obj: *mut c_void) {}
        
        let init_with_dtor = resource_type_init_with_dtor(test_dtor);
        assert_eq!(init_with_dtor.members, 1);
        assert!(init_with_dtor.dtor.is_some());
    }

    #[test]
    fn test_resource_macro_compilation() {
        // These should compile without errors
        // (Can't actually run without AtomVM runtime)
        
        let _create_usage = || -> NifResult<*mut c_void> {
            // Note: This test requires the paste crate and a registered resource type
            // resource_type!(TEST_RESOURCE_TYPE, TestResource, test_destructor);
            
            create_resource!(TEST_RESOURCE_TYPE, TestResource {
                id: 42,
                #[cfg(feature = "alloc")]
                name: "test".to_string(),
                active: true,
            })
        };

        // Note: get_resource! and make_resource_term! need Term/Env types
        // to be fully implemented before testing
    }
}