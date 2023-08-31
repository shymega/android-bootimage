use core::{hash::Hasher, result::Result};
use core2::io::{Error, Read, Write};

pub mod consts;
mod samsung_header;
mod android_header;

pub use self::samsung_header::SamsungHeader;
pub use self::android_header::*;

#[derive(Debug, Default)]
pub enum HeaderKind {
    AospHdr0(AospHeader0),
    AospHdr1(AospHeader1),
    AospHdr2(AospHeader2),
    AospHdr3(AospHeader3),
    AospHdr4(AospHeader4),
    Samsung(SamsungHeader),
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

pub trait SamsungHeaderTrait: HeaderTrait {

    fn parse(src: &[u8; consts::samsung::SAMSUNG_HEADER_SIZE]) -> Self
    where
        Self: Sized;
}
