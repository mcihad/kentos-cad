//! A covering (kaplama, docs/adr/8070 §10; EK-5 Tablo 5.1 KPL, EK-6 Tablo 6.20): a slab that is no floor of the building
//! but a plate covering one — a suspended ceiling, a floor finish drawn by hand, a roof's covering — written as an
//! `IfcCovering` of its type. It stands on its own: walls, columns, beams and rooms do not stop at it, and the quantities
//! do not count it as a slab. Its type is IFC 4.3's (`IfcCoveringTypeEnum`, IFC4X3_ADD2) as EK-6's list spells it;
//! `USERDEFINED` takes its name as its object type.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// What makes a slab a covering (docs/adr/8070 §10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Covering {
    /// Its IFC 4.3 type ([`Covering::TYPES`]): `CEILING` a ceiling (asma tavan), `FLOORING` a floor covering…;
    /// `USERDEFINED` takes `name` as its object type.
    pub predefined: String,
    /// What it is in words ("Asma tavan", "Parke"): its IFC name's words, and its object type when its type is
    /// `USERDEFINED`, which needs it; absent: its type's own word ([`Covering::word`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
}

impl Covering {
    /// IFC 4.3's covering types (IFC4X3_ADD2's `IfcCoveringTypeEnum`, EK-6 Tablo 6.20's list), `USERDEFINED` last;
    /// `NOTDEFINED` is no type.
    pub const TYPES: [&'static str; 13] = [
        "CEILING",
        "CLADDING",
        "COPING",
        "FLOORING",
        "INSULATION",
        "MEMBRANE",
        "MOLDING",
        "ROOFING",
        "SKIRTINGBOARD",
        "SLEEVING",
        "TOPPING",
        "WRAPPING",
        "USERDEFINED",
    ];
    /// The longest name, in characters.
    pub const MAX_NAME: usize = 120;

    /// A suspended ceiling: `CEILING` "Asma tavan".
    pub fn ceiling() -> Self {
        Covering {
            predefined: "CEILING".into(),
            name: Some("Asma tavan".into()),
        }
    }

    /// A floor covering: `FLOORING`.
    pub fn flooring() -> Self {
        Covering {
            predefined: "FLOORING".into(),
            name: None,
        }
    }

    /// A type's word in Turkish (EK-2 Tablo 2.3's where it has the element); the type itself for one not on the list.
    pub fn word(predefined: &str) -> &str {
        match predefined {
            "CEILING" => "Tavan kaplaması",
            "CLADDING" => "Duvar kaplaması",
            "COPING" => "Harpuşta",
            "FLOORING" => "Zemin kaplaması",
            "INSULATION" => "Isı yalıtımı",
            "MEMBRANE" => "Su yalıtımı",
            "MOLDING" => "Kartonpiyer",
            "ROOFING" => "Çatı kaplaması",
            "SKIRTINGBOARD" => "Süpürgelik",
            "SLEEVING" => "Kılıf",
            "TOPPING" => "Tesviye betonu",
            "WRAPPING" => "Sargı",
            "USERDEFINED" => "Kullanıcı tanımlı",
            other => other,
        }
    }

