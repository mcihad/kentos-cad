//! The container (docs/specs/kcad-v2.md §2–§4): the fixed header, the
//! required extensions, the payload and the SHA-256 trailer over everything
//! before it. Reading checks in the specification's order and hands back the
//! payload only when the whole file is intact.

use sha2::{Digest, Sha256};

use crate::error::{Code, KcadError};
use crate::watch::{HASH_CHUNK, Quiet, Step, Watch, report};

/// `\x89KCAD\r\n\x1a\n` (§3.1).
pub const MAGIC: [u8; 9] = [0x89, b'K', b'C', b'A', b'D', 0x0d, 0x0a, 0x1a, 0x0a];
/// The part of the signature that says “KCAD” even when a text transfer changed the rest.
pub(crate) const MAGIC_PREFIX: [u8; 5] = [0x89, b'K', b'C', b'A', b'D'];
/// The fixed part of the header (§3).
pub const FIXED_HEADER: usize = 36;
/// The container version this crate writes and reads.
pub const MAJOR: u8 = 2;
pub const MINOR: u8 = 0;
/// `encoding` 1: KentOS CBOR profile 1 with document schema 2 (§3.3).
pub const ENCODING_CBOR_PROFILE_1: u8 = 1;
/// `codec` 0: none, the payload is stored as it is (§3.3).
pub const CODEC_NONE: u8 = 0;
/// The SHA-256 trailer (§3.7).
pub const HASH_SIZE: usize = 32;
/// The largest payload, stored or decoded (§3.4).
pub const MAX_PAYLOAD: u64 = 1 << 30;
const MAX_EXTENSIONS: u16 = 64;
const MAX_EXTENSION_NAME: usize = 64;

/// What the header of a readable file says (`inspect`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub major: u8,
    pub minor: u8,
    pub min_reader_minor: u8,
    pub header_length: u16,
    pub encoding: u8,
    pub codec: u8,
    pub payload_length: u64,
    /// The SHA-256 of the header and the payload: the file's last 32 bytes.
    pub sha256: [u8; HASH_SIZE],
}

/// The file around a payload, as a 2.0 writer writes it: no codec, no flags, no extensions.
pub(crate) fn write(payload: &[u8]) -> Result<Vec<u8>, KcadError> {
    let length = payload.len() as u64;
    if length > MAX_PAYLOAD {
        return Err(KcadError::new(
            Code::TooLarge,
            format!(
                "Çizim KCAD 2 olarak yazılamıyor: kodlanmış hâli {length} bayt, bir dosya en çok {MAX_PAYLOAD} bayt taşır. Çizimi birkaç dosyaya bölün."
            ),
        ));
    }
    let mut out = Vec::with_capacity(FIXED_HEADER + payload.len() + HASH_SIZE);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&[MAJOR, MINOR, 0]);
    out.extend_from_slice(&(FIXED_HEADER as u16).to_le_bytes());
    out.extend_from_slice(&[ENCODING_CBOR_PROFILE_1, CODEC_NONE]);
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(payload);
    // In chunks, as the reader hashes: many short calls, which a browser's WebAssembly engine
    // optimises, not one long one it would run in its baseline code (docs/adr/0030).
    let mut hasher = Sha256::new();
    for chunk in out.chunks(HASH_CHUNK) {
        hasher.update(chunk);
    }
    let hash = hasher.finalize();
    out.extend_from_slice(&hash);
    Ok(out)
}

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn u64_at(data: &[u8], at: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&data[at..at + 8]);
    u64::from_le_bytes(b)
}

fn bad_header(what: &str) -> KcadError {
    KcadError::new(
        Code::BadHeader,
        format!(
            "Dosya başlığı geçersiz: {what}. Dosya bozuk ya da KCAD 2 kurallarına uymayan bir programla yazılmış; özgün dosyayı ya da bir yedeği açın."
        ),
    )
}

/// Checks the container in the specification's order (§4, steps 1–12) and
/// returns the header and the payload.
pub fn read(data: &[u8]) -> Result<(Header, &[u8]), KcadError> {
    read_watched(data, &mut Quiet)
}

