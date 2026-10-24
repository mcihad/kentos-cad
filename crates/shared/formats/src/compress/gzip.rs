//! GZIP (RFC 1952; docs/adr/0628 §1), Parquet's GZIP codec: members of a header, a deflate stream (`miniz_oxide`) and
//! the CRC-32 and size of what it holds, checked.

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::{DecompressorOxide, decompress as inflate, inflate_flags};

/// The bytes the gzip members of `input` hold, exactly `size` of them.
pub fn decompress(input: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let mut out = vec![0u8; size];
    let mut written = 0usize;
    let mut i = 0usize;
    while i < input.len() {
        let start = i;
        let head = input.get(i..i + 10).ok_or("GZIP başlığı yarım")?;
        if head[0] != 0x1f || head[1] != 0x8b || head[2] != 8 {
            return Err("GZIP başlığı tanınmıyor".into());
        }
        let flags = head[3];
        i += 10;
        if flags & 0x04 != 0 {
            let b = input.get(i..i + 2).ok_or("GZIP ek alanı yarım")?;
            i += 2 + usize::from(u16::from_le_bytes([b[0], b[1]]));
        }
        for flag in [0x08u8, 0x10] {
            if flags & flag != 0 {
                let end = input
                    .get(i..)
                    .and_then(|r| r.iter().position(|&b| b == 0))
                    .ok_or("GZIP başlığının adı yarım")?;
                i += end + 1;
            }
        }
        if flags & 0x02 != 0 {
            i += 2;
        }
        let stream = input.get(i..).ok_or("GZIP akışı yarım")?;
        let mut state = DecompressorOxide::new();
        let (status, used, made) = inflate(
            &mut state,
            stream,
            &mut out,
            written,
            inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
        );
        if status != TINFLStatus::Done {
            return Err(format!("GZIP akışı açılamadı ({status:?})"));
        }
        let member = &out[written..written + made];
        i += used;
        let tail = input.get(i..i + 8).ok_or("GZIP sonu yarım")?;
        let crc = u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        let length = u32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]);
        if crate::zip::crc32(member) != crc || length != made as u32 {
            return Err("GZIP sağlaması tutmuyor".into());
        }
        written += made;
        i += 8;
        if i <= start {
            return Err("GZIP üyesi ilerlemiyor".into());
        }
    }
    if written != size {
        return Err(format!("GZIP {written} bayt açıldı, beklenen {size}"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255];
        out.extend(miniz_oxide::deflate::compress_to_vec(data, 6));
        out.extend_from_slice(&crate::zip::crc32(data).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out
    }

    #[test]
    fn members_open_and_a_wrong_sum_is_refused() {
        let a = "parsel ".repeat(300);
        let b = "ada ".repeat(100);
        let mut both = gzip(a.as_bytes());
        both.extend(gzip(b.as_bytes()));
        let got = decompress(&both, a.len() + b.len()).unwrap();
        assert_eq!(got, [a.as_bytes(), b.as_bytes()].concat());
        let mut bad = gzip(a.as_bytes());
        let n = bad.len();
        bad[n - 8] ^= 1;
        assert!(decompress(&bad, a.len()).is_err());
    }
}
