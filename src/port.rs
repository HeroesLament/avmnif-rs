//! Port collection macros for AtomVM
//! 
//! Provides safe Rust wrappers around AtomVM's port driver API

use crate::term::{Term, NifError};
use crate::context::{Context, GlobalContext, ContextExt, PlatformData, PortBuilder};
use core::ffi::{c_void, c_char, c_int};

// Suppress warnings for unused items since this is a library
#[allow(unused_imports)]
use alloc::boxed::Box;

// AtomVM port types (reuse from context module)
pub type ErlNifEnv = c_void;
pub type ERL_NIF_TERM = u64;

/// Port message type
pub type Message = c_void;

/// Port result enum
#[repr(C)]
pub enum PortResult {
    Continue = 0,
    Terminate = 1,
}

/// Port driver function type signatures
pub type PortInitFn = fn(&mut GlobalContext);
pub type PortDestroyFn = fn(&mut GlobalContext);  
pub type PortCreateFn = fn(&GlobalContext, Term) -> *mut Context;
pub type PortHandlerFn = fn(&mut Context, &Message) -> PortResult;

/// C-compatible function types for FFI boundary
type CPortCreateFn = extern "C" fn(*const GlobalContext, ERL_NIF_TERM) -> *mut Context;
type CPortHandlerFn = extern "C" fn(*mut Context, *const Message) -> PortResult;

/// Port driver registration structure
#[repr(C)]
pub struct AtomVMPortDriver {
    pub name: *const c_char,
    pub init: Option<PortInitFn>,
    pub destroy: Option<PortDestroyFn>,
    pub create_port: CPortCreateFn,
    pub message_handler: CPortHandlerFn,
}

unsafe impl Sync for AtomVMPortDriver {}

// AtomVM Port API FFI declarations
extern "C" {
    /// Send a reply to an Erlang process from port context
    pub fn port_send_reply(
        ctx: *mut Context,
        pid: ERL_NIF_TERM,
        reference: ERL_NIF_TERM,
        reply: ERL_NIF_TERM,
    );
    
    /// Send an async message to an Erlang process from any context (ISR-safe)
    pub fn port_send_message_from_task(
        global: *mut GlobalContext,
        pid: u32,
        message: ERL_NIF_TERM,
    );
    
    /// Parse a generic port message into components
    pub fn parse_port_message(
        message: *const Message,
        pid: *mut ERL_NIF_TERM,
        reference: *mut ERL_NIF_TERM,
        command: *mut ERL_NIF_TERM,
    ) -> c_int;
}

