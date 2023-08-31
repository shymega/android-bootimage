pub use self::android::*;

pub mod samsung {
        pub const SAMSUNG_HEADER_SIZE: usize = 616;
        pub const SAMSUNG_MAGIC: [u8; SAMSUNG_MAGIC_SIZE] =
            [0x41, 0x4E, 0x44, 0x52, 0x4F, 0x49, 0x44, 0x21];
        pub const SAMSUNG_MAGIC_STR: &'static str = "ANDROID!";
        pub const SAMSUNG_MAGIC_SIZE: usize = 8;
        pub const PRODUCT_NAME_SIZE: usize = 24;
        pub const BOOT_ARGUMENTS_SIZE: usize = 512;
        pub const UNIQUE_ID_SIZE: usize = 32;
}

mod android {
    pub mod aosp_header_0 {}

    pub mod aosp_header_1 {}

    pub mod aosp_header_2 {}

    pub mod aosp_header_3 {}

    pub mod aosp_header_4 {}
}
