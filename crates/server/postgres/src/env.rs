//! Local settings file (`.env.local` at the repository root, written by
//! `kentosd db-setup`, never committed): `KEY=VALUE` lines, `#` comments.
//! A variable already set in the process environment wins over the file.

use std::collections::BTreeMap;
use std::path::Path;

/// The file's variables; an absent file is empty, a malformed line is an error naming it.
pub fn read_file(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(format!("{} okunamadı: {e}", path.display())),
    };
    parse(&text).map_err(|(line, why)| format!("{}:{line}: {why}", path.display()))
}

pub fn parse(text: &str) -> Result<BTreeMap<String, String>, (usize, &'static str)> {
    let mut out = BTreeMap::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or((i + 1, "KEY=VALUE bekleniyor"))?;
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err((i + 1, "geçersiz değişken adı"));
        }
        out.insert(key.to_string(), value.trim().to_string());
    }
    Ok(out)
}

/// A setting: the process environment first, then the file.
pub fn lookup(file: &BTreeMap<String, String>, key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| file.get(key).cloned())
}

/// Writes the file with owner-only permissions (it holds database passwords).
pub fn write_file(
    path: &Path,
    vars: &BTreeMap<String, String>,
    header: &str,
) -> std::io::Result<()> {
    let mut text = String::new();
    for line in header.lines() {
        text.push_str("# ");
        text.push_str(line);
        text.push('\n');
    }
    for (k, v) in vars {
        text.push_str(&format!("{k}={v}\n"));
    }
    std::fs::write(path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pairs_and_skips_comments() {
        let v = parse("# yorum\nA=1\n\n B = x=y \n").unwrap();
        assert_eq!(v["A"], "1");
        assert_eq!(v["B"], "x=y");
        assert_eq!(parse("A=1\nbozuk\n"), Err((2, "KEY=VALUE bekleniyor")));
        assert_eq!(parse("A-B=1"), Err((1, "geçersiz değişken adı")));
    }
}
