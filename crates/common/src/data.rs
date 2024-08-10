use std::io::{Cursor, Read};

use crate::error::Error;

#[inline(always)]
pub fn read_u8(data: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    let mut buffer = [0x00; size_of::<u8>()];
    data.read_exact(&mut buffer)?;
    Ok(u8::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_u16(data: &mut Cursor<&[u8]>) -> Result<u16, Error> {
    let mut buffer = [0x00; size_of::<u16>()];
    data.read_exact(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_u32(data: &mut Cursor<&[u8]>) -> Result<u32, Error> {
    let mut buffer = [0x00; size_of::<u32>()];
    data.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}
