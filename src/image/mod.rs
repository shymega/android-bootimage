mod android;
mod samsung;

use alloc::vec::Vec;
use super::header::HeaderKind;
use core2::io::{Read, Seek, Write, Error as IoError};


pub trait BootImage {
    fn insert_header(&mut self, kind: HeaderKind) -> Result<HeaderKind, ()>;
    fn insert_kernel(&mut self, replacement: Vec<u8>) -> Vec<u8>;
    fn insert_ramdisk(&mut self, replacement: Vec<u8>) -> Vec<u8>;
    fn insert_second_ramdisk(&mut self, replacement: Vec<u8>) -> Vec<u8>;
    fn insert_device_tree(&mut self, replacement: Vec<u8>) -> Vec<u8>;
    fn update_all_sizes(&mut self);
    fn get_page_size(&self) -> usize;
    fn get_kernel(&self) -> &[u8];
    fn get_ramdisk(&self) -> &[u8];
    fn get_second_ramdisk(&self) -> &[u8];
    fn get_device_tree(&self) -> &[u8];
    fn read_from<R>(src: &mut R, page_size: Option<u32>) -> Result<Self, IoError>
    where
        Self: Sized,
        R: Read + Seek;
    fn write_all_to<W>(&self, dst: &mut W) -> Result<usize, IoError>
    where
        W: Write;
    fn write_header_to<W>(&self, dst: &mut W) -> Result<usize, IoError>
    where
        W: Write;
    fn write_kernel_to<W>(&self, dst: &mut W) -> Result<usize, IoError>
    where
        W: Write;
    fn write_ramdisk_to<W>(&self, dst: &mut W) -> Result<usize, IoError>
    where
        W: Write;
    fn write_second_ramdisk_to<W>(&self, dst: &mut W) -> Result<usize, IoError>
    where
        W: Write;
    fn write_device_tree_to<W>(&self, dst: &mut W) -> Result<usize, IoError>
    where
        W: Write;
}
