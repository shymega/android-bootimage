extern crate android_bootimage;
#[macro_use]
extern crate clap;
extern crate termcolor;
extern crate humansize;

use android_bootimage::{MagicHeader, Section};
use clap::{App, Arg, ArgMatches};
use console::ConsoleOutputHandler;
use std::io::{Read, Seek, Write};
use std::path::Path;
use termcolor::ColorChoice;

const ARG_UNPACK_ALL_LONG_HELP: &'static str = "
Unpack all sections of the boot image to their default locations. 

The default locations for the different sections are 'boot/SECTION_NAME.img', where 'SECTION_NAME' is the name of the section. For example, the kernel will be put into 'boot/kernel.img'.
";

fn main() {
    let console = ConsoleOutputHandler::new(ColorChoice::Auto);

    match create_app().get_matches().subcommand() {
        ("unpack", Some(arguments)) => main_unpack(arguments, console),
        ("sections", Some(arguments)) => main_sections(arguments, console),
        _ => unreachable!(),
    }
}

fn create_app() -> App<'static, 'static> {
    use clap::AppSettings;
    App::new("android-bootimage")
        .setting(AppSettings::SubcommandRequired)
        .version(crate_version!())
        .author(crate_authors!())
        .about("Program for handling samsung boot images.")
        .subcommand(create_app_unpack())
        .subcommand(create_app_sections())
}

fn create_app_unpack() -> App<'static, 'static> {
    App::new("unpack")
        .about("Unpacks samsung boot images.")
        .arg(
            Arg::with_name("input_file")
                .required(true)
                .help("The boot image, for example 'boot.img'")
                .value_name("INPUT_FILE"),
        )
        .arg(
            Arg::with_name("unpack_all")
                .long("unpack-all")
                .short("a")
                .help(
                    "Unpack all sections of the boot image to their default locations",
                )
                .long_help(ARG_UNPACK_ALL_LONG_HELP),
        )
        .arg(
            Arg::with_name("output_kernel_file")
                .long("kernel")
                .help("The file to extract the kernel into")
                .long_help(
                    "The file to extract the kernel into. If this file already exists \
                     it will be emptied first.",
                )
                .value_name("OUTPUT_KERNEL_FILE")
                .default_value_if("unpack_all", None, "boot/kernel.img"),
        )
        .arg(
            Arg::with_name("output_ramdisk_file")
                .long("ramdisk")
                .help("The file to extract the ramdisk into")
                .long_help(
                    "The file to extract the ramdisk into. If this file already exists \
                     it will be emptied first.",
                )
                .value_name("OUTPUT_RAMDISK_FILE")
                .default_value_if("unpack_all", None, "boot/ramdisk.img"),
        )
        .arg(
            Arg::with_name("output_second_file")
                .long("second")
                .help("The file to extract the optional second file into")
                .long_help(
                    "The file to extract the optional second file into. If this file already \
                     exists it will be emptied first.",
                )
                .value_name("OUTPUT_SECOND_FILE")
                .default_value_if("unpack_all", None, "boot/second.img"),
        )
        .arg(
            Arg::with_name("output_tree_file")
                .long("tree")
                .help("The file to extract the device tree file into")
                .long_help(
                    "The file to extract the device tree file into. If this file already \
                     exists it will be emptied first.",
                )
                .value_name("OUTPUT_TREE_FILE")
                .default_value_if("unpack_all", None, "boot/device_tree.img"),
        )
        .arg(
            Arg::with_name("no_magic_check")
                .help("Do not check if the magic signature is correct")
                .long("--no-magic-check"),
        )
        .arg(
            Arg::with_name("page_size")
                .short("p")
                .long("page-size")
                .help("Use a custom page size")
                .long_help(
                    "Use a custom page size. This may be required on some boot images.",
                )
                .value_name("PAGE_SIZE"),
        )
}

