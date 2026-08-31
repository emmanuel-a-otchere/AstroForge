use crate::debayer::BayerPattern;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DngMetadata {
    pub cfa_pattern: Option<BayerPattern>,
    pub cfa_repeat_pattern_dim: Option<(u16, u16)>,
    pub black_level: Option<f32>,
    pub white_level: Option<f32>,
    pub width: u32,
    pub height: u32,
}

pub fn parse_dng_tags(data: &[u8]) -> DngMetadata {
    let width = read_tiff_tag_u32(data, 256).unwrap_or(0);
    let height = read_tiff_tag_u32(data, 257).unwrap_or(0);

    let cfa_pattern_raw = read_tiff_tag_bytes(data, 33422);
    let cfa_repeat_dim_raw = read_tiff_tag_bytes(data, 33421);

    let cfa_pattern = cfa_pattern_raw.as_ref().and_then(|raw| {
        if raw.len() >= 4 {
            let pattern_str = raw
                .iter()
                .map(|b| match b {
                    0 => 'R',
                    1 => 'G',
                    2 => 'B',
                    _ => '?',
                })
                .collect::<String>();
            BayerPattern::from_str(&pattern_str)
        } else if raw.len() >= 2 {
            let pattern_str = raw
                .iter()
                .map(|b| match b {
                    0 => 'R',
                    1 => 'G',
                    2 => 'B',
                    _ => '?',
                })
                .collect::<String>();
            let padded = format!("{}{}", pattern_str, pattern_str);
            BayerPattern::from_str(&padded[..4])
        } else {
            None
        }
    });

    let cfa_repeat_pattern_dim = cfa_repeat_dim_raw.as_ref().and_then(|raw| {
        if raw.len() >= 4 {
            let dim_x = u16::from_le_bytes([raw[0], raw[1]]);
            let dim_y = u16::from_le_bytes([raw[2], raw[3]]);
            Some((dim_x, dim_y))
        } else {
            None
        }
    });

    let black_level = read_tiff_tag_f32(data, 62476);
    let white_level = read_tiff_tag_u32(data, 65496).map(|v| v as f32);

    DngMetadata {
        cfa_pattern,
        cfa_repeat_pattern_dim,
        black_level,
        white_level,
        width,
        height,
    }
}

fn read_tiff_tag_u32(data: &[u8], tag: u16) -> Option<u32> {
    if data.len() < 8 {
        return None;
    }

    let byte_order = &data[0..2];
    let little_endian = byte_order == b"II";
    let read_u16 = |bytes: &[u8]| -> u16 {
        if little_endian {
            u16::from_le_bytes([bytes[0], bytes[1]])
        } else {
            u16::from_be_bytes([bytes[0], bytes[1]])
        }
    };
    let read_u32 = |bytes: &[u8]| -> u32 {
        if little_endian {
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        } else {
            u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        }
    };

    let ifd_offset = read_u32(&data[4..8]) as usize;
    if ifd_offset + 2 > data.len() {
        return None;
    }

    let num_entries = read_u16(&data[ifd_offset..ifd_offset + 2]) as usize;
    let entries_start = ifd_offset + 2;

    for i in 0..num_entries {
        let offset = entries_start + i * 12;
        if offset + 12 > data.len() {
            break;
        }
        let entry_tag = read_u16(&data[offset..offset + 2]);
        if entry_tag == tag {
            let type_id = read_u16(&data[offset + 2..offset + 4]);
            let count = read_u32(&data[offset + 4..offset + 8]);
            if type_id == 3 && count == 1 {
                let val = read_u16(&data[offset + 8..offset + 10]);
                return Some(val as u32);
            } else if type_id == 4 && count == 1 {
                return Some(read_u32(&data[offset + 8..offset + 12]));
            }
        }
    }

    None
}

fn read_tiff_tag_bytes(data: &[u8], tag: u16) -> Option<Vec<u8>> {
    if data.len() < 8 {
        return None;
    }

    let byte_order = &data[0..2];
    let little_endian = byte_order == b"II";
    let read_u16 = |bytes: &[u8]| -> u16 {
        if little_endian {
            u16::from_le_bytes([bytes[0], bytes[1]])
        } else {
            u16::from_be_bytes([bytes[0], bytes[1]])
        }
    };
    let read_u32 = |bytes: &[u8]| -> u32 {
        if little_endian {
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        } else {
            u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        }
    };

    let ifd_offset = read_u32(&data[4..8]) as usize;
    if ifd_offset + 2 > data.len() {
        return None;
    }

    let num_entries = read_u16(&data[ifd_offset..ifd_offset + 2]) as usize;
    let entries_start = ifd_offset + 2;

    for i in 0..num_entries {
        let offset = entries_start + i * 12;
        if offset + 12 > data.len() {
            break;
        }
        let entry_tag = read_u16(&data[offset..offset + 2]);
        if entry_tag == tag {
            let count = read_u32(&data[offset + 4..offset + 8]) as usize;
            let value_offset = read_u32(&data[offset + 8..offset + 12]) as usize;
            if count <= 4 {
                return Some(data[offset + 8..offset + 8 + count].to_vec());
            } else if value_offset + count <= data.len() {
                return Some(data[value_offset..value_offset + count].to_vec());
            }
        }
    }

    None
}

fn read_tiff_tag_f32(data: &[u8], tag: u16) -> Option<f32> {
    read_tiff_tag_u32(data, tag).map(|v| v as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_tiff(width: u32, height: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"II");
        buf.extend_from_slice(&42u16.to_le_bytes());
        buf.extend_from_slice(&8u32.to_le_bytes());

        let num_entries = 2u16;
        buf.extend_from_slice(&num_entries.to_le_bytes());

        buf.extend_from_slice(&256u16.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(width as u16).to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());

        buf.extend_from_slice(&257u16.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(height as u16).to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());

        buf.extend_from_slice(&0u32.to_le_bytes());
        buf
    }

    #[test]
    fn test_parse_dng_tags_dimensions() {
        let data = make_minimal_tiff(1920, 1080);
        let metadata = parse_dng_tags(&data);
        assert_eq!(metadata.width, 1920);
        assert_eq!(metadata.height, 1080);
    }

    #[test]
    fn test_parse_dng_tags_no_cfa() {
        let data = make_minimal_tiff(640, 480);
        let metadata = parse_dng_tags(&data);
        assert!(metadata.cfa_pattern.is_none());
        assert!(metadata.cfa_repeat_pattern_dim.is_none());
    }

    #[test]
    fn test_parse_dng_tags_empty_data() {
        let metadata = parse_dng_tags(&[]);
        assert_eq!(metadata.width, 0);
        assert_eq!(metadata.height, 0);
    }
}
