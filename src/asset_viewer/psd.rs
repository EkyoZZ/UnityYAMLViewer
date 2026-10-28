//! Read-only PSD v1 composite preview. Layout follows Adobe's PSD specification.
use std::{io::{Read, Seek, SeekFrom}, path::Path};

pub fn load(path: &Path) -> Result<image::RgbaImage, String> {
    decode(std::fs::File::open(path).map_err(|error| error.to_string())?)
}

struct Reader<R> { input: R, length: u64 }
impl<R: Read + Seek> Reader<R> {
    fn bytes(&mut self, count: usize) -> Result<Vec<u8>, String> {
        let mut bytes = vec![0; count];
        self.input.read_exact(&mut bytes).map_err(|_| "Truncated PSD data".to_owned())?;
        Ok(bytes)
    }
    fn u16(&mut self) -> Result<u16, String> {
        let bytes = self.bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
    fn u32(&mut self) -> Result<u32, String> {
        let bytes = self.bytes(4)?;
        Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
    }
    fn position(&mut self) -> Result<u64, String> { self.input.stream_position().map_err(|error| error.to_string()) }
    fn seek(&mut self, position: u64) -> Result<(), String> {
        if position > self.length { return Err("Truncated PSD section".into()); }
        self.input.seek(SeekFrom::Start(position)).map_err(|error| error.to_string())?;
        Ok(())
    }
    fn skip_section(&mut self) -> Result<(), String> {
        let length = self.u32()? as u64;
        let position = self.position()?;
        self.seek(position + length)
    }
}

fn decode<R: Read + Seek>(mut input: R) -> Result<image::RgbaImage, String> {
    let length = input.seek(SeekFrom::End(0)).map_err(|error| error.to_string())?;
    input.seek(SeekFrom::Start(0)).map_err(|error| error.to_string())?;
    let mut reader = Reader { input, length };
    if reader.bytes(4)? != b"8BPS" || reader.u16()? != 1 {
        return Err("Only PSD version 1 is supported (PSB is not supported)".into());
    }
    reader.bytes(6)?;
    let channels = reader.u16()? as usize;
    let height = reader.u32()?;
    let width = reader.u32()?;
    let depth = reader.u16()?;
    let mode = reader.u16()?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 || !(1..=56).contains(&channels) {
        return Err("PSD dimensions exceed the preview limit (8192 × 8192) or channel count is invalid".into());
    }
    if depth != 8 || !matches!(mode, 1 | 3) {
        return Err("PSD preview supports 8-bit RGB or grayscale only".into());
    }
    let colors = if mode == 3 { 3 } else { 1 };
    if channels < colors { return Err("PSD is missing color channels".into()); }
    reader.skip_section()?; // color mode data
    reader.skip_section()?; // image resources; ICC color management is not applied
    let layer_length = reader.u32()? as u64;
    let layer_start = reader.position()?;
    let layer_end = layer_start + layer_length;
    let mut has_alpha = false;
    if layer_length >= 6 {
        let info_length = reader.u32()? as u64;
        if info_length > layer_length - 4 { return Err("Invalid PSD layer section".into()); }
        if info_length >= 2 { has_alpha = (reader.u16()? as i16) < 0; }
    }
    reader.seek(layer_end)?;
    if has_alpha && channels <= colors { return Err("PSD is missing merged transparency".into()); }
    let compression = reader.u16()?;
    if compression > 1 { return Err("PSD preview supports Raw and RLE compression only; ZIP is not supported".into()); }
    let mut row_lengths = Vec::new();
    if compression == 1 {
        for _ in 0..channels * height as usize { row_lengths.push(reader.u16()? as usize); }
    }
    let mut output = image::RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 255]));
    for channel in 0..channels {
        let keep = channel < colors || (has_alpha && channel == colors);
        for y in 0..height as usize {
            let count = if compression == 0 { width as usize } else { row_lengths[channel * height as usize + y] };
            if !keep {
                let position = reader.position()?;
                reader.seek(position + count as u64)?;
                continue;
            }
            let bytes = reader.bytes(count)?;
            let row = if compression == 0 { bytes } else { unpack_row(&bytes, width as usize)? };
            for (x, value) in row.into_iter().enumerate() {
                let pixel = output.get_pixel_mut(x as u32, y as u32);
                if channel == colors { pixel[3] = value; }
                else if mode == 1 { pixel[0] = value; pixel[1] = value; pixel[2] = value; }
                else { pixel[channel] = value; }
            }
        }
    }
    Ok(output)
}

fn unpack_row(bytes: &[u8], width: usize) -> Result<Vec<u8>, String> {
    let mut row = Vec::with_capacity(width);
    let mut position = 0;
    while position < bytes.len() {
        let control = bytes[position] as i8;
        position += 1;
        if control >= 0 {
            let count = control as usize + 1;
            if position + count > bytes.len() || row.len() + count > width { return Err("Invalid PSD RLE row".into()); }
            row.extend_from_slice(&bytes[position..position + count]);
            position += count;
        } else if control != -128 {
            let count = (1i16 - control as i16) as usize;
            if position >= bytes.len() || row.len() + count > width { return Err("Invalid PSD RLE run".into()); }
            row.resize(row.len() + count, bytes[position]);
            position += 1;
        }
    }
    if row.len() != width { return Err("Incomplete PSD RLE row".into()); }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(compression: u16, data: &[u8]) -> Vec<u8> {
        let mut bytes = b"8BPS\0\x01\0\0\0\0\0\0\0\x03".to_vec();
        bytes.extend(1u32.to_be_bytes()); bytes.extend(2u32.to_be_bytes());
        bytes.extend(8u16.to_be_bytes()); bytes.extend(3u16.to_be_bytes());
        for _ in 0..3 { bytes.extend(0u32.to_be_bytes()); }
        bytes.extend(compression.to_be_bytes()); bytes.extend(data); bytes
    }
    #[test]
    fn raw_rgb_and_rle_composite_decode() {
        let raw = decode(std::io::Cursor::new(fixture(0, &[255, 0, 0, 255, 10, 20]))).unwrap();
        assert_eq!(raw.get_pixel(0, 0).0, [255, 0, 10, 255]);
        let rle = decode(std::io::Cursor::new(fixture(1, &[0,3,0,3,0,3, 1,255,0, 1,0,255, 1,10,20]))).unwrap();
        assert_eq!(raw, rle);
    }
    #[test]
    fn malformed_runs_and_missing_composite_are_rejected() {
        assert!(unpack_row(&[127, 1], 2).is_err());
        assert!(unpack_row(&[254, 1], 2).is_err());
        assert!(decode(std::io::Cursor::new(fixture(0, &[1]))).is_err());
        assert!(decode(std::io::Cursor::new(fixture(2, &[]))).is_err());
    }

    #[test]
    fn merged_transparency_and_unsupported_depth() {
        let mut bytes = fixture(0, &[255, 0, 0, 255, 10, 20, 128, 0]);
        bytes[12..14].copy_from_slice(&4u16.to_be_bytes());
        bytes.splice(34..38, [0,0,0,6, 0,0,0,2, 255,255]);
        let image = decode(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 10, 128]);
        assert_eq!(image.get_pixel(1, 0).0, [0, 255, 20, 0]);
        let mut bytes = fixture(0, &[]);
        bytes[22..24].copy_from_slice(&16u16.to_be_bytes());
        assert!(decode(std::io::Cursor::new(bytes)).is_err());
    }
}
