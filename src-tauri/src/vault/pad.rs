use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use super::error::Result;

pub fn read_at(path: &Path, offset: u64, len: usize) -> Result<Vec<u8>> {
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn zeroize_range(path: &Path, offset: u64, len: u64) -> Result<()> {
    if len == 0 {
        return Ok(());
    }
    let mut f = OpenOptions::new().write(true).open(path)?;
    f.seek(SeekFrom::Start(offset))?;
    const CHUNK: usize = 64 * 1024;
    let zeros = [0u8; CHUNK];
    let mut remaining = len;
    while remaining > 0 {
        let n = std::cmp::min(remaining, CHUNK as u64) as usize;
        f.write_all(&zeros[..n])?;
        remaining -= n as u64;
    }
    f.sync_data()?;
    Ok(())
}
