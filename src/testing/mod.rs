//! Testing utilities and mock implementations for avmnif-rs
//! 
//! This module provides centralized testing infrastructure including:
//! - Mock implementations of AtomVM components
//! - Test helpers and utilities
//! - Common test fixtures and data
//! 
//! All code in this module is conditionally compiled only for tests.

#[cfg(test)]
pub mod mocks;

#[cfg(test)]
pub mod helpers;

#[cfg(test)]
pub mod fixtures;

#[cfg(test)]
pub mod nifs;

#[cfg(test)]
pub mod resources;

#[cfg(test)]
pub mod tagged;

#[cfg(test)]
pub mod ports;

// Re-export everything for convenient imports
#[cfg(test)]
pub use mocks::*;

#[cfg(test)]
pub use helpers::*;

#[cfg(test)]
pub use fixtures::*;