# sc_extract

[![build badge][]][build link] [![release badge][]][release link] [![crates.io badge][]][crates.io link] [![license badge][]][license link] [![rust 2018 badge]][rust 2018 link]

`sc_extract` is a very fast tool to extract graphics and decode csv files from Supercell's game files.

This tool is simply intended to get high quality graphics and data from the files. It is in no way an attempt to:

- modify the game in any way
- create a clone or any other game based on any of Supercell's games
- make profit

## Features

Some of sc_extract's features include:

- very fast speed (upto 10 times faster than Python tools)
- precompiled binaries for multiple operating systems (Linux, macOS, Windows)
- multiple file formats support
- very extensive command line support

sc_extract can extract/process the following files found in Supercell's games:

- `_tex.sc`
- `.csv`
- `.sc` files extracted from QuickBMS

sc_extract is a standalone tool but provides a simple Rust crate with a clean API allowing developers to implement their own tools with minimal work.

## About The Tool

This tool is a Rust implementation of these scripts:

- [cr-sc-dump]: tex_sc and csv extraction
- [sc_decode]: extraction of individual images from tex_sc sprites

The Python implementations take a long time to extract images from huge files. sc_extract can extract data very quickly. Some comparisons are as below:

- To extract all `_tex.sc` sprites from Brawl Stars' version `27.269`, sc_extract takes **less than 20 seconds** whereas cr-sc-dump takes **over 4 minutes**

- To extract all individual images from `ui` file of Brawl Stars' version `27.269`, sc_extract takes **about 2.5 minutes** whereas sc_decode takes **over 10 minutes**.

**Note:** The times were tested on a MacBook Air with 1.8 GHz Dual-Core Intel Core i5 processor and 8 GB of RAM. They may wary on your machine.