fn create_app_sections() -> App<'static, 'static> {
    App::new("sections")
        .about("Lists the sections in a boot image.")
        .arg(
            Arg::with_name("input_file")
                .required(true)
                .help("The boot image, for example 'boot.img'")
                .value_name("INPUT_FILE"),
        )
        .arg(
            Arg::with_name("no_magic_check")
                .help("Do not check if the magic signature is correct")
                .long("--no-magic-check"),
        )
        .arg(
            Arg::with_name("page_size")
                .short("p")
                .long("page-size")
                .help("Use a custom page size")
                .long_help(
                    "Use a custom page size. This may be required on some boot images.",
                )
                .value_name("PAGE_SIZE"),
        )
}

fn main_unpack(arguments: &ArgMatches, mut console: ConsoleOutputHandler) {
    use std::fs::File;

    let input_path = Path::new(arguments.value_of("input_file").unwrap());

    let mut input_file = match File::open(input_path) {
        Ok(file) => file,
        Err(error) => {
            console.print_fatal_error(
                &format!("to open boot image file '{}'", input_path.display()),
                Some(&error),
            )
        }
    };

    let header = read_header(
        &mut input_file,
        !arguments.is_present("no_magic_check"),
        arguments.value_of("page_size").map(|_| {
            value_t!(arguments.value_of("page_size"), u32).unwrap_or_else(|error| error.exit())
        }),
        &mut console,
    );

    console.print_status_success("Parsed", "header.");

    let copy_requested = {
        // Helper function to extract sections out of the boot image. Returns true if
        // the user requested to extract this section, false otherwise.
        let mut copy_data = |output_key, section| if let Some(output_path) =
            arguments.value_of(output_key).map(|path| Path::new(path))
        {
            let data = match header.read_section_from(&mut input_file, section) {
                Ok(data) => data,
                Err(error) => {
                    console.print_error_as_warning(
                        &format!(
                            "Failed to read '{}' section from boot image '{}'",
                            section,
                            input_path.display()
                        ),
                        Some(&error),
                    );
                    return true;
                }
            };

            {
                if let Some(parent_path) = output_path.parent() {
                    if !parent_path.exists() {
                        use std::fs::create_dir_all;
                        if let Err(ref error) = create_dir_all(parent_path) {
                            console.print_error_as_warning(
                                &format!(
                                    "Could not create '{}' directory. ",
                                    parent_path.display()
                                ),
                                Some(error),
                            )
                        } else {
                            console.print_status_success(
                                "Created",
                                &format!("directory '{}'.", parent_path.display()),
                            )
                        }
                    }
                }
            }

            match File::create(output_path).and_then(|mut file| file.write_all(&data)) {
                Ok(_) => {
                    console.print_status_success(
                        "Unpacked",
                        &format!("'{}' section into '{}'.", section, output_path.display()),
                    )
                }
                Err(error) => {
                    console.print_error_as_warning(
                        &format!(
                            "Failed to write '{}' section into '{}'",
                            section,
                            output_path.display()
                        ),
                        Some(&error),
                    )
                }
            }

            return true;
        } else {
            return false;
        };

        [
            copy_data("output_kernel_file", Section::Kernel),
            copy_data("output_ramdisk_file", Section::Ramdisk),
            copy_data("output_second_file", Section::Second),
            copy_data("output_tree_file", Section::DeviceTree),
        ].iter()
            .any(|&copy_requested| copy_requested)
    };

    if !copy_requested {
        console.print_warning_message(
            "No sections extracted, as no sections were requested to be extracted.",
        )
    }
}

fn main_sections(arguments: &ArgMatches, mut console: ConsoleOutputHandler) {
    use std::fs::File;

    let input_path = Path::new(arguments.value_of("input_file").unwrap());

    let mut input_file = match File::open(input_path) {
        Ok(file) => file,
        Err(error) => {
            console.print_fatal_error(
                &format!("to open boot image file '{}'", input_path.display()),
                Some(&error),
            )
        }
    };

    let header = read_header(
        &mut input_file,
        !arguments.is_present("no_magic_check"),
        arguments.value_of("page_size").map(|_| {
            value_t!(arguments.value_of("page_size"), u32).unwrap_or_else(|error| error.exit())
        }),
        &mut console,
    );

    for section in header.sections() {
        use humansize::FileSize;
        use humansize::file_size_opts::BINARY as BINARY_FILE_SIZE;

        match header.section_location(section) {
            Ok((start, size)) => {
                println!(
                    "0x{:08X} - {: <12} (size: {})",
                    start,
                    section,
                    size.file_size(BINARY_FILE_SIZE).unwrap()
                );
            }
            Err(ref error) => {
                println!("0x???????? - {: <12} (size: ?)", section);
                console.print_error_as_warning(
                    &format!("Could not get loction of '{}' section.", section),
                    Some(error),
                );
            }
        }
    }
}

