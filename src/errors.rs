/* use core2::io::Error as IoError; */

#[derive(Debug)]
pub enum BadHeaderError {
    NoPageSize,
    BadMagic,
}

#[derive(Debug)]
pub enum ReadBootImageError {
    Io,
    BadHeader,
}
