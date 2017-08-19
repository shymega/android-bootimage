extern crate byteorder;
#[macro_use]
extern crate quick_error;

mod header;
mod image;

pub use header::{HEADER_SIZE, Header};
pub use image::{BadHeaderError, BootImage, ReadBootImageError};
