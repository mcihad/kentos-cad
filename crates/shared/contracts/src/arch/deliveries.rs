//! Teslim kaydı (docs/adr/8099; plan/bim-plan.md BIM-15; e-PYS Yönetmeliği 4–5): which package of the project went to
//! which authority, when and through which channel, and what came back — accepted, or returned with its reasons, each
//! reason a review note bound to the objects it is about (docs/adr/1000). Kept in the drawing
//! (`settings.arch.deliveries`, Mimari data 17). A package is named by its publication's manifest (`yayin.json`'s
//! SHA-256, docs/adr/0617) and its EK-2 revision number, so a corrected package can be told from the one before it.
//!
//! The file reader checks the structure here; the command checks the drawing (that a review note exists).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most deliveries a drawing keeps.
pub const MAX_DELIVERIES: usize = 1000;
/// The most characters of a reason.
pub const MAX_DELIVERY_TEXT: usize = 4000;
/// The most characters of an authority's name, a publisher's, a receipt's or a path.
pub const MAX_DELIVERY_LABEL: usize = 300;
/// The most review notes a response is bound to.
pub const MAX_DELIVERY_NOTES: usize = 1000;

/// How a package went to the authority (e-PYS Yönetmeliği 4/3, 6): the ministry's system, the authority's own software
/// (whose records go to e-PYS, 7/7), on paper by hand (6: with no e-signature, a printed set besides), another way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DeliveryChannel {
    /// e-PYS.
    #[default]
    Epys,
    /// İdarenin yazılımı.
    Authority,
    /// Basılı (elden).
    Paper,
    /// Öbür.
    Other,
}

impl DeliveryChannel {
    pub const ALL: [DeliveryChannel; 4] = [
        DeliveryChannel::Epys,
        DeliveryChannel::Authority,
        DeliveryChannel::Paper,
        DeliveryChannel::Other,
    ];

    /// As the form names it.
    pub fn label(self) -> &'static str {
        match self {
            DeliveryChannel::Epys => "e-PYS",
            DeliveryChannel::Authority => "İdarenin yazılımı",
            DeliveryChannel::Paper => "Basılı (elden)",
            DeliveryChannel::Other => "Öbür",
        }
    }
}

/// What came back (e-PYS Yönetmeliği 5/2–3): accepted, or returned with its reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DeliveryOutcome {
    /// Kabul.
    Accepted,
    /// İade.
    Returned,
}

impl DeliveryOutcome {
    pub fn label(self) -> &'static str {
        match self {
            DeliveryOutcome::Accepted => "Kabul",
            DeliveryOutcome::Returned => "İade",
        }
    }
}

/// The package a delivery took: its publication's manifest's SHA-256, the files it holds, where it was written and the
/// drawing's revision it was published from.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DeliveryPackage {
    /// `yayin.json`'s SHA-256, 64 lowercase hexadecimal digits.
    pub manifest: String,
    /// The files the package holds.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub files: u32,
    /// Its folder, as the author's machine names it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// The drawing's revision it was published from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
}

/// The authority's answer: accepted or returned, when, why (a return's reason is said), the review notes the reasons are
/// bound to objects by, and the system's receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DeliveryResponse {
    pub outcome: DeliveryOutcome,
    /// The day it came, YYYY-AA-GG.
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
    /// The review notes' ids, each a reason bound to the objects it is about.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub receipt: Option<String>,
}

/// One delivery of the project's package to an authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Delivery {
    /// Lowercase, hyphenated UUID; unique in the drawing.
    pub id: String,
    /// 1, 2, 3 …: one more than the largest a delivery in the drawing has.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    /// The package's EK-2 revision number: three digits, 000 unrevised.
    pub revision: String,
    pub package: DeliveryPackage,
    /// The authority it went to.
    pub to: String,
    #[serde(default)]
    pub channel: DeliveryChannel,
    /// The day it went, YYYY-AA-GG.
    pub at: String,
    /// Who published it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<String>,
    /// The system's receipt for the delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub receipt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub response: Option<DeliveryResponse>,
}

/// Whether `s` is a lowercase, hyphenated UUID.
pub fn is_delivery_id(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(c),
        })
}

/// Whether `s` is a day as YYYY-AA-GG (a real one).
pub fn is_delivery_day(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let n = |r: std::ops::Range<usize>| s.get(r).and_then(|t| t.parse::<u32>().ok());
    let (Some(y), Some(m), Some(d)) = (n(0..4), n(5..7), n(8..10)) else {
        return false;
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1900..=2200).contains(&y) && (1..=days).contains(&d)
}

