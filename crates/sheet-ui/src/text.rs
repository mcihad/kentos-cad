//! Search text: Turkish lower case (İ → i, I → ı), then the Turkish letters
//! folded to their Latin ones, so “isik” finds “Işık” and “IŞIK” alike.

pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'İ' | 'I' | 'ı' => vec!['i'],
            'Ğ' | 'ğ' => vec!['g'],
            'Ü' | 'ü' => vec!['u'],
            'Ş' | 'ş' => vec!['s'],
            'Ö' | 'ö' => vec!['o'],
            'Ç' | 'ç' => vec!['c'],
            other => other.to_lowercase().collect(),
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Turkish lower case (İ → i, I → ı), as the web's `toLocaleLowerCase('tr')`.
pub fn lower(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'İ' => vec!['i'],
            'I' => vec!['ı'],
            other => other.to_lowercase().collect(),
        })
        .collect()
}

/// Today's date, ISO (the program's clock, UTC; a template's dates when the host gave none).
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil(i64::try_from(secs / 86_400).unwrap_or(0));
    format!("{y:04}-{m:02}-{d:02}")
}

/// A day number since 1970-01-01 as a civil date (H. Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    #[test]
    fn days_are_civil_dates() {
        assert_eq!(super::civil(0), (1970, 1, 1));
        assert_eq!(super::civil(20_728), (2026, 10, 2));
        assert_eq!(super::civil(-1), (1969, 12, 31));
        assert_eq!(super::today().len(), 10);
    }

    #[test]
    fn turkish_letters_fold() {
        assert_eq!(super::fold(" IŞIK Ölçeği "), "isik olcegi");
        assert_eq!(super::fold("İmar"), super::fold("imar"));
    }
}
