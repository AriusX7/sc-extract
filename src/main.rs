use colored::Colorize;
use rayon::prelude::*;
use sc_extract::{process_csv, process_sc, process_tex};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};
use structopt::StructOpt;

/// A very fast tool to extract graphics and data from Supercell's game files.
///
/// sc_extract supports extraction of the following files: `_tex.sc`, extracted `.sc` and `.csv`.
#[derive(StructOpt)]
#[structopt(name = "sce")]
struct Options {
    /// The path to a file to extract or directory with files to extract.
    ///
    /// If not specified, the current directory is used.
    #[structopt(parse(from_os_str))]
    path: Option<PathBuf>,

    /// The path to directory where an extracts folder is created to save output.
    ///
    /// If not specified, `extracts` directory is created inside `path`.
    #[structopt(parse(from_os_str), short = "o", long = "out")]
    out_dir: Option<PathBuf>,

    /// If this flag is supplied, the source `_tex.sc` or `.csv` files are deleted after extracting.
    #[structopt(short = "d", long = "delete")]
    delete: bool,

    /// Extracts all images in parallel, making the process faster.
    #[structopt(short = "p", long = "parallelize")]
    parallelize: bool,

    /// The path to directory where a `_tex.sc` file's extracted images are stored.
    ///
    /// It is required for cutting images using extracted `.sc` files. If the
    /// path is not specified, sc_extract will look for the png files in the
    /// directory where the source (extracted `.sc`) file(s) is/are present.
    #[structopt(parse(from_os_str), short = "P", long = "png")]
    png_dir: Option<PathBuf>,

    /// Specifies the type of files you want to extract.
    ///
    /// Possible values are "csv", "sc" and "tex". By default, all types are considered.
    #[structopt(short = "t", long = "type")]
    kind: Option<FileType>,

    /// Disable filtering of common error-prone files.
    ///
    /// sc_extract automatically filters some common error-prone files like
    /// `quickbms` and `.DS_Store`. You can disable this filter by adding this
    /// flag.
    #[structopt(short = "F", long = "disable-filter")]
    disable_filter: bool,
}

/// Represents a single file type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileType {
    /// Represents `.csv` files.
    Csv,
    /// Represents `.sc` files.
    Sc,
    /// Represents `_tex.sc` files.
    Tex,
}

impl FromStr for FileType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "csv" => Ok(Self::Csv),
            "sc" => Ok(Self::Sc),
            "tex" => Ok(Self::Tex),
            _ => Err("File type must be one of `csv`, `sc` and `tex`."),
        }
    }
}

/// Deletes the file with given path. It deletion fails, prints it on stdout.
fn delete_file(path: &Path) {
    match fs::remove_file(&path) {
        Ok(_) => (),
        Err(_) => println!(
            "{}: {}",
            "Failed to remove file".red(),
            path.to_str().unwrap().red()
        ),
    };
}

/// Returns correct file type depending on the file extension and/or data.
///
/// If the extension and/or data don't match any expected file type,
/// `None` is returned.
///
/// The data passed here must be compressed/raw.
fn get_file_type(data: &[u8], path: &Path, filter: bool) -> Option<FileType> {
    // Some common mistakenly used file types are filtered here.
    let path_str = path.file_name().unwrap().to_str().unwrap();

    if filter && [".DS_Store", "quickbms"].contains(&path_str) {
        return None;
    }

    if data.is_empty() {
        None
    } else if path.extension().is_none() {
        Some(FileType::Sc)
    } else if data[0] == 83 && path_str.ends_with("_tex.sc") {
        Some(FileType::Tex)
    } else if data.len() >= 2 && data[..2] == [93, 0] && path_str.ends_with(".csv") {
        Some(FileType::Csv)
    } else {
        None
    }
}

