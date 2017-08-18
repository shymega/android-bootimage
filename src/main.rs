extern crate android_bootimage;
#[macro_use]
extern crate clap;
extern crate termcolor;
extern crate humansize;

use android_bootimage::{BootImage, Header, Section};
use clap::{App, Arg, ArgMatches};
use console::ConsoleOutputHandler;
use std::io::{Read, Seek, Write};
use std::path::Path;
use termcolor::ColorChoice;

const ARG_UNPACK_ALL_LONG_HELP: &'static str = "
Unpack all sections of the boot image to their default locations.

The default locations for the different sections are 'boot/SECTION_NAME.img', where \
'SECTION_NAME' is the name of the section. For example, the kernel will be put \
into 'boot/kernel.img'.
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
        Err(error) => console.print_fatal_error(
            &format!("to open boot image file '{}'", input_path.display()),
            Some(&error),
        ),
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
                Ok(_) => console.print_status_success(
                    "Unpacked",
                    &format!("'{}' section into '{}'.", section, output_path.display()),
                ),
                Err(error) => console.print_error_as_warning(
                    &format!(
                        "Failed to write '{}' section into '{}'",
                        section,
                        output_path.display()
                    ),
                    Some(&error),
                ),
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
    let input_path = arguments.value_of("input_file").unwrap();
    let override_page_size = arguments.value_of("page_size").map(|_| {
        value_t!(arguments.value_of("page_size"), u32).unwrap_or_else(|error| error.exit())
    });

    let boot_image = match BootImage::read_from_file(input_path, override_page_size) {
        Ok(boot_image) => boot_image,
        Err(ref error) => console.print_fatal_error(
            format!("Could not read boot image from '{}'.", input_path),
            Some(error),
        ),
    };

    print_sections(&boot_image);
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

fn read_header<R: Read + Seek>(
    source: &mut R,
    magic_check: bool,
    override_page_size: Option<u32>,
    console: &mut ConsoleOutputHandler,
) -> Header {
    if !magic_check {
        // Make clear that any following errors might be caused by it not being a valid
        // header.
        console.print_warning_message("Skipping header magic check.");
    } else {
        console.print_warning_message("Magic check is currently not implemented.");
    }

    let mut header_data = Header::create_buffer();
    if let Err(ref error) = source.read_exact(&mut *header_data) {
        console.print_fatal_error("Failed to read boot image header.", Some(error))
    }

    let mut header = Header::parse(&header_data);

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
            ConsoleOutputHandler {
                stream: StandardStream::stdout(color),
            }
        }

        pub fn print_message<M: AsRef<str>>(&mut self, message: M) {
            let _ = self.stream.set_color(&ColorSpec::new());
            let _ = writeln!(self.stream, "{}", message.as_ref());
        }

        pub fn print_error_message<M: AsRef<str>>(&mut self, message: M) {
            let _ = self.stream
                .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));

            let _ = write!(self.stream, "error: ");
            self.print_message(message);
        }

        pub fn print_warning_message<M: AsRef<str>>(&mut self, message: M) {
            let _ = self.stream
                .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true));

            let _ = write!(self.stream, "warning: ");
            self.print_message(message);
        }

        fn print_status<M1: AsRef<str>, M2: AsRef<str>>(
            &mut self,
            colour: &ColorSpec,
            status: M1,
            message: M2,
        ) {
            let _ = self.stream.set_color(colour);
            let _ = write!(self.stream, "{: >12}", status.as_ref());
            let _ = self.stream.set_color(&ColorSpec::new());
            let _ = writeln!(self.stream, " {}", message.as_ref());
        }

        pub fn print_status_success(&mut self, status: &str, message: &str) {
            self.print_status(
                ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true),
                status,
                message,
            );
        }

        fn print_error_cause(&mut self, mut error_opt: Option<&Error>, colour: Color) {
            let _ = self.stream
                .set_color(ColorSpec::new().set_fg(Some(colour.clone())));

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

        pub fn print_error_as_error<M: AsRef<str>>(
            &mut self,
            message: M,
            error_opt: Option<&Error>,
        ) {
            self.print_error_message(message);
            self.print_error_cause(error_opt, Color::Red);
        }

        pub fn print_error_as_warning<M: AsRef<str>>(
            &mut self,
            message: M,
            error_opt: Option<&Error>,
        ) {
            self.print_warning_message(message);
            self.print_error_cause(error_opt, Color::Yellow);
        }

        pub fn print_fatal_error<M: AsRef<str>>(
            &mut self,
            message: M,
            error_opt: Option<&Error>,
        ) -> ! {
            use std::process::exit;
            self.print_error_as_error(message, error_opt);
            exit(1);
        }
    }
}
