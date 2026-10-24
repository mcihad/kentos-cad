//! The block codecs Parquet pages use (docs/adr/0628 §1), our own: Snappy both ways, LZ4, GZIP (`miniz_oxide`'s
//! inflate) and Zstandard decoding. Every decoder is given the size the page says it opens to and refuses more.

pub mod gzip;
pub mod lz4;
pub mod snappy;
pub mod zstd;

/// A codec as Parquet names it (`CompressionCodec`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    None,
    Snappy,
    Gzip,
    Lzo,
    Brotli,
    Lz4,
    Zstd,
    Lz4Raw,
}

impl Codec {
    pub fn from_parquet(code: i32) -> Option<Codec> {
        Some(match code {
            0 => Codec::None,
            1 => Codec::Snappy,
            2 => Codec::Gzip,
            3 => Codec::Lzo,
            4 => Codec::Brotli,
            5 => Codec::Lz4,
            6 => Codec::Zstd,
            7 => Codec::Lz4Raw,
            _ => return None,
        })
    }

    pub fn parquet(self) -> i32 {
        match self {
            Codec::None => 0,
            Codec::Snappy => 1,
            Codec::Gzip => 2,
            Codec::Lzo => 3,
            Codec::Brotli => 4,
            Codec::Lz4 => 5,
            Codec::Zstd => 6,
            Codec::Lz4Raw => 7,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Codec::None => "UNCOMPRESSED",
            Codec::Snappy => "SNAPPY",
            Codec::Gzip => "GZIP",
            Codec::Lzo => "LZO",
            Codec::Brotli => "BROTLI",
            Codec::Lz4 => "LZ4",
            Codec::Zstd => "ZSTD",
            Codec::Lz4Raw => "LZ4_RAW",
        }
    }
}

/// `input` opened by `codec` to exactly `size` bytes.
pub fn decompress(codec: Codec, input: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let out = match codec {
        Codec::None => input.to_vec(),
        Codec::Snappy => snappy::decompress(input, size)?,
        Codec::Gzip => gzip::decompress(input, size)?,
        Codec::Zstd => zstd::decompress(input, size)?,
        Codec::Lz4Raw => lz4::decompress_block(input, size)?,
        Codec::Lz4 => lz4::decompress_legacy(input, size)?,
        Codec::Lzo | Codec::Brotli => {
            return Err(format!(
                "{} sıkıştırması açılmaz; dosyayı Snappy, ZSTD, GZIP ya da LZ4 ile yeniden yazın",
                codec.name()
            ));
        }
    };
    if out.len() != size {
        return Err(format!(
            "{} sayfası {} bayt açıldı, beklenen {size}",
            codec.name(),
            out.len()
        ));
    }
    Ok(out)
}