/// Processes the given file (path).
///
/// It automatically detects file type (`_tex.sc`, `.csv` or extracted `.sc`)
/// and processes them appropriately. If processing a file fails, formatted
/// error messages gets printed on `stdout`.
///
/// ## Panic
///
/// The process may panic in case of lack of permissions to read/write files.
fn process_file(path: &Path, out_dir: &Path, parallelize: bool, opts: &Options) -> Result<(), ()> {
    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(_) => return Err(()),
    };

    let res = if let Some(file_type) = get_file_type(data.as_slice(), path, !opts.disable_filter) {
        if let Some(ft) = opts.kind {
            if ft != file_type {
                return Ok(());
            }
        }
        let file_name = path
            .file_name()
            .expect("Expected file to have a name.")
            .to_str()
            .expect("Expected file to have a valid UTF-8 name.");

        match file_type {
            FileType::Tex => process_tex(&data, file_name, &out_dir, parallelize),
            FileType::Csv => process_csv(&data, file_name, &out_dir),
            FileType::Sc => {
                let png_dir = match opts.png_dir.as_ref() {
                    Some(p) => p,
                    None => match path.parent() {
                        Some(p) => p,
                        None => {
                            println!("{}", "Could not determine the path for png files.".red());

                            return Ok(());
                        }
                    },
                };

                let out_dir = out_dir.join(format!("{}_out", file_name));
                if !out_dir.exists() {
                    // We want to panic if a directory can't be created.
                    fs::create_dir(&out_dir).unwrap();
                }

                process_sc(&data, file_name, &out_dir, png_dir, parallelize)
            }
        }
    } else {
        return Err(());
    };

    if let Err(e) = res {
        println!("\n{}: {}", e.inner().red(), path.to_str().unwrap().red());

        // Don't delete file if there was an error.
        return Ok(());
    }

    if opts.delete {
        delete_file(&path);
    }

    Ok(())
}

fn main() {
    let opts: Options = Options::from_args();

    let path = if let Some(ref p) = opts.path {
        p.clone()
    } else if let Ok(p) = std::env::current_dir() {
        p
    } else {
        println!("{}", "Expected to access the current directory.".red());
        std::process::exit(1);
    };

    let out_dir = match &opts.out_dir {
        Some(p) => p.join("extracts"),
        None => {
            if path.is_dir() {
                path.join("extracts")
            } else if path.is_file() {
                if let Some(p) = path.parent() {
                    p.join("extracts")
                } else {
                    println!("{}", "Expected path to have a parent.".red());
                    std::process::exit(1);
                }
            } else if let Ok(p) = std::env::current_dir() {
                p.join("extracts")
            } else {
                println!("{}", "Expected to access the current directory.".red());
                std::process::exit(1);
            }
        }
    };

    let created_out = if !out_dir.exists() {
        fs::create_dir_all(&out_dir).expect("Expected to be able to create a directory.");

        true
    } else {
        false
    };

    if path.is_dir() {
        let found_one = AtomicBool::new(false);
        let dir_entries = match fs::read_dir(&path) {
            Ok(e) => e,
            Err(_) => {
                println!(
                    "{}",
                    format!(
                        "Failed to read contents of {} directory/folder.",
                        path.to_str()
                            .expect("Expected path to be valid UTF-8.")
                            .red()
                    )
                    .red()
                );
                std::process::exit(1);
            }
        };

        let mut entries = Vec::new();
        for entry in dir_entries {
            entries.push(entry);
        }

        if opts.parallelize {
            entries.into_par_iter().for_each(|entry| {
                let file_path = entry.unwrap().path();
                if process_file(&file_path, &out_dir, true, &opts).is_ok() {
                    let _ = found_one.compare_exchange_weak(
                        false,
                        true,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                }
            })
        } else {
            for entry in entries {
                let file_path = entry.unwrap().path();
                if process_file(&file_path, &out_dir, false, &opts).is_ok() {
                    let _ = found_one.compare_exchange_weak(
                        false,
                        true,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                }
            }
        }

        if !found_one.into_inner() {
            println!(
                "{}",
                "No valid `_tex.sc` or `.csv` file in the given directory!"
                    .red()
                    .bold()
            );
            std::process::exit(1);
        }
    } else if path.is_file() {
        let _ = process_file(&path, &out_dir, false, &opts);
    }

    if created_out {
        // Returns an error if directory is not empty. We ignore that.
        let _ = fs::remove_dir(&out_dir);
    }

    println!("\n{}", "Extraction finished!".green().bold());
}
