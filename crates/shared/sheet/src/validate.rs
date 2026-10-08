//! What a book must be before anything works on it, and its one normal form.
//!
//! Validation refuses what no operation can make: duplicate ids or names, a
//! group cycle, a value out of its range, a link to an item of the wrong
//! kind where only one kind can stand. A link that points nowhere (a scale
//! bar whose map was removed) is not refused: the preflight reports it, and
//! nothing ever relinks it silently (PiriCAD's lesson).
//!
//! Normalisation lowers colours to lowercase and makes every group's frame
//! its children's together; `apply` and the readers return books in this form.

use std::collections::{BTreeMap, BTreeSet};

use crate::bind;
use crate::error::{Result, SheetError};
use crate::kinds::*;
use crate::model::*;
use crate::paper;
use crate::style::{Stroke, TextStyle, is_color};
use crate::units::{FULL_TURN, RectUm, SizeUm, Um, rotated_bounds};

/// The largest paper side (5 m) and the smallest (10 mm).
pub const PAGE_MAX: Um = 5_000_000;
pub const PAGE_MIN: Um = 10_000;
/// How far from the page an item may be, and how large.
pub const FRAME_LIMIT: Um = 10_000_000;
/// The smallest item side (design §3.2).
pub const ITEM_MIN: Um = 1_000;

fn err(code: &str, path: &str, msg: impl Into<String>) -> SheetError {
    SheetError::at(code, path, msg)
}

/// Whether an id is usable: 1–128 characters, no control character.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().count() <= 128 && !id.chars().any(char::is_control)
}

/// A variable's name: a letter or `_`, then letters, digits and `_`.
pub fn valid_var_name(n: &str) -> bool {
    let mut cs = n.chars();
    match cs.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }
    n.chars().count() <= 64 && cs.all(|c| c.is_alphanumeric() || c == '_')
}

fn color(path: &str, c: &str) -> Result<()> {
    if is_color(c) {
        Ok(())
    } else {
        Err(err(
            "bad_color",
            path,
            format!("“{c}” bir renk değil: #rrggbb ya da #rrggbbaa biçiminde yazın."),
        ))
    }
}

fn stroke(path: &str, s: &Stroke) -> Result<()> {
    color(&format!("{path}.color"), &s.color)?;
    if !(0..=20_000).contains(&s.width) {
        return Err(err(
            "out_of_range",
            &format!("{path}.width"),
            "Çizgi kalınlığı 0 ile 20 mm arasında olmalı.",
        ));
    }
    if s.dash.len() > 16 || s.dash.iter().any(|d| !(1..=100_000).contains(d)) {
        return Err(err(
            "out_of_range",
            &format!("{path}.dash"),
            "Kesik çizginin parçaları 0,001 ile 100 mm arasında, en çok 16 tane olmalı.",
        ));
    }
    Ok(())
}

fn text_style(path: &str, t: &TextStyle) -> Result<()> {
    if t.font.is_empty() || t.font.len() > 64 {
        return Err(err(
            "bad_font",
            &format!("{path}.font"),
            "Yazı tipi adı boş olamaz.",
        ));
    }
    if !(200..=500_000).contains(&t.size) {
        return Err(err(
            "out_of_range",
            &format!("{path}.size"),
            "Yazı boyu 0,2 ile 500 mm arasında olmalı.",
        ));
    }
    if !(100..=900).contains(&t.weight) || !t.weight.is_multiple_of(100) {
        return Err(err(
            "out_of_range",
            &format!("{path}.weight"),
            "Yazı kalınlığı 100'ün katı ve 100 ile 900 arasında olmalı (400 normal, 700 kalın).",
        ));
    }
    color(&format!("{path}.color"), &t.color)
}

/// The objects a coordinate list reads (docs/adr/0206 §2): at most
/// `MAX_COORD_OBJECTS` ids, each one an id.
pub const MAX_COORD_OBJECTS: usize = 10_000;

fn coord_source(path: &str, s: &CoordSource) -> Result<()> {
    if let CoordSource::Objects(o) = s {
        if o.uids.len() > MAX_COORD_OBJECTS {
            return Err(err(
                "out_of_range",
                &format!("{path}.uids"),
                format!(
                    "Koordinat listesi en çok {MAX_COORD_OBJECTS} nesne okur; {} verildi. Daha az nesne seçin ya da katmanı kaynak yapın.",
                    o.uids.len()
                ),
            ));
        }
        if let Some(i) = o.uids.iter().position(|u| !valid_id(u)) {
            return Err(err(
                "bad_id",
                &format!("{path}.uids[{i}]"),
                "Nesne kimliği 1–128 karakter, denetim karaktersiz olmalı.",
            ));
        }
    }
    Ok(())
}

/// A coordinate list's headings (docs/adr/0206 §3): one line, at most
/// `MAX_COLUMN_NAME` characters each.
fn coord_columns(path: &str, c: &CoordColumns) -> Result<()> {
    for (key, what, v) in [
        ("point", "Nokta sütununun başlığı", &c.point),
        ("east", "Doğu sütununun başlığı", &c.east),
        ("north", "Kuzey sütununun başlığı", &c.north),
        ("z", "Z sütununun başlığı", &c.z),
        ("area", "Alan satırının sözü", &c.area),
    ] {
        if let Some(v) = v
            && (v.chars().count() > MAX_COLUMN_NAME || v.chars().any(char::is_control))
        {
            return Err(err(
                "invalid_column_name",
                &format!("{path}.{key}"),
                format!(
                    "{what} tek satır, en çok {MAX_COLUMN_NAME} karakter olmalı. Kısaltın ya da boş bırakın (varsayılan)."
                ),
            ));
        }
    }
    Ok(())
}

fn range<T: PartialOrd + Copy + std::fmt::Display>(
    path: &str,
    v: T,
    lo: T,
    hi: T,
    what: &str,
) -> Result<()> {
    if v < lo || v > hi {
        Err(err(
            "out_of_range",
            path,
            format!("{what} {lo} ile {hi} arasında olmalı; {v} verildi."),
        ))
    } else {
        Ok(())
    }
}

