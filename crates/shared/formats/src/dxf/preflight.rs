//! Structural and format validation before any entity is emitted (CAD-IO-09/19).
use super::lexer::Lexer;
use crate::watch::{STOPPED, Watch};

pub const MAX_BYTES: usize = 256 * 1024 * 1024;
pub const ASCII_VERSIONS: &[&str] = &[
    "AC1006", "AC1009", "AC1012", "AC1014", "AC1015", "AC1018", "AC1021", "AC1024", "AC1027",
    "AC1032",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadError {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ReadError {}
fn fail(code: &'static str, message: impl Into<String>) -> ReadError {
    ReadError {
        code,
        message: message.into(),
    }
}

pub fn check(bytes: &[u8], watch: &mut dyn Watch) -> Result<(), ReadError> {
    if bytes.len() > MAX_BYTES {
        return Err(fail(
            "cad_file_too_large",
            "DXF dosyası 256 MiB sınırını aşıyor; dosyayı bölün.",
        ));
    }
    if bytes.starts_with(b"AutoCAD Binary DXF") {
        return Err(fail(
            "unsupported_cad_encoding",
            "Bu dosya ikili (binary) DXF. ASCII DXF olarak kaydedin; binary DXF bu okuyucunun kapsamına alınmamıştır.",
        ));
    }
    if bytes.starts_with(b"AC1") {
        return Err(fail(
            "unsupported_cad_format",
            "Bu bir DWG dosyası; DWG içe aktar / file.import.dwg komutunu kullanın.",
        ));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return Err(fail(
            "unsupported_cad_encoding",
            "UTF-16 DXF desteklenmiyor; dosyayı ASCII DXF / UTF-8 olarak kaydedin.",
        ));
    }
    let malformed = |message| fail("import_unreadable", message);
    let mut lex = Lexer::new(bytes);
    let mut section = false;
    let mut name_expected = false;
    let mut eof = false;
    let mut seen = false;
    let mut blocks = 0u32;
    let mut tables = 0u32;
    let mut var = String::new();
    let mut version = String::new();
    let mut encoding = String::new();
    let mut count = 0u64;
    while let Some(p) = lex.next().map_err(malformed)? {
        count += 1;
        if count.is_multiple_of(2048) && !watch.step(0, 1000) {
            return Err(fail("cancelled", STOPPED));
        }
        if p.code == 999 {
            continue;
        }
        if eof {
            return Err(malformed(format!(
                "Satır {}: EOF sonrasında kayıt var; dosyayı kaynak yazılımda onarın.",
                p.line
            )));
        }
        if name_expected {
            if p.code != 2 {
                return Err(malformed(format!(
                    "Satır {}: SECTION ardından bölüm adı (2) bekleniyor.",
                    p.line
                )));
            }
            name_expected = false;
            continue;
        }
        if !section && p.code != 0 {
            return Err(malformed(format!(
                "Satır {}: bu bir DXF dosyası değil; SECTION bekleniyor.",
                p.line
            )));
        }
        if p.code == 9 {
            var = p.text().trim().to_uppercase();
        }
        if p.code == 1 && var == "$ACADVER" {
            version = p.text().trim().to_uppercase();
            var.clear();
        }
        if p.code == 3 && var == "$DWGCODEPAGE" {
            encoding = p.text().trim().to_uppercase();
            var.clear();
        }
        if p.code != 0 {
            continue;
        }
        match p.text().trim().to_uppercase().as_str() {
            "SECTION" if !section => {
                section = true;
                seen = true;
                name_expected = true;
            }
            "SECTION" => {
                return Err(malformed(format!(
                    "Satır {}: önceki bölüm kapatılmadan SECTION başladı.",
                    p.line
                )));
            }
            "ENDSEC" if section && blocks == 0 && tables == 0 => section = false,
            "ENDSEC" => {
                return Err(malformed(format!(
                    "Satır {}: bölüm, blok veya tablo kapanışı tutarsız.",
                    p.line
                )));
            }
            "BLOCK" => blocks += 1,
            "ENDBLK" if blocks > 0 => blocks -= 1,
            "ENDBLK" => return Err(malformed("ENDBLK için açık BLOCK yok.".into())),
            "TABLE" => tables += 1,
            "ENDTAB" if tables > 0 => tables -= 1,
            "ENDTAB" => return Err(malformed("ENDTAB için açık TABLE yok.".into())),
            "EOF" if !section && blocks == 0 && tables == 0 => eof = true,
            "EOF" => return Err(malformed("EOF öncesinde kapanmamış DXF bölümü var.".into())),
            _ if !section => {
                return Err(malformed(format!(
                    "Satır {}: kayıt bir DXF bölümünün dışında.",
                    p.line
                )));
            }
            _ => {}
        }
    }
    if !seen || !eof || section || name_expected {
        return Err(malformed("DXF dosyası tamamlanmamış: kapalı SECTION ve son EOF kaydı gerekli; kaynak yazılımda yeniden kaydedin.".into()));
    }
    if !version.is_empty() && !ASCII_VERSIONS.contains(&version.as_str()) {
        return Err(fail(
            "unsupported_cad_version",
            format!(
                "DXF sürümü {version} desteklenmiyor; AC1032 veya AC1021 ASCII DXF olarak kaydedin."
            ),
        ));
    }
    if version.as_str() < "AC1021"
        && !encoding.is_empty()
        && !matches!(
            encoding.as_str(),
            "ANSI_1252" | "ANSI_1254" | "UTF-8" | "UTF8"
        )
    {
        return Err(fail(
            "unsupported_cad_encoding",
            format!(
                "DXF kod sayfası {encoding} desteklenmiyor; UTF-8 AC1021 veya sonraki ASCII DXF biçiminde kaydedin."
            ),
        ));
    }
    if (version.as_str() >= "AC1021" || matches!(encoding.as_str(), "UTF-8" | "UTF8"))
        && std::str::from_utf8(bytes).is_err()
    {
        return Err(fail(
            "unsupported_cad_encoding",
            "UTF-8 olarak bildirilen DXF geçersiz bayt içeriyor; kaynak dosyanın kodlamasını düzeltin.",
        ));
    }
    if !watch.step(0, 1000) {
        return Err(fail("cancelled", STOPPED));
    }
    Ok(())
}
