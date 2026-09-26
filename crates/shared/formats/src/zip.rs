//! Zip archives in (docs/adr/0053): the files of a zipped Shapefile. Only
//! what GIS programs write is read: stored and deflated entries in one
//! archive, no encryption, no ZIP64. The central directory is the index;
//! each entry's local header is checked against it and its bytes against
//! its CRC-32. A hostile archive cannot fill memory: the entries, the bytes
//! unpacked in all and each entry's unpack ratio are bounded ([`Limits`]),
//! and the inflater stops at the size the directory declares. Nothing here
//! panics on any input.

/// How much an archive may hold.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Entries in the central directory.
    pub entries: usize,
    /// Bytes unpacked in all.
    pub total: u64,
    /// Bytes an entry may unpack to per byte packed (a zip bomb's tell).
    pub ratio: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 1000,
            total: 512 * 1024 * 1024,
            ratio: 1000,
        }
    }
}

/// An entry of the central directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// The name as the archive writes it, folders included (`katmanlar/yollar.shp`).
    pub name: String,
    packed: u64,
    unpacked: u64,
    method: u16,
    crc: u32,
    encrypted: bool,
    local: u64,
}

impl Listed {
    /// The name without its folders (`yollar.shp`).
    pub fn file_name(&self) -> &str {
        self.name.rsplit('/').next().unwrap_or(&self.name)
    }

    /// The size it unpacks to, as declared.
    pub fn size(&self) -> u64 {
        self.unpacked
    }
}

/// An unpacked entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub data: Vec<u8>,
}

const EOCD: u32 = 0x0605_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Whether the bytes start as a zip archive does (a local header or an empty archive).
pub fn sniff(bytes: &[u8]) -> bool {
    matches!(u32_at(bytes, 0), Some(LOCAL | EOCD))
}

/// The archive's entries, folders and macOS resource forks left out,
/// without unpacking anything.
pub fn list(bytes: &[u8], limits: &Limits) -> Result<Vec<Listed>, String> {
    let broken = |why: &str| format!("Zip arşivi bozuk: {why}. Arşivi yeniden oluşturup deneyin.");
    // The end of central directory record: the last 22 bytes plus a comment of up to 64 KiB.
    let lowest = bytes.len().saturating_sub(22 + 0xffff);
    let end = (lowest..=bytes.len().saturating_sub(22))
        .rev()
        .find(|&at| u32_at(bytes, at) == Some(EOCD))
        .ok_or_else(|| broken("sonundaki dizin kaydı bulunamadı"))?;
    let disk = u16_at(bytes, end + 4).unwrap_or(1);
    let cd_disk = u16_at(bytes, end + 6).unwrap_or(1);
    let here = u16_at(bytes, end + 8).unwrap_or(0);
    let count = u16_at(bytes, end + 10).unwrap_or(0);
    let size = u32_at(bytes, end + 12).unwrap_or(0);
    let offset = u32_at(bytes, end + 16).unwrap_or(0);
    if count == 0xffff || size == 0xffff_ffff || offset == 0xffff_ffff {
        return Err(
            "Zip64 arşivi okunmuyor (4 GB'tan büyük ya da 65535'ten çok dosyalı arşiv). Dosyaları sıkıştırmadan birlikte seçin."
                .to_owned(),
        );
    }
    if disk != 0 || cd_disk != 0 || here != count {
        return Err(
            "Birden çok parçaya bölünmüş zip arşivi okunmuyor. Arşivi tek parça olarak yeniden oluşturun."
                .to_owned(),
        );
    }
    if usize::from(count) > limits.entries {
        return Err(format!(
            "Zip arşivinde {count} dosya var; en çok {} dosya okunur.",
            limits.entries
        ));
    }
    let start = offset as usize;
    if start.checked_add(size as usize).is_none_or(|e| e > end) {
        return Err(broken("dizin, dosyanın dışını gösteriyor"));
    }
    let mut out = Vec::with_capacity(usize::from(count));
    let mut at = start;
    for _ in 0..count {
        if u32_at(bytes, at) != Some(CENTRAL) {
            return Err(broken("dizin kaydı beklenen yerde değil"));
        }
        let field = |o: usize| u16_at(bytes, at + o).ok_or_else(|| broken("dizin kaydı kesik"));
        let word = |o: usize| u32_at(bytes, at + o).ok_or_else(|| broken("dizin kaydı kesik"));
        let flags = field(8)?;
        let method = field(10)?;
        let crc = word(16)?;
        let packed = word(20)?;
        let unpacked = word(24)?;
        let name_len = usize::from(field(28)?);
        let extra_len = usize::from(field(30)?);
        let comment_len = usize::from(field(32)?);
        let local = word(42)?;
        let name_bytes = bytes
            .get(at + 46..at + 46 + name_len)
            .ok_or_else(|| broken("dosya adı kesik"))?;
        // Bit 11: the name is UTF-8; otherwise it is IBM 437, as ASCII names are in both.
        let name = String::from_utf8_lossy(name_bytes).replace('\\', "/");
        at += 46 + name_len + extra_len + comment_len;
        if packed == 0xffff_ffff || unpacked == 0xffff_ffff || local == 0xffff_ffff {
            return Err(
                "Zip64 arşivi okunmuyor (4 GB'tan büyük dosya). Dosyaları sıkıştırmadan birlikte seçin."
                    .to_owned(),
            );
        }
        if name.ends_with('/')
            || name.starts_with("__MACOSX/")
            || name.contains("/._")
            || name.starts_with("._")
        {
            continue;
        }
        out.push(Listed {
            name,
            packed: u64::from(packed),
            unpacked: u64::from(unpacked),
            method,
            crc,
            encrypted: flags & 1 != 0,
            local: u64::from(local),
        });
    }
    Ok(out)
}

