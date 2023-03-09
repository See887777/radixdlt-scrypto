#![cfg_attr(not(feature = "std"), no_std)]

// Jemalloc disabled for wasmer builds due to issue with handling of stack overflow.
#[cfg(not(feature = "wasmer"))]
use tikv_jemallocator::Jemalloc;
#[cfg(not(feature = "wasmer"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

extern crate core;
#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!("Either feature `std` or `alloc` must be enabled for this crate.");
#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!("Feature `std` and `alloc` can't be enabled at the same time.");

/// Radix Engine kernel, defining state, ownership and (low-level) invocation semantics.
pub mod kernel;
/// Radix Engine ledge state abstraction.
pub mod ledger;
/// Radix Engine system, defining packages (a.k.a. classes), components (a.k.a. objects) and invocation semantics.
pub mod system;
/// Radix Engine transaction interface.
pub mod transaction;

/// Native blueprints (to be moved to individual crates)
pub mod blueprints;

/// State manager for the Radix Engine
pub mod state_manager;

/// Wasm validation, instrumentation and execution.
pub mod wasm;

/// Scrypto/SBOR types required by Radix Engine.
pub mod types;

pub mod errors;