fn sha(path: &str, s: &str) -> Result<()> {
    if s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(err(
            "bad_asset",
            path,
            "Varlık özeti 64 küçük onaltılık basamaklı bir SHA-256 olmalı.",
        ))
    }
}

pub fn validate_page(path: &str, p: &Page) -> Result<()> {
    range(
        &format!("{path}.size.width"),
        p.size.width,
        PAGE_MIN,
        PAGE_MAX,
        "Kâğıt genişliği (µm)",
    )?;
    range(
        &format!("{path}.size.height"),
        p.size.height,
        PAGE_MIN,
        PAGE_MAX,
        "Kâğıt yüksekliği (µm)",
    )?;
    match paper::paper_size(p.paper, p.orientation) {
        Some(size) if size != p.size => {
            return Err(err(
                "page_size_mismatch",
                &format!("{path}.size"),
                format!(
                    "{} {} kâğıdı {} × {} mm'dir; boyu değiştirmek için kâğıdı “özel” yapın.",
                    p.paper.id().to_uppercase(),
                    if p.orientation == Orientation::Landscape {
                        "yatay"
                    } else {
                        "dikey"
                    },
                    size.width / 1000,
                    size.height / 1000
                ),
            ));
        }
        Some(_) => {}
        None if p.paper != Paper::Custom => {
            return Err(err(
                "unknown_paper",
                &format!("{path}.paper"),
                "Kâğıt tabloda yok.",
            ));
        }
        None => {
            if paper::orientation_of(p.size) != p.orientation {
                return Err(err(
                    "page_orientation",
                    &format!("{path}.orientation"),
                    "Özel kâğıdın yönü boyuyla uyuşmuyor: genişliği yüksekliğinden büyükse yatay, değilse dikeydir.",
                ));
            }
        }
    }
    let m = &p.margins;
    for (v, n) in [
        (m.top, "top"),
        (m.right, "right"),
        (m.bottom, "bottom"),
        (m.left, "left"),
    ] {
        if v < 0 {
            return Err(err(
                "out_of_range",
                &format!("{path}.margins.{n}"),
                "Kenar boşluğu eksi olamaz.",
            ));
        }
    }
    if i64::from(m.left) + i64::from(m.right) + i64::from(PAGE_MIN) > i64::from(p.size.width)
        || i64::from(m.top) + i64::from(m.bottom) + i64::from(PAGE_MIN) > i64::from(p.size.height)
    {
        return Err(err(
            "margins_too_large",
            &format!("{path}.margins"),
            "Kenar boşlukları kâğıda sığmıyor: basılabilir alan en az 10 mm kalmalı.",
        ));
    }
    if let Some(b) = &p.background {
        color(&format!("{path}.background"), b)?;
    }
    Ok(())
}

/// A list of variables: names valid and unique (Turkish letters folded), values of their kind.
pub fn validate_variables(path: &str, vars: &[Variable]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for (i, v) in vars.iter().enumerate() {
        let p = format!("{path}[{i}]");
        if !valid_var_name(&v.name) {
            return Err(err(
                "bad_variable",
                &format!("{p}.name"),
                format!(
                    "“{}” değişken adı olamaz: harf ya da _ ile başlayıp harf, rakam ve _ ile sürmeli.",
                    v.name
                ),
            ));
        }
        if !seen.insert(crate::expr::fold(&v.name)) {
            return Err(err(
                "duplicate_variable",
                &format!("{p}.name"),
                format!("“@{}” değişkeni iki kez tanımlanmış.", v.name),
            ));
        }
        let fits = match (&v.kind, &v.value) {
            (_, VarValue::Null) => true,
            (VarKind::Text, VarValue::Text(_)) => true,
            (VarKind::Number, VarValue::Number(x)) => x.is_finite(),
            (VarKind::Bool, VarValue::Bool(_)) => true,
            (VarKind::Date, VarValue::Text(t)) => is_iso_date(t),
            _ => false,
        };
        if !fits {
            return Err(err(
                "bad_variable_value",
                &format!("{p}.value"),
                format!("“@{}” değişkeninin değeri türüne uymuyor.", v.name),
            ));
        }
    }
    Ok(())
}

/// YYYY-AA-GG.
pub fn is_iso_date(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && (1..=12).contains(&t[5..7].parse::<u8>().unwrap_or(0))
        && (1..=31).contains(&t[8..10].parse::<u8>().unwrap_or(0))
}

fn frame(path: &str, f: &RectUm) -> Result<()> {
    range(
        &format!("{path}.left"),
        f.left,
        -FRAME_LIMIT,
        FRAME_LIMIT,
        "Sol (µm)",
    )?;
    range(
        &format!("{path}.top"),
        f.top,
        -FRAME_LIMIT,
        FRAME_LIMIT,
        "Üst (µm)",
    )?;
    range(
        &format!("{path}.width"),
        f.width,
        ITEM_MIN,
        FRAME_LIMIT,
        "Genişlik (µm)",
    )?;
    range(
        &format!("{path}.height"),
        f.height,
        ITEM_MIN,
        FRAME_LIMIT,
        "Yükseklik (µm)",
    )
}

