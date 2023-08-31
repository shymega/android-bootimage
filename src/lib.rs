//! Lightweight library for handling Android Boot Images (including Samsung!)
#![no_std]
#![deny(
    warnings,
    missing_copy_implementations,
    unused_imports,
    missing_debug_implementations,
    missing_docs,
    clippy::all,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    variant_size_differences
)]

extern crate alloc;
extern crate byteorder;
extern crate core2;

/* mod errors; */
pub mod header;
pub mod image;
