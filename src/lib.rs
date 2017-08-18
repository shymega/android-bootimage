extern crate byteorder;
#[macro_use]
extern crate quick_error;

mod header;
mod image;

pub use header::{Header, LocateSectionError, HEADER_SIZE};
pub use image::BootImage;
use std::fmt;

/// Enum representing a single section in a boot image.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Section {
    Header,
    Kernel,
    Ramdisk,
    Second,
    DeviceTree,
}

impl fmt::Display for Section {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{}",
            match *self {
                Section::Header => "header",
                Section::Kernel => "kernel",
                Section::Ramdisk => "ramdisk",
                Section::Second => "second",
                Section::DeviceTree => "device_tree",
            }
        )
    }
}
