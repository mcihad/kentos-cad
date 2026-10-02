//! Kenar eşleme's words (the web's `ui/calc/edgematchWords.ts`): the
//! choices with their names and hints, an object as the links table names
//! it, the summary under the table and the report.

use std::fmt;

use kentos_contracts::Entity;
use kentos_domain::Document as Model;
use kentos_geometry_core::ops::edgematch::{Meet, Method};

use super::{Form, LAYER_KEY, Scope, USE};
use crate::calc::traverse::mm_text;
use crate::exchange::words::Kind as Line;

pub const TITLE: &str = "Kenar eşleme";

/// A meeting place as the segmented control names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NamedMeet(pub(super) Meet);

pub(super) const MEETS: [NamedMeet; 3] = [
    NamedMeet(Meet::Adjacent),
    NamedMeet(Meet::Middle),
    NamedMeet(Meet::Border),
];

impl fmt::Display for NamedMeet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(meet_name(self.0))
    }
}

pub(super) fn meet_name(m: Meet) -> &'static str {
    match m {
        Meet::Adjacent => "Komşunun ucunda",
        Meet::Middle => "Ortada",
        Meet::Border => "Sınırda",
    }
}

pub(super) fn meet_hint(m: Meet) -> &'static str {
    match m {
        Meet::Adjacent => "Komşu pafta yerinde kalır; kaynak ucu komşunun ucuna gider.",
        Meet::Middle => "İki uç aralarının ortasında buluşur; komşu çizgi de düzeltilir.",
        Meet::Border => {
            "İki uç sınırın, ortalarına en yakın noktasında buluşur; komşu çizgi de düzeltilir."
        }
    }
}

/// A method as the segmented control names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NamedMethod(pub(super) Method);

pub(super) const METHODS: [NamedMethod; 3] = [
    NamedMethod(Method::Move),
    NamedMethod(Method::Segment),
    NamedMethod(Method::Adjust),
];

impl fmt::Display for NamedMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(method_name(self.0))
    }
}

pub(super) fn method_name(m: Method) -> &'static str {
    match m {
        Method::Move => "Ucu taşı",
        Method::Segment => "Parça ekle",
        Method::Adjust => "Köşeleri ayarla",
    }
}

pub(super) fn method_hint(m: Method) -> &'static str {
    match m {
        Method::Move => "Yalnız uç taşınır; ucun kenarı doğru kalır.",
        Method::Segment => {
            "Uç yerinde kalır, buluşma yerine düz bir parça eklenir; çizgi çoklu çizgi olur."
        }
        Method::Adjust => "Uçtaki kayma çizgi boyunca öbür uca doğru azalarak dağıtılır.",
    }
}

/// Kaynak's scope with the selection's count, as the segmented control shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Counted(pub(super) Scope, pub(super) usize);

impl fmt::Display for Counted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Scope::Selection => write!(f, "Seçili ({})", self.1),
            Scope::Layer => f.write_str("Katman"),
        }
    }
}

/// An object as the links table names it: its layer, then its label, its
/// first attribute's value or its kind (the web's `describe`).
pub(super) fn describe(e: &Entity, layer: &str) -> String {
    let base = e.base();
    let named = base
        .label
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .or_else(|| {
            base.attrs
                .values()
                .map(|v| v.trim())
                .find(|v| !v.is_empty())
        });
    let kind = match e {
        Entity::Line(_) => "çizgi",
        Entity::Polyline(_) => "çoklu çizgi",
        _ => e.kind(),
    };
    format!("{layer}: {}", named.unwrap_or(kind))
}