/// Unpacks the listed entries `want` accepts, each checked against its CRC-32.
pub fn unpack(
    bytes: &[u8],
    listed: &[Listed],
    limits: &Limits,
    want: impl Fn(&Listed) -> bool,
) -> Result<Vec<Entry>, String> {
    let mut total: u64 = 0;
    let mut out = Vec::new();
    for l in listed.iter().filter(|l| want(l)) {
        let name = l.file_name();
        if l.encrypted {
            return Err(format!(
                "“{name}” zip içinde şifreli; şifreli arşiv okunmuyor. Şifresiz arşivleyin ya da dosyaları sıkıştırmadan seçin."
            ));
        }
        total = total.saturating_add(l.unpacked);
        if total > limits.total {
            return Err(format!(
                "Zip arşivindeki dosyalar açılınca {} MB'ı geçiyor; bu kadar büyük arşiv okunmuyor.",
                limits.total / (1024 * 1024)
            ));
        }
        if l.method == 8 && l.unpacked > l.packed.saturating_mul(limits.ratio).max(1024) {
            return Err(format!(
                "“{name}” zip içinde olağan dışı sıkıştırılmış ({} bayttan {} bayta); zip bombası olabileceği için açılmadı.",
                l.packed, l.unpacked
            ));
        }
        let broken = |why: &str| {
            format!("“{name}” zip içinde bozuk: {why}. Arşivi yeniden oluşturup deneyin.")
        };
        let at = usize::try_from(l.local).map_err(|_| broken("yerel kayıt dosyanın dışında"))?;
        if u32_at(bytes, at) != Some(LOCAL) {
            return Err(broken("yerel kayıt beklenen yerde değil"));
        }
        let name_len =
            usize::from(u16_at(bytes, at + 26).ok_or_else(|| broken("yerel kayıt kesik"))?);
        let extra_len =
            usize::from(u16_at(bytes, at + 28).ok_or_else(|| broken("yerel kayıt kesik"))?);
        let from = at + 30 + name_len + extra_len;
        let packed = usize::try_from(l.packed).map_err(|_| broken("boyut okunamadı"))?;
        let data = bytes
            .get(from..from.saturating_add(packed))
            .ok_or_else(|| broken("dosya kesik"))?;
        let unpacked = usize::try_from(l.unpacked).map_err(|_| broken("boyut okunamadı"))?;
        let data = match l.method {
            0 if data.len() == unpacked => data.to_vec(),
            0 => return Err(broken("sıkıştırmasız boyut tutmuyor")),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(data, unpacked)
                .map_err(|_| broken("açılamadı ya da bildirdiğinden büyük"))?,
            other => {
                return Err(format!(
                    "“{name}” zip içinde desteklenmeyen yöntemle sıkıştırılmış ({other}); yalnız “deflate” ve sıkıştırmasız okunur."
                ));
            }
        };
        if data.len() != unpacked {
            return Err(broken("açılan boyut bildirilenle tutmuyor"));
        }
        if crc32(&data) != l.crc {
            return Err(broken("sağlama (CRC-32) tutmuyor"));
        }
        out.push(Entry {
            name: l.name.clone(),
            data,
        });
    }
    Ok(out)
}

