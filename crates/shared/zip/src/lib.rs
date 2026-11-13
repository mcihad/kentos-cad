//! Zip archives in (docs/adr/0053): the files of a zipped Shapefile, a `.kek` extension package (docs/adr/8057 §1). Only
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
    /// "Version made by": its high byte is the host that wrote the entry (3: Unix).
    made_by: u16,
    /// The external attributes: on a Unix host the file's mode in the high 16 bits.
    external: u32,
    /// The name as written held a backslash (made `/` in `name`).
    backslash: bool,
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

    /// A folder's entry (its name ends in `/`).
    pub fn is_folder(&self) -> bool {
        self.name.ends_with('/')
    }

    /// A Mac's resource fork or `__MACOSX` folder: not a file of the archive's.
    pub fn is_mac_junk(&self) -> bool {
        self.name.starts_with("__MACOSX/")
            || self.name.contains("/._")
            || self.name.starts_with("._")
    }

    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /// The entry's Unix mode, when a Unix host wrote it.
    pub fn unix_mode(&self) -> Option<u32> {
        (self.made_by >> 8 == 3).then_some(self.external >> 16)
    }

    /// A symbolic link (a Unix host's `S_IFLNK`): its bytes are the link's target, not a file.
    pub fn is_link(&self) -> bool {
        self.unix_mode()
            .is_some_and(|mode| mode & 0o170_000 == 0o120_000)
    }

    /// The name as written held a backslash; `name` has it as `/`.
    pub fn had_backslash(&self) -> bool {
        self.backslash
    }

    /// The bytes it takes in the archive, as declared.
    pub fn packed(&self) -> u64 {
        self.packed
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
    Ok(list_every(bytes, limits)?
        .into_iter()
        .filter(|l| !l.is_folder() && !l.is_mac_junk())
        .collect())
}

