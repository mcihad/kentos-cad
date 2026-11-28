//! A member (eleman, docs/adr/8070 §8; EK-5 Tablo 5.1 ELM, EK-6 Tablo 6.40): a column or a beam that is not the frame's —
//! a pergola's post, a facade's rail — and is written as an `IfcMember` of its type, not as an `IfcColumn` or `IfcBeam`.
//! Its type is IFC 4.3's (`IfcMemberTypeEnum`, IFC4X3_ADD2), as EK-6's list spells it; `USERDEFINED` takes its name as its
//! object type, since nothing on the list is a facade's rail.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// What makes a column a post or a beam a rail (docs/adr/8070 §8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Member {
    /// Its IFC 4.3 type ([`Member::TYPES`]): `POST` a post (dikme), `PURLIN` an aşık, `RAFTER` a mertek…;
    /// `USERDEFINED` takes `name` as its object type.
    pub predefined: String,
    /// What it is in words ("Pergola dikmesi", "Cephe kaydı"): its IFC name's words, and its object type when its type is
    /// `USERDEFINED`, which needs it; absent: its type's own word ([`Member::word`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
}

impl Member {
    /// IFC 4.3's member types (IFC4X3_ADD2's `IfcMemberTypeEnum`, EK-6 Tablo 6.40's list), `USERDEFINED` last;
    /// `NOTDEFINED` is no type.
    pub const TYPES: [&'static str; 20] = [
        "ARCH_SEGMENT",
        "BRACE",
        "CHORD",
        "COLLAR",
        "MEMBER",
        "MULLION",
        "PLATE",
        "POST",
        "PURLIN",
        "RAFTER",
        "STAY_CABLE",
        "STIFFENING_RIB",
        "STRINGER",
        "STRUCTURALCABLE",
        "STRUT",
        "STUD",
        "SUSPENDER",
        "SUSPENSION_CABLE",
        "TIEBAR",
        "USERDEFINED",
    ];
    /// The longest name, in characters.
    pub const MAX_NAME: usize = 120;
    /// A rail's words: IFC 4.3 has no type for a facade's rail (docs/adr/8070 §8).
    pub const RAIL: &'static str = "Kayıt";

    /// A post (dikme): `POST`, "a linear (usually vertical) member used to support something or to mark a point".
    pub fn post() -> Self {
        Member {
            predefined: "POST".into(),
            name: None,
        }
    }

    /// A rail (kayıt): `USERDEFINED` "Kayıt".
    pub fn rail() -> Self {
        Member {
            predefined: "USERDEFINED".into(),
            name: Some(Self::RAIL.into()),
        }
    }

    /// A type's word in Turkish, the building trades' where they have one; the type itself for one not on the list.
    pub fn word(predefined: &str) -> &str {
        match predefined {
            "ARCH_SEGMENT" => "Kemer parçası",
            "BRACE" => "Payanda",
            "CHORD" => "Başlık çubuğu",
            "COLLAR" => "Kuşak",
            "MEMBER" => "Çubuk",
            "MULLION" => "Cephe dikmesi",
            "PLATE" => "Başlık kaydı",
            "POST" => "Dikme",
            "PURLIN" => "Aşık",
            "RAFTER" => "Mertek",
            "STAY_CABLE" => "Eğik askı kablosu",
            "STIFFENING_RIB" => "Berkitme",
            "STRINGER" => "Limon",
            "STRUCTURALCABLE" => "Taşıyıcı kablo",
            "STRUT" => "Basınç çubuğu",
            "STUD" => "Ara dikme",
            "SUSPENDER" => "Askı",
            "SUSPENSION_CABLE" => "Asma kablo",
            "TIEBAR" => "Gergi çubuğu",
            "USERDEFINED" => "Kullanıcı tanımlı",
            other => other,
        }
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

    /// What is wrong with it: the field (under `member`) and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if !Self::TYPES.contains(&self.predefined.as_str()) {
            return Some((
                "member.predefined".into(),
                format!(
                    "“{}” elemanın tiplerinden biri değil ({}).",
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
                "member.name".into(),
                format!(
                    "Elemanın adı boş, kenarı boşluklu ya da denetim karakterli olamaz; en çok {} karakter.",
                    Self::MAX_NAME
                ),
            ));
        }
        if self.predefined == "USERDEFINED" && self.name.is_none() {
            return Some((
                "member.name".into(),
                "Kullanıcı tanımlı elemanın ne olduğu adında yazılır (USERDEFINED'ın nesne tipi); bir ad verin ya da \
                 listeden bir tip seçin."
                    .into(),
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_s_type_is_ifc_s_and_a_user_defined_one_says_what_it_is() {
        assert_eq!(Member::post().problem(), None);
        assert_eq!(Member::rail().problem(), None);
        assert_eq!(Member::post().words(), "Dikme");
        assert_eq!(Member::post().object_type(), None);
        assert_eq!(Member::rail().object_type().as_deref(), Some("Kayıt"));
        let bad = Member {
            predefined: "NOTDEFINED".into(),
            name: None,
        };
        assert_eq!(
            bad.problem().map(|p| p.0).as_deref(),
            Some("member.predefined")
        );
        let nameless = Member {
            predefined: "USERDEFINED".into(),
            name: None,
        };
        assert_eq!(
            nameless.problem().map(|p| p.0).as_deref(),
            Some("member.name")
        );
        let padded = Member {
            predefined: "PURLIN".into(),
            name: Some(" Aşık".into()),
        };
        assert_eq!(
            padded.problem().map(|p| p.0).as_deref(),
            Some("member.name")
        );
    }
}
