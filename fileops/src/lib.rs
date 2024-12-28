//! File operations utilities.
//!
//! This module provides utilities for file operations, such as copying, moving, deleting, etc.
//! with features like progress tracking, directory statistics, etc.
//!
//! # Crate features
//! - `loom`---enables Loom concurrency permutation testing tool. Required for some tests.
//!
//! ## Filesystem backend
//! Currently only one backend is supported:
//! - `async-fs`---uses `async-fs` crate for async file operations. (default)

pub mod dirstats;
mod error;
mod pathext;
pub use error::{Error, Result};
pub use pathext::PathExt;
pub mod progress;

pub mod file_size;
pub mod fraction;
pub mod prelude;
#[cfg(windows)]
pub mod volume;