impl Form {
    /// The summary under the table (the web's `summaryLines`).
    pub(super) fn summary_lines(&self) -> Vec<(Line, String)> {
        if let Some(problem) = &self.problem {
            return vec![(Line::Warn, problem.clone())];
        }
        let Some(f) = &self.found else {
            return Vec::new();
        };
        let bordered = self.border.is_some();
        let mut lines = Vec::new();
        let used: Vec<usize> = (0..self.rows.len())
            .filter(|&i| self.rows[i][USE] != "0")
            .collect();
        if f.links.is_empty() {
            lines.push((
                Line::Info,
                format!(
                    "Arama uzaklığında devamı bulunan uç yok. Arama uzaklığını ya da açı toleransını büyütün{}.",
                    if bordered {
                        "; uçlar sınıra yakın olmalı"
                    } else {
                        ""
                    }
                ),
            ));
        } else {
            lines.push((
                Line::Ok,
                format!(
                    "{} bağ bulundu; {} bağ kullanılacak.",
                    f.links.len(),
                    used.len()
                ),
            ));
        }
        if let Some(worst) = self.worst() {
            let mean = used.iter().map(|&i| f.links[i].gap).sum::<f64>() / used.len() as f64;
            lines.push((
                Line::Info,
                format!(
                    "En büyük aralık {}: {}. satır; ortalama {}.",
                    mm_text(f.links[worst].gap),
                    worst + 1,
                    mm_text(mean)
                ),
            ));
        }
        if !f.unmatched.is_empty() {
            lines.push((
                Line::Info,
                format!(
                    "{} uç eşsiz kaldı: {} yakın ama devamı bulunamadı.",
                    f.unmatched.len(),
                    if bordered {
                        "sınıra"
                    } else {
                        "komşu bir uca"
                    }
                ),
            ));
        }
        if f.junctions > 0 {
            lines.push((
                Line::Info,
                format!(
                    "{} kavşak ucu eşlenmedi; yalnız biri taşınırsa kavşak bozulurdu.",
                    f.junctions
                ),
            ));
        }
        if self.others > 0 {
            lines.push((
                Line::Info,
                format!(
                    "{} nesne katılmadı: yalnız çizgiler ve açık çoklu çizgiler eşlenir.",
                    self.others
                ),
            ));
        }
        if self.locked > 0 {
            lines.push((
                Line::Info,
                format!(
                    "{} nesne kilitli katmanda olduğu için katılmadı.",
                    self.locked
                ),
            ));
        }
        if !self.refused.is_empty() {
            let rows: Vec<String> = self.refused.iter().map(usize::to_string).collect();
            lines.push((
                Line::Warn,
                format!(
                    "{} bağ yazılamaz ({}. satır): nesnesinde sıfır boylu kenar kalırdı. Kullan'dan çıkarın ya da başka bir yöntem seçin.",
                    self.refused.len(),
                    rows.join(", ")
                ),
            ));
        }
        lines
    }

    /// The report (the web's `report`): the settings, the links and the counts, tab-separated.
    pub fn report(&self, model: &Model, selected: usize) -> Option<Vec<Vec<String>>> {
        let f = self.found.as_ref()?;
        let layers = model.layers();
        let path =
            |id: &Option<String>| id.as_deref().map(|id| layers.path(id)).unwrap_or_default();
        let border = self
            .border
            .as_deref()
            .and_then(|uid| super::border_slot(model, uid))
            .and_then(|s| model.get(s))
            .map_or_else(
                || "—".to_owned(),
                |e| describe(e, &layers.path(&e.base().layer_id)),
            );
        let line = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
        let mut lines = vec![
            line(&[TITLE]),
            vec![
                "Kaynak".to_owned(),
                match self.scope {
                    Scope::Selection => format!("Seçili ({selected})"),
                    Scope::Layer => path(&self.source),
                },
            ],
            vec!["Komşu".to_owned(), path(&self.adjacent)],
            vec!["Sınır".to_owned(), border],
            vec!["Arama uzaklığı (m)".to_owned(), self.distance.clone()],
            vec!["Açı toleransı (°)".to_owned(), self.angle.clone()],
            vec![
                "Eşleşme ölçütü".to_owned(),
                match self.key.as_str() {
                    "" => "Yok".to_owned(),
                    LAYER_KEY => "Katman adı".to_owned(),
                    attr => attr.to_owned(),
                },
            ],
            vec!["Buluşma".to_owned(), meet_name(self.meet).to_owned()],
            vec!["Yöntem".to_owned(), method_name(self.method).to_owned()],
            line(&[
                "Kullan",
                "No",
                "Kaynak",
                "Komşu",
                "Aralık (mm)",
                "Açı farkı (°)",
            ]),
        ];
        for (i, row) in self.rows.iter().enumerate() {
            lines.push(vec![
                if row[USE] == "0" { "hayır" } else { "evet" }.to_owned(),
                (i + 1).to_string(),
                row[super::SOURCE].clone(),
                row[super::ADJACENT].clone(),
                row[super::GAP].clone(),
                row[super::ANGLE].clone(),
            ]);
        }
        lines.push(vec![
            "Eşsiz uç".to_owned(),
            f.unmatched.len().to_string(),
            "Kavşak ucu".to_owned(),
            f.junctions.to_string(),
            "Katılmayan".to_owned(),
            self.others.to_string(),
        ]);
        Some(lines)
    }
}
