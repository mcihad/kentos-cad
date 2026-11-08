//! The words of an exchange trip's losses and of the support matrix (`file.exchange.check`, `file.exchange.matrix`;
//! docs/adr/1660, 1880, 2380), shared by the desktop's Dışa aktarma kaybı denetimi and the delivery package's report
//! `devir/devir-raporu.csv` (`file.package`, docs/adr/2780): an aspect, a cell, a loss and its detail in Turkish.

use crate::{ExchangeCell, ExchangeState};

/// A difference as metres to the micrometre, comma decimal, no trailing zeros; below a micrometre said so.
pub fn small(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    if v.abs() < 1e-6 {
        return "< 0,000001".into();
    }
    let t = format!("{v:.6}");
    t.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

/// An object kind in words.
pub fn kind_word(kind: &str) -> String {
    match kind {
        "point" => "Nokta",
        "line" => "Çizgi",
        "polyline" => "Çoklu çizgi",
        "polygon" => "Alan",
        "circle" => "Daire",
        "arc" => "Yay",
        "ellipse" => "Elips",
        "spline" => "Eğri",
        "nurbs" => "NURBS",
        "text" => "Yazı",
        "dimension" => "Ölçü",
        "hatch" => "Tarama",
        "insert" => "Blok",
        "leader" => "Kılavuz",
        "table" => "Tablo",
        "image" => "Resim",
        "raster" => "Raster",
        other => return other.to_owned(),
    }
    .to_owned()
}

/// A loss's detail in words: kinds by their Turkish names, a style by its own, an attribute as it is named.
pub fn detail_words(loss: &str, detail: &str) -> String {
    match loss {
        "notWritten" | "chorded" | "extra" => kind_word(detail),
        "kindChanged" => detail.split(" → ").map(kind_word).collect::<Vec<_>>().join(" → "),
        "styleLost" => match detail {
            "color" => "Renk".into(),
            "lineWeight" => "Çizgi kalınlığı".into(),
            "symbol" => "Sembol".into(),
            other => other.into(),
        },
        "tableChanged" => match detail {
            "cells" => "hücreler".into(),
            "sizes" => "satır ve sütun boyları".into(),
            "merges" => "birleşik hücreler".into(),
            other => other.into(),
        },
        _ => detail.to_owned(),
    }
}

/// A matrix row's aspect in words.
pub fn aspect_word(aspect: &str) -> &'static str {
    match aspect {
        "objects" => "Nesneler",
        "kinds" => "Tür",
        "curves" => "Eğriler",
        "z" => "Kot (Z)",
        "m" => "Ölçü (M)",
        "attributes" => "Öznitelikler",
        "style" => "Nesne stili",
        "identity" => "Kalıcı kimlik",
        "position" => "Köşe konumu",
        "extra" => "Fazla nesne",
        "blocks" => "Bloklar",
        "tables" => "Tablolar",
        "relations" => "İlişkiler",
        _ => "—",
    }
}

/// A matrix cell in words: what the format did, how many, and what.
pub fn cell_words(aspect: &str, c: &ExchangeCell) -> String {
    let detail = c.detail.as_deref().map(|d| match aspect {
        "objects" => d
            .split(", ")
            .map(|kc| match kc.rsplit_once(' ') {
                Some((k, n)) => format!("{} {n}", kind_word(k).to_lowercase()),
                None => kc.to_owned(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        "position" => d.parse::<f64>().map_or_else(|_| d.to_owned(), |v| format!("{} m", small(v))),
        // Blocks and tables (docs/adr/2380): lost by kind and count, converted by a kind's change or what changed.
        "blocks" | "tables" if c.state == ExchangeState::Lost => d
            .split(", ")
            .map(|kc| match kc.rsplit_once(' ') {
                Some((k, n)) => format!("{} {n}", kind_word(k).to_lowercase()),
                None => kc.to_owned(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        "blocks" | "tables" => d
            .split(", ")
            .map(|x| {
                if x.contains(" → ") {
                    detail_words("kindChanged", x)
                } else {
                    detail_words(if aspect == "tables" { "tableChanged" } else { "blockChanged" }, x)
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
        // Kinds and styles by their Turkish names, item by item (the losses' own words); attributes as named.
        _ => {
            let loss = match aspect {
                "kinds" => "kindChanged",
                "curves" => "chorded",
                "style" => "styleLost",
                "extra" => "extra",
                "tables" => "tableChanged",
                _ => "",
            };
            d.split(", ").map(|x| detail_words(loss, x)).collect::<Vec<_>>().join(", ")
        }
    });
    let word = match c.state {
        ExchangeState::Kept => "Korunur",
        ExchangeState::Approximated => "Yaklaşık",
        ExchangeState::Converted => "Dönüşür",
        ExchangeState::Added => "Eklenir",
        ExchangeState::Lost => "Kaybolur",
    };
    match (c.state, detail) {
        (ExchangeState::Kept, Some(d)) if aspect == "position" => format!("{word} ({d})"),
        (ExchangeState::Kept, _) => word.to_owned(),
        (_, Some(d)) => format!("{word}: {d}"),
        (_, None) => format!("{word} ({})", c.count),
    }
}

/// A loss's kind in words.
pub fn loss_word(kind: &str) -> &'static str {
    match kind {
        "notWritten" => "Yazılmadı",
        "kindChanged" => "Türü değişti",
        "chorded" => "Eğri kirişe döndü",
        "zLost" => "Kot kayboldu",
        "zAdded" => "Kot eklendi",
        "mLost" => "Ölçü (M) kayboldu",
        "attrLost" => "Öznitelik kayboldu",
        "attrChanged" => "Öznitelik değişti",
        "attrAdded" => "Öznitelik eklendi",
        "styleLost" => "Nesne stili kayboldu",
        "identityLost" => "Kalıcı kimlik korunmadı",
        "extra" => "Fazla geri okunan",
        "blockChanged" => "Blok başka tanımla geri okundu",
        "tableChanged" => "Tablo değişti",
        "relationLost" => "İlişki geri okunmadı",
        _ => "—",
    }
}
