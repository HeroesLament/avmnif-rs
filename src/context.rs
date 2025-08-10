//! Context management for AtomVM ports and NIFs
//! 
//! Provides safe wrappers around AtomVM's context structures

use alloc::boxed::Box;
use crate::term::Term;
use core::ffi::c_void;

/// Opaque context structure that matches AtomVM's internal representation
#[repr(C)]
pub struct Context {
    _private: [u8; 0],
}

/// Global AtomVM context
pub type GlobalContext = c_void;

// AtomVM Context API FFI declarations
extern "C" {
    /// Create a new port context
    pub fn create_port_context(global: *const GlobalContext) -> *mut Context;
    
    /// Destroy a port context and clean up resources
    pub fn destroy_port_context(ctx: *mut Context);
    
    /// Check if a port is still alive
    pub fn port_is_alive(ctx: *const Context) -> i32;
    
    /// Get platform data from context
    pub fn context_get_platform_data(ctx: *const Context) -> *mut c_void;
    
    /// Set platform data in context
    pub fn context_set_platform_data(ctx: *mut Context, data: *mut c_void);
    
    /// Get user data from context (for storing Erlang terms)
    pub fn context_get_user_data(ctx: *const Context) -> u64;
    
    /// Set user data in context
    pub fn context_set_user_data(ctx: *mut Context, data: u64);
    
    /// Get the global context pointer (for ISR use)
    pub fn global_context_ptr() -> *mut GlobalContext;
}

/// Context extension trait for safe platform data management
pub trait ContextExt {
    /// Set platform-specific data in the context
    unsafe fn set_platform_data(&mut self, data: *mut c_void);
    
    /// Get platform-specific data from the context
    unsafe fn get_platform_data(&self) -> *mut c_void;
    
    /// Set user data (for storing Erlang terms as raw u64)
    unsafe fn set_user_data(&mut self, data: u64);
    
    /// Get user data
    unsafe fn get_user_data(&self) -> u64;
    
    /// Safely cast platform data to a specific type
    unsafe fn get_platform_data_as<T>(&self) -> *mut T {
        self.get_platform_data() as *mut T
    }
    
    /// Safely set platform data from a boxed value
    unsafe fn set_platform_data_box<T>(&mut self, data: Box<T>) {
        self.set_platform_data(Box::into_raw(data) as *mut c_void);
    }
    
    /// Safely take ownership of platform data back as a box
    unsafe fn take_platform_data_box<T>(&mut self) -> Option<Box<T>> {
        let ptr = self.get_platform_data() as *mut T;
        if ptr.is_null() {
            None
        } else {
            self.set_platform_data(core::ptr::null_mut());
            Some(Box::from_raw(ptr))
        }
    }
    
    /// Set user data from a Term
    unsafe fn set_user_term(&mut self, term: Term) {
        self.set_user_data(term.raw().try_into().unwrap());
    }
    
    /// Get user data as a Term
    unsafe fn get_user_term(&self) -> Term {
        Term::from_raw(self.get_user_data().try_into().unwrap())
    }
    
    /// Check if platform data is set
    fn has_platform_data(&self) -> bool {
        unsafe { !self.get_platform_data().is_null() }
    }
    
    /// Check if user data is set
    fn has_user_data(&self) -> bool {
        unsafe { self.get_user_data() != 0 }
    }
}

impl ContextExt for Context {
    unsafe fn set_platform_data(&mut self, data: *mut c_void) {
        context_set_platform_data(self, data);
    }
    
    unsafe fn get_platform_data(&self) -> *mut c_void {
        context_get_platform_data(self)
    }
    
    unsafe fn set_user_data(&mut self, data: u64) {
        context_set_user_data(self, data);
    }
    
    unsafe fn get_user_data(&self) -> u64 {
        context_get_user_data(self)
    }
}

/// Safe wrapper for creating port contexts
pub fn create_port_context_safe(global: &GlobalContext) -> *mut Context {
    unsafe { create_port_context(global as *const GlobalContext) }
}

/// Safe wrapper for destroying port contexts
pub fn destroy_port_context_safe(ctx: *mut Context) {
    if !ctx.is_null() {
        unsafe { destroy_port_context(ctx) }
    }
}

/// Check if a port is still alive
pub fn is_port_alive(ctx: &Context) -> bool {
    unsafe { port_is_alive(ctx as *const Context) != 0 }
}

/// Get the global context for ISR use
pub fn get_global_context() -> *mut GlobalContext {
    unsafe { global_context_ptr() }
}

/// Port builder for ergonomic port creation
pub struct PortBuilder<T> {
    data: T,
}

impl<T> PortBuilder<T> {
    /// Create a new port builder with the given data
    pub fn new(data: T) -> Self {
        Self { data }
    }
    
    /// Build the port context with the data
    pub fn build(self, global: &GlobalContext) -> *mut Context {
        let ctx = create_port_context_safe(global);
        if !ctx.is_null() {
            unsafe {
                let boxed_data = Box::new(self.data);
                (*ctx).set_platform_data_box(boxed_data);
            }
        }
        ctx
    }
    