/// Every entry of the central directory as the archive names it, folders and a Mac's resource forks too: a `.kek`
/// package refuses what the Shapefile reader skips (docs/adr/8057 §1).
pub fn list_every(bytes: &[u8], limits: &Limits) -> Result<Vec<Listed>, String> {
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
        let made_by = field(4)?;
        let flags = field(8)?;
        let method = field(10)?;
        let crc = word(16)?;
        let packed = word(20)?;
        let unpacked = word(24)?;
        let name_len = usize::from(field(28)?);
        let extra_len = usize::from(field(30)?);
        let comment_len = usize::from(field(32)?);
        let external = word(38)?;
        let local = word(42)?;
        let name_bytes = bytes
            .get(at + 46..at + 46 + name_len)
            .ok_or_else(|| broken("dosya adı kesik"))?;
        // Bit 11: the name is UTF-8; otherwise it is IBM 437, as ASCII names are in both.
        let backslash = name_bytes.contains(&b'\\');
        let name = String::from_utf8_lossy(name_bytes).replace('\\', "/");
        at += 46 + name_len + extra_len + comment_len;
        if packed == 0xffff_ffff || unpacked == 0xffff_ffff || local == 0xffff_ffff {
            return Err(
                "Zip64 arşivi okunmuyor (4 GB'tan büyük dosya). Dosyaları sıkıştırmadan birlikte seçin."
                    .to_owned(),
            );
        }
        out.push(Listed {
            name,
            packed: u64::from(packed),
            unpacked: u64::from(unpacked),
            method,
            crc,
            encrypted: flags & 1 != 0,
            local: u64::from(local),
            made_by,
            external,
            backslash,
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

/// What makes two readers see the archive differently, when anything does: bytes before the first entry, a local
/// header whose name is not the directory's, two entries over the same bytes. One reads the directory (this crate,
/// the central directory's names), another walks the local headers from the start; a `.kek` package must read the
/// same in a reviewer's tool and in KentOS (docs/adr/8057 §1). The listing as [`list_every`] gives it.
pub fn layout_problem(bytes: &[u8], listed: &[Listed]) -> Option<String> {
    let mut spans: Vec<(u64, u64, &str)> = Vec::with_capacity(listed.len());
    for l in listed {
        let name = l.name.as_str();
        let broken = || Some(format!("“{name}” girdisinin yerel kaydı okunamadı."));
        let Ok(at) = usize::try_from(l.local) else {
            return broken();
        };
        if u32_at(bytes, at) != Some(LOCAL) {
            return broken();
        }
        let (Some(flags), Some(name_len), Some(extra_len)) = (
            u16_at(bytes, at + 6),
            u16_at(bytes, at + 26),
            u16_at(bytes, at + 28),
        ) else {
            return broken();
        };
        let (name_len, extra_len) = (usize::from(name_len), usize::from(extra_len));
        let Some(local_name) = bytes.get(at + 30..at + 30 + name_len) else {
            return broken();
        };
        if String::from_utf8_lossy(local_name).replace('\\', "/") != l.name {
            return Some(format!(
                "“{name}” girdisinin yerel kaydında başka bir ad yazıyor (“{}”); arşivi okuyan araçlar farklı \
                 dosyalar görür.",
                String::from_utf8_lossy(local_name)
            ));
        }
        // The data descriptor after the bytes, when bit 3 says so: 12 bytes, 16 with its signature.
        let mut end = (at + 30 + name_len + extra_len) as u64 + l.packed;
        if flags & 8 != 0 {
            end += 12;
            if usize::try_from(end - 12)
                .ok()
                .and_then(|e| u32_at(bytes, e))
                == Some(0x0807_4b50)
            {
                end += 4;
            }
        }
        spans.push((l.local, end, name));
    }
    spans.sort_unstable();
    if let Some((start, _, _)) = spans.first()
        && *start != 0
    {
        return Some(format!(
            "Arşivin başında {start} bayt başka veri var (ör. bir program ya da başka bir dosya); paket ZIP olarak \
             başlamalı."
        ));
    }
    spans.windows(2).find(|w| w[0].1 > w[1].0).map(|w| {
        format!(
            "“{}” ile “{}” girdileri aynı baytları paylaşıyor; arşiv olağan dışı kurulmuş.",
            w[0].2, w[1].2
        )
    })
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

/// How an entry is packed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pack {
    /// Deflated, or stored when that is no smaller.
    Best,
    /// Deflated at a lighter level, or stored when that is no smaller: a
    /// workbook's sheet of tens of megabytes packs three times as fast for a
    /// tenth more bytes (docs/adr/0223 §8).
    Fast,
    /// Stored as it is (an OpenDocument's mimetype, docs/adr/0223 §2).
    Stored,
}

/// A zip archive of `entries` (name, bytes), each deflated (stored when that
/// is no smaller), its name in UTF-8 (the language flag set), dated 1 January
/// 1980 so the same entries always make the same bytes (KMZ, docs/adr/0222 §4).
/// No ZIP64: an entry or the archive past 4 GiB is refused.
pub fn write(entries: &[(&str, &[u8])]) -> Result<Vec<u8>, String> {
    let packed: Vec<(&str, &[u8], Pack)> =
        entries.iter().map(|(n, d)| (*n, *d, Pack::Best)).collect();
    write_with(&packed)
}

/// `write` with each entry's packing given.
pub fn write_with(entries: &[(&str, &[u8], Pack)]) -> Result<Vec<u8>, String> {
    const UTF8: u16 = 0x0800;
    // DOS date of 1 January 1980, time 00:00.
    const DATE: u16 = (1 << 5) | 1;
    let big = |n: usize| {
        u32::try_from(n).map_err(|_| "ZIP dosyası 4 GiB'ı aşıyor; yazılmadı.".to_owned())
    };
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data, pack) in entries {
        let deflated = match pack {
            Pack::Best => miniz_oxide::deflate::compress_to_vec(data, 6),
            Pack::Fast => miniz_oxide::deflate::compress_to_vec(data, 3),
            Pack::Stored => Vec::new(),
        };
        let (method, packed): (u16, &[u8]) = if *pack != Pack::Stored && deflated.len() < data.len()
        {
            (8, &deflated)
        } else {
            (0, data)
        };
        let crc = crc32(data);
        let local = big(out.len())?;
        let fields = |out: &mut Vec<u8>| -> Result<(), String> {
            out.extend(20u16.to_le_bytes());
            out.extend(UTF8.to_le_bytes());
            out.extend(method.to_le_bytes());
            out.extend(0u16.to_le_bytes());
            out.extend(DATE.to_le_bytes());
            out.extend(crc.to_le_bytes());
            out.extend(big(packed.len())?.to_le_bytes());
            out.extend(big(data.len())?.to_le_bytes());
            out.extend(
                u16::try_from(name.len())
                    .map_err(|_| "ZIP girdisinin adı çok uzun.".to_owned())?
                    .to_le_bytes(),
            );
            out.extend(0u16.to_le_bytes());
            Ok(())
        };
        out.extend(LOCAL.to_le_bytes());
        fields(&mut out)?;
        out.extend(name.as_bytes());
        out.extend(packed);
        central.extend(CENTRAL.to_le_bytes());
        central.extend(20u16.to_le_bytes());
        fields(&mut central)?;
        // Comment length, disk, internal and external attributes, the local header's place.
        central.extend([0u8; 10]);
        central.extend(local.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let offset = big(out.len())?;
    let count = u16::try_from(entries.len()).map_err(|_| "ZIP'te çok fazla girdi.".to_owned())?;
    out.extend(&central);
    out.extend(EOCD.to_le_bytes());
    out.extend([0u8; 4]);
    out.extend(count.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(big(central.len())?.to_le_bytes());
    out.extend(offset.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    Ok(out)
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
    fn written_archives_read_back_and_repeat_bit_for_bit() {
        let text = "<kml>Ada 1244 İÇŞĞÜÖ</kml>".repeat(50);
        let entries: [(&str, &[u8]); 2] =
            [("doc.kml", text.as_bytes()), ("files/ç.bin", &[7, 8, 9])];
        let z = write(&entries).expect("writes");
        assert_eq!(write(&entries).expect("writes"), z);
        let got = all(&z).expect("reads");
        assert_eq!(got.len(), 2);
        assert_eq!(
            (got[0].name.as_str(), got[0].data.as_slice()),
            ("doc.kml", text.as_bytes())
        );
        assert_eq!(
            (got[1].name.as_str(), got[1].data.as_slice()),
            ("files/ç.bin", &[7u8, 8, 9][..])
        );
        assert!(z.len() < text.len());
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

    /// The archive with `prefix` before it, its offsets moved to match: what a self-extracting program looks like.
    fn after(prefix: &[u8], z: &[u8]) -> Vec<u8> {
        let shift = prefix.len() as u32;
        let mut out = [prefix, z].concat();
        let end = out.len() - 22;
        let cd = u32_at(&out, end + 16).unwrap() + shift;
        out[end + 16..end + 20].copy_from_slice(&cd.to_le_bytes());
        let mut at = cd as usize;
        while u32_at(&out, at) == Some(CENTRAL) {
            let local = u32_at(&out, at + 42).unwrap() + shift;
            out[at + 42..at + 46].copy_from_slice(&local.to_le_bytes());
            let len = |o: usize| usize::from(u16_at(&out, at + o).unwrap());
            at += 46 + len(28) + len(30) + len(32);
        }
        out
    }

    #[test]
    fn an_archive_two_readers_would_read_differently_is_named() {
        let limits = Limits::default();
        let entries: [(&str, &[u8]); 2] = [("a.txt", b"bir"), ("b/c.txt", b"iki")];
        let z = write(&entries).unwrap();
        assert_eq!(layout_problem(&z, &list_every(&z, &limits).unwrap()), None);
        let hand = archive(&entries, true);
        assert_eq!(
            layout_problem(&hand, &list_every(&hand, &limits).unwrap()),
            None
        );

        let prefixed = after(b"MZ\x90\x00 a program", &z);
        let listed = list_every(&prefixed, &limits).expect("still a readable archive");
        assert_eq!(
            unpack(&prefixed, &listed, &limits, |_| true).unwrap().len(),
            2
        );
        assert!(
            layout_problem(&prefixed, &listed)
                .unwrap()
                .contains("başında 14 bayt")
        );

        let mut renamed = z.clone();
        renamed[30..35].copy_from_slice(b"x.txt");
        let listed = list_every(&renamed, &limits).unwrap();
        assert!(
            layout_problem(&renamed, &listed)
                .unwrap()
                .contains("“x.txt”")
        );

        // The second entry's directory record points at the first's bytes.
        let twice = archive(&[("a.txt", b"bir"), ("a.txt", b"bir")], false);
        let mut overlapping = twice.clone();
        let cd = u32_at(&overlapping, overlapping.len() - 22 + 16).unwrap() as usize;
        let second = cd + 46 + 5;
        overlapping[second + 42..second + 46].copy_from_slice(&0u32.to_le_bytes());
        let listed = list_every(&overlapping, &limits).unwrap();
        assert!(
            layout_problem(&overlapping, &listed)
                .unwrap()
                .contains("aynı baytları")
        );
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