fn title_rows(path: &str, rows: &[TitleRow], depth: usize) -> Result<()> {
    if depth > 4 {
        return Err(err(
            "out_of_range",
            path,
            "Antet hücreleri en çok dört kat iç içe bölünebilir.",
        ));
    }
    if rows.is_empty() || rows.len() > 64 {
        return Err(err(
            "out_of_range",
            path,
            "Antetin 1 ile 64 arasında satırı olmalı.",
        ));
    }
    for (r, row) in rows.iter().enumerate() {
        let rp = format!("{path}[{r}]");
        range(
            &format!("{rp}.height"),
            row.height,
            1,
            1000,
            "Satır ağırlığı",
        )?;
        if row.cells.is_empty() || row.cells.len() > 32 {
            return Err(err(
                "out_of_range",
                &format!("{rp}.cells"),
                "Antet satırının 1 ile 32 arasında hücresi olmalı.",
            ));
        }
        for (c, cell) in row.cells.iter().enumerate() {
            let cp = format!("{rp}.cells[{c}]");
            range(
                &format!("{cp}.width"),
                cell.width,
                1,
                1000,
                "Hücre ağırlığı",
            )?;
            if let Some(s) = &cell.style {
                text_style(&format!("{cp}.style"), s)?;
            }
            if let Some(f) = &cell.fill {
                color(&format!("{cp}.fill"), f)?;
            }
            if let Some(p) = &cell.picture {
                sha(&format!("{cp}.picture"), p)?;
            }
            if !cell.rows.is_empty() {
                title_rows(&format!("{cp}.rows"), &cell.rows, depth + 1)?;
            }
        }
    }
    Ok(())
}

fn cell_style(path: &str, c: &CellStyle) -> Result<()> {
    text_style(&format!("{path}.text"), &c.text)?;
    if let Some(f) = &c.fill {
        color(&format!("{path}.fill"), f)?;
    }
    range(
        &format!("{path}.padding"),
        c.padding,
        0,
        20_000,
        "Hücre boşluğu (µm)",
    )
}

fn lines(path: &str, l: &TableLines) -> Result<()> {
    for (s, n) in [
        (&l.outer, "outer"),
        (&l.header, "header"),
        (&l.rows, "rows"),
        (&l.columns, "columns"),
    ] {
        if let Some(s) = s {
            stroke(&format!("{path}.{n}"), s)?;
        }
    }
    Ok(())
}

/// Frame-relative points: each coordinate between 0 and 1 000 000.
fn frame_points(path: &str, pts: &[[i32; 2]]) -> Result<()> {
    if pts
        .iter()
        .any(|p| !(0..=FRAME_ONE).contains(&p[0]) || !(0..=FRAME_ONE).contains(&p[1]))
    {
        return Err(err(
            "out_of_range",
            path,
            "Noktalar çerçeveye göre milyonda birle verilir: 0 ile 1 000 000 arasında.",
        ));
    }
    Ok(())
}