/// CRC-32 (IEEE 802.3, the zip format's), bit by bit over a table built once.
pub fn crc32(data: &[u8]) -> u32 {
    const fn table() -> [u32; 256] {
        let mut t = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            t[i] = c;
            i += 1;
        }
        t
    }
    const TABLE: [u32; 256] = table();
    !data
        .iter()
        .fold(!0u32, |c, &b| TABLE[usize::from((c as u8) ^ b)] ^ (c >> 8))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zip archive of these entries, stored or deflated (the format by hand, so every field can be broken).
    pub(crate) fn archive(entries: &[(&str, &[u8])], deflate: bool) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data) in entries {
            let packed = if deflate {
                miniz_oxide::deflate::compress_to_vec(data, 6)
            } else {
                data.to_vec()
            };
            let local = out.len() as u32;
            let method: u16 = if deflate { 8 } else { 0 };
            let head = |sig: u32, out: &mut Vec<u8>| {
                out.extend(sig.to_le_bytes());
            };
            head(LOCAL, &mut out);
            out.extend(20u16.to_le_bytes());
            out.extend(0u16.to_le_bytes());
            out.extend(method.to_le_bytes());
            out.extend([0u8; 4]);
            out.extend(crc32(data).to_le_bytes());
            out.extend((packed.len() as u32).to_le_bytes());
            out.extend((data.len() as u32).to_le_bytes());
            out.extend((name.len() as u16).to_le_bytes());
            out.extend(0u16.to_le_bytes());
            out.extend(name.as_bytes());
            out.extend(&packed);
            head(CENTRAL, &mut central);
            central.extend(20u16.to_le_bytes());
            central.extend(20u16.to_le_bytes());
            central.extend(0u16.to_le_bytes());
            central.extend(method.to_le_bytes());
            central.extend([0u8; 4]);
            central.extend(crc32(data).to_le_bytes());
            central.extend((packed.len() as u32).to_le_bytes());
            central.extend((data.len() as u32).to_le_bytes());
            central.extend((name.len() as u16).to_le_bytes());
            central.extend([0u8; 12]);
            central.extend(local.to_le_bytes());
            central.extend(name.as_bytes());
        }
        let offset = out.len() as u32;
        out.extend(&central);
        out.extend(EOCD.to_le_bytes());
        out.extend([0u8; 4]);
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((central.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out
    }

    fn all(bytes: &[u8]) -> Result<Vec<Entry>, String> {
        let limits = Limits::default();
        let listed = list(bytes, &limits)?;
        unpack(bytes, &listed, &limits, |_| true)
    }

    #[test]
    fn stored_and_deflated_entries_come_back_whole() {
        let text = "Ada 1244, parsel 12: İÇŞĞÜÖ çşğüöı".repeat(40);
        for deflate in [false, true] {
            let z = archive(
                &[("a/b.txt", text.as_bytes()), ("c.bin", &[0, 1, 2, 255])],
                deflate,
            );
            assert!(sniff(&z));
            let got = all(&z).expect("reads");
            assert_eq!(got.len(), 2);
            assert_eq!(got[0].name, "a/b.txt");
            assert_eq!(got[0].data, text.as_bytes());
            assert_eq!(got[1].data, [0, 1, 2, 255]);
        }
    }

    #[test]
    fn crc_32_is_the_zip_formats() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn a_broken_or_hostile_archive_is_refused_never_a_panic() {
        let z = archive(&[("x.txt", &[7u8; 5000])], true);
        // Every cut of the archive.
        for n in 0..z.len() {
            let _ = all(&z[..n]);
        }
        // Every byte flipped.
        for i in 0..z.len() {
            let mut bad = z.clone();
            bad[i] ^= 0x5a;
            let _ = all(&bad);
        }
        // A wrong checksum.
        let mut bad = z.clone();
        let crc_at = z.len() - 22 - (46 + 5) + 16;
        bad[crc_at] ^= 1;
        assert!(all(&bad).unwrap_err().contains("CRC-32"));
    }

    #[test]
    fn a_zip_bomb_encryption_and_zip64_are_refused() {
        let zeros = vec![0u8; 8 * 1024 * 1024];
        let bomb = archive(&[("sifir.dbf", &zeros)], true);
        assert!(
            all(&bomb).unwrap_err().contains("zip bombası"),
            "8 MiB of zeros packs to a few KiB"
        );
        let mut locked = archive(&[("a.shp", b"x")], false);
        let central = locked.len() - 22 - (46 + 5);
        locked[central + 8] |= 1;
        assert!(all(&locked).unwrap_err().contains("şifreli"));
        let mut z64 = archive(&[("a.shp", b"x")], false);
        let end = z64.len() - 22;
        z64[end + 10..end + 12].copy_from_slice(&0xffffu16.to_le_bytes());
        assert!(all(&z64).unwrap_err().contains("Zip64"));
    }

    #[test]
    fn folders_and_macos_forks_are_not_files() {
        let z = archive(
            &[
                ("katman/", b""),
                ("__MACOSX/katman/._a.shp", b"x"),
                ("katman/._a.shp", b"x"),
                ("katman/a.shp", b"shape"),
            ],
            false,
        );
        let listed = list(&z, &Limits::default()).expect("lists");
        let names: Vec<&str> = listed.iter().map(Listed::file_name).collect();
        assert_eq!(names, ["a.shp"]);
    }

    #[test]
    fn too_many_entries_are_refused() {
        let entries: Vec<(String, Vec<u8>)> =
            (0..12).map(|i| (format!("{i}.txt"), vec![b'x'])).collect();
        let refs: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        let z = archive(&refs, false);
        let limits = Limits {
            entries: 10,
            ..Limits::default()
        };
        assert!(list(&z, &limits).unwrap_err().contains("12 dosya"));
    }
}
