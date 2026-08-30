use crate::image::F32Image;
use ndarray::s;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};

pub const HEADER_RECORD_SIZE: usize = 80;
pub const HEADER_BLOCK_SIZE: usize = 2880;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitsHeader {
    pub cards: HashMap<String, String>,
}

impl FitsHeader {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.cards.get(key).map(|s| s.as_str())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.cards.insert(key.to_string(), value.to_string());
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.trim().parse().ok())
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.trim().parse().ok())
    }

    pub fn imagetyp(&self) -> Option<&str> {
        self.get("IMAGETYP")
    }

    pub fn exptime(&self) -> Option<f64> {
        self.get_f64("EXPTIME")
    }

    pub fn filter(&self) -> Option<&str> {
        self.get("FILTER")
    }

    pub fn date_obs(&self) -> Option<&str> {
        self.get("DATE-OBS")
    }

    pub fn ccd_temp(&self) -> Option<f64> {
        self.get_f64("CCD-TEMP")
    }

    pub fn bayerpat(&self) -> Option<&str> {
        self.get("BAYERPAT")
    }

    pub fn xbayroff(&self) -> Option<i64> {
        self.get_i64("XBAYROFF")
    }

    pub fn ybayroff(&self) -> Option<i64> {
        self.get_i64("YBAYROFF")
    }

    pub fn naxis(&self) -> Option<i64> {
        self.get_i64("NAXIS")
    }

    pub fn naxis1(&self) -> Option<i64> {
        self.get_i64("NAXIS1")
    }

    pub fn naxis2(&self) -> Option<i64> {
        self.get_i64("NAXIS2")
    }

    pub fn naxis3(&self) -> Option<i64> {
        self.get_i64("NAXIS3")
    }

    pub fn bitpix(&self) -> Option<i64> {
        self.get_i64("BITPIX")
    }
}

impl Default for FitsHeader {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_header(data: &[u8]) -> Result<FitsHeader, FitsError> {
    let mut header = FitsHeader::new();
    let mut offset = 0;

    loop {
        if offset + HEADER_RECORD_SIZE > data.len() {
            return Err(FitsError::TruncatedHeader);
        }

        let record = &data[offset..offset + HEADER_RECORD_SIZE];
        let record_str = String::from_utf8_lossy(record);

        if record_str.trim().is_empty() || record_str.starts_with("END") {
            break;
        }

        if record_str.len() >= 8 {
            let key = record_str[..8].trim().to_string();
            if !key.is_empty() {
                let value = if record_str.len() > 10 {
                    let val_part = &record_str[10..];
                    let val = val_part.trim_start_matches('=').trim();
                    val.trim_matches('\'').trim().to_string()
                } else {
                    String::new()
                };
                header.set(&key, &value);
            }
        }

        offset += HEADER_RECORD_SIZE;
        if offset % HEADER_BLOCK_SIZE == 0 {
            let block_end = &data[offset..offset.min(data.len())];
            if block_end.iter().all(|b| *b == b' ') || block_end.is_empty() {
                break;
            }
        }
    }

    Ok(header)
}

pub fn write_header(header: &FitsHeader, writer: &mut impl Write) -> Result<(), FitsError> {
    let mut records: Vec<String> = Vec::new();

    records.push(format_card("SIMPLE", "T"));
    records.push(format_card("BITPIX", "-32"));

    let naxis = header.naxis().unwrap_or(2);
    records.push(format_card("NAXIS", &naxis.to_string()));

    if let Some(n1) = header.naxis1() {
        records.push(format_card("NAXIS1", &n1.to_string()));
    }
    if let Some(n2) = header.naxis2() {
        records.push(format_card("NAXIS2", &n2.to_string()));
    }
    if let Some(n3) = header.naxis3() {
        records.push(format_card("NAXIS3", &n3.to_string()));
    }

    for (key, value) in &header.cards {
        if !matches!(key.as_str(), "SIMPLE" | "BITPIX" | "NAXIS" | "NAXIS1" | "NAXIS2" | "NAXIS3") {
            records.push(format_card(key, value));
        }
    }

    records.push("END".to_string());

    let mut block = Vec::new();
    for record in &records {
        let mut card = format!("{:<80}", record);
        card.truncate(HEADER_RECORD_SIZE);
        block.extend_from_slice(card.as_bytes());
    }

    while block.len() % HEADER_BLOCK_SIZE != 0 {
        block.push(b' ');
    }

    writer.write_all(&block)?;
    Ok(())
}

fn format_card(key: &str, value: &str) -> String {
    let key_padded = format!("{:<8}", key);
    if value.eq_ignore_ascii_case("T") || value.eq_ignore_ascii_case("F") {
        format!("{}= {}", key_padded, value)
    } else if value.parse::<f64>().is_ok() || value.parse::<i64>().is_ok() {
        format!("{}= {:>20}", key_padded, value)
    } else {
        format!("{}= '{}'", key_padded, value)
    }
}

pub fn read_f32_image(
    data: &[u8],
    header: &FitsHeader,
) -> Result<F32Image, FitsError> {
    let width = header.naxis1().ok_or(FitsError::MissingAxis("NAXIS1"))? as usize;
    let height = header.naxis2().ok_or(FitsError::MissingAxis("NAXIS2"))? as usize;
    let channels = header.naxis3().unwrap_or(1) as usize;

    let data_offset = find_data_offset(data)?;
    let expected = width * height * channels * 4;
    let available = data.len().saturating_sub(data_offset);

    if available < expected {
        return Err(FitsError::TruncatedData {
            expected,
            available,
        });
    }

    let mut image = F32Image::new(width, height, channels);
    let raw = &data[data_offset..data_offset + expected];

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                let idx = ((c * height + y) * width + x) * 4;
                let bytes: [u8; 4] = raw[idx..idx + 4].try_into().unwrap();
                let val = f32::from_be_bytes(bytes);
                image[(c, y, x)] = val;
            }
        }
    }

    Ok(image)
}

