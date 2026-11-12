//! A dimension's geometry as the commands take it (docs/adr/0147 §6): only
//! a slope has elevations, an ordinate's axis is 0 (its Y) or 90 (its X),
//! and a new kind is one the core can draw (`dimension_fault`). Refused as
//! `invalid_dimension` with the place to mend; the web's
//! `product/dimension.ts` says the same.

use kentos_commands::{DimensionStyle, EntityGeometry, Vec2};
use kentos_geometry_core::geom::dimension::{DimensionFault, DimensionGeom, dimension_fault};

use crate::checks::{Stop, error};
use crate::codes;

/// Checks a dimension's geometry (other geometries pass); `at` names a
/// field's place in the input.
pub(crate) fn check(g: &EntityGeometry, at: &dyn Fn(&str) -> Option<String>) -> Result<(), Stop> {
    let EntityGeometry::Dimension {
        a,
        b,
        offset,
        height,
        style,
        angle,
        c,
        za,
        zb,
        look,
        level,
        ..
    } = g
    else {
        return Ok(());
    };
    let refuse = |message: String, field: &str| {
        Err(Stop::failed(error(
            codes::INVALID_DIMENSION,
            message,
            at(field),
        )))
    };
    // A Kot's level (docs/adr/0721 §2): an elevation mark's only, a level's id and a finite offset.
    if let Some(l) = level {
        if *style != Some(DimensionStyle::Elevation) {
            return refuse("Katı yalnız kot ölçüsü anar. level'i kaldırın ya da ölçünün biçimini kot yapın.".into(), ".level");
        }
        if let Some(words) = l.problem() {
            return refuse(words, ".level");
        }
    }
    if *style == Some(DimensionStyle::Elevation) && zb.is_some() {
        return refuse(
            "Kot ölçüsünün yalnız bir kotu (za) olur. zb'yi kaldırın.".into(),
            ".zb",
        );
    }
    if !matches!(
        *style,
        Some(DimensionStyle::Slope | DimensionStyle::Elevation)
    ) && (za.is_some() || zb.is_some())
    {
        return refuse(
            "Kot yalnız eğim ölçüsünde olur. Kotları (za, zb) kaldırın ya da ölçünün biçimini eğim yapın.".into(),
            if za.is_some() { ".za" } else { ".zb" },
        );
    }
    if *style == Some(DimensionStyle::Ordinate)
        && let Some(t) = angle
        && *t != 0.0
        && *t != 90.0
    {
        return refuse(
            format!(
                "Koordinat ölçüsünün ekseni 0 (Y) ya da 90 (X) olmalı; {t} verildi. Y için 0, X için 90 verin."
            ),
            ".angle",
        );
    }
    let v = |p: &Vec2| kentos_geometry_core::Vec2::new(p.x, p.y);
    let geom = DimensionGeom {
        a: v(a),
        b: v(b),
        offset: *offset,
        height: *height,
        style: style.map(|s| s.name().to_owned()),
        angle: *angle,
        c: c.as_ref().map(v),
        za: *za,
        zb: *zb,
        look: crate::geometry::core_look(look),
    };
    let Some(fault) = dimension_fault(&geom) else {
        return Ok(());
    };
    let edge = if *style == Some(DimensionStyle::Slope) {
        "Eğim"
    } else {
        "Semt"
    };
    let (message, field) = match fault {
        DimensionFault::OrdinateTooShort => (
            "Koordinat ölçüsünün çizgisi noktadan eksene dik yönde yazı yüksekliğinin yarısından uzun olmalı. Çizginin ucunu (b) noktadan daha uzağa verin.".to_owned(),
            ".b",
        ),
        DimensionFault::ArcNoCentre => (
            "Yay uzunluğu ölçüsünün merkezi (c) verilmeli. Ölçülen yayın merkezini verin.".into(),
            ".c",
        ),
        DimensionFault::ArcNoRadius => (
            "Yay uzunluğu ölçüsünde yayın başlangıcı (a) merkezde (c); yarıçap sıfır. Başlangıcı yayın üstünde verin.".into(),
            ".a",
        ),
        DimensionFault::ArcNoSweep => (
            "Yay uzunluğu ölçüsünde yayın iki ucu merkezden aynı doğrultuda; yayın açısı sıfır. Sonu (b) başka bir doğrultuda verin.".into(),
            ".b",
        ),
        DimensionFault::ArcInside => (
            "Yay uzunluğu ölçüsünün ölçü yayı merkeze ulaşıyor: içe ötelenme yarıçaptan küçük olmalı. Ötelenmeyi büyütün.".into(),
            ".offset",
        ),
        DimensionFault::JoggedNoCentre => (
            "Kırıklı yarıçap ölçüsünün gösterilen merkezi (c) verilmeli. Çizginin başlayacağı noktayı verin.".into(),
            ".c",
        ),
        DimensionFault::JoggedNoRadius => (
            "Kırıklı yarıçap ölçüsünde yaydaki nokta (b) merkezde (a); yarıçap sıfır. Noktayı yayın üstünde verin.".into(),
            ".b",
        ),
        DimensionFault::JoggedCentre => (
            "Kırıklı yarıçap ölçüsünde gösterilen merkez (c) yarıçap boyunca yaydaki noktadan (b) geride olmalı; yarıçap çizgisinden uzaklığı bu geriliği aşmamalı. Gösterilen merkezi yayın içinde, yarıçapa yakın verin.".into(),
            ".c",
        ),
        DimensionFault::EdgeTooShort => (
            format!(
                "{edge} ölçüsünün iki ucu aynı nokta; ölçülecek kenar yok. Kenarın öbür ucunu (b) verin."
            ),
            ".b",
        ),
        DimensionFault::SlopeNoElevations => (
            "Eğim ölçüsünün iki ucunun da kotu verilmeli. Eksik kotu (za ya da zb) metre olarak verin.".into(),
            if za.is_none() { ".za" } else { ".zb" },
        ),
        DimensionFault::MarkNoRadius => (
            "Merkez işaretinin eğrideki noktası (b) merkezde (a); yarıçap sıfır. Noktayı eğrinin üstünde verin.".into(),
            ".b",
        ),
        DimensionFault::LevelNoElevation => (
            "Kot ölçüsünün kotu (za) verilmeli. Noktanın kotunu metre olarak verin.".into(),
            ".za",
        ),
    };
    refuse(message, field)
}