/// Register a port collection with AtomVM
/// 
/// # Usage
/// ```rust
/// port_collection!(
///     my_port,
///     init = my_port_init,
///     destroy = my_port_destroy,
///     create_port = my_port_create,
///     handler = my_port_handler
/// );
/// ```
#[macro_export]
macro_rules! port_collection {
    (
        $port_name:ident,
        init = $init_fn:ident,
        destroy = $destroy_fn:ident,
        create_port = $create_port_fn:ident,
        handler = $handler_fn:ident
    ) => {
        paste::paste! {
            // Wrapper functions that convert between C and Rust types
            extern "C" fn [<$create_port_fn _wrapper>](
                global: *const $crate::context::GlobalContext,
                opts: $crate::port::ERL_NIF_TERM
            ) -> *mut $crate::context::Context {
                let global_ref = unsafe { &*global };
                let opts_term = $crate::term::Term::from_raw(opts.try_into().unwrap());
                $create_port_fn(global_ref, opts_term)
            }
            
            extern "C" fn [<$handler_fn _wrapper>](
                ctx: *mut $crate::context::Context,
                message: *const $crate::port::Message
            ) -> $crate::port::PortResult {
                let ctx_ref = unsafe { &mut *ctx };
                let message_ref = unsafe { &*message };
                $handler_fn(ctx_ref, message_ref)
            }
            
            // Create the port driver structure using wrapper functions
            static [<$port_name:upper _PORT_DRIVER>]: $crate::port::AtomVMPortDriver = $crate::port::AtomVMPortDriver {
                name: concat!(stringify!($port_name), "\0").as_ptr() as *const core::ffi::c_char,
                init: Some($init_fn),
                destroy: Some($destroy_fn),
                create_port: [<$create_port_fn _wrapper>],
                message_handler: [<$handler_fn _wrapper>],
            };
            
            // Export the port driver registration function
            #[no_mangle]
            pub extern "C" fn [<$port_name _port_driver_init>]() -> *const $crate::port::AtomVMPortDriver {
                &[<$port_name:upper _PORT_DRIVER>]
            }
            
            // Export individual functions for debugging/testing
            #[no_mangle]
            pub extern "C" fn [<$port_name _init>](global: *mut $crate::context::GlobalContext) {
                let global_ref = unsafe { &mut *global };
                $init_fn(global_ref);
            }
            
            #[no_mangle]
            pub extern "C" fn [<$port_name _destroy>](global: *mut $crate::context::GlobalContext) {
                let global_ref = unsafe { &mut *global };
                $destroy_fn(global_ref);
            }
            
            #[no_mangle]
            pub extern "C" fn [<$port_name _create_port>](
                global: *const $crate::context::GlobalContext,
                opts: $crate::port::ERL_NIF_TERM
            ) -> *mut $crate::context::Context {
                [<$create_port_fn _wrapper>](global, opts)
            }
            
            #[no_mangle]
            pub extern "C" fn [<$port_name _message_handler>](
                ctx: *mut $crate::context::Context,
                message: *const $crate::port::Message
            ) -> $crate::port::PortResult {
                [<$handler_fn _wrapper>](ctx, message)
            }
        }
    };
    
    // Version without init/destroy functions
    (
        $port_name:ident,
        create_port = $create_port_fn:ident,
        handler = $handler_fn:ident
    ) => {
        paste::paste! {
            // Wrapper functions that convert between C and Rust types
            extern "C" fn [<$create_port_fn _wrapper>](
                global: *const $crate::context::GlobalContext,
                opts: $crate::port::ERL_NIF_TERM
            ) -> *mut $crate::context::Context {
                let global_ref = unsafe { &*global };
                let opts_term = $crate::term::Term::from_raw(opts.try_into().unwrap());
                $create_port_fn(global_ref, opts_term)
            }
            
            extern "C" fn [<$handler_fn _wrapper>](
                ctx: *mut $crate::context::Context,
                message: *const $crate::port::Message
            ) -> $crate::port::PortResult {
                let ctx_ref = unsafe { &mut *ctx };
                let message_ref = unsafe { &*message };
                $handler_fn(ctx_ref, message_ref)
            }
            
            static [<$port_name:upper _PORT_DRIVER>]: $crate::port::AtomVMPortDriver = $crate::port::AtomVMPortDriver {
                name: concat!(stringify!($port_name), "\0").as_ptr() as *const core::ffi::c_char,
                init: None,
                destroy: None,
                create_port: [<$create_port_fn _wrapper>],
                message_handler: [<$handler_fn _wrapper>],
            };
            
            #[no_mangle]
            pub extern "C" fn [<$port_name _port_driver_init>]() -> *const $crate::port::AtomVMPortDriver {
                &[<$port_name:upper _PORT_DRIVER>]
            }
            
            #[no_mangle]
            pub extern "C" fn [<$port_name _create_port>](
                global: *const $crate::context::GlobalContext,
                opts: $crate::port::ERL_NIF_TERM
            ) -> *mut $crate::context::Context {
                [<$create_port_fn _wrapper>](global, opts)
            }
            
            #[no_mangle]
            pub extern "C" fn [<$port_name _message_handler>](
                ctx: *mut $crate::context::Context,
                message: *const $crate::port::Message
            ) -> $crate::port::PortResult {
                [<$handler_fn _wrapper>](ctx, message)
            }
        }
    };
}

/// Helper functions for port message handling

/// Parse a generic port message into its components
pub fn parse_gen_message(message: &Message) -> Result<(Term, Term, Term), NifError> {
    let mut pid: u64 = 0;
    let mut reference: u64 = 0;
    let mut command: u64 = 0;
    
    let result = unsafe {
        parse_port_message(
            message as *const _ as *const c_void,
            &mut pid,
            &mut reference,
            &mut command,
        )
    };
    
    if result != 0 {
        Ok((
            Term::from_raw(pid.try_into().unwrap()),
            Term::from_raw(reference.try_into().unwrap()),
            Term::from_raw(command.try_into().unwrap()),
        ))
    } else {
        Err(NifError::BadArg)
    }
}

/// Send a reply to an Erlang process
pub fn send_reply(ctx: &Context, pid: Term, reference: Term, reply: Term) {
    unsafe {
        port_send_reply(
            ctx as *const _ as *mut Context,
            pid.raw().try_into().unwrap(),
            reference.raw().try_into().unwrap(),
            reply.raw().try_into().unwrap(),
        );
    }
}

/// Send an async message to an Erlang process (ISR-safe)
pub fn send_async_message(pid: u32, message: Term) {
    unsafe {
        port_send_message_from_task(
            crate::context::get_global_context(),
            pid,
            message.raw().try_into().unwrap(),
        );
    }
}

