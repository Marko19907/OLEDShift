# OLEDShift

A small system tray utility that moves around the windows on your screen.
It's useful for OLED screens, where you want to move around the windows often to prevent burn-in and extend the lifespan of the display.

One of the main ideas behind this program is to be as minimal as possible and to use as little resources as possible so that it can run in the background with minimal impact on the system.
It uses less than **2MB** of RAM on my system, launch time is nearly instant.

The [Win32 API](https://docs.microsoft.com/en-us/windows/win32/apiindex/windows-api-list) is used to move the windows
and [Native Windows GUI (NWG)](https://github.com/gabdube/native-windows-gui) is used for the GUI as a Rust wrapper around the GUI part of the Win32 API.

## Usage

### GitHub Releases [Recommended]

You can download the latest release from [GitHub Releases](https://github.com/Marko19907/OLEDShift/releases). <br>
This is the preferred method.

### GitHub Actions

The program is built automatically on every push to the `main` branch and the executables are uploaded as artifacts.
One can also fork the repository and run the workflow manually to build the program, no installation necessary.

### Building from source locally

You can build the program from source by running the following command on a Windows machine:

```shell
cargo build --release --target x86_64-pc-windows-msvc
```

or for ARM64:

```shell
cargo build --release --target aarch64-pc-windows-msvc
```

If everything goes well, the executable will be located in the `target/release` directory.


## Prerequisites

- Rust 1.7.0 or later
- Windows 11 SDK
- MSVC toolchain


## Known issues

* [Snapped windows get moved](https://github.com/Marko19907/OLEDShift/issues/2)
* [The dialog doesn't have an icon](https://github.com/Marko19907/OLEDShift/issues/3)
* [The dialog also doesn't allow keyboard input](https://github.com/Marko19907/OLEDShift/issues/4)
* [The ARM64 build was not tested on an actual WOA machine, but it should work](https://github.com/Marko19907/OLEDShift/issues/5)
* [The user can't specify the amount of pixels to move the windows by (yet)](https://github.com/Marko19907/OLEDShift/issues/6)


## Limitations

* Only works on Windows
* [Animations are not supported, windows are moved instantly (unsure if this is even possible)](https://github.com/Marko19907/OLEDShift/issues/8)
* [Data is not persisted between sessions, settings revert to defaults on every launch](https://github.com/Marko19907/OLEDShift/issues/9)