/// `read`, the integrity check reported to `watch` a few megabytes at a time
/// (it is the longest part of reading the container) and stopped when it says so.
pub(crate) fn read_watched<'d>(
    data: &'d [u8],
    watch: &mut dyn Watch,
) -> Result<(Header, &'d [u8]), KcadError> {
    let size = data.len();
    if size == 0 {
        return Err(KcadError::new(
            Code::Empty,
            "Dosya boş; içinde çizim yok. Başka bir dosya seçin ya da bir yedeği açın.",
        ));
    }
    if !data.starts_with(&MAGIC) {
        if data.starts_with(&MAGIC_PREFIX) {
            if size < MAGIC.len() && MAGIC.starts_with(data) {
                return Err(truncated(MAGIC.len() as u64, size));
            }
            return Err(KcadError::new(
                Code::DamagedSignature,
                "Dosyanın KCAD imzası bozuk: dosya metin olarak aktarılmış (satır sonları çevrilmiş) olabilir. Özgün dosyayı ikili (binary) olarak yeniden kopyalayın ya da indirin.",
            ));
        }
        return Err(KcadError::new(
            Code::NotKcad,
            "KentOS çizim dosyası (KCAD v2) değil: dosyanın başında KCAD imzası yok. DXF ve koordinat listeleri Dosya → İçe aktar ile açılır.",
        ));
    }
    if size < FIXED_HEADER {
        return Err(truncated(FIXED_HEADER as u64, size));
    }
    let (major, minor, min_reader) = (data[9], data[10], data[11]);
    if major != MAJOR {
        let which = if major > MAJOR {
            "daha yeni"
        } else {
            "tanımsız"
        };
        return Err(KcadError::new(
            Code::UnsupportedVersion,
            format!(
                "Bu dosya KCAD {major} biçiminde ({which} bir sürüm); bu KentOS yalnız KCAD {MAJOR}'yi okur. KentOS'u güncelleyin ya da dosyayı yazan sürümle açın."
            ),
        ));
    }
    if min_reader > MINOR {
        return Err(KcadError::new(
            Code::NewerVersion,
            format!(
                "Bu dosya KCAD {MAJOR}.{min_reader} özellikleri kullanıyor; bu KentOS {MAJOR}.{MINOR}'ı okuyor. Dosyayı açmak için KentOS'u güncelleyin."
            ),
        ));
    }
    let header_length = u16_at(data, 12);
    let (encoding, codec) = (data[14], data[15]);
    let payload_length = u64_at(data, 16);
    let decoded_length = u64_at(data, 24);
    let flags = u16_at(data, 32);
    let count = u16_at(data, 34);
    if usize::from(header_length) < FIXED_HEADER {
        return Err(bad_header(&format!(
            "başlık uzunluğu {header_length}, en az {FIXED_HEADER} olmalı"
        )));
    }
    if payload_length > MAX_PAYLOAD || decoded_length > MAX_PAYLOAD {
        return Err(KcadError::new(
            Code::TooLarge,
            format!(
                "Dosyanın yükü {} bayt; KCAD 2 en çok {MAX_PAYLOAD} bayt taşır. Dosya bozuk olabilir; özgün dosyayı ya da bir yedeği açın.",
                payload_length.max(decoded_length)
            ),
        ));
    }
    // Cannot overflow: a u16, a value ≤ 2³⁰ and 32.
    let total = u64::from(header_length) + payload_length + HASH_SIZE as u64;
    if (size as u64) < total {
        return Err(truncated(total, size));
    }
    if size as u64 > total {
        return Err(KcadError::new(
            Code::TrailingData,
            format!(
                "Dosyanın sonunda {} fazla bayt var: dosya başka bir veriyle birleştirilmiş ya da bozulmuş. Özgün dosyayı yeniden alın.",
                size as u64 - total
            ),
        ));
    }
    let body = &data[..size - HASH_SIZE];
    let mut sha256 = [0u8; HASH_SIZE];
    sha256.copy_from_slice(&data[size - HASH_SIZE..]);
    let mut hasher = Sha256::new();
    let all = body.len() as u64;
    let mut done = 0u64;
    for chunk in body.chunks(HASH_CHUNK) {
        report(watch, Step::Checking { done, total: all })?;
        hasher.update(chunk);
        done += chunk.len() as u64;
    }
    report(watch, Step::Checking { done, total: all })?;
    if hasher.finalize()[..] != sha256[..] {
        return Err(KcadError::new(
            Code::HashMismatch,
            "Dosya bozuk: bütünlük özeti (SHA-256) tutmuyor. Dosya diskte ya da aktarımda bozulmuş; özgün kopyayı ya da bir yedeği açın.",
        ));
    }
    if minor < min_reader {
        return Err(bad_header(&format!(
            "küçük sürüm {minor}, en az okuyucu sürümünden ({min_reader}) küçük"
        )));
    }
    if flags != 0 {
        return Err(bad_header(&format!("bilinmeyen bayraklar {flags:#06x}")));
    }
    if count > MAX_EXTENSIONS {
        return Err(bad_header(&format!(
            "{count} zorunlu uzantı; en çok {MAX_EXTENSIONS}"
        )));
    }
    let header_end = usize::from(header_length);
    let mut extensions = Vec::new();
    let mut at = FIXED_HEADER;
    for _ in 0..count {
        if at >= header_end {
            return Err(bad_header("uzantı listesi başlığın dışına taşıyor"));
        }
        let length = usize::from(data[at]);
        let end = at + 1 + length;
        let name = data.get(at + 1..end).filter(|_| end <= header_end);
        let valid = |n: &[u8]| {
            (1..=MAX_EXTENSION_NAME).contains(&n.len())
                && n.iter().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'_' | b'-')
                })
        };
        match name {
            Some(n) if valid(n) => extensions.push(String::from_utf8_lossy(n).into_owned()),
            _ => return Err(bad_header("uzantı adı geçersiz")),
        }
        at = end;
    }
    if at != header_end {
        return Err(bad_header("başlık uzunluğu uzantı listesiyle bitmiyor"));
    }
    if encoding != ENCODING_CBOR_PROFILE_1 {
        return Err(KcadError::new(
            Code::UnknownEncoding,
            format!(
                "Dosyanın yük kodlaması ({encoding}) bu KentOS'ta tanımsız. KentOS'u güncelleyin."
            ),
        ));
    }
    if codec != CODEC_NONE {
        return Err(KcadError::new(
            Code::UnknownCodec,
            format!(
                "Dosya bu KentOS'un tanımadığı bir sıkıştırmayla ({codec}) yazılmış. KentOS'u güncelleyin."
            ),
        ));
    }
    if decoded_length != payload_length {
        return Err(bad_header(
            "sıkıştırmasız dosyada çözülmüş uzunluk yük uzunluğuna eşit olmalı",
        ));
    }
    if !extensions.is_empty() {
        return Err(KcadError::new(
            Code::UnknownExtension,
            format!(
                "Dosya bu KentOS'un tanımadığı zorunlu uzantılar kullanıyor: {}. Eksik ya da yanlış gösterilmemesi için açılmadı; KentOS'u güncelleyin.",
                extensions.join(", ")
            ),
        ));
    }
    let start = header_end;
    // `total` fits `size`, so the payload's end does too.
    let payload = &data[start..start + payload_length as usize];
    Ok((
        Header {
            major,
            minor,
            min_reader_minor: min_reader,
            header_length,
            encoding,
            codec,
            payload_length,
            sha256,
        },
        payload,
    ))
}

fn truncated(expected: u64, size: usize) -> KcadError {
    KcadError::new(
        Code::Truncated,
        format!(
            "Dosya eksik: en az {expected} bayt olmalıydı, {size} bayt var. Dosya yarım kopyalanmış ya da indirilmiş olabilir; özgün dosyayı yeniden alın."
        ),
    )
}
