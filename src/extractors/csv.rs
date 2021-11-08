use crate::{error::Error, utils};
use colored::Colorize;
use std::{fs, path::Path};

/// Processes encoded, raw `.csv` file data.
///
/// The data passed here must be **compressed/raw**. Passing uncompressed or
/// decoded csv file data will result in [`Error::DecompressionError`].
///
/// ## Error
///
/// If decompression is unsuccessful, [`Error::DecompressionError`] is returned.
///
/// [`Error::IoError`] is returned if an IO operation fails.
///
/// [`Error::DecompressionError`]: ./error/enum.Error.html#variant.DecompressionError
/// [`Error::IoError`]: ./error/enum.Error.html#variant.IoError
pub fn process_csv(data: &[u8], file_name: &str, out_dir: &Path) -> Result<(), Error> {
    let mut output = Vec::new();

    match utils::decompress(data, &mut output) {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    println!("\nExtracting {} file...", file_name.green().bold());

    fs::write(out_dir.join(file_name), output)?;

    Ok(())
}
