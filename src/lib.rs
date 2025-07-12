#![no_std]

extern crate alloc;

mod log;
mod term;
mod context;
mod registry;

pub use context::Context;
pub use term::{Term, NifResult};
pub use crate::log::log_info;

#[cfg(test)]
mod tests;