/// What is wrong with a text field: empty when it must be said, too long, a control character but a line break or a tab.
fn text_problem(s: &str, most: usize, said: bool) -> Option<String> {
    if said && s.trim().is_empty() {
        return Some("boş olamaz".into());
    }
    if s.trim() != s {
        return Some("başında ya da sonunda boşluk olamaz".into());
    }
    if s.chars().count() > most {
        return Some(format!("en çok {most} karakter olmalı"));
    }
    if s.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        return Some("denetim karakteri içeremez".into());
    }
    None
}

impl Delivery {
    /// What is wrong with a drawing's deliveries: the field (under `deliveries`) and the words. Ids and numbers unique;
    /// a revision of three digits; the manifest's 64 lowercase hexadecimal digits; the authority and the days said; a
    /// return's reason said; the texts' lengths; a response's notes unique.
    pub fn list_problem(list: &[Delivery]) -> Option<(String, String)> {
        if list.len() > MAX_DELIVERIES {
            return Some((
                "deliveries".into(),
                format!("En çok {MAX_DELIVERIES} teslim kaydı tutulur."),
            ));
        }
        for (i, d) in list.iter().enumerate() {
            let at = |f: &str| format!("deliveries.{i}.{f}");
            let n = d.number;
            if !is_delivery_id(&d.id) {
                return Some((
                    at("id"),
                    format!("{n}. teslimin kimliği küçük harfli, tireli bir UUID olmalı."),
                ));
            }
            if list[..i].iter().any(|e| e.id == d.id) {
                return Some((
                    at("id"),
                    format!("{n}. teslimin kimliği başka bir teslimde de var."),
                ));
            }
            if n == 0 || list[..i].iter().any(|e| e.number == n) {
                return Some((
                    at("number"),
                    format!("Teslimin numarası ({n}) birden başlar ve tektir."),
                ));
            }
            if !(d.revision.len() == 3 && d.revision.bytes().all(|c| c.is_ascii_digit())) {
                return Some((
                    at("revision"),
                    format!(
                        "{n}. teslimin revizyon numarası üç rakam olmalı (EK-2: 000 revizyonsuz, 001, 002 …); “{}” verildi.",
                        d.revision
                    ),
                ));
            }
            let m = &d.package.manifest;
            if !(m.len() == 64
                && m.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)))
            {
                return Some((
                    at("package.manifest"),
                    format!(
                        "{n}. teslimin paketi yayının yayin.json'unun SHA-256'sıyla (64 küçük onaltılık rakam) anılır."
                    ),
                ));
            }
            for (field, value) in [
                ("package.path", &d.package.path),
                ("package.source", &d.package.source),
            ] {
                if let Some(why) = value
                    .as_deref()
                    .and_then(|v| text_problem(v, MAX_DELIVERY_LABEL, true))
                {
                    return Some((
                        at(field),
                        format!("{n}. teslimin paketinin yeri ya da kaynağı {why}."),
                    ));
                }
            }
            if let Some(why) = text_problem(&d.to, MAX_DELIVERY_LABEL, true) {
                return Some((at("to"), format!("{n}. teslimin kurumu {why}.")));
            }
            if !is_delivery_day(&d.at) {
                return Some((
                    at("at"),
                    format!(
                        "{n}. teslimin günü YYYY-AA-GG olmalı (2026-11-17); “{}” verildi.",
                        d.at
                    ),
                ));
            }
            for (field, value, words) in [
                ("by", &d.by, "yayımlayanı"),
                ("receipt", &d.receipt, "makbuz numarası"),
            ] {
                if let Some(why) = value
                    .as_deref()
                    .and_then(|v| text_problem(v, MAX_DELIVERY_LABEL, true))
                {
                    return Some((at(field), format!("{n}. teslimin {words} {why}.")));
                }
            }
            if let Some(r) = &d.response {
                if !is_delivery_day(&r.at) {
                    return Some((
                        at("response.at"),
                        format!(
                            "{n}. teslimin cevabının günü YYYY-AA-GG olmalı; “{}” verildi.",
                            r.at
                        ),
                    ));
                }
                if r.at < d.at {
                    return Some((
                        at("response.at"),
                        format!(
                            "{n}. teslimin cevabı ({}) tesliminden ({}) önce olamaz.",
                            r.at, d.at
                        ),
                    ));
                }
                if r.outcome == DeliveryOutcome::Returned
                    && r.reason.as_deref().is_none_or(|t| t.trim().is_empty())
                {
                    return Some((
                        at("response.reason"),
                        format!("{n}. teslimin iadesinin gerekçesini yazın."),
                    ));
                }
                if let Some(why) = r
                    .reason
                    .as_deref()
                    .and_then(|v| text_problem(v, MAX_DELIVERY_TEXT, true))
                {
                    return Some((
                        at("response.reason"),
                        format!("{n}. teslimin cevabının gerekçesi {why}."),
                    ));
                }
                if let Some(why) = r
                    .receipt
                    .as_deref()
                    .and_then(|v| text_problem(v, MAX_DELIVERY_LABEL, true))
                {
                    return Some((
                        at("response.receipt"),
                        format!("{n}. teslimin cevabının makbuz numarası {why}."),
                    ));
                }
                if r.notes.len() > MAX_DELIVERY_NOTES {
                    return Some((
                        at("response.notes"),
                        format!("Bir cevap en çok {MAX_DELIVERY_NOTES} inceleme notuna bağlanır."),
                    ));
                }
                for (j, note) in r.notes.iter().enumerate() {
                    if !is_delivery_id(note) || r.notes[..j].contains(note) {
                        return Some((
                            at(&format!("response.notes.{j}")),
                            format!(
                                "{n}. teslimin cevabının inceleme notları tek ve kimlikleri UUID olmalı."
                            ),
                        ));
                    }
                }
            }
        }
        None
    }

    /// The delivery before it to the same authority: the one of the largest number below its own.
    pub fn previous<'a>(&self, list: &'a [Delivery]) -> Option<&'a Delivery> {
        list.iter()
            .filter(|d| d.to == self.to && d.number < self.number)
            .max_by_key(|d| d.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delivery(n: u32) -> Delivery {
        Delivery {
            id: format!("00000000-0000-4000-8000-{n:012}"),
            number: n,
            revision: "000".into(),
            package: DeliveryPackage {
                manifest: "a".repeat(64),
                files: 12,
                path: None,
                source: None,
            },
            to: "Sivas Belediyesi İmar ve Şehircilik Müdürlüğü".into(),
            channel: DeliveryChannel::Epys,
            at: "2026-11-17".into(),
            by: None,
            receipt: None,
            response: None,
        }
    }

    /// Teslim kaydı (docs/adr/8099): written by its fields, a return's reason said, its notes unique, its days real.
    #[test]
    fn a_delivery_is_written_and_checked_as_the_record_of_a_package() {
        let mut list = vec![delivery(1), delivery(2)];
        list[1].revision = "001".into();
        list[0].response = Some(DeliveryResponse {
            outcome: DeliveryOutcome::Returned,
            at: "2026-11-20".into(),
            reason: Some("Kat yüksekliği planın notunu aşıyor.".into()),
            notes: vec!["00000000-0000-4000-8000-0000000000aa".into()],
            receipt: None,
        });
        assert_eq!(Delivery::list_problem(&list), None);
        let json = serde_json::to_value(&list[0]).unwrap();
        assert_eq!(json["channel"], "epys");
        assert_eq!(json["response"]["outcome"], "returned");
        assert_eq!(serde_json::from_value::<Delivery>(json).unwrap(), list[0]);
        assert_eq!(list[1].previous(&list).map(|d| d.number), Some(1));
        let path = |l: &[Delivery]| Delivery::list_problem(l).map(|p| p.0);
        let mut bad = list.clone();
        bad[0].response.as_mut().unwrap().reason = None;
        assert_eq!(
            path(&bad).as_deref(),
            Some("deliveries.0.response.reason"),
            "a return says why"
        );
        let mut bad = list.clone();
        bad[1].revision = "1".into();
        assert_eq!(path(&bad).as_deref(), Some("deliveries.1.revision"));
        let mut bad = list.clone();
        bad[1].at = "2026-02-30".into();
        assert_eq!(path(&bad).as_deref(), Some("deliveries.1.at"));
        let mut bad = list.clone();
        bad[0].response.as_mut().unwrap().at = "2026-11-16".into();
        assert_eq!(
            path(&bad).as_deref(),
            Some("deliveries.0.response.at"),
            "no answer before the delivery"
        );
        let mut bad = list.clone();
        bad[1].number = 1;
        assert_eq!(path(&bad).as_deref(), Some("deliveries.1.number"));
        let mut bad = list.clone();
        bad[0].package.manifest = "A".repeat(64);
        assert_eq!(path(&bad).as_deref(), Some("deliveries.0.package.manifest"));
        assert!(is_delivery_day("2028-02-29") && !is_delivery_day("2027-02-29"));
    }
}
