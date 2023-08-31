use core::{hash::Hasher, result::Result};
use core2::io::{Error, Read, Write};

mod consts;
mod samsung_header;

use self::consts::samsung;
use self::samsung_header::SamsungHeader;

#[derive(Debug, Default)]
pub enum HeaderKind {
    /* Disabled, temporarily.
    * Aosp0(AospHeader0),
    Aosp1(AospHeader1),
    Aosp2(AospHeader2),
    Aosp3(AospHeader3),
    Aosp4(AospHeader4), */
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
    fn write<W>(&self, dst: &mut W) -> Result<usize, Error>
    where
        Self: Sized,
        W: Write + Hasher;
}

pub trait SamsungHeaderTrait: HeaderTrait {
    fn parse(src: &[u8; samsung::SAMSUNG_HEADER_SIZE]) -> Self
    where
        Self: Sized;
}
