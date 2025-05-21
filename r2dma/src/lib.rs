//! r2dma
//!
//! A Rust RDMA library.
pub mod verbs;

mod core;
pub use core::*;

mod buf;
pub use buf::*;

mod error;
pub use error::*;