fn kind(path: &str, item: &Item) -> Result<()> {
    let p = format!("{path}.kind");
    match &item.kind {
        ItemKind::Map(m) => {
            match &m.view {
                MapView::Fixed(f) => {
                    range(
                        &format!("{p}.view.scale"),
                        f.scale,
                        1,
                        100_000_000,
                        "Ölçek paydası",
                    )?;
                    range(
                        &format!("{p}.view.rotation"),
                        f.rotation,
                        0,
                        FULL_TURN - 1,
                        "Dönüş",
                    )?;
                    if let Some(c) = &f.center
                        && !(c.x.is_finite() && c.y.is_finite())
                    {
                        return Err(err(
                            "out_of_range",
                            &format!("{p}.view.center"),
                            "Harita merkezi sayı olmalı.",
                        ));
                    }
                }
                MapView::Atlas(a) => {
                    range(
                        &format!("{p}.view.rotation"),
                        a.rotation,
                        0,
                        FULL_TURN - 1,
                        "Dönüş",
                    )?;
                    match &a.policy {
                        AtlasScale::Fit(f) => range(
                            &format!("{p}.view.policy.marginPct"),
                            f.margin_pct,
                            0,
                            200,
                            "Kenar payı (%)",
                        )?,
                        AtlasScale::Fixed(f) => range(
                            &format!("{p}.view.policy.scale"),
                            f.scale,
                            1,
                            100_000_000,
                            "Ölçek paydası",
                        )?,
                        AtlasScale::Predefined(s) => {
                            if s.scales.is_empty()
                                || s.scales.iter().any(|x| *x == 0 || *x > 100_000_000)
                            {
                                return Err(err(
                                    "out_of_range",
                                    &format!("{p}.view.policy.scales"),
                                    "Önceden tanımlı ölçekler boş olamaz; her biri 1 ile 100 000 000 arasında olmalı.",
                                ));
                            }
                        }
                    }
                }
            }
            if let Some(o) = &m.overview_of
                && *o == item.id
            {
                return Err(err(
                    "bad_link",
                    &format!("{p}.overviewOf"),
                    "Harita kendisinin genel bakışı olamaz.",
                ));
            }
            stroke(&format!("{p}.overviewFrame"), &m.overview_frame)?;
            let half = item.frame.width.min(item.frame.height) / 2;
            range(
                &format!("{p}.labelBand"),
                m.label_band,
                0,
                half.max(0),
                "Yazı bandı (µm)",
            )?;
            for (g, grid) in m.grids.iter().enumerate() {
                let gp = format!("{p}.grids[{g}]");
                for (v, n) in [(grid.interval, "interval"), (grid.offset, "offset")] {
                    if !(v[0].is_finite() && v[1].is_finite())
                        || (n == "interval" && (v[0] < 0.0 || v[1] < 0.0))
                    {
                        return Err(err(
                            "out_of_range",
                            &format!("{gp}.{n}"),
                            "Karelaj aralığı ve kayması sayı olmalı; aralık eksi olamaz.",
                        ));
                    }
                }
                stroke(&format!("{gp}.stroke"), &grid.stroke)?;
                range(
                    &format!("{gp}.crossSize"),
                    grid.cross_size,
                    0,
                    50_000,
                    "Artı kolu (µm)",
                )?;
                range(
                    &format!("{gp}.frameWidth"),
                    grid.frame_width,
                    0,
                    20_000,
                    "Çerçeve genişliği (µm)",
                )?;
                text_style(&format!("{gp}.labels.style"), &grid.labels.style)?;
                range(
                    &format!("{gp}.labels.gap"),
                    grid.labels.gap,
                    0,
                    20_000,
                    "Yazı aralığı (µm)",
                )?;
                if let GridLabelFormat::Metres(f) = &grid.labels.format {
                    range(
                        &format!("{gp}.labels.format.decimals"),
                        f.decimals,
                        0,
                        6,
                        "Ondalık",
                    )?;
                }
            }
        }
        ItemKind::Text(t) => {
            text_style(&format!("{p}.style"), &t.style)?;
            range(
                &format!("{p}.lineHeight"),
                t.line_height,
                50,
                400,
                "Satır aralığı (%)",
            )?;
            if t.content.chars().count() > 20_000 {
                return Err(err(
                    "out_of_range",
                    &format!("{p}.content"),
                    "Metin en çok 20 000 karakter olabilir.",
                ));
            }
        }
        ItemKind::ScaleBar(s) => {
            range(&format!("{p}.segments"), s.segments, 1, 20, "Parça sayısı")?;
            range(
                &format!("{p}.leftSegments"),
                s.left_segments,
                0,
                5,
                "Sol parça sayısı",
            )?;
            range(
                &format!("{p}.subdivisions"),
                s.subdivisions,
                0,
                10,
                "Alt bölüm",
            )?;
            range(
                &format!("{p}.height"),
                s.height,
                100,
                50_000,
                "Çubuk kalınlığı (µm)",
            )?;
            range(
                &format!("{p}.lineWidth"),
                s.line_width,
                0,
                5_000,
                "Çizgi kalınlığı (µm)",
            )?;
            text_style(&format!("{p}.text"), &s.text)?;
            color(&format!("{p}.color"), &s.color)?;
            color(&format!("{p}.secondary"), &s.secondary)?;
            if let ScaleBarLength::Fixed(f) = &s.length
                && !(f.metres.is_finite() && f.metres > 0.0)
            {
                return Err(err(
                    "out_of_range",
                    &format!("{p}.length.metres"),
                    "Parça uzunluğu artı bir sayı olmalı.",
                ));
            }
        }
        ItemKind::NorthArrow(n) => {
            range(
                &format!("{p}.declination"),
                n.declination,
                -180_000,
                180_000,
                "Manyetik sapma",
            )?;
            color(&format!("{p}.color"), &n.color)?;
            text_style(&format!("{p}.text"), &n.text)?;
        }
        ItemKind::Legend(l) => {
            range(&format!("{p}.columns"), l.columns, 1, 10, "Sütun sayısı")?;
            range(
                &format!("{p}.symbol.width"),
                l.symbol.width,
                500,
                100_000,
                "Simge genişliği (µm)",
            )?;
            range(
                &format!("{p}.symbol.height"),
                l.symbol.height,
                500,
                100_000,
                "Simge yüksekliği (µm)",
            )?;
            range(
                &format!("{p}.wrapWidth"),
                l.wrap_width,
                0,
                FRAME_LIMIT,
                "Kaydırma genişliği (µm)",
            )?;
            for s in [&l.title_style, &l.group_style, &l.label_style] {
                text_style(&format!("{p}.style"), s)?;
            }
        }
        ItemKind::Picture(pic) => {
            if let Some(a) = &pic.asset {
                sha(&format!("{p}.asset"), a)?;
            }
        }
        ItemKind::Shape(s) => {
            if let Some(st) = &s.stroke {
                stroke(&format!("{p}.stroke"), st)?;
            }
            if let Some(f) = &s.fill {
                color(&format!("{p}.fill"), f)?;
            }
            match &s.shape {
                ShapeKind::Rect(r) => range(
                    &format!("{p}.shape.radius"),
                    r.radius,
                    0,
                    FRAME_LIMIT,
                    "Köşe yarıçapı (µm)",
                )?,
                ShapeKind::Polygon(poly) => {
                    if poly.points.len() < 3 || poly.points.len() > 10_000 {
                        return Err(err(
                            "out_of_range",
                            &format!("{p}.shape.points"),
                            "Çokgenin 3 ile 10 000 arasında köşesi olmalı.",
                        ));
                    }
                    frame_points(&format!("{p}.shape.points"), &poly.points)?;
                }
                ShapeKind::Ellipse(_) | ShapeKind::Triangle(_) => {}
            }
        }
        ItemKind::Line(l) => {
            if l.points.len() < 2 || l.points.len() > 10_000 {
                return Err(err(
                    "out_of_range",
                    &format!("{p}.points"),
                    "Çizginin 2 ile 10 000 arasında noktası olmalı.",
                ));
            }
            frame_points(&format!("{p}.points"), &l.points)?;
            stroke(&format!("{p}.stroke"), &l.stroke)?;
        }
        ItemKind::Table(t) => {
            if t.columns.len() > 64 {
                return Err(err(
                    "out_of_range",
                    &format!("{p}.columns"),
                    "Tablonun en çok 64 sütunu olabilir.",
                ));
            }
            for (c, col) in t.columns.iter().enumerate() {
                range(
                    &format!("{p}.columns[{c}].width"),
                    col.width,
                    0,
                    FRAME_LIMIT,
                    "Sütun genişliği (µm)",
                )?;
                if let Some(d) = col.decimals {
                    range(&format!("{p}.columns[{c}].decimals"), d, 0, 9, "Ondalık")?;
                }
            }
            if let TableSource::Fixed(f) = &t.source {
                for (r, row) in f.rows.iter().enumerate() {
                    if row.len() > t.columns.len().max(1) {
                        return Err(err(
                            "out_of_range",
                            &format!("{p}.source.rows[{r}]"),
                            "Satırda sütun sayısından çok hücre var.",
                        ));
                    }
                }
                if f.rows.len() > 10_000 {
                    return Err(err(
                        "out_of_range",
                        &format!("{p}.source.rows"),
                        "Tablonun en çok 10 000 satırı olabilir.",
                    ));
                }
            }
            text_style(&format!("{p}.titleStyle"), &t.title_style)?;
            cell_style(&format!("{p}.headerStyle"), &t.header_style)?;
            cell_style(&format!("{p}.cellStyle"), &t.cell_style)?;
            lines(&format!("{p}.lines"), &t.lines)?;
            if let Some(z) = &t.zebra {
                color(&format!("{p}.zebra"), z)?;
            }
        }
        ItemKind::CoordinateList(c) => {
            range(&format!("{p}.decimals"), c.decimals, 0, 6, "Ondalık")?;
            coord_source(&format!("{p}.source"), &c.source)?;
            if let Some(cols) = &c.columns {
                coord_columns(&format!("{p}.columns"), cols)?;
            }
            text_style(&format!("{p}.titleStyle"), &c.title_style)?;
            cell_style(&format!("{p}.headerStyle"), &c.header_style)?;
            cell_style(&format!("{p}.cellStyle"), &c.cell_style)?;
            lines(&format!("{p}.lines"), &c.lines)?;
        }
        ItemKind::TitleBlock(tb) => {
            title_rows(&format!("{p}.rows"), &tb.rows, 0)?;
            stroke(&format!("{p}.outer"), &tb.outer)?;
            stroke(&format!("{p}.inner"), &tb.inner)?;
            text_style(&format!("{p}.labelStyle"), &tb.label_style)?;
            text_style(&format!("{p}.valueStyle"), &tb.value_style)?;
            range(
                &format!("{p}.cellPadding"),
                tb.cell_padding,
                0,
                20_000,
                "Hücre boşluğu (µm)",
            )?;
        }
        ItemKind::Border(b) => {
            let half = item.frame.width.min(item.frame.height) / 2;
            range(
                &format!("{p}.inset"),
                b.inset,
                0,
                half.max(0),
                "İç pay (µm)",
            )?;
            range(&format!("{p}.gap"), b.gap, 0, 100_000, "Çizgi arası (µm)")?;
            stroke(&format!("{p}.stroke"), &b.stroke)?;
            stroke(&format!("{p}.innerStroke"), &b.inner_stroke)?;
            if let Some(z) = &b.zones {
                range(
                    &format!("{p}.zones.size"),
                    z.size,
                    10_000,
                    1_000_000,
                    "Bölge boyu (µm)",
                )?;
                text_style(&format!("{p}.zones.text"), &z.text)?;
            }
        }
        ItemKind::Group(_) => {}
    }
    Ok(())
}

fn item(path: &str, it: &Item) -> Result<()> {
    if !valid_id(&it.id) {
        return Err(err(
            "bad_id",
            &format!("{path}.id"),
            "Öğe kimliği 1 ile 128 karakter arasında olmalı.",
        ));
    }
    if it.name.trim().is_empty() || it.name.chars().count() > 120 {
        return Err(err(
            "bad_name",
            &format!("{path}.name"),
            "Öğe adı boş olamaz ve en çok 120 karakter olabilir.",
        ));
    }
    frame(&format!("{path}.frame"), &it.frame)?;
    range(
        &format!("{path}.rotation"),
        it.rotation,
        0,
        FULL_TURN - 1,
        "Dönüş (binde bir derece)",
    )?;
    range(&format!("{path}.opacity"), it.opacity, 0, 100, "Saydamlık")?;
    let half = it.frame.width.min(it.frame.height) / 2;
    range(
        &format!("{path}.padding"),
        it.padding,
        0,
        half.max(0),
        "İç boşluk (µm)",
    )?;
    if let Some(b) = &it.border {
        stroke(&format!("{path}.border"), b)?;
    }
    if let Some(f) = &it.fill {
        color(&format!("{path}.fill"), f)?;
    }
    if it.bindings.len() > 64 {
        return Err(err(
            "out_of_range",
            &format!("{path}.bindings"),
            "Bir öğenin en çok 64 bağı olabilir.",
        ));
    }
    let mut props = BTreeSet::new();
    for (b, binding) in it.bindings.iter().enumerate() {
        let bp = format!("{path}.bindings[{b}]");
        if bind::bind_type(&binding.property, &it.kind).is_none() {
            return Err(err(
                "bad_binding",
                &format!("{bp}.property"),
                format!(
                    "“{}” özelliği {} öğesinde veriye bağlanamaz.",
                    binding.property,
                    it.kind.label()
                ),
            ));
        }
        if !props.insert(binding.property.as_str()) {
            return Err(err(
                "bad_binding",
                &format!("{bp}.property"),
                format!("“{}” özelliği iki kez bağlanmış.", binding.property),
            ));
        }
        if binding.expression.trim().is_empty() {
            return Err(err(
                "bad_binding",
                &format!("{bp}.expression"),
                "Bağın ifadesi boş olamaz.",
            ));
        }
    }
    kind(path, it)
}