    /// Build the port context and also set user data
    pub fn build_with_user_data(self, global: &GlobalContext, user_data: u64) -> *mut Context {
        let ctx = self.build(global);
        if !ctx.is_null() {
            unsafe {
                (*ctx).set_user_data(user_data);
            }
        }
        ctx
    }
    
    /// Build the port context and also set user term
    pub fn build_with_user_term(self, global: &GlobalContext, user_term: Term) -> *mut Context {
        let ctx = self.build(global);
        if !ctx.is_null() {
            unsafe {
                (*ctx).set_user_term(user_term);
            }
        }
        ctx
    }
}

/// RAII wrapper for automatic context cleanup
pub struct ContextGuard {
    ctx: *mut Context,
}

impl ContextGuard {
    /// Create a new context guard
    /// 
    /// # Safety
    /// The caller must ensure the context pointer is valid
    pub unsafe fn new(ctx: *mut Context) -> Self {
        Self { ctx }
    }
    
    /// Get a reference to the context
    pub fn context(&self) -> &Context {
        unsafe { &*self.ctx }
    }
    
    /// Get a mutable reference to the context
    pub fn context_mut(&mut self) -> &mut Context {
        unsafe { &mut *self.ctx }
    }
    
    /// Release the context without destroying it
    pub fn release(mut self) -> *mut Context {
        let ctx = self.ctx;
        self.ctx = core::ptr::null_mut();
        ctx
    }
    
    /// Check if the guard holds a valid context
    pub fn is_valid(&self) -> bool {
        !self.ctx.is_null()
    }
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        destroy_port_context_safe(self.ctx);
    }
}

/// Context manager for handling multiple contexts
pub struct ContextManager {
    contexts: alloc::vec::Vec<*mut Context>,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new() -> Self {
        Self {
            contexts: alloc::vec::Vec::new(),
        }
    }
    
    /// Add a context to be managed
    pub fn add_context(&mut self, ctx: *mut Context) {
        if !ctx.is_null() {
            self.contexts.push(ctx);
        }
    }
    
    /// Remove a context from management (doesn't destroy it)
    pub fn remove_context(&mut self, ctx: *mut Context) -> bool {
        if let Some(pos) = self.contexts.iter().position(|&x| x == ctx) {
            self.contexts.remove(pos);
            true
        } else {
            false
        }
    }
    
    /// Get the number of managed contexts
    pub fn count(&self) -> usize {
        self.contexts.len()
    }
    
    /// Check if a context is being managed
    pub fn contains(&self, ctx: *mut Context) -> bool {
        self.contexts.contains(&ctx)
    }
    
    /// Destroy all managed contexts
    pub fn destroy_all(&mut self) {
        for &ctx in &self.contexts {
            destroy_port_context_safe(ctx);
        }
        self.contexts.clear();
    }
}

impl Drop for ContextManager {
    fn drop(&mut self) {
        self.destroy_all();
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can be stored as platform data
pub trait PlatformData: Sized {
    /// Called when the platform data is being cleaned up
    fn cleanup(&mut self) {}
    
    /// Store this data in a context
    unsafe fn store_in_context(self, ctx: &mut Context) {
        ctx.set_platform_data_box(Box::new(self));
    }
    
    /// Retrieve this data from a context
    unsafe fn from_context(ctx: &Context) -> Option<&Self> {
        let ptr = ctx.get_platform_data_as::<Self>();
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    }
    
    /// Retrieve this data mutably from a context
    unsafe fn from_context_mut(ctx: &mut Context) -> Option<&mut Self> {
        let ptr = ctx.get_platform_data_as::<Self>();
        if ptr.is_null() {
            None
        } else {
            Some(&mut *ptr)
        }
    }
    
    /// Take ownership of this data from a context
    unsafe fn take_from_context(ctx: &mut Context) -> Option<Self> {
        ctx.take_platform_data_box::<Self>().map(|boxed| *boxed)
    }
}

/// Macro for implementing PlatformData with custom cleanup
#[macro_export]
macro_rules! impl_platform_data {
    ($type:ty) => {
        impl $crate::context::PlatformData for $type {}
    };
    ($type:ty, cleanup = $cleanup:expr) => {
        impl $crate::context::PlatformData for $type {
            fn cleanup(&mut self) {
                $cleanup(self)
            }
        }
    };
}

/// Helper functions for common context operations

/// Safely execute a function with platform data
pub fn with_platform_data<T, R, F>(ctx: &Context, f: F) -> Option<R>
where
    T: PlatformData,
    F: FnOnce(&T) -> R,
{
    unsafe {
        T::from_context(ctx).map(f)
    }
}

/// Safely execute a function with mutable platform data
pub fn with_platform_data_mut<T, R, F>(ctx: &mut Context, f: F) -> Option<R>
where
    T: PlatformData,
    F: FnOnce(&mut T) -> R,
{
    unsafe {
        T::from_context_mut(ctx).map(f)
    }
}

/// Initialize platform data in a context
pub fn init_platform_data<T: PlatformData>(ctx: &mut Context, data: T) {
    unsafe {
        data.store_in_context(ctx);
    }
}

/// Clean up platform data from a context
pub fn cleanup_platform_data<T: PlatformData>(ctx: &mut Context) -> Option<T> {
    unsafe {
        T::take_from_context(ctx)
    }
}
