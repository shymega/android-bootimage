use Section;
use byteorder::{LittleEndian, ReadBytesExt};
use quick_error::ResultExt;
use std::io::{Error as IoError, Read, Seek, SeekFrom};

pub const MAGIC_STR: &'static str = "ANDROID!";
const MAGIC_SIZE: usize = 8;
const PRODUCT_NAME_SIZE: usize = 24;
const BOOT_ARGUMENTS_SIZE: usize = 512;
const UNIQUE_ID_SIZE: usize = 32;
const HEADER_SIZE: usize = 616;

/// The different sections in a Samsung boot image, in order.
const SECTIONS: &'static [Section] = &[
    Section::Header,
    Section::Kernel,
    Section::Ramdisk,
    Section::Second,
    Section::DeviceTree,
];

/// Contains a magic header.
#[derive(Debug)]
pub struct MagicHeader {
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

impl MagicHeader {
    /// Reads a magic header from the supplied source.
    pub fn read_from<R: ReadBytesExt>(
        source: &mut R,
        check_magic: bool,
    ) -> Result<Self, MagicHeaderParseError> {
        let header = MagicHeader {
            magic: {
                let mut buffer = [0; MAGIC_SIZE];
                source.read_exact(&mut buffer)?;
                buffer
            },
            kernel_size: source.read_u32::<LittleEndian>()?,
            kernel_load_address: source.read_u32::<LittleEndian>()?,
            ramdisk_size: source.read_u32::<LittleEndian>()?,
            ramdisk_load_address: source.read_u32::<LittleEndian>()?,
            second_size: source.read_u32::<LittleEndian>()?,
            second_load_address: source.read_u32::<LittleEndian>()?,
            device_tree_size: source.read_u32::<LittleEndian>()?,
            _reserved: source.read_u32::<LittleEndian>()?,
            kernel_tags_address: source.read_u32::<LittleEndian>()?,
            page_size: source.read_u32::<LittleEndian>()?,
            product_name: {
                let mut buffer = [0; PRODUCT_NAME_SIZE];
                source.read_exact(&mut buffer)?;
                buffer
            },
            boot_arguments: unsafe {
                use std::mem::transmute;
                let mut buffer = [0; BOOT_ARGUMENTS_SIZE];
                source.read_exact(&mut buffer)?;
                transmute(buffer)
            },
            unique_id: {
                let mut buffer = [0u8; UNIQUE_ID_SIZE];
                source.read_exact(&mut buffer)?;
                buffer
            },
        };

        if check_magic && header.magic != MAGIC_STR.as_bytes() {
            Err(MagicHeaderParseError::InvalidMagic(header))
        } else {
            Ok(header)
        }
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

quick_error! {
    #[derive(Debug)]
    pub enum MagicHeaderParseError {
        Io(error: IoError) {
            description("I/O error while parsing header")
            display("I/O error while parsing header.")
            cause(error)
            from(error: IoError) -> (error)
        }
        InvalidMagic(header: MagicHeader) {
            description("The header did not have the valid magic prefix")
            display("The header did not have the valid '{}' magic prefix.", MAGIC_STR)
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