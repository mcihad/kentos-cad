//! The model quality control form's table in the drawing (Mimarlık ve Mühendislik Projelerinin Dijital Olarak
//! Hazırlanması Hakkında Yönetmelik Md. 15, EK-9 Tablo 9.1, Mimari; docs/adr/8047 §10): its header, by which a drawing's
//! table is known as the form. Its rows, the 21 questions answered, are the standards package's
//! (`dijital_proje.form.tablo`, docs/adr/8074), read from the file KentOS writes: the form's rules are Python's
//! (plan/bim-plan.md K10), what this crate gives them is the file's facts (`crate::read::file_facts`).

/// The form's table header: Tablo 9.1's columns and KentOS's note beside them. A drawing's table with this header is the
/// form (Mimari › Metraj ▾ › Model kalite kontrol formu places it, Ruhsat takımı writes its answers again); the
/// package's `BASLIK` is the same.
pub const FORM_HEADER: [&str; 6] = [
    "No",
    "Kategori / Disiplin",
    "Kontrol Başlığı",
    "Açıklama",
    "Evet / Hayır",
    "Not",
];

/// Whether a table's first row is the form's header.
pub fn is_form(header: &[String]) -> bool {
    header.len() == FORM_HEADER.len() && header.iter().zip(FORM_HEADER).all(|(a, b)| a == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The package writes the header this crate knows the form by.
    #[test]
    fn the_package_s_header_is_the_form_s() {
        let package =
            include_str!("../../../../catalog/standards/dijital-proje/src/dijital_proje/form.py");
        let line = package
            .lines()
            .find(|l| l.starts_with("BASLIK = ["))
            .expect("the package's header");
        let words: Vec<String> = FORM_HEADER.iter().map(|h| format!("\"{h}\"")).collect();
        assert_eq!(line, format!("BASLIK = [{}]", words.join(", ")));
        assert!(is_form(&FORM_HEADER.map(str::to_owned)));
        assert!(!is_form(&["No".to_owned()]));
    }
}
