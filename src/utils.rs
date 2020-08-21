use super::error::Error;
use byteorder::{LittleEndian, ReadBytesExt};
use lzma_rs::lzma_decompress;
use std::io::{Cursor, Read};

/// Wrapper for reading data from stream.
pub(crate) struct Reader {
    stream: Cursor<Vec<u8>>,
    bytes_left: usize,
}

impl Reader {
    /// Create new `Reader` instance from a stream.
    pub fn new(stream: Cursor<Vec<u8>>) -> Self {
        let bytes_left = stream.get_ref().len();

        Self { stream, bytes_left }
    }

    /// Bytes left in the data stream.
    pub fn len(&self) -> usize {
        self.bytes_left
    }

    /// Read exact number of bytes from the stream.
    pub fn read(&mut self, size: usize) -> Vec<u8> {
        if size > self.bytes_left {
            self.bytes_left = 0;
        } else {
            self.bytes_left -= size;
        }

        let mut buf = vec![0; size];
        if self.bytes_left == 0 {
            self.stream.read_to_end(&mut buf).unwrap_or_default();

            buf
        } else {
            self.stream.read_exact(&mut buf).unwrap_or_default();

            buf
        }
    }

    /// Read one byte from the stream.
    pub fn read_byte(&mut self) -> u8 {
        if 1 > self.bytes_left {
            self.bytes_left = 0;
        } else {
            self.bytes_left -= 1;
        }

        self.stream.read_u8().unwrap_or_default()
    }

    /// Read an unsigned 16-bit little-endian integer from the stream.
    pub fn read_uint16(&mut self) -> u16 {
        if 2 > self.bytes_left {
            self.bytes_left = 0;
        } else {
            self.bytes_left -= 2;
        }

        self.stream.read_u16::<LittleEndian>().unwrap_or_default()
    }

    /// Read an unsigned 32-bit little-endian integer from the stream.
    pub fn read_uint32(&mut self) -> u32 {
        if 4 > self.bytes_left {
            self.bytes_left = 0;
        } else {
            self.bytes_left -= 4;
        }

        self.stream.read_u32::<LittleEndian>().unwrap_or_default()
    }

    /// Read an signed 16-bit little-endian integer from the stream.
    pub fn read_int16(&mut self) -> i16 {
        if 2 > self.bytes_left {
            self.bytes_left = 0;
        } else {
            self.bytes_left -= 2;
        }

        self.stream.read_i16::<LittleEndian>().unwrap_or_default()
    }

    /// Read an signed 32-bit little-endian integer from the stream.
    pub fn read_int32(&mut self) -> i32 {
        if 4 > self.bytes_left {
            self.bytes_left = 0;
        } else {
            self.bytes_left -= 4;
        }

        self.stream.read_i32::<LittleEndian>().unwrap_or_default()
    }

    /// Read `length` bytes from the stream and return the output as a `String`.
    pub fn read_string(&mut self, length: usize) -> String {
        if length > self.bytes_left {
            self.bytes_left = 0;
        } else {
            self.bytes_left -= length;
        }

        String::from_utf8_lossy(self.read(length).as_slice()).to_string()
    }
}

/// Decompresses `.tex_sc` or `.csv` data.
///
/// Before decompressing the data using LZMA decompression,
/// four `\x00` bytes are added to `raw_data` after the eigth index.
/// A `Cursor` containing the transformed raw data is returned.
///
/// `_tex.sc` files found in Supercell's games require the header
/// to be removed before decompression.
///
/// If the decompression fails due to any reason,
/// [`Error::DecompressionError`] is returned.
///
/// [`Error::DecompressionError`]: ./error/enum.Error.html#variant.DecompressionError
pub(crate) fn decompress(raw_data: &[u8]) -> Result<Cursor<Vec<u8>>, Error> {
    let data = [&raw_data[0..9], &[b'\x00'; 4], &raw_data[9..]].concat();

    let mut decomp: Vec<u8> = Vec::new();
    match lzma_decompress(&mut data.as_slice(), &mut decomp) {
        Ok(_) => Ok(Cursor::new(decomp)),
        Err(_) => Err(Error::DecompressionError(
            "Failed to decompress file".to_string(),
        )),
    }
}
