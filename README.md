# OLEDShift

[![Rust Cargo build](https://github.com/Marko19907/OLEDShift/actions/workflows/rust.yml/badge.svg)](https://github.com/Marko19907/OLEDShift/actions/workflows/rust.yml)
<a title="Text" href="https://github.com/Marko19907/OLEDShift/releases/latest"><img alt="GitHub all releases" src="https://img.shields.io/github/downloads/Marko19907/OLEDShift/total?label=GitHub%20downloads"></a>

A small system tray utility that moves around the windows on your screen.
It's useful for OLED screens, where you want to move around the windows often to prevent burn-in and extend the lifespan of the display.

One of the main ideas behind this program is to be as minimal as possible and to use as little resources as possible so that it can run in the background with minimal impact on the system.
It uses less than **2MB** of RAM on my system, launch time is nearly instant.

The [Win32 API](https://docs.microsoft.com/en-us/windows/win32/apiindex/windows-api-list) is used to move the windows
and [Native Windows GUI (NWG)](https://github.com/gabdube/native-windows-gui) is used for the GUI as a Rust wrapper around the GUI part of the Win32 API.

## Installation

### Microsoft Store [Recommended]

Install OLEDShift from the Microsoft Store for an easy and clean install/uninstall + automatic updates:

<a href="https://apps.microsoft.com/detail/9pj4957v45cn?referrer=appbadge&mode=full">
	<img src="https://get.microsoft.com/images/en-us%20dark.svg" width="200" alt="Microsoft Store"/>
</a>

This is the preferred method.

### GitHub Releases

You can also download the latest portable release from [GitHub Releases](https://github.com/Marko19907/OLEDShift/releases).

### GitHub Actions

The program is built automatically on every push and the executables are uploaded as artifacts, these builds are not guaranteed to be stable or functional, they expire after 90 days, and you might need to be logged in to GitHub to download them.

If you're struggling to find the artifacts here on GitHub, you can download the latest build of the main branch from nightly.link for x86_64 [here](https://nightly.link/Marko19907/OLEDShift/workflows/rust/main/windows-x64-binaries.zip) 
and for arm64 [here](https://nightly.link/Marko19907/OLEDShift/workflows/rust/main/windows-arm64-binaries.zip).

One can also fork the repository and run the workflow manually to build the program. None of these options require you to have anything installed on your machine.

### Building from source locally

You can build the program from source by running the following command on a Windows machine:

```shell
cargo build --release --target x86_64-pc-windows-msvc
```

or for arm64:

```shell
cargo build --release --target aarch64-pc-windows-msvc
```

If everything goes well, the executable will be located in the `target/release` directory.


## Prerequisites

- Rust 1.70.0 or later
- Windows 11 SDK
- MSVC toolchain

Follow the [Rust installation guide](https://rust-lang.github.io/rustup/installation/windows-msvc.html) to install the prerequisites.


## Known issues

* [The dialog doesn't have an icon](https://github.com/Marko19907/OLEDShift/issues/3)
* [The dialog also doesn't allow keyboard input](https://github.com/Marko19907/OLEDShift/issues/4)
* [The arm64 build was not tested on an actual WOA machine, but it should work](https://github.com/Marko19907/OLEDShift/issues/5)

## Limitations

* Only works on Windows
* [Animations are not supported, windows are moved instantly (unsure if this is even possible)](https://github.com/Marko19907/OLEDShift/issues/8)
