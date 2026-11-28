//! Altlık modeller (docs/adr/8101; plan/ifc-uyum.md IFC-09, plan/bim-plan.md B18): other disciplines' IFC models a
//! project shows read-only under its drawing. A record names the file as it is — its path, its SHA-256 (the revision
//! shown) and size, its schema — with the discipline and revision the user gives it, how it is placed (by its map
//! conversion onto the project's system, or at the origin by the user's choice) and whether it is shown. The model is
//! never copied into the drawing: its bodies are read from the file each time it is shown (`settings.arch.underlays`,
//! Mimari data 18).
//!
//! The file reader checks the structure here; the command reads the file (its digest, its schema, its placement).

use serde::{Deserialize, Serialize};

/// The most underlays a drawing keeps.
pub const MAX_UNDERLAYS: usize = 50;
/// The most characters of an underlay's name, discipline or revision.
pub const MAX_UNDERLAY_LABEL: usize = 200;
/// The most characters of an underlay's path.
pub const MAX_UNDERLAY_PATH: usize = 1000;

/// How an underlay is placed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum UnderlayPlacement {
    /// By its map conversion onto the project's system (Harita dönüşümüyle).
    #[default]
    Map,
    /// Its metres as the drawing's, by the user's choice (Başlangıç noktasına).
    Origin,
}

impl UnderlayPlacement {
    pub fn label(self) -> &'static str {
        match self {
            UnderlayPlacement::Map => "Harita dönüşümüyle",
            UnderlayPlacement::Origin => "Başlangıç noktasına",
        }
    }

    fn is_map(&self) -> bool {
        *self == UnderlayPlacement::Map
    }
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

/// One underlay model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelUnderlay {
    /// Lowercase, hyphenated UUID; unique in the drawing.
    pub id: String,
    /// As the list shows it (the file's name, unless the user named it).
    pub name: String,
    /// The file, as the author's machine names it.
    pub file: String,
    /// The file's SHA-256 when it was read in, 64 lowercase hexadecimal digits: the revision shown.
    pub sha256: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    /// Its `FILE_SCHEMA` (`IFC4X3_ADD2`, `IFC4`, `IFC2X3`).
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discipline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "UnderlayPlacement::is_map")]
    pub placement: UnderlayPlacement,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub shown: bool,
}

/// Whether `text` is a lowercase, hyphenated UUID.
fn is_id(text: &str) -> bool {
    text.len() == 36
        && text.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_digit() || ('a'..='f').contains(&c),
        })
}

/// What is wrong with a label of at most `most` characters, when anything is.
fn label_problem(text: &str, most: usize) -> Option<String> {
    if text.trim().is_empty() {
        return Some("boş olamaz".into());
    }
    if text.trim() != text {
        return Some("başında ya da sonunda boşluk olamaz".into());
    }
    if text.chars().count() > most {
        return Some(format!("en çok {most} karakter olmalı"));
    }
    if text.chars().any(char::is_control) {
        return Some("denetim karakteri içeremez".into());
    }
    None
}

impl ModelUnderlay {
    /// The first problem of a list of underlays, with its path (`underlays.{i}.field`).
    pub fn list_problem(list: &[ModelUnderlay]) -> Option<(String, String)> {
        if list.len() > MAX_UNDERLAYS {
            return Some((
                "underlays".into(),
                format!("En çok {MAX_UNDERLAYS} altlık model tutulur."),
            ));
        }
        for (i, u) in list.iter().enumerate() {
            let at = |f: &str| format!("underlays.{i}.{f}");
            let n = i + 1;
            if !is_id(&u.id) {
                return Some((
                    at("id"),
                    format!("{n}. altlığın kimliği küçük harfli, tireli bir UUID olmalı."),
                ));
            }
            if list[..i].iter().any(|v| v.id == u.id) {
                return Some((
                    at("id"),
                    format!("{n}. altlığın kimliği başka bir altlıkta da var."),
                ));
            }
            if let Some(why) = label_problem(&u.name, MAX_UNDERLAY_LABEL) {
                return Some((at("name"), format!("{n}. altlığın adı {why}.")));
            }
            if let Some(why) = label_problem(&u.file, MAX_UNDERLAY_PATH) {
                return Some((at("file"), format!("{n}. altlığın dosyası {why}.")));
            }
            if u.sha256.len() != 64
                || !u
                    .sha256
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
            {
                return Some((
                    at("sha256"),
                    format!(
                        "{n}. altlığın dosyası SHA-256'sıyla (64 küçük onaltılık rakam) anılır."
                    ),
                ));
            }
            if let Some(why) = label_problem(&u.schema, 64) {
                return Some((at("schema"), format!("{n}. altlığın şeması {why}.")));
            }
            for (field, value, words) in [
                ("discipline", &u.discipline, "disiplini"),
                ("revision", &u.revision, "revizyonu"),
            ] {
                if let Some(why) = value
                    .as_deref()
                    .and_then(|v| label_problem(v, MAX_UNDERLAY_LABEL))
                {
                    return Some((at(field), format!("{n}. altlığın {words} {why}.")));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one() -> ModelUnderlay {
        ModelUnderlay {
            id: "0192f5a1-8101-7000-8000-000000000001".into(),
            name: "Statik model".into(),
            file: "/proje/statik.ifc".into(),
            sha256: "ab".repeat(32),
            bytes: 141_965,
            schema: "IFC4X3_ADD2".into(),
            discipline: Some("Statik".into()),
            revision: Some("R2".into()),
            placement: UnderlayPlacement::Map,
            shown: true,
        }
    }

    #[test]
    fn an_underlay_is_written_and_checked_as_a_file_by_its_digest() {
        let u = one();
        let json = serde_json::to_value(&u).unwrap();
        assert!(
            json.get("placement").is_none() && json.get("shown").is_none(),
            "the defaults are not written"
        );
        let back: ModelUnderlay = serde_json::from_value(json).unwrap();
        assert_eq!(back, u);
        let mut hidden = one();
        (hidden.shown, hidden.placement) = (false, UnderlayPlacement::Origin);
        let json = serde_json::to_value(&hidden).unwrap();
        assert_eq!(
            (json["shown"].clone(), json["placement"].clone()),
            (false.into(), "origin".into())
        );
        assert_eq!(ModelUnderlay::list_problem(&[one()]), None);
        let mut bad = one();
        bad.sha256 = "AB".repeat(32);
        assert_eq!(
            ModelUnderlay::list_problem(&[bad]).map(|p| p.0),
            Some("underlays.0.sha256".into())
        );
        let mut bad = one();
        bad.name = " Statik".into();
        assert_eq!(
            ModelUnderlay::list_problem(&[bad]).map(|p| p.0),
            Some("underlays.0.name".into())
        );
        assert_eq!(
            ModelUnderlay::list_problem(&[one(), one()]).map(|p| p.0),
            Some("underlays.1.id".into())
        );
        let mut bad = one();
        bad.discipline = Some(String::new());
        assert_eq!(
            ModelUnderlay::list_problem(&[bad]).map(|p| p.0),
            Some("underlays.0.discipline".into())
        );
    }
}
