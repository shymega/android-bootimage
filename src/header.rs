use Section;
use byteorder::{LittleEndian, ReadBytesExt};
use quick_error::ResultExt;
use std::io::{Error as IoError, Read, Seek, SeekFrom};

/// The size of the header, in bytes. This might not match up with the
/// amount of bytes the structure consumes while in memory.
pub const HEADER_SIZE: usize = 616;
const MAGIC: [u8; MAGIC_SIZE] = [0x41, 0x4E, 0x44, 0x52, 0x4F, 0x49, 0x44, 0x21];
pub const MAGIC_STR: &'static str = "ANDROID!";
const MAGIC_SIZE: usize = 8;
const PRODUCT_NAME_SIZE: usize = 24;
const BOOT_ARGUMENTS_SIZE: usize = 512;
const UNIQUE_ID_SIZE: usize = 32;

/// The different sections in a Samsung boot image, in order.
const SECTIONS: &'static [Section] = &[
    Section::Header,
    Section::Kernel,
    Section::Ramdisk,
    Section::Second,
    Section::DeviceTree,
];

/// Contains a magic header.
#[derive(Debug, Clone)]
pub struct Header {
    /// Header magic. Used to make sure this is in fact a header.
    pub magic: [u8; MAGIC_SIZE],
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

impl Header {
    /// Reads a header from the supplied source. This does not perform the
    /// magic check, and as a result cannot error.
    pub fn parse(source: &[u8; HEADER_SIZE]) -> Self {
        let mut source = &source[..];

        Header {
            magic: {
                let mut buffer = [0; MAGIC_SIZE];
                source.read_exact(&mut buffer).unwrap();
                buffer
            },
            kernel_size: source.read_u32::<LittleEndian>().unwrap(),
            kernel_load_address: source.read_u32::<LittleEndian>().unwrap(),
            ramdisk_size: source.read_u32::<LittleEndian>().unwrap(),
            ramdisk_load_address: source.read_u32::<LittleEndian>().unwrap(),
            second_size: source.read_u32::<LittleEndian>().unwrap(),
            second_load_address: source.read_u32::<LittleEndian>().unwrap(),
            device_tree_size: source.read_u32::<LittleEndian>().unwrap(),
            _reserved: source.read_u32::<LittleEndian>().unwrap(),
            kernel_tags_address: source.read_u32::<LittleEndian>().unwrap(),
            page_size: source.read_u32::<LittleEndian>().unwrap(),
            product_name: {
                let mut buffer = [0; PRODUCT_NAME_SIZE];
                source.read_exact(&mut buffer).unwrap();
                buffer
            },
            boot_arguments: unsafe {
                use std::mem::transmute;
                let mut buffer = [0; BOOT_ARGUMENTS_SIZE];
                source.read_exact(&mut buffer).unwrap();
                transmute(buffer)
            },
            unique_id: {
                let mut buffer = [0u8; UNIQUE_ID_SIZE];
                source.read_exact(&mut buffer).unwrap();
                buffer
            },
        }
    }

    pub fn read_from<R: Read>(source: &mut R) -> Result<Self, IoError> {
        let mut buffer = [0; HEADER_SIZE];
        source.read_exact(&mut buffer)?;
        Ok(Header::parse(&buffer))
    }

    pub fn correct_magic(&self) -> bool {
        self.magic == MAGIC_STR.as_bytes()
    }

    /// Creates a buffer useful for storing header data in.
    pub fn create_buffer() -> Box<[u8; HEADER_SIZE]> {
        Box::new(unsafe { ::std::mem::uninitialized() })
    }

    /// Returns the size of a section, in bytes.
    pub fn section_size(&self, section: Section) -> u64 {
        match section {
            Section::Header => HEADER_SIZE as u64,
            Section::Kernel => self.kernel_size as u64,
            Section::Ramdisk => self.ramdisk_size as u64,
            Section::Second => self.second_size as u64,
            Section::DeviceTree => self.device_tree_size as u64,
        }
    }

    /// Returns the start location of a section, in bytes.
    ///
    /// Do note that this function can fail because it cannot find a section
    /// other than the one that was requested. Sections depend on the other
    /// sections for their locations.
    pub fn section_start(&self, section: Section) -> Result<u64, LocateSectionError> {
        if self.page_size == 0 {
            Err(LocateSectionError::NoPageSize)
        } else {
            let offset_in_pages: u64 = SECTIONS
                .iter()
                .cloned()
                // Take every section that comes before the one we want to get the offset for.
                .take_while(|&i_section| i_section != section)
                // For every of these sections, calculate the amount of pages it occupies.
                .map(|section| {
                    (self.section_size(section) + self.page_size as u64 - 1) / self.page_size as u64
                })
                // Calculate how much pages all of these pages together occupy.
                .sum();

            // Multiply with the size of a page to get the offset in bytes.
            Ok(offset_in_pages * self.page_size as u64)
        }
    }

    /// Returns the start and the end location of a section, in bytes.
    ///
    /// Do note that this function can fail because it cannot find a section
    /// other than the one that was requested. Sections depend on the other
    /// sections for their locations.
    pub fn section_location(&self, section: Section) -> Result<(u64, u64), LocateSectionError> {
        Ok((self.section_start(section)?, self.section_size(section)))
    }

    /// Reads a section from the given readable resource.
    ///
    /// Do note that this function can fail because it cannot find a section
    /// other than the one that was requested. Sections depend on the other
    /// sections for their locations.
    pub fn read_section_from<R: Read + Seek>(
        &self,
        source: &mut R,
        section: Section,
    ) -> Result<Vec<u8>, ReadSectionError> {
        let (start, size) = self.section_location(section).context(section)?;
        try!(source.seek(SeekFrom::Start(start)).context(section));
        let mut data = vec![0u8; size as usize];
        try!(source.read_exact(&mut data).context(section));
        return Ok(data);
    }

    /// Returns the sections in this boot image, in order. Zero-size sections
    /// are omitted.
    pub fn sections(&self) -> Vec<Section> {
        SECTIONS
            .iter()
            .filter(|&&section| self.section_size(section) > 0)
            .cloned()
            .collect()
    }
}

impl Default for Header {
    fn default() -> Header {
        Header {
            magic: MAGIC,
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

quick_error! {
    #[derive(Debug)]
    pub enum LocateSectionError {
        NoPageSize {
            description("The header's page size is zero")
            display("The header's page size is 0.")
        }
        NoSection(section: Section) {
            description("The section does not exist")
            display("The '{}' section does not exist.", section)
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum ReadSectionError {
        LocateSection(section: Section, cause: LocateSectionError) {
            context(section: Section, cause: LocateSectionError) -> (section, cause)
            description("Could not locate the section")
            display("Cannot locate the '{}' section.", section)
            cause(cause)
        }
        IoError(section: Section, cause: IoError) {
            context(section: Section, cause: IoError) -> (section, cause)
            description("I/O error while reading the section")
            display("I/O error while reading the '{}' section.", section)
            cause(cause)
        }
    }
}
