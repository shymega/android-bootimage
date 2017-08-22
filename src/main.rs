extern crate android_bootimage;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate clap;
extern crate colored;
extern crate humansize;

use android_bootimage::{BadHeaderError, BootImage, Header, ReadBootImageError};
use clap::{App, Arg, ArgMatches};
use logger::{log_debug, log_error, log_error_cause, log_warning, log_warning_cause};
use quick_error::ResultExt;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

fn main() {
    let result = match create_app().get_matches().subcommand() {
        ("repack", Some(arguments)) => main_repack(arguments),
        _ => panic!("No subcommand was used."),
    };

    if let Err(error) = result {
        use std::error::Error;

        match error.cause() {
            Some(cause) => log_error_cause(format!("{}", error), cause),
            None => log_error(format!("{}", error)),
        }
    }
}

fn create_app() -> App<'static, 'static> {
    use clap::AppSettings;
    App::new("android-bootimage")
        .setting(AppSettings::SubcommandRequired)
        .version(crate_version!())
        .author(crate_authors!())
        .about("Program for handling samsung boot images.")
        .subcommand(create_app_repack())
        .max_term_width(120)
}

fn create_app_repack() -> App<'static, 'static> {
    App::new("repack")
        .about(
            "Handles inserting into and extracting sections out of a boot image.",
        )
        .arg(
            Arg::with_name("list_sections")
                .help("Lists all the sections in the boot image")
                .long_help(
"Lists all the sections in the boot image, providing information including the section name, \
offset and size."
                )
                .short("l")
                .long("list")
                .visible_alias("list-sections")
        )
        .arg(
            Arg::with_name("input_boot_file")
                .long("input-boot-file")
                .visible_alias("ibf")
                .help("Supplies a boot image to extract sections from")
                .long_help(
"Supplies a boot image to extract sections from. If neither this parameter, nor the \
'--input-header-file' parameter is used, a default header will be supplied to the boot image.",
                )
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("input_header_file")
                .help("Supplies a header image to insert into the boot image")
                .long_help(
"Supplies a header image to insert into the boot image. If neither this parameter, nor the \
'--input-boot-file' parameter is used, a default header will be supplied to the boot image.",
                )
                .long("input-header-file")
                .visible_alias("ihf")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("input_kernel_file")
                .long("input-kernel-file")
                .visible_alias("ikf")
                .help("Supplies a kernel image to insert into the boot image")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("input_ramdisk_file")
                .long("input-ramdisk-file")
                .visible_alias("irf")
                .help("Supplies a ramdisk image to insert into the boot image")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("input_second_ramdisk_file")
                .long("input-second-ramdisk-file")
                .visible_alias("isf")
                .help("Supplies a second ramdisk image to insert into the boot image")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("input_device_tree_file")
                .long("input-device-tree-file")
                .visible_alias("idf")
                .help("Supplies a device tree to insert into the boot image")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("input_page_size")
                .long("input-page-size")
                .help("Treat the input boot image as if it had this page size")
                .visible_aliases(&["ip", "ipage"])
                .long_help(
"Treat the input boot image as if it had this page size. This switch is required if the input \
boot image has its page size set to 0.",
                )
                .value_name("INPUT_PAGE_SIZE")
                .requires("input_boot_file")
        )
        .arg(
            Arg::with_name("output_boot_image_file")
                .long("output-boot-image-file")
                .visible_alias("obf")
                .help("Write the boot image to a file")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("output_kernel_file")
                .long("output-kernel-file")
                .visible_alias("okf")
                .default_value_if("output_all_default", None, "boot/header.img")
                .help("Extract the boot image's kernel to a file")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("output_ramdisk_file")
                .long("output-ramdisk-file")
                .visible_alias("orf")
                .default_value_if("output_all_default", None, "boot/ramdisk.img")
                .help("Extract the boot image's ramdisk to a file")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("output_second_ramdisk_file")
                .long("output-second-ramdisk-file")
                .visible_alias("osf")
                .default_value_if("output_all_default", None, "boot/second.img")
                .help("Extract the boot image's second ramdisk to a file")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("output_device_tree_file")
                .long("output-device-tree-file")
                .visible_alias("odf")
                .default_value_if("output_all_default", None, "boot/dt.img")
                .help("Extract the boot image's device tree to a file")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("output_all_default")
            .long("output-all")
            .short("a")
            .help("Output the boot image sections to the default locations")
            .long_help(
"Output the boot image sections to the default locations. This does not output the boot image \
itself. Locations can still be overriden with the respective output parameters.

Default locations:
 - Header: 'boot/header.img'
 - Kernel: 'boot/kernel.img'
 - Ramdisk: 'boot/ramdisk.img'
 - Second Ramdisk: 'boot/second.img'
 - Device Tree: 'boot/dt.img'"
            )
        )
}

