use Header;
use std::io::{Error as IoError, Read, Seek};

/// A structure representing a boot image in memory. Used to modify the boot
/// image through a convenient interface.
pub struct BootImage {
    /// The header of this boot image.
    header: Header,
    /// The kernel.
    kernel: Vec<u8>,
    /// The ramdisk.
    ramdisk: Vec<u8>,
    /// The second ramdisk.
    second_ramdisk: Vec<u8>,
    /// The device tree.
    device_tree: Vec<u8>,
}

impl BootImage {
    /// Inserts a new header into this boot image. The sizes of the different
    /// sections (kernel, ramdisk, ...) will be updated with the ones in this
    /// boot image.
    ///
    /// This function fails when the header does not have the valid magic, or
    /// when its page size is set to 0.
    ///
    /// Returns the old header on success.
    pub fn insert_header(&mut self, mut new_header: Header) -> Result<Header, BadHeaderError> {
        if !new_header.correct_magic() {
            Err(BadHeaderError::BadMagic(new_header))
        } else if new_header.page_size == 0 {
            Err(BadHeaderError::NoPageSize(new_header))
        } else {
            ::std::mem::swap(&mut self.header, &mut new_header);
            self.update_all_sizes();
            Ok(new_header)
        }
    }

    /// Inserts a kernel into this boot image, returning the old one.
    pub fn insert_kernel(&mut self, mut new_kernel: Vec<u8>) -> Vec<u8> {
        self.header.kernel_size = new_kernel.len() as u32;
        ::std::mem::swap(&mut self.kernel, &mut new_kernel);
        new_kernel
    }

    /// Inserts a ramdisk into this boot image, returning the old one.
    pub fn insert_ramdisk(&mut self, mut new_ramdisk: Vec<u8>) -> Vec<u8> {
        self.header.ramdisk_size = new_ramdisk.len() as u32;
        ::std::mem::swap(&mut self.ramdisk, &mut new_ramdisk);
        new_ramdisk
    }

    /// Inserts a second ramdisk into this boot image, returning the old one.
    pub fn insert_second(&mut self, mut new_second_ramdisk: Vec<u8>) -> Vec<u8> {
        self.header.second_size = new_second_ramdisk.len() as u32;
        ::std::mem::swap(&mut self.second_ramdisk, &mut new_second_ramdisk);
        new_second_ramdisk
    }

    /// Inserts a device tree into this boot image, returning the old one.
    pub fn insert_device_tree(&mut self, mut new_device_tree: Vec<u8>) -> Vec<u8> {
        self.header.device_tree_size = new_device_tree.len() as u32;
        ::std::mem::swap(&mut self.device_tree, &mut new_device_tree);
        new_device_tree
    }

    /// Makes sure all the section sizes in the header are correct.
    fn update_all_sizes(&mut self) {
        self.header.kernel_size = self.kernel.len() as u32;
        self.header.ramdisk_size = self.ramdisk.len() as u32;
        self.header.second_size = self.second_ramdisk.len() as u32;
        self.header.device_tree_size = self.device_tree.len() as u32;
    }

    /// Returns the size of a single page.
    pub fn page_size(&self) -> usize {
        self.header.page_size as usize
    }

    /// Returns a reference to the kernel.
    pub fn kernel(&self) -> &[u8] {
        &self.kernel
    }

    /// Returns a reference to the ramdisk.
    pub fn ramdisk(&self) -> &[u8] {
        &self.ramdisk
    }

    /// Returns a reference to the second ramdisk.
    pub fn second_ramdisk(&self) -> &[u8] {
        &self.second_ramdisk
    }

    /// Returns a reference to the device tree.
    pub fn device_tree(&self) -> &[u8] {
        &self.device_tree
    }

    /// Returns how many pages the header is big.
    pub fn header_size_in_pages(&self) -> usize {
        size_to_size_in_pages(::std::mem::size_of::<Header>(), self.page_size())
    }

    /// Returns how many pages the kernel is big.
    pub fn kernel_size_in_pages(&self) -> usize {
        size_to_size_in_pages(self.kernel.len(), self.page_size())
    }

    /// Returns how many pages the ramdisk is big.
    pub fn ramdisk_size_in_pages(&self) -> usize {
        size_to_size_in_pages(self.ramdisk.len(), self.page_size())
    }

    /// Returns how many pages the second ramdisk is big.
    pub fn second_ramdisk_size_in_pages(&self) -> usize {
        size_to_size_in_pages(self.second_ramdisk.len(), self.page_size())
    }

    /// Returns how many pages the second ramdisk is big.
    pub fn device_tree_size_in_pages(&self) -> usize {
        size_to_size_in_pages(self.device_tree.len(), self.page_size())
    }

    /// Returns the offset to the header, in pages.
    pub fn header_offset_in_pages(&self) -> usize {
        0
    }

    /// Returns the offset to the kernel, in pages.
    pub fn kernel_offset_in_pages(&self) -> usize {
        self.header_offset_in_pages() + self.header_size_in_pages()
    }

    /// Returns the offset to the ramdisk, in pages.
    pub fn ramdisk_offset_in_pages(&self) -> usize {
        self.kernel_offset_in_pages() + self.kernel_size_in_pages()
    }

    /// Returns the offset to the second ramdisk, in pages.
    pub fn second_ramdisk_offset_in_pages(&self) -> usize {
        self.ramdisk_offset_in_pages() + self.ramdisk_size_in_pages()
    }

    /// Returns the offset to the device tree, in pages.
    pub fn device_tree_offset_in_pages(&self) -> usize {
        self.second_ramdisk_offset_in_pages() + self.second_ramdisk_size_in_pages()
    }

    pub fn read_from<R: Read + Seek>(source: &mut R) -> Result<Self, ReadBootImageError> {
        let mut boot_image = BootImage::default();
        let _ = boot_image.insert_header(Header::read_from(source)?)?;
        Ok(boot_image)
    }
}

/// Helper function to calculate how big something would be in pages, given
/// the size and the page size.
fn size_to_size_in_pages(size: usize, page_size: usize) -> usize {
    (size + page_size - 1) / page_size
}

impl Default for BootImage {
    /// Creates a new default boot image, with no sections at all.
    fn default() -> Self {
        BootImage {
            header: Header::default(),
            kernel: Vec::new(),
            ramdisk: Vec::new(),
            second_ramdisk: Vec::new(),
            device_tree: Vec::new(),
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum BadHeaderError {
        NoPageSize(header: Header) {
            description("The header does not have a page size set")
            display("The header does not have a page size set.")
        }
        BadMagic(header: Header) {
            description("The header does not contain the 'ANDROID!' magic")
            display("The header does not contain the 'ANDROID!' magic.")
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum ReadBootImageError {
        Io(cause: IoError) {
            description("An I/O error occured")
            display("An I/O error occured.")
            cause(cause)
            from(cause: IoError) -> (cause)
        }
        BadHeader(cause: BadHeaderError) {
            description("Could not parse image header")
            display("Could not parse the boot image header")
            cause(cause)
            from(cause: BadHeaderError) -> (cause)
        }
    }
}