/// Trait for port data types to implement cleanup and message handling
pub trait PortData: PlatformData {
    /// Called when the port receives a message
    fn handle_message(&mut self, message: &Message) -> PortResult {
        let _ = message; // Suppress unused warning
        PortResult::Continue
    }
    
    /// Get the owner PID for this port (if any)
    fn get_owner_pid(&self) -> Option<u32> {
        None
    }
    
    /// Set the owner PID for this port
    fn set_owner_pid(&mut self, _pid: u32) {}
    
    /// Check if the port is active
    fn is_active(&self) -> bool {
        true
    }
    
    /// Activate/deactivate the port
    fn set_active(&mut self, _active: bool) {}
}

/// Generic port data wrapper with standard functionality
#[repr(C)]
pub struct GenericPortData<T: PortData> {
    pub inner: T,
    pub owner_pid: u32,
    pub active: bool,
}

impl<T: PortData> GenericPortData<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            owner_pid: 0,
            active: false,
        }
    }
    
    pub fn set_owner(&mut self, pid: u32) {
        self.owner_pid = pid;
        self.active = true;
        self.inner.set_owner_pid(pid);
    }
    
    pub fn deactivate(&mut self) {
        self.active = false;
        self.inner.set_active(false);
        self.inner.cleanup();
    }
    
    pub fn get_inner(&self) -> &T {
        &self.inner
    }
    
    pub fn get_inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: PortData> PlatformData for GenericPortData<T> {
    fn cleanup(&mut self) {
        self.deactivate();
    }
}

impl<T: PortData> PortData for GenericPortData<T> {
    fn handle_message(&mut self, message: &Message) -> PortResult {
        if self.active {
            self.inner.handle_message(message)
        } else {
            PortResult::Terminate
        }
    }
    
    fn get_owner_pid(&self) -> Option<u32> {
        if self.owner_pid != 0 {
            Some(self.owner_pid)
        } else {
            None
        }
    }
    
    fn set_owner_pid(&mut self, pid: u32) {
        self.owner_pid = pid;
    }
    