fn read_header<R: Read + Seek>(
    source: &mut R,
    magic_check: bool,
    override_page_size: Option<u32>,
    console: &mut ConsoleOutputHandler,
) -> MagicHeader {
    if !magic_check {
        // Make clear that any following errors might be caused by it not being a valid
        // header.
        console.print_warning_message("Skipping header magic check.");
    }

    let mut header = match MagicHeader::read_from(source, magic_check) {
        Ok(header) => header,
        Err(error) => console.print_fatal_error("Failed to parse boot image header", Some(&error)),
    };

    if let Some(page_size) = override_page_size {
        header.page_size = page_size;
    }

    header
}

mod console {
    use std::error::Error;
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    /// An interface for the application to the console output. Handles things
    /// like
    /// formatting.
    ///
    /// If this structure ever fails writing, the error will be silently
    /// ignored.
    pub struct ConsoleOutputHandler {
        stream: StandardStream,
    }

    impl ConsoleOutputHandler {
        /// Creates a new structure.
        pub fn new(color: ColorChoice) -> Self {
            ConsoleOutputHandler { stream: StandardStream::stdout(color) }
        }

        pub fn print_message(&mut self, message: &str) {
            let _ = self.stream.set_color(&ColorSpec::new());
            let _ = writeln!(self.stream, "{}", message);
        }

        pub fn print_error_message(&mut self, message: &str) {
            let _ = self.stream.set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Red))
                    .set_bold(true),
            );

            let _ = write!(self.stream, "error: ");
            self.print_message(message);
        }

        pub fn print_warning_message(&mut self, message: &str) {
            let _ = self.stream.set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Yellow))
                    .set_bold(true),
            );

            let _ = write!(self.stream, "warning: ");
            self.print_message(message);
        }

        fn print_status(&mut self, colour: &ColorSpec, status: &str, message: &str) {
            let _ = self.stream.set_color(colour);
            let _ = write!(self.stream, "{: >12}", status);
            let _ = self.stream.set_color(&ColorSpec::new());
            let _ = writeln!(self.stream, " {}", message);
        }

        pub fn print_status_success(&mut self, status: &str, message: &str) {
            self.print_status(
                ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true),
                status,
                message,
            );
        }

        fn print_error_cause(&mut self, mut error_opt: Option<&Error>, colour: Color) {
            let _ = self.stream.set_color(
                ColorSpec::new().set_fg(Some(colour.clone())),
            );

            let colour_spec = {
                let mut colour_spec = ColorSpec::new();
                colour_spec.set_fg(Some(colour));
                colour_spec
            };

            while let Some(error) = error_opt {
                let _ = self.stream.set_color(&colour_spec);
                let _ = write!(self.stream, "caused by: ");
                self.print_message(&format!("{}", error));
                error_opt = error.cause();
            }
        }

        pub fn print_error_as_error(&mut self, message: &str, error_opt: Option<&Error>) {
            self.print_error_message(message);
            self.print_error_cause(error_opt, Color::Red);
        }

        pub fn print_error_as_warning(&mut self, message: &str, error_opt: Option<&Error>) {
            self.print_warning_message(message);
            self.print_error_cause(error_opt, Color::Yellow);
        }

        pub fn print_fatal_error(&mut self, message: &str, error_opt: Option<&Error>) -> ! {
            use std::process::exit;
            self.print_error_as_error(message, error_opt);
            exit(1);
        }
    }
}