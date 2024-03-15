use core2::io::Error as IoError;
use thiserror_no_std::Error;
use alloc::boxed::Box;
use header::SamsungHeader;


#[derive(Debug, Error)]
pub enum BadHeaderError {
    #[error("No Page Size specified.")]
    NoPageSize(Box<SamsungHeader>),
    #[error("Bad magic number.")]
    BadMagic(Box<SamsungHeader>),
}

#[derive(Debug, Error)]
pub enum ReadBootImageError {
    #[error("IO error whilst reading boot image from Reader.")]
    Io(#[from] IoError),
    #[error("Bad Header read into memory.")]
    BadHeader(Box<SamsungHeader>),
}