/// The rules every owner's item list follows: names unique, groups real and acyclic, continuations sound.
fn item_list(path: &str, items: &[Item]) -> Result<()> {
    let mut names = BTreeSet::new();
    let by_id: BTreeMap<&str, &Item> = items.iter().map(|i| (i.id.as_str(), i)).collect();
    for (i, it) in items.iter().enumerate() {
        let p = format!("{path}[{i}]");
        item(&p, it)?;
        if !names.insert(it.name.as_str()) {
            return Err(err(
                "duplicate_name",
                &format!("{p}.name"),
                format!(
                    "“{}” adı bu paftada iki öğede var: her öğenin adı tekil olmalı.",
                    it.name
                ),
            ));
        }
        if let Some(g) = &it.group {
            match by_id.get(g.as_str()) {
                Some(parent) if parent.is_group() => {}
                Some(_) => {
                    return Err(err(
                        "bad_group",
                        &format!("{p}.group"),
                        format!("“{g}” bir grup değil."),
                    ));
                }
                None => {
                    return Err(err(
                        "bad_group",
                        &format!("{p}.group"),
                        format!("“{g}” grubu bu paftada yok."),
                    ));
                }
            }
            // A cycle: walking up from here comes back.
            let mut seen = BTreeSet::new();
            seen.insert(it.id.as_str());
            let mut up = Some(g.as_str());
            while let Some(u) = up {
                if !seen.insert(u) {
                    return Err(err(
                        "group_cycle",
                        &format!("{p}.group"),
                        "Gruplar birbirini içeriyor.",
                    ));
                }
                up = by_id.get(u).and_then(|x| x.group.as_deref());
            }
        }
        if it.constraints.relative_to == ConstraintBox::Group && it.group.is_none() {
            return Err(err(
                "bad_constraint",
                &format!("{p}.constraints.relativeTo"),
                "Gruba göre kısıt yalnız bir grubun öğesinde olabilir.",
            ));
        }
    }
    // Groups have children.
    for (i, it) in items.iter().enumerate() {
        if it.is_group()
            && !items
                .iter()
                .any(|c| c.group.as_deref() == Some(it.id.as_str()))
        {
            return Err(err(
                "empty_group",
                &format!("{path}[{i}]"),
                format!("“{}” grubunun öğesi yok.", it.name),
            ));
        }
    }
    // Continuation frames: each target a `continued` table of this list, used once, not itself.
    let mut used = BTreeSet::new();
    for (i, it) in items.iter().enumerate() {
        let overflow = match &it.kind {
            ItemKind::Table(t) => &t.overflow,
            ItemKind::CoordinateList(c) => &c.overflow,
            _ => continue,
        };
        if let Overflow::ContinueIn(c) = overflow {
            if matches!(&it.kind, ItemKind::Table(t) if matches!(t.source, TableSource::Continued(_)))
            {
                return Err(err(
                    "bad_continuation",
                    &format!("{path}[{i}].kind.overflow"),
                    "Devam çerçevesi başka bir yere devam edemez.",
                ));
            }
            for target in &c.items {
                let ok = by_id.get(target.as_str()).is_some_and(|t| {
                    matches!(&t.kind, ItemKind::Table(tt) if matches!(tt.source, TableSource::Continued(_)))
                });
                if !ok || target == &it.id {
                    return Err(err(
                        "bad_continuation",
                        &format!("{path}[{i}].kind.overflow"),
                        format!("“{target}” bu paftada “devam” kaynaklı bir tablo değil."),
                    ));
                }
                if !used.insert(target.as_str()) {
                    return Err(err(
                        "bad_continuation",
                        &format!("{path}[{i}].kind.overflow"),
                        format!("“{target}” çerçevesine iki tablo devam ediyor."),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn variant_frames(path: &str, items: &[Item], frames: &[VariantFrame]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for (k, f) in frames.iter().enumerate() {
        let p = format!("{path}[{k}]");
        let Some(it) = items.iter().find(|i| i.id == f.item) else {
            return Err(err(
                "unknown_item",
                &format!("{p}.item"),
                format!("Yerleşim düzeni olmayan bir öğeyi anıyor (“{}”).", f.item),
            ));
        };
        if !seen.insert(f.item.as_str()) {
            return Err(err(
                "duplicate_id",
                &format!("{p}.item"),
                format!("“{}” öğesi düzende iki kez anılıyor.", it.name),
            ));
        }
        frame(&format!("{p}.frame"), &f.frame)?;
        if f.constraints.relative_to == ConstraintBox::Group && it.group.is_none() {
            return Err(err(
                "bad_constraint",
                &format!("{p}.constraints.relativeTo"),
                format!("“{}” bir grupta değil; kısıtı gruba göre olamaz.", it.name),
            ));
        }
    }
    Ok(())
}

fn reference_size(path: &str, s: &SizeUm) -> Result<()> {
    range(
        &format!("{path}.width"),
        s.width,
        PAGE_MIN,
        PAGE_MAX,
        "Düzenin kâğıt genişliği (µm)",
    )?;
    range(
        &format!("{path}.height"),
        s.height,
        PAGE_MIN,
        PAGE_MAX,
        "Düzenin kâğıt yüksekliği (µm)",
    )
}

/// The layout variants of a sheet or master page (design §3.2a).
fn layouts(
    path: &str,
    items: &[Item],
    variants: &[LayoutVariant],
    active: &Option<String>,
    base: &Option<BaseLayout>,
) -> Result<()> {
    let mut ids = BTreeSet::new();
    for (i, v) in variants.iter().enumerate() {
        let p = format!("{path}.variants[{i}]");
        if !valid_id(&v.id) || !ids.insert(v.id.as_str()) {
            return Err(err(
                "duplicate_id",
                &format!("{p}.id"),
                "Yerleşim düzeni kimlikleri boş olmamalı ve tekil olmalı.",
            ));
        }
        if v.name.trim().is_empty() || v.name.chars().count() > 120 {
            return Err(err(
                "bad_name",
                &format!("{p}.name"),
                "Yerleşim düzeninin adı boş olamaz ve en çok 120 karakter olabilir.",
            ));
        }
        reference_size(&format!("{p}.reference"), &v.reference)?;
        let w = &v.when;
        for (lo, hi, n) in [
            (w.min_width, w.max_width, "Width"),
            (w.min_height, w.max_height, "Height"),
        ] {
            for (x, side) in [(lo, "min"), (hi, "max")] {
                if let Some(x) = x {
                    range(
                        &format!("{p}.when.{side}{n}"),
                        x,
                        0,
                        PAGE_MAX,
                        "Kâğıt boyu (µm)",
                    )?;
                }
            }
            if let (Some(a), Some(b)) = (lo, hi)
                && a > b
            {
                return Err(err(
                    "out_of_range",
                    &format!("{p}.when"),
                    "Düzenin koşulunda en az, en çoktan büyük olamaz.",
                ));
            }
        }
        variant_frames(&format!("{p}.frames"), items, &v.frames)?;
    }
    match (active, base) {
        (Some(a), Some(b)) => {
            if !ids.contains(a.as_str()) {
                return Err(err(
                    "unknown_variant",
                    &format!("{path}.activeVariant"),
                    format!("“{a}” yerleşim düzeni yok."),
                ));
            }
            reference_size(&format!("{path}.baseLayout.reference"), &b.reference)?;
            variant_frames(&format!("{path}.baseLayout.frames"), items, &b.frames)
        }
        (None, None) => Ok(()),
        _ => Err(err(
            "bad_layout",
            &format!("{path}.baseLayout"),
            "Bir yerleşim düzeni geçerliyken temel düzen saklanır; biri olmadan öbürü olamaz.",
        )),
    }
}

fn guides(path: &str, gs: &[Guide]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for (i, g) in gs.iter().enumerate() {
        if !valid_id(&g.id) || !ids.insert(g.id.as_str()) {
            return Err(err(
                "duplicate_id",
                &format!("{path}[{i}].id"),
                "Kılavuz kimlikleri boş olmamalı ve tekil olmalı.",
            ));
        }
        range(
            &format!("{path}[{i}].at"),
            g.at,
            -FRAME_LIMIT,
            FRAME_LIMIT,
            "Kılavuz yeri (µm)",
        )?;
    }
    Ok(())
}

/// Every rule of the book; the first one broken is the error.
pub fn validate_book(book: &SheetBook) -> Result<()> {
    if book.schema != BOOK_SCHEMA {
        return Err(err(
            "unknown_schema",
            "schema",
            format!(
                "Pafta kitabının biçimi “{}”; bu sürüm “{BOOK_SCHEMA}” okur. Uygulamayı güncelleyin.",
                book.schema
            ),
        ));
    }
    let mut item_ids = BTreeSet::new();
    let mut sheet_ids = BTreeSet::new();
    let mut master_ids = BTreeSet::new();
    for (i, m) in book.masters.iter().enumerate() {
        let p = format!("masters[{i}]");
        if !valid_id(&m.id) || !master_ids.insert(m.id.as_str()) {
            return Err(err(
                "duplicate_id",
                &format!("{p}.id"),
                "Ana sayfa kimlikleri boş olmamalı ve tekil olmalı.",
            ));
        }
        if m.name.trim().is_empty() {
            return Err(err(
                "bad_name",
                &format!("{p}.name"),
                "Ana sayfanın adı boş olamaz.",
            ));
        }
        validate_page(&format!("{p}.page"), &m.page)?;
        item_list(&format!("{p}.items"), &m.items)?;
        guides(&format!("{p}.guides"), &m.guides)?;
        layouts(&p, &m.items, &m.variants, &m.active_variant, &m.base_layout)?;
        for (j, it) in m.items.iter().enumerate() {
            if !item_ids.insert(it.id.as_str()) {
                return Err(err(
                    "duplicate_id",
                    &format!("{p}.items[{j}].id"),
                    format!("“{}” kimliği kitapta iki kez var.", it.id),
                ));
            }
        }
    }
    for (i, s) in book.sheets.iter().enumerate() {
        let p = format!("sheets[{i}]");
        if !valid_id(&s.id) || !sheet_ids.insert(s.id.as_str()) {
            return Err(err(
                "duplicate_id",
                &format!("{p}.id"),
                "Pafta kimlikleri boş olmamalı ve tekil olmalı.",
            ));
        }
        if s.name.trim().is_empty() || s.name.chars().count() > 120 {
            return Err(err(
                "bad_name",
                &format!("{p}.name"),
                "Paftanın adı boş olamaz ve en çok 120 karakter olabilir.",
            ));
        }
        validate_page(&format!("{p}.page"), &s.page)?;
        if let Some(m) = &s.master
            && !master_ids.contains(m.as_str())
        {
            return Err(err(
                "unknown_master",
                &format!("{p}.master"),
                format!("“{m}” ana sayfası kitapta yok."),
            ));
        }
        item_list(&format!("{p}.items"), &s.items)?;
        guides(&format!("{p}.guides"), &s.guides)?;
        layouts(&p, &s.items, &s.variants, &s.active_variant, &s.base_layout)?;
        range(
            &format!("{p}.snapGrid.spacing"),
            s.snap_grid.spacing,
            100,
            100_000,
            "Izgara aralığı (µm)",
        )?;
        range(
            &format!("{p}.export.dpi"),
            s.export.dpi,
            72,
            2400,
            "Çözünürlük (dpi)",
        )?;
        if let Some(a) = &s.atlas
            && a.page_name.trim().is_empty()
        {
            return Err(err(
                "bad_atlas",
                &format!("{p}.atlas.pageName"),
                "Atlas sayfa adı ifadesi boş olamaz.",
            ));
        }
        validate_variables(&format!("{p}.variables"), &s.variables)?;
        for (j, it) in s.items.iter().enumerate() {
            if !item_ids.insert(it.id.as_str()) {
                return Err(err(
                    "duplicate_id",
                    &format!("{p}.items[{j}].id"),
                    format!("“{}” kimliği kitapta iki kez var.", it.id),
                ));
            }
        }
    }
    let mut shas = BTreeSet::new();
    for (i, a) in book.assets.iter().enumerate() {
        let p = format!("assets[{i}]");
        sha(&format!("{p}.sha256"), &a.sha256)?;
        if !shas.insert(a.sha256.as_str()) {
            return Err(err(
                "duplicate_id",
                &format!("{p}.sha256"),
                "Aynı varlık iki kez kayıtlı.",
            ));
        }
        if a.width == 0 || a.height == 0 || a.width > 100_000 || a.height > 100_000 {
            return Err(err(
                "bad_asset",
                &format!("{p}.width"),
                "Resmin piksel boyu 1 ile 100 000 arasında olmalı.",
            ));
        }
        if a.bytes > ASSET_MAX_BYTES {
            return Err(err(
                "asset_too_large",
                &format!("{p}.bytes"),
                "Bir resim en çok 4 MB olabilir; küçültüp yeniden ekleyin.",
            ));
        }
        if a.name.chars().count() > 255 {
            return Err(err(
                "bad_asset",
                &format!("{p}.name"),
                "Varlığın adı en çok 255 karakter olabilir.",
            ));
        }
    }
    validate_variables("variables", &book.variables)
}

/// The book in its normal form: colours in lowercase, every group's frame its children's together.
pub fn normalize(book: &mut SheetBook) {
    for s in &mut book.sheets {
        if let Some(b) = &mut s.page.background {
            *b = b.to_ascii_lowercase();
        }
        normalize_items(&mut s.items);
    }
    for m in &mut book.masters {
        if let Some(b) = &mut m.page.background {
            *b = b.to_ascii_lowercase();
        }
        normalize_items(&mut m.items);
    }
}

fn lower(s: &mut str) {
    s.make_ascii_lowercase();
}

fn lower_stroke(s: &mut Stroke) {
    lower(&mut s.color);
}

fn lower_text(t: &mut TextStyle) {
    lower(&mut t.color);
}

fn lower_cells(c: &mut CellStyle) {
    lower_text(&mut c.text);
    if let Some(f) = &mut c.fill {
        lower(f);
    }
}

fn lower_lines(l: &mut TableLines) {
    for s in [&mut l.outer, &mut l.header, &mut l.rows, &mut l.columns]
        .into_iter()
        .flatten()
    {
        lower_stroke(s);
    }
}

fn lower_rows(rows: &mut [TitleRow]) {
    for r in rows {
        for c in &mut r.cells {
            if let Some(s) = &mut c.style {
                lower_text(s);
            }
            if let Some(f) = &mut c.fill {
                lower(f);
            }
            lower_rows(&mut c.rows);
        }
    }
}

fn normalize_item_colors(it: &mut Item) {
    if let Some(b) = &mut it.border {
        lower_stroke(b);
    }
    if let Some(f) = &mut it.fill {
        lower(f);
    }
    match &mut it.kind {
        ItemKind::Map(m) => {
            lower_stroke(&mut m.overview_frame);
            for g in &mut m.grids {
                lower_stroke(&mut g.stroke);
                lower_text(&mut g.labels.style);
            }
        }
        ItemKind::Text(t) => lower_text(&mut t.style),
        ItemKind::ScaleBar(s) => {
            lower_text(&mut s.text);
            lower(&mut s.color);
            lower(&mut s.secondary);
        }
        ItemKind::NorthArrow(n) => {
            lower(&mut n.color);
            lower_text(&mut n.text);
        }
        ItemKind::Legend(l) => {
            lower_text(&mut l.title_style);
            lower_text(&mut l.group_style);
            lower_text(&mut l.label_style);
        }
        ItemKind::Shape(s) => {
            if let Some(st) = &mut s.stroke {
                lower_stroke(st);
            }
            if let Some(f) = &mut s.fill {
                lower(f);
            }
        }
        ItemKind::Line(l) => lower_stroke(&mut l.stroke),
        ItemKind::Table(t) => {
            lower_text(&mut t.title_style);
            lower_cells(&mut t.header_style);
            lower_cells(&mut t.cell_style);
            lower_lines(&mut t.lines);
            if let Some(z) = &mut t.zebra {
                lower(z);
            }
        }
        ItemKind::CoordinateList(c) => {
            lower_text(&mut c.title_style);
            lower_cells(&mut c.header_style);
            lower_cells(&mut c.cell_style);
            lower_lines(&mut c.lines);
        }
        ItemKind::TitleBlock(tb) => {
            lower_stroke(&mut tb.outer);
            lower_stroke(&mut tb.inner);
            lower_text(&mut tb.label_style);
            lower_text(&mut tb.value_style);
            lower_rows(&mut tb.rows);
        }
        ItemKind::Border(b) => {
            lower_stroke(&mut b.stroke);
            lower_stroke(&mut b.inner_stroke);
            if let Some(z) = &mut b.zones {
                lower_text(&mut z.text);
            }
        }
        ItemKind::Picture(_) | ItemKind::Group(_) => {}
    }
}

/// Colours lowered and every group's frame recomputed, deepest first.
pub fn normalize_items(items: &mut [Item]) {
    for it in items.iter_mut() {
        normalize_item_colors(it);
    }
    group_frames(items);
}

/// Every group's frame: the upright box around its children (turned ones by their turned corners), deepest groups first.
pub fn group_frames(items: &mut [Item]) {
    // Depth of each group, so a group of groups is computed after its children.
    let depth = |items: &[Item], id: &str| -> usize {
        let mut d = 0;
        let mut up = items
            .iter()
            .find(|i| i.id == id)
            .and_then(|i| i.group.clone());
        while let Some(u) = up {
            d += 1;
            if d > items.len() {
                break;
            }
            up = items
                .iter()
                .find(|i| i.id == u)
                .and_then(|i| i.group.clone());
        }
        d
    };
    let mut groups: Vec<(usize, String)> = items
        .iter()
        .filter(|i| i.is_group())
        .map(|g| (depth(items, &g.id), g.id.clone()))
        .collect();
    groups.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    for (_, gid) in groups {
        let mut bounds: Option<RectUm> = None;
        for c in items
            .iter()
            .filter(|c| c.group.as_deref() == Some(gid.as_str()))
        {
            let b = rotated_bounds(&c.frame, c.rotation);
            bounds = Some(bounds.map_or(b, |u| u.union(&b)));
        }
        if let (Some(b), Some(g)) = (bounds, items.iter_mut().find(|i| i.id == gid)) {
            g.frame = RectUm::new(b.left, b.top, b.width.max(ITEM_MIN), b.height.max(ITEM_MIN));
            g.rotation = 0;
        }
    }
}

/// Reads a book from JSON: parsed (an unknown field is an error), validated and normalised.
pub fn read_book(json: &str) -> Result<SheetBook> {
    let mut book: SheetBook =
        serde_json::from_str(json).map_err(|e| SheetError::json("Pafta kitabı", &e))?;
    validate_book(&book)?;
    normalize(&mut book);
    Ok(book)
}