    fn is_active(&self) -> bool {
        self.active
    }
    
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

/// Macro for creating simple port data structures
#[macro_export]
macro_rules! port_data {
    (
        $name:ident {
            $(
                $field:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        #[repr(C)]
        pub struct $name {
            $(
                pub $field: $field_type,
            )*
        }
        
        impl $crate::context::PlatformData for $name {}
        impl $crate::port::PortData for $name {}
        
        impl $name {
            pub fn new() -> Self {
                Self {
                    $(
                        $field: Default::default(),
                    )*
                }
            }
        }
        
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Error handling for port operations
#[derive(Debug, Clone, Copy)]
pub enum PortError {
    /// Invalid message format
    InvalidMessage,
    /// Port not active
    PortInactive,
    /// Hardware error
    HardwareError,
    /// Out of memory
    OutOfMemory,
    /// Generic error
    Generic,
}

impl From<PortError> for PortResult {
    fn from(_error: PortError) -> Self {
        PortResult::Terminate
    }
}

/// Result type for port operations
pub type PortOpResult<T> = Result<T, PortError>;

/// Utility functions for common port operations

/// Extract PID as u32 from Term (for use in async messaging)
pub fn term_to_pid(term: Term) -> PortOpResult<u32> {
    // This would need to be implemented based on actual Term structure
    // For now, return a placeholder
    Ok(term.raw() as u32) // This is obviously wrong, but demonstrates the interface
}

/// Create a standard error reply
pub fn create_error_reply(reason: &str) -> Term {
    // This would use the actual term construction API
    // For now, return a placeholder
    let _ = reason;
    Term::from_raw(0) // Obviously wrong, but demonstrates interface
}

/// Create a standard success reply
pub fn create_ok_reply(data: Term) -> Term {
    // This would use the actual term construction API
    let _ = data;
    Term::from_raw(0) // Obviously wrong, but demonstrates interface
}

/// Standard message handler template
pub fn handle_standard_message<T: PortData>(
    ctx: &mut Context,
    message: &Message,
) -> PortResult {
    let port_data = unsafe {
        let data_ptr = ctx.get_platform_data_as::<GenericPortData<T>>();
        if data_ptr.is_null() {
            return PortResult::Terminate;
        }
        &mut *data_ptr
    };
    
    if let Ok((pid, reference, command)) = parse_gen_message(message) {
        // Convert command to TermValue for pattern matching
        let command_value = match command.to_value() {
            Ok(val) => val,
            Err(_) => {
                let reply = create_error_reply("invalid_command");
                send_reply(ctx, pid, reference, reply);
                return PortResult::Continue;
            }
        };
        
        // Handle standard commands using TermValue pattern matching
        if command_value.is_atom_str("start") {
            if let Ok(pid_u32) = term_to_pid(pid) {
                port_data.set_owner(pid_u32);
                let reply = create_ok_reply(Term::from_raw(0)); // atom "ok"
                send_reply(ctx, pid, reference, reply);
                PortResult::Continue
            } else {
                let reply = create_error_reply("invalid_pid");
                send_reply(ctx, pid, reference, reply);
                PortResult::Continue
            }
        } else if command_value.is_atom_str("stop") {
            port_data.deactivate();
            let reply = create_ok_reply(Term::from_raw(0)); // atom "ok"
            send_reply(ctx, pid, reference, reply);
            PortResult::Terminate
        } else if command_value.is_atom_str("status") {
            let _status = if port_data.is_active() {
                "active"
            } else {
                "inactive"
            };
            let reply = create_ok_reply(Term::from_raw(0)); // would be atom with status
            send_reply(ctx, pid, reference, reply);
            PortResult::Continue
        } else {
            // Delegate to the port data's message handler
            port_data.handle_message(message)
        }
    } else {
        PortResult::Terminate
    }
}

/// Create a port with automatic platform data setup
pub fn create_port_with_data<T: PortData>(
    global: &GlobalContext,
    data: T,
) -> *mut Context {
    let wrapped_data = GenericPortData::new(data);
    PortBuilder::new(wrapped_data).build(global)
}

/// Create a port with data and user term
pub fn create_port_with_data_and_term<T: PortData>(
    global: &GlobalContext,
    data: T,
    user_term: Term,
) -> *mut Context {
    let wrapped_data = GenericPortData::new(data);
    PortBuilder::new(wrapped_data).build_with_user_term(global, user_term)
}

/// Safely execute a function with port data
pub fn with_port_data<T: PortData, R, F>(ctx: &Context, f: F) -> Option<R>
where
    F: FnOnce(&GenericPortData<T>) -> R,
{
    unsafe {
        let data_ptr = ctx.get_platform_data_as::<GenericPortData<T>>();
        if data_ptr.is_null() {
            None
        } else {
            Some(f(&*data_ptr))
        }
    }
}

/// Safely execute a function with mutable port data
pub fn with_port_data_mut<T: PortData, R, F>(ctx: &mut Context, f: F) -> Option<R>
where
    F: FnOnce(&mut GenericPortData<T>) -> R,
{
    unsafe {
        let data_ptr = ctx.get_platform_data_as::<GenericPortData<T>>();
        if data_ptr.is_null() {
            None
        } else {
            Some(f(&mut *data_ptr))
        }
    }
}

/// High-level port creation macro that handles common patterns
#[macro_export]
macro_rules! simple_port {
    (
        $port_name:ident,
        data = $data_type:ty,
        init_data = $init_expr:expr
    ) => {
        fn [<$port_name _create>](global: &$crate::context::GlobalContext, opts: $crate::term::Term) -> *mut $crate::context::Context {
            let _ = opts; // suppress unused warning
            let data: $data_type = $init_expr;
            $crate::port::create_port_with_data(global, data)
        }
        
        fn [<$port_name _handler>](ctx: &mut $crate::context::Context, message: &$crate::port::Message) -> $crate::port::PortResult {
            $crate::port::handle_standard_message::<$data_type>(ctx, message)
        }
        
        $crate::port_collection!(
            $port_name,
            create_port = [<$port_name _create>],
            handler = [<$port_name _handler>]
        );
    };
    
    (
        $port_name:ident,
        data = $data_type:ty,
        init_data = $init_expr:expr,
        init = $init_fn:ident,
        destroy = $destroy_fn:ident
    ) => {
        fn [<$port_name _create>](global: &$crate::context::GlobalContext, opts: $crate::term::Term) -> *mut $crate::context::Context {
            let _ = opts; // suppress unused warning
            let data: $data_type = $init_expr;
            $crate::port::create_port_with_data(global, data)
        }
        
        fn [<$port_name _handler>](ctx: &mut $crate::context::Context, message: &$crate::port::Message) -> $crate::port::PortResult {
            $crate::port::handle_standard_message::<$data_type>(ctx, message)
        }
        
        $crate::port_collection!(
            $port_name,
            init = $init_fn,
            destroy = $destroy_fn,
            create_port = [<$port_name _create>],
            handler = [<$port_name _handler>]
        );
    };
}