    /// EK-2 Tablo 2.3's element its type is drawn on: Asma Tavan, Zemin Kaplama, Duvar Kaplama, Çatı Kaplama, Isı
    /// Yalıtımı, Su Yalıtımı, Kartonpiyer, Süpürgelik, Harpuşta, Tesviye Betonu; Diğer for the rest.
    pub fn element(predefined: &str) -> &'static str {
        match predefined {
            "CEILING" => "ASTV",
            "FLOORING" => "ZMNK",
            "CLADDING" => "DUVK",
            "ROOFING" => "CKAP",
            "INSULATION" => "ISYA",
            "MEMBRANE" => "SUYA",
            "MOLDING" => "KARP",
            "SKIRTINGBOARD" => "SUPR",
            "COPING" => "HARP",
            "TOPPING" => "TSVY",
            _ => "DIGR",
        }
    }

    /// The covering an object is (docs/adr/8070 §10, 8173): a slab's plate or a beam's profile; none for the rest.
    pub fn of(e: &crate::Entity) -> Option<&Covering> {
        match e {
            crate::Entity::Slab(s) => s.slab.covering.as_ref(),
            crate::Entity::Beam(b) => b.beam.covering.as_ref(),
            _ => None,
        }
    }

    /// A facade moulding's name (docs/adr/8173): a `MOLDING` so named is the facade's band, drawn on EK-2's Cephe, not
    /// a room's Kartonpiyer.
    pub const FACADE_MOULDING: &'static str = "Silme";

    /// EK-2 Tablo 2.3's element it is drawn on: its type's ([`Covering::element`]); a facade moulding the Cephe's.
    pub fn element_of(&self) -> &'static str {
        if self.predefined == "MOLDING" && self.name.as_deref() == Some(Self::FACADE_MOULDING) {
            "CEPH"
        } else {
            Self::element(&self.predefined)
        }
    }

    /// A facade moulding (silme, docs/adr/8173): `MOLDING` "Silme".
    pub fn facade_moulding() -> Self {
        Covering { predefined: "MOLDING".into(), name: Some(Self::FACADE_MOULDING.into()) }
    }

    /// A coping (harpuşta, docs/adr/8173): `COPING`, its type's word.
    pub fn coping() -> Self {
        Covering { predefined: "COPING".into(), name: None }
    }

    /// The words its IFC name takes: its name, else its type's word.
    pub fn words(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| Self::word(&self.predefined).to_owned())
    }

    /// Its object type: its name when its type is `USERDEFINED`; none otherwise.
    pub fn object_type(&self) -> Option<String> {
        (self.predefined == "USERDEFINED").then(|| self.words())
    }

    /// What is wrong with it: the field (under `covering`) and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if !Self::TYPES.contains(&self.predefined.as_str()) {
            return Some((
                "covering.predefined".into(),
                format!(
                    "“{}” kaplamanın tiplerinden biri değil ({}).",
                    self.predefined,
                    Self::TYPES.join(", ")
                ),
            ));
        }
        if let Some(t) = &self.name
            && (t.trim().is_empty()
                || t.trim() != t
                || t.chars().any(char::is_control)
                || t.chars().count() > Self::MAX_NAME)
        {
            return Some((
                "covering.name".into(),
                format!(
                    "Kaplamanın adı boş, kenarı boşluklu ya da denetim karakterli olamaz; en çok {} karakter.",
                    Self::MAX_NAME
                ),
            ));
        }
        if self.predefined == "USERDEFINED" && self.name.is_none() {
            return Some((
                "covering.name".into(),
                "Kullanıcı tanımlı kaplamanın ne olduğu adında yazılır (USERDEFINED'ın nesne tipi); bir ad verin ya da \
                 listeden bir tip seçin."
                    .into(),
            ));
        }
        None
    }
}

/// Whether `e` is a covering (a slab that is one, docs/adr/8070 §10): what the structure's rules and the quantities
/// leave out.
pub fn is_covering(e: &crate::Entity) -> bool {
    matches!(e, crate::Entity::Slab(s) if s.slab.covering.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_covering_s_type_is_ifc_s_and_drawn_on_its_ek2_element() {
        assert_eq!(Covering::ceiling().problem(), None);
        assert_eq!(Covering::ceiling().words(), "Asma tavan");
        assert_eq!(Covering::flooring().words(), "Zemin kaplaması");
        assert_eq!(Covering::element("CEILING"), "ASTV");
        assert_eq!(Covering::element("SLEEVING"), "DIGR");
        let bad = Covering {
            predefined: "NOTDEFINED".into(),
            name: None,
        };
        assert_eq!(bad.problem().map(|p| p.0).as_deref(), Some("covering.predefined"));
        let nameless = Covering {
            predefined: "USERDEFINED".into(),
            name: None,
        };
        assert_eq!(nameless.problem().map(|p| p.0).as_deref(), Some("covering.name"));
    }
}
