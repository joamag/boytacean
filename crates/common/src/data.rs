use std::{
    io::{Read, Write},
    mem::size_of,
};

use crate::error::Error;

#[inline(always)]
pub fn write_u8<W: Write>(writer: &mut W, value: u8) -> Result<(), Error> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_i8<W: Write>(writer: &mut W, value: i8) -> Result<(), Error> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_u16<W: Write>(writer: &mut W, value: u16) -> Result<(), Error> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_i16<W: Write>(writer: &mut W, value: i16) -> Result<(), Error> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_u32<W: Write>(writer: &mut W, value: u32) -> Result<(), Error> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_i32<W: Write>(writer: &mut W, value: i32) -> Result<(), Error> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_u64<W: Write>(writer: &mut W, value: u64) -> Result<(), Error> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_i64<W: Write>(writer: &mut W, value: i64) -> Result<(), Error> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline(always)]
pub fn write_bytes<W: Write>(writer: &mut W, buffer: &[u8]) -> Result<(), Error> {
    writer.write_all(buffer)?;
    Ok(())
}

#[inline(always)]
pub fn read_u8<R: Read>(reader: &mut R) -> Result<u8, Error> {
    let mut buffer = [0x00; size_of::<u8>()];
    reader.read_exact(&mut buffer)?;
    Ok(u8::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_i8<R: Read>(reader: &mut R) -> Result<i8, Error> {
    let mut buffer = [0x00; size_of::<i8>()];
    reader.read_exact(&mut buffer)?;
    Ok(i8::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_u16<R: Read>(reader: &mut R) -> Result<u16, Error> {
    let mut buffer = [0x00; size_of::<u16>()];
    reader.read_exact(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_i16<R: Read>(reader: &mut R) -> Result<i16, Error> {
    let mut buffer = [0x00; size_of::<i16>()];
    reader.read_exact(&mut buffer)?;
    Ok(i16::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_u32<R: Read>(reader: &mut R) -> Result<u32, Error> {
    let mut buffer = [0x00; size_of::<u32>()];
    reader.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_i32<R: Read>(reader: &mut R) -> Result<i32, Error> {
    let mut buffer = [0x00; size_of::<i32>()];
    reader.read_exact(&mut buffer)?;
    Ok(i32::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_u64<R: Read>(reader: &mut R) -> Result<u64, Error> {
    let mut buffer = [0x00; size_of::<u64>()];
    reader.read_exact(&mut buffer)?;
    Ok(u64::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_i64<R: Read>(reader: &mut R) -> Result<i64, Error> {
    let mut buffer = [0x00; size_of::<i64>()];
    reader.read_exact(&mut buffer)?;
    Ok(i64::from_le_bytes(buffer))
}

#[inline(always)]
pub fn read_bytes<R: Read>(reader: &mut R, count: usize) -> Result<Vec<u8>, Error> {
    let mut buffer = vec![0; count];
    let bytes_read = reader.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    Ok(buffer)
}

#[inline(always)]
pub fn read_into<R: Read>(reader: &mut R, buffer: &mut [u8]) -> Result<(), Error> {
    reader.read_exact(buffer)?;
    Ok(())
}
