# android-bootimage

A tool to handle android boot images. It currently only works for Samsung boot images.


# Table of contents
<!-- TOC -->

- [android-bootimage](#android-bootimage)
- [Table of contents](#table-of-contents)
- [Building](#building)
- [How does it work?](#how-does-it-work)
    - [Unpacking](#unpacking)
        - [Extracting a ramdisk](#extracting-a-ramdisk)
    - [Listing the sections](#listing-the-sections)
- [Future work](#future-work)

<!-- /TOC -->

# Building

The entire project is written in the Rust programming language, and built using cargo. To compile the program, install Rust and Cargo, and simply run the following commands. The binary will then be available to you in `./target/release/android-bootimage`.

```
git clone https://gitlab.com/binero/android-bootimage.git
cd android-bootimage
cargo build --release
```

# How does it work?

At the moment the tool only works for Samsung based boot images. Samsung uses a different image format than Android for their images. The tool can unpack these boot images, and it can show the user the composition of the images.

## Unpacking

To unpack an image, we use the following command:

```
android-bootimage unpack boot.img --unpack-all
```

This will create a `boot/` directory, relative to where the command was run from. In this directory you will find a `SECTION.img` for every section of the boot image. For more fine tuned control over which files to extract, and where to extract them to, pass the `--help` flag. The following example extracts just the kernel into a `zImage` file in the current directory.

```
android-bootimage unpack boot.img --kernel ./zImage
```

Some devices have the page size of their boot image set to 0. When this happens the tool cannot extract the boot image, and will warn the user. If the user knows the page size, it can pass it using the `--page-size` parameter. The page size is usually 2048 bytes.

```
android-bootimage unpack boot.img --page-size 2048 --unpack-all
```

### Extracting a ramdisk

As a final example, we will extract the ramdisk from the boot image file, and extract it.

```
android-bootimage unpack boot.img --page-size 2048 --ramdisk ramdisk.img
mkdir ramdisk
cd ramdisk
gzip -dc < ../ramdisk.img | cpio -i
```

## Listing the sections

To list the different sections in a boot image, simply run:

```
android-bootimage secions boot.img
```

On platforms which do not have a valid page size set, it will have to be specified manually. Usually it's 2048 bytes.

```
android bootimage sections boot.img --page-size 2048
```

This will output something similar to the following. Do note that empty (size 0) sections are not listed.

```
0x00000000 - header (size: 616 B)
0x00000800 - kernel (size: 5.31 MiB)
0x0054F000 - ramdisk (size: 4.48 MiB)
0x009C9800 - device_tree (size: 256 B)
```

# Future work

The project still lacks a few important features.

* It cannot create boot images. I am actively working on making sure this will be possible in the future.
* It cannot handle non-Samsung boot images. 

I will not be able to get this program to work for non-Samsung devices on my own, as I only have two Samsung devices available (for testing anyway). If anyone offers to help out I will be glad to implement this as well.