The time sc_extract takes can be further reduced by using the [`parallelize` flag](#flags-and-options).

## Installation

`sc_extract` can be installed in three ways. [The first method](#downloading-precompiled-binary-recommended) is the fastest and does not require you to install Rust. [Second](#using-cargo-install) and [third](#building-from-source) methods require you to install [Rust's 2018 version]. Rust is available for a large number of operating systems. Clicking on the above link above will take you to Rust's installation page.

### Downloading Precompiled Binary (Recommended)

You can find precompiled binaries for multiple operating systems and architectures [here][releases].

Download the binary which is appropriate for your machine. After downloading, unzip the folder. You should see the following three files inside:

- `sc_extract` (or `sc_extract.exe` on Windows)
- `README.md`
- `LICENSE`

Now, `cd` into this directory and follow steps described [here](#usage) to use it!

### Using `cargo install`

sc_extract is available on [crates.io][crates.io link]. You can install and build this tool by simply using `cargo install sc_extract` command from the terminal. The installation process will take a few minutes to build all dependencies, but once that's done, the tool will work very, very fast.

It will also add this tool to the shell path automatically, allowing you to use the tool from *any* directory.

### Building From Source

You can download this tool's [source code] and build it yourself by using `cargo build --release` command. You need to `cd` into this tool's directory before executing that command. Do not forgot the `--release` flag or your tool will work very slowly.

**Note:** In the below example commands, it will be assumed that you have installed the tool using first or second method. If you installed from the source, you will have to replace `sc_extract` with `cargo run --release` in all commands.

## Usage

**Note 1:** You may need to replace `sc_extract` by `./sc_extract`, `sc_extract.exe` or `cargo run --release` in the commands below.

**Note 2:** Extracted `sc` will be used to denote `.sc` files extracted using QuickBMS in the following section(s). Extracted `sc` files have no extension, they appear as `ui`, `loading`, etc.

You will need the `_tex.sc`, extracted `sc` or `.csv` files of the Supercell game you wish to extract. You can get the files by downloading the APK of the game, changing the extension to `.zip`, unzipping it and navigating to `/assets/sc` (_tex.sc files), `/assets/csv_logic` (csv files) or `csv_client` (csv files) folder inside the unzipped folder. To get extracted `sc` files, see [this section](#using-quickbms-to-extract-sc-files).

After installing this tool, `cd` into the directory with the tool (not required if you add it to your path or use the second method).

```sh
cd path_to_sc_extract
```

Then, simply use the following command to extract the required files!

```sh
sc_extract [FLAGS] [OPTIONS] <path>
```

`path` must be a valid path pointing to a single `_tex.sc`, extracted `sc` or `.csv` file or a directory containing those files. See [Flags and Options](#flags-and-options) section to know more about them.

If you installed the tool using the source code, you may want to build the tool and all the dependencies prior to extracting the files. You can do so by run the `cargo build --release` command in the tool's directory. Building will take a couple of minutes, but running the tool in future will be very fast.

### Flags and Options

The following optional flags and options can be specified to control the extraction. You might be required to specify the `png` option (see below) to extract images from extracted `sc` files.

|      Flags       | Short |                                  Description                                   |
|:----------------:|:-----:|:------------------------------------------------------------------------------:|
|     --delete     |  -d   |                     Deletes source files after extracting                      |
|  --parallelize   |  -p   |             Extracts files in parallel, making the process faster              |
| --disable-filter |  -F   | Disables filtering of common error-prone files like `quickbms` and `.DS_Store` |
|      --help      |  -h   |                            Prints help information                             |
|    --version     |  -V   |                           Prints version information                           |

|     Options      |     Short     |                                                                                                                                         Description                                                                                                                                         |                              out_path                               |
|:----------------:|:-------------:|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------:|:-------------------------------------------------------------------:|
| --out <out-path> | -o <out-path> |                                                                                            Specifies the output directory. If not specified, a directory named `extracts` is created in `path`.                                                                                             |            `out-path` must be a valid path-like string.             |
| --png <png-dir>  | -P <png-dir>  | The path to directory where a `_tex.sc` file's extracted images are stored. It is required for cutting images using extracted `.sc` files. If the path is not specified, sc_extract will look for the png files in the directory where the source (extracted `sc`) file(s) is/are present. |             `png-dir` must be a valid path-like string.             |
|  --type <kind>   |   -t <kind>   |                                                                                                   Specifies the type of files you want to extract. By default, all types are considered.                                                                                                    | `kind` can be one of "csv", "sc" and "tex" (without double quotes). |

**Example Commands:**

```sh
sc_extract ./sc -F
```

The above command uses `./sc` as the source directory. It goes over all files in the directory one-by-one and extracts all valid files. The output is saved in `./sc/extracts` directory. It does not ignore files like `quickbms` and `.DS_Store` because of the `-F` flag.

```sh
sc_extract --delete -p ./sc --out ./extracts
```

The above command uses `./sc` as the source directory. It goes over all files in the directory parallelly and extracts all valid files. The output is saved in `./extracts` directory. `png` flag is not supplied, so it looks for sprites extracted from `_tex.sc` in `./sc`. It will fail to cut images from extracted `sc` files if `./sc` directory will not contain the sprites. The rest of extraction will not be affected. After extracting, all valid `_tex.sc` and `.csv` and extracted `sc` (with png images) files are deleted.

```sh
sc_extract -p ./sc --out ./sc_out -t sc --png ./sc/extracts
```

The above command uses `./sc` as the source directory. It goes over all files in the directory parallelly and cuts images using all valid extracted `sc` files. The output is saved in `./sc_out` directory. The png files used for extraction as searched for in `./sc/extracts` directory.

### Using QuickBMS To Extract `.sc` Files

[QuickBMS] is required to extract `.sc` files. You will also need [clash_royale.bms]. QuickBMS can be downloaded for macOS [here][quickbms macos].

Copy QuickBMS and clash_royale.bms into the directory with `.sc` files (not `_tex.sc`) and then run this command:

```sh
find . -not -name '*_tex.sc' -name '*.sc' -exec ./quickbms ./clash_royale.bms {} \;
```

You will have to adjust this command as appropriate if you are using Windows.

Running the command will create a new file for each `.sc` file in the folder. These files will have the same name as their corresponding `.sc` files but no extension. These are referred to as extracted `sc` files and are used to cut the sprites.

## Updating

If you used a pre-compiled binary, you'll simply have to download a new binary for the newer version from the [Releases][releases] page.

If you installed the tool using `cargo install`, you can update the tool by simply reusing the `cargo install sc_extract` command. If it fails to update the tool, you can force it by adding the `--force` flag, like so: `cargo install sc_extract --force`.

If you installed using the source code, you will have to repeat the process described in [Building From Source](#building-from-source) section using the new source code.

## License

sc_extract is available under the `MIT` license. See [LICENSE](LICENSE) for more details.

## Credits

This tool wouldn't exist if the following didn't create the original Python scripts.

- [athlan20]
- [clanner]
- [Galaxy1036]
- [umop-aplsdn]

[cr-sc-dump]: https://github.com/123456abcdef/cr-sc-dump
[sc_decode]: https://github.com/Galaxy1036/sc_decode
[Rust's 2018 version]: https://www.rust-lang.org/tools/install
[releases]: https://github.com/AriusX7/sc-extract/releases
[source code]: https://codeload.github.com/AriusX7/sc-extract/zip/master
[QuickBMS]: https://aluigi.altervista.org/quickbms.html
[quickbms macos]: https://github.com/ryopei/quickbms-macos/releases/tag/v0.8.0
[clash_royale.bms]: http://aluigi.altervista.org/bms/clash_royale.bms
[athlan20]: https://github.com/athlan20
[clanner]: https://github.com/clanner
[Galaxy1036]: https://github.com/Galaxy1036
[umop-aplsdn]: https://github.com/umop-aplsdn
[build link]: https://travis-ci.com/github/AriusX7/sc-extract
[build badge]: https://img.shields.io/travis/AriusX7/sc-extract?style=for-the-badge
[release badge]: https://img.shields.io/github/v/release/AriusX7/sc-extract?color=00A1D0&style=for-the-badge
[release link]: https://github.com/AriusX7/sc-extract/releases/tag/v1.0.1
[license badge]: https://img.shields.io/badge/license-MIT-8E3FFF.svg?style=for-the-badge
[license link]: https://github.com/AriusX7/sc-extract/blob/master/LICENSE
[crates.io link]: https://crates.io/crates/sc_extract
[crates.io badge]: https://img.shields.io/crates/v/sc_extract?color=ea8b27&style=for-the-badge
[rust 2018 badge]: https://img.shields.io/badge/rust-2018-93450a.svg?style=for-the-badge
[rust 2018 link]: https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html