fn main_repack(arguments: &ArgMatches) -> Result<(), ApplicationError> {
    if arguments.is_present("input_page_size") &&
        !(arguments.is_present("input_boot_file") || arguments.is_present("input_header_file"))
    {
        log_warning(
            "Input page size was supplied, but no input boot image or input header to apply it to.",
        );
    }

    let mut boot_image = {
        let override_page_size = arguments.value_of("input_page_size").map(|_| {
            value_t!(arguments.value_of("input_page_size"), u32)
                .unwrap_or_else(|error| error.exit())
        });
        read_boot_image(arguments.value_of("input_boot_file"), override_page_size)?
    };

    insert_sections_from_files(
        &mut boot_image,
        arguments.value_of("input_header_file"),
        arguments.value_of("input_kernel_file"),
        arguments.value_of("input_ramdisk_file"),
        arguments.value_of("input_second_ramdisk_file"),
        arguments.value_of("input_device_tree_file"),
    )?;

    if arguments.is_present("list_sections") {
        print_sections(&boot_image);
    }

    extract_boot_image_into_files(
        &boot_image,
        arguments.value_of("output_boot_image_file"),
        arguments.value_of("output_header_file"),
        arguments.value_of("output_kernel_file"),
        arguments.value_of("output_ramdisk_file"),
        arguments.value_of("output_second_ramdisk_file"),
        arguments.value_of("output_device_tree_file"),
    );

    return Ok(());
}

fn insert_sections_from_files(
    boot_image: &mut BootImage,
    header_path: Option<&str>,
    kernel_path: Option<&str>,
    ramdisk_path: Option<&str>,
    second_ramdisk_path: Option<&str>,
    device_tree_path: Option<&str>,
) -> Result<(), ApplicationError> {
    use std::fs::File;
    use std::io::Read;

    if let Some(path) = header_path {
        let header = File::open(path)
            .and_then(|ref mut file| Header::read_from(file))
            .map_err(|e| {
                ApplicationError::ReadSectionFromFile("header".into(), path.into(), e)
            })?;
        boot_image.insert_header(header).context(path)?;
    }

    {
        if let Some(path) = kernel_path {
            boot_image.insert_kernel(read_vector_section("kernel", path)?);
        }
        if let Some(path) = ramdisk_path {
            boot_image.insert_ramdisk(read_vector_section("ramdisk", path)?);
        }
        if let Some(path) = second_ramdisk_path {
            boot_image.insert_second_ramdisk(read_vector_section("second ramdisk", path)?);
        }
        if let Some(path) = device_tree_path {
            boot_image.insert_device_tree(read_vector_section("device tree", path)?);
        }
    }

    fn read_vector_section(section_name: &str, path: &str) -> Result<Vec<u8>, ApplicationError> {
        let mut output = Vec::new();
        File::open(path)
            .and_then(|mut f| f.read_to_end(&mut output))
            .map(|_| output)
            .map_err(|e| {
                ApplicationError::ReadSectionFromFile(section_name.into(), path.into(), e)
            })
    }

    Ok(())
}

/// Write the boot image and its sections to the specified files. Warn when a
/// section could not be written.
fn extract_boot_image_into_files(
    boot_image: &BootImage,
    boot_image_path: Option<&str>,
    header_path: Option<&str>,
    kernel_path: Option<&str>,
    ramdisk_path: Option<&str>,
    second_ramdisk_path: Option<&str>,
    device_tree_path: Option<&str>,
) {
    use std::fs::File;

    if let Some(path) = boot_image_path {
        if let Err(ref error) =
            File::create(path).and_then(|mut file| boot_image.write_to(&mut file))
        {
            log_warning_cause(
                format!("Could not write the boot image to '{}'.", path,),
                error,
            );
        }
    }

    if let Some(path) = header_path {
        log_result(
            "header",
            path,
            File::create(path).and_then(|mut file| boot_image.write_header_to(&mut file)),
        );
    }

    if let Some(path) = kernel_path {
        log_result(
            "kernel",
            path,
            File::create(path).and_then(|mut file| boot_image.write_kernel_to(&mut file)),
        );
    }

    if let Some(path) = ramdisk_path {
        log_result(
            "ramdisk",
            path,
            File::create(path).and_then(|mut file| boot_image.write_ramdisk_to(&mut file)),
        );
    }

    if let Some(path) = second_ramdisk_path {
        log_result(
            "second ramdisk",
            path,
            File::create(path).and_then(|mut file| boot_image.write_second_ramdisk_to(&mut file)),
        );
    }

    if let Some(path) = device_tree_path {
        log_result(
            "device tree",
            path,
            File::create(path).and_then(|mut file| boot_image.write_device_tree_to(&mut file)),
        );
    }

    fn log_result(section: &str, path: &str, result: Result<usize, IoError>) {
        use humansize::FileSize;
        use humansize::file_size_opts::BINARY as BINARY_FILE_SIZE;

        match result {
            Ok(size) => log_debug(format!(
                "Written '{}' section to '{}'. ({})",
                section,
                path,
                size.file_size(BINARY_FILE_SIZE).unwrap()
            )),
            Err(ref error) => log_warning_cause(
                format!("Could not write the '{}' section to '{}'.", section, path),
                error,
            ),
        }
    }
}