pub fn write_f32_image(
    header: &FitsHeader,
    image: &F32Image,
    writer: &mut impl Write,
) -> Result<(), FitsError> {
    write_header(header, writer)?;

    let channels = image.channels();
    let height = image.height();
    let width = image.width();

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                let val = image[(c, y, x)];
                writer.write_all(&val.to_be_bytes())?;
            }
        }
    }

    let data_len = channels * height * width * 4;
    let padding = (HEADER_BLOCK_SIZE - (data_len % HEADER_BLOCK_SIZE)) % HEADER_BLOCK_SIZE;
    if padding > 0 {
        let zeros = vec![0u8; padding];
        writer.write_all(&zeros)?;
    }

    Ok(())
}

fn find_data_offset(data: &[u8]) -> Result<usize, FitsError> {
    let mut offset = 0;
    loop {
        if offset >= data.len() {
            return Err(FitsError::TruncatedHeader);
        }
        let block_end = (offset + HEADER_BLOCK_SIZE).min(data.len());
        let block = &data[offset..block_end];
        for i in (0..block.len()).step_by(HEADER_RECORD_SIZE) {
            let record = &block[i..i.min(block.len())];
            let record_str = String::from_utf8_lossy(record);
            if record_str.starts_with("END") {
                return Ok(offset + HEADER_BLOCK_SIZE);
            }
        }
        offset += HEADER_BLOCK_SIZE;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FitsError {
    #[error("Truncated FITS header")]
    TruncatedHeader,
    #[error("Truncated FITS data: expected {expected} bytes, got {available}")]
    TruncatedData { expected: usize, available: usize },
    #[error("Missing axis info: {0}")]
    MissingAxis(&'static str),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_header_parse_and_write_roundtrip() {
        let mut header = FitsHeader::new();
        header.set("IMAGETYP", "LIGHT");
        header.set("EXPTIME", "120");
        header.set("FILTER", "Ha");
        header.set("DATE-OBS", "2026-01-15T03:00:00");
        header.set("CCD-TEMP", "-10.5");
        header.set("NAXIS1", "1920");
        header.set("NAXIS2", "1080");
        header.set("NAXIS", "2");
        header.set("BITPIX", "-32");

        let mut buf = Vec::new();
        write_header(&header, &mut buf).unwrap();

        let parsed = parse_header(&buf).unwrap();
        assert_eq!(parsed.imagetyp(), Some("LIGHT"));
        assert_eq!(parsed.exptime(), Some(120.0));
        assert_eq!(parsed.filter(), Some("Ha"));
        assert_eq!(parsed.ccd_temp(), Some(-10.5));
        assert_eq!(parsed.naxis1(), Some(1920));
        assert_eq!(parsed.naxis2(), Some(1080));
    }

    #[test]
    fn test_f32_image_roundtrip() {
        let width = 4;
        let height = 3;
        let image = F32Image::new(width, height, 1);
        let mut image = image;
        for y in 0..height {
            for x in 0..width {
                image[(0, y, x)] = (y * width + x) as f32;
            }
        }

        let mut header = FitsHeader::new();
        header.set("NAXIS1", &width.to_string());
        header.set("NAXIS2", &height.to_string());
        header.set("NAXIS3", "1");
        header.set("NAXIS", "3");
        header.set("BITPIX", "-32");

        let mut buf = Vec::new();
        write_f32_image(&header, &image, &mut buf).unwrap();

        let parsed_header = parse_header(&buf).unwrap();
        let parsed_image = read_f32_image(&buf, &parsed_header).unwrap();

        assert_eq!(parsed_image.width(), width);
        assert_eq!(parsed_image.height(), height);
        for y in 0..height {
            for x in 0..width {
                assert_eq!(parsed_image[(0, y, x)], image[(0, y, x)]);
            }
        }
    }

    #[test]
    fn test_header_helper_methods() {
        let mut header = FitsHeader::new();
        header.set("IMAGETYP", "BIAS");
        header.set("EXPTIME", "0");
        header.set("BAYERPAT", "RGGB");
        header.set("XBAYROFF", "0");
        header.set("YBAYROFF", "0");

        assert_eq!(header.imagetyp(), Some("BIAS"));
        assert_eq!(header.exptime(), Some(0.0));
        assert_eq!(header.bayerpat(), Some("RGGB"));
        assert_eq!(header.xbayroff(), Some(0));
        assert_eq!(header.ybayroff(), Some(0));
    }
}
