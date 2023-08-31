
use crate::header::{HeaderTrait, SamsungHeaderTrait};
use byteorder::{ByteOrder, LittleEndian};
use core::hash::Hasher;
use core::mem::transmute;
use super::consts::samsung::*;
use core2::io::{Error as IoError, Read, Write};

/// Contains a magic header.
#[derive(Debug, Clone, Copy)]
pub struct SamsungHeader {
    /// Header magic. Used to make sure this is in fact a header.
    pub magic: [u8; SAMSUNG_MAGIC_SIZE],
    /// Ramdisk size, in bytes.
    pub kernel_size: u32,
    /// Address the ramdisk should be loaded to.
    pub kernel_load_address: u32,
    /// Ramdisk size, in bytes.
    pub ramdisk_size: u32,
    /// Address the ramdisk should be loaded to.
    pub ramdisk_load_address: u32,
    /// Size of an optional second file.
    pub second_size: u32,
    /// Address the optional second file should be loaded to.
    pub second_load_address: u32,
    /// The size of the device tree, in bytes.
    pub device_tree_size: u32,
    /// Room for future expansion. This should always be set to 0.
    _reserved: u32,
    /// Physical address of the kernel tags.
    pub kernel_tags_address: u32,
    /// The page size.
    pub page_size: u32,
    /// Name of the product. This is a null-terminated ASCII string.
    pub product_name: [u8; PRODUCT_NAME_SIZE],
    /// Arguments to pass to the kernel during boot. This is a nested array, as
    /// rust does not allow us to have arrays larger than 32 in size.
    pub boot_arguments: [[u8; BOOT_ARGUMENTS_SIZE / 16]; 16],
    /// Used to uniquely identify boot images.
    pub unique_id: [u8; UNIQUE_ID_SIZE],
}

impl HeaderTrait for SamsungHeader {
    fn get_header_size(&self) -> usize {
        SAMSUNG_HEADER_SIZE
    }

    fn get_magic_size(&self) -> usize {
        self.magic.len()
    }

    fn has_correct_magic(&self) -> bool {
        self.magic == SAMSUNG_MAGIC_STR.as_bytes()
    }

    fn read_from<R>(src: &mut R) -> Result<Self, IoError>
    where
        Self: Sized,
        R: Read,
    {
        let mut buffer = [0; SAMSUNG_HEADER_SIZE];
        src.read_exact(&mut buffer)?;
        Ok(Self::parse(&buffer))
    }

    fn write_to<W>(&self, dst: &mut W) -> Result<usize, IoError>
    where
        Self: Sized,
        W: Write + Hasher,
    {
        dst.write_all(&self.magic)?;
        dst.write_u32(self.kernel_size);
        dst.write_u32(self.kernel_load_address);
        dst.write_u32(self.ramdisk_size);
        dst.write_u32(self.ramdisk_load_address);
        dst.write_u32(self.second_size);
        dst.write_u32(self.second_load_address);
        dst.write_u32(self.device_tree_size);
        dst.write_u32(self._reserved);
        dst.write_u32(self.kernel_tags_address);
        dst.write_u32(self.page_size);
        dst.write_all(&self.product_name)?;
        for ii in self.boot_arguments.iter() {
            dst.write_all(ii)?;
        }
        dst.write_all(&self.unique_id)?;
        Ok(SAMSUNG_HEADER_SIZE)
    }
}

impl SamsungHeaderTrait for SamsungHeader {
    /// Reads a header from the supplied source. This does not perform the
    /// magic check, and as a result cannot error.
    fn parse(src: &[u8; SAMSUNG_HEADER_SIZE]) -> Self
    where
        Self: Sized,
    {
        let mut src = &src[..];

        Self {
            magic: {
                let mut buffer = [0; SAMSUNG_MAGIC_SIZE];
                src.read_exact(&mut buffer).unwrap();
                buffer
            },
            kernel_size: LittleEndian::read_u32(&src),
            kernel_load_address: LittleEndian::read_u32(&src),
            ramdisk_size: LittleEndian::read_u32(&src),
            ramdisk_load_address: LittleEndian::read_u32(&src),
            second_size: LittleEndian::read_u32(&src),
            second_load_address: LittleEndian::read_u32(&src),
            device_tree_size: LittleEndian::read_u32(&src),
            _reserved: LittleEndian::read_u32(&src),
            kernel_tags_address: LittleEndian::read_u32(&src),
            page_size: LittleEndian::read_u32(&src),
            product_name: {
                let mut buffer = [0; PRODUCT_NAME_SIZE];
                src.read_exact(&mut buffer).unwrap();
                buffer
            },
            boot_arguments: unsafe {
                let mut buffer = [0; BOOT_ARGUMENTS_SIZE];
                src.read_exact(&mut buffer).unwrap();
                transmute(buffer)
            },
            unique_id: {
                let mut buffer = [0u8; UNIQUE_ID_SIZE];
                src.read_exact(&mut buffer).unwrap();
                buffer
            },
        }
    }
}

impl Default for SamsungHeader {
    fn default() -> Self {
        Self {
            magic: SAMSUNG_MAGIC,
            kernel_size: 0,
            kernel_load_address: 0x10008000,
            ramdisk_size: 0,
            ramdisk_load_address: 0x11000000,
            second_size: 0,
            second_load_address: 0x100f0000,
            device_tree_size: 0,
            _reserved: 0x02000000,
            kernel_tags_address: 0x10000100,
            page_size: 2048,
            product_name: [0; PRODUCT_NAME_SIZE],
            boot_arguments: [[0; 32]; 16],
            unique_id: [0; UNIQUE_ID_SIZE],
        }
    }
}
