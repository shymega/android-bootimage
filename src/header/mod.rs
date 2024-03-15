use core::{
    fmt::Debug,
    hash::Hasher,
    result::Result
};
use core2::io::{Error, Read, Write};

use alloc::boxed::Box;

pub mod consts;
mod samsung_header;
mod android_header;

pub use self::samsung_header::SamsungHeader;
pub use self::android_header::*;

#[derive(Debug, Default)]
pub enum HeaderKind {
    AospHeaderv0(Box<dyn AndroidHeaderTrait>),
    AospHeaderv1(Box<dyn AndroidHeaderTrait>),
    AospHeaderv2(Box<dyn AndroidHeaderTrait>),
    AospHeaderv3(Box<dyn AndroidHeaderTrait>),
    AospHeaderv4(Box<dyn AndroidHeaderTrait>),
    SamsungHeader(Box<dyn SamsungHeaderTrait>),
    #[default]
    Undefined,
}

pub trait HeaderTrait {
    fn get_header_size(&self) -> usize;
    fn get_magic_size(&self) -> usize;
    fn has_correct_magic(&self) -> bool;
    fn read_from<R>(src: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
        R: Read;
    fn write_to<W>(&self, dst: &mut W) -> Result<usize, Error>
    where
        Self: Sized,
        W: Write + Hasher;
}

impl Debug for dyn HeaderTrait {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HeaderTrait")
    }
}

pub trait SamsungHeaderTrait: HeaderTrait {

    fn parse(src: &[u8; consts::samsung::SamsungConsts::SAMSUNG_HEADER_SIZE]) -> Self
    where
        Self: Sized;
}

impl Debug for dyn SamsungHeaderTrait {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SamsungHeaderTrait")
    }
}

pub trait AndroidHeaderTrait: HeaderTrait {
}

impl Debug for dyn AndroidHeaderTrait {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "AndroidHeaderTrait")
    }
}