fn read_boot_image(
    boot_image_file: Option<&str>,
    override_page_size: Option<u32>,
) -> Result<BootImage, ApplicationError> {
    match boot_image_file {
        Some(path) => BootImage::read_from_file(path, override_page_size)
            .context(path)
            .map_err(|e| e.into()),
        None => Ok(BootImage::default()),
    }
}

fn print_sections(bi: &BootImage) {
    use android_bootimage::HEADER_SIZE;

    print_section("Header", bi.header_offset(), HEADER_SIZE);
    print_section("Kernel", bi.kernel_offset(), bi.kernel().len());
    print_section("Ramdisk", bi.ramdisk_offset(), bi.ramdisk().len());
    print_section(
        "Second Ramdisk",
        bi.second_ramdisk_offset(),
        bi.second_ramdisk().len(),
    );
    print_section(
        "Device Tree",
        bi.device_tree_offset(),
        bi.device_tree().len(),
    );

    fn print_section(section: &str, start: usize, size: usize) {
        if size != 0 {
            // Only print sections that are there.
            use humansize::FileSize;
            use humansize::file_size_opts::BINARY as BINARY_FILE_SIZE;

            println!(
                "0x{:08X} - {: <14} (size: {})",
                start,
                section,
                size.file_size(BINARY_FILE_SIZE).unwrap()
            );
        }
    }
}

mod logger {
    use colored::Colorize;
    use std::error::Error;

    #[cfg(debug_assertions)]
    pub fn log_debug<S: AsRef<str>>(message: S) {
        eprintln!("{} {}", "debug:".bold().blue(), message.as_ref());
    }

    #[cfg(not(debug_assertions))]
    pub fn log_debug<S: AsRef<str>>(_message: S) {}

    pub fn log_error<S: AsRef<str>>(message: S) {
        eprintln!("{} {}", "error:".bold().red(), message.as_ref());
    }

    pub fn log_warning<S: AsRef<str>>(message: S) {
        eprintln!("{} {}", "warning:".bold().yellow(), message.as_ref());
    }

    pub fn log_warning_cause<S: AsRef<str>>(message: S, cause: &Error) {
        log_warning(message);
        let mut cause_opt = Some(cause);
        while let Some(cause) = cause_opt {
            eprintln!("{} {}", "caused by".yellow(), cause);
            cause_opt = cause.cause();
        }
    }

    pub fn log_error_cause<S: AsRef<str>>(message: S, cause: &Error) {
        log_error(message);
        let mut cause_opt = Some(cause);
        while let Some(cause) = cause_opt {
            eprintln!("{} {}", "caused by".red(), cause);
            cause_opt = cause.cause();
        }
    }
}

quick_error! {
    #[derive(Debug)]
    enum ApplicationError {
        ReadBootImage(path: PathBuf, cause: ReadBootImageError) {
            description("Could not read boot image.")
            display("Could not read boot image from '{}'.", path.display())
            context(path: AsRef<Path>, cause: ReadBootImageError) -> (path.as_ref().into(), cause)
            cause(cause)
        }
        ReadSectionFromFile(section_name: String, path: PathBuf, cause: IoError) {
            description("Could not read section from file.")
            display("Could not read the '{}' section from '{}'.", section_name, path.display())
            cause(cause)
        }
        InsertHeaderError(path: PathBuf, cause: BadHeaderError) {
            description("Could not insert header into boot image.")
            display("Could not insert header from '{}' into boot image.", path.display())
            context(path: AsRef<Path>, cause: BadHeaderError) -> (path.as_ref().into(), cause)
            cause(cause)
        }
    }
}
