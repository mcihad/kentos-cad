//! Count/spacing is semantic input. Geometry, anchors and numbering are planned
//! here; the edit command keeps its existing all-or-none checks and transaction.
use kentos_commands::{
    Entity, MAX_OPENINGS, OpeningArrayLayout, OpeningArrayNumbering, OpeningArraySpacing,
    OpeningEntity, WallEnd,
};
use kentos_domain::{
    Document,
    opening_follow::{self, WallLine},
};
use kentos_geometry_core::geom::opening::{SPAN_TOLERANCE, at_of_centre};
use std::collections::HashSet;

use crate::{
    checks::{self, Stop},
    codes,
    openings::{self, fail},
};

fn invalid(message: &str, field: &str) -> Stop {
    fail(codes::INVALID_OPENING, message.into(), field)
}

fn nonnegative(value: f64, field: &str) -> Result<(), Stop> {
    checks::finite(value, "Uzaklık", "Sonlu bir uzunluk verin.", field)?;
    if value < 0.0 {
        return Err(invalid("Uzaklık negatif olamaz; yönü ayrı seçin.", field));
    }
    Ok(())
}

pub(super) fn copies(
    doc: &Document,
    source: &OpeningEntity,
    line: &WallLine,
    layout: &OpeningArrayLayout,
    numbering: Option<&OpeningArrayNumbering>,
) -> Result<Vec<(OpeningEntity, String)>, Stop> {
    let count = match layout {
        OpeningArrayLayout::Spacing { count, .. } | OpeningArrayLayout::Fit { count, .. } => *count,
    };
    if count == 0 || count as usize > MAX_OPENINGS {
        return Err(invalid(
            "Kopya adedi 1–10000 arasında olmalı; kaynak bu sayıya dahil değildir.",
            "operation.layout.count",
        ));
    }
    let (first, step) = match layout {
        OpeningArrayLayout::Spacing {
            distance,
            reference,
            direction,
            ..
        } => {
            nonnegative(*distance, "operation.layout.distance")?;
            let pitch = match reference {
                OpeningArraySpacing::Clear => source.opening.width + distance,
                OpeningArraySpacing::Centers => *distance,
            };
            if pitch <= 0.0 {
                return Err(invalid(
                    "Merkez aralığı sıfırdan büyük olmalı.",
                    "operation.layout.distance",
                ));
            }
            let sign = if *direction == WallEnd::Start {
                -1.0
            } else {
                1.0
            };
            // Work in path centre coordinates; preserve even a non-centre anchor.
            let (lo, hi) = opening_follow::span(&source.opening);
            (lo + (hi - lo) / 2.0 + sign * pitch, sign * pitch)
        }
        OpeningArrayLayout::Fit {
            start_margin,
            end_margin,
            ..
        } => {
            nonnegative(*start_margin, "operation.layout.startMargin")?;
            nonnegative(*end_margin, "operation.layout.endMargin")?;
            let lo = *start_margin;
            let hi = line.path.length - end_margin;
            let available = hi - lo;
            // Match the existing jamb/span policy at decimal boundaries; do not
            // reject e.g. two 0.1 m gaps in exactly 0.2 m due to subtraction ULPs.
            if available < 0.0
                || available + SPAN_TOLERANCE < source.opening.width * f64::from(count)
            {
                return Err(invalid(
                    "Uç payları arasında bu adet ve genişlikte açıklığa yer yok; adedi/eni veya payları azaltın.",
                    "operation.layout",
                ));
            }
            if count == 1 {
                (lo + (hi - lo) / 2.0, 0.0)
            } else {
                (
                    lo + source.opening.width / 2.0,
                    (available - source.opening.width) / f64::from(count - 1),
                )
            }
        }
    };
    checks::finite(
        first,
        "İlk konum",
        "Hesaplanan konum sonlu olmalı.",
        "operation.layout",
    )?;
    checks::finite(
        step,
        "Aralık",
        "Hesaplanan aralık sonlu olmalı.",
        "operation.layout",
    )?;
    if let Some(n) = numbering {
        if n.step == 0 {
            return Err(invalid(
                "Numara adımı sıfırdan büyük olmalı.",
                "operation.numbering.step",
            ));
        }
        if n.prefix.chars().count() > 128 {
            return Err(invalid(
                "Numara öneki en çok 128 karakter olabilir.",
                "operation.numbering.prefix",
            ));
        }
    }
    let mut used: HashSet<String> = doc
        .entities()
        .filter(|_| numbering.is_some())
        .filter_map(|e| match e {
            Entity::Opening(o) if o.opening.form == source.opening.form => o.opening.number.clone(),
            _ => None,
        })
        .collect();
    let mut added = Vec::with_capacity(count as usize);
    for index in 0..count {
        let path = format!("operation.layout.copies[{index}]");
        let centre = first + f64::from(index) * step;
        checks::finite(centre, "Konum", "Hesaplanan konum sonlu olmalı.", &path)?;
        let mut fields = source.opening.clone();
        fields.at = at_of_centre(centre, opening_follow::anchor(&fields), fields.width);
        fields.number = None;
        if let Some(n) = numbering {
            let value = u64::from(n.start) + u64::from(index) * u64::from(n.step);
            let mark = format!("{}{value}", n.prefix);
            if !used.insert(mark.clone()) {
                return Err(invalid(
                    &format!(
                        "“{mark}” numarası aynı biçimdeki başka bir açıklıkta kullanılıyor; öneki veya başlangıcı değiştirin. Hiçbir kopya yazılmadı."
                    ),
                    "operation.numbering",
                ));
            }
            fields.number = Some(mark);
        }
        openings::valid(&fields, &path)?;
        let mut copy = source.clone();
        copy.base.id = 0;
        copy.base.label = None;
        copy.base.label_pins.clear();
        copy.base.attachments.clear();
        copy.opening = openings::placed(&fields, line, &path)?;
        added.push((copy, path));
    }
    Ok(added)
}
