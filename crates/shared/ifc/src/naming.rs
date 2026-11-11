//! The digital project regulation's names (Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında
//! Yönetmelik, RG 5.8.2026/33331, EK-5 §5.5; docs/adr/8047): `Disiplin-KategoriKodu-Açıklama`. The discipline two
//! letters (EK-2 Tablo 2.2), the category code three (EK-5 Tablo 5.1, [`crate::building::Class::code`]), the
//! description free: "alt kategori, fonksiyon, ölçü, malzeme", its fields joined by "_" with no white space, and no "-"
//! in it (EK-5 §5.5.3); a space's its number and its name (EK-5 §5.5.4, EK-3).

/// Mimarlık's code (EK-2 Tablo 2.2).
pub const ARCHITECTURE: &str = "MM";

/// A BIM model's file name without its extension (EK-2 Tablo 2.1): the project's code (EK-1), its building's, MM, no
/// item (XXXX, Tablo 2.3), Model (MD, Tablo 2.12), no type (XXX, Tablo 2.13: none of Mimari's types is a model), not
/// one storey's (NNN, Tablo 2.14), the first of its kind (01) and the revision, joined by "_":
/// `260123_00_MM_XXXX_MD_XXX_NNN_01_000`.
pub fn model_file(project: &str, building: &str, revision: &str) -> String {
    format!("{project}_{building}_{ARCHITECTURE}_XXXX_MD_XXX_NNN_01_{revision}")
}

/// A name's description (EK-5 §5.5.3): its fields joined by "_" with no white space, no "-" in it.
pub fn description(words: &str) -> String {
    let mut out = String::new();
    for w in words.split(|c: char| c.is_whitespace() || c == '-' || c == '_') {
        if w.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('_');
        }
        out.push_str(w);
    }
    out
}

/// An architectural object's name (EK-5 §5.5): MM, its category code, its description.
pub fn name(code: &str, words: &str) -> String {
    let d = description(words);
    if d.is_empty() {
        format!("{ARCHITECTURE}-{code}")
    } else {
        format!("{ARCHITECTURE}-{code}-{d}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_s_description_has_no_white_space_or_dash() {
        assert_eq!(name("DVR", "Dış duvar 25"), "MM-DVR-Dış_duvar_25");
        assert_eq!(name("MHL", "Z01 Yatak - odası"), "MM-MHL-Z01_Yatak_odası");
        assert_eq!(name("KAP", ""), "MM-KAP");
    }

    #[test]
    fn a_model_file_is_named_as_ek_2_names_it() {
        assert_eq!(
            model_file("260123", "00", "000"),
            "260123_00_MM_XXXX_MD_XXX_NNN_01_000"
        );
    }
}
