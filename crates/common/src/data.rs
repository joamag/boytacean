use std::{
    io::{Cursor, Read, Write},
    mem::size_of,
};

use crate::error::Error;

#[inline(always)]
pub fn write_u8(data: &mut Cursor<Vec<u8>>, value: u8) -> Result<(), Error> {
    data.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_u16(data: &mut Cursor<Vec<u8>>, value: u16) -> Result<(), Error> {
    data.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_u32(data: &mut Cursor<Vec<u8>>, value: u32) -> Result<(), Error> {
    data.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn read_u8(data: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    let mut buffer = [0x00; size_of::<u8>()];
    data.read_exact(&mut buffer)?;
    Ok(u8::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_vec_u8(data: &mut Cursor<Vec<u8>>) -> Result<u8, Error> {
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
