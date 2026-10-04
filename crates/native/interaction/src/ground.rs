//! Plane, ellipsoid and ground in Mesafe ölç and Alan hesapla
//! (docs/adr/0171 §4a): after their own line, what was measured on the
//! project's ellipsoid and on the ground at its mean ellipsoidal height
//! (`crs::ground`); the web's `model/groundMeasures.ts` writes the same. Only
//! a project that names the height asks for them: any other says what it
//! said before, its own line last (the status bar shows the last), and a
//! project without a coordinate system has no ellipsoid.

use kentos_contracts::ProjectSettings;
use kentos_geometry_core::crs::ground::{Grid, ground_measures, has_point_scale};
use kentos_geometry_core::crs::measure::Ring;
use kentos_geometry_core::display::fixed;
use kentos_project::systems;

use crate::format::Format;

/// Said in place of both when a point is beyond the project's system.
pub const UNREACHED: &str = "Elipsoit üstünde: ölçülen yerin bir noktası projenin sisteminin ulaştığı yerin dışında; değer yazılmadı.";

/// The grid the survey windows take measured lengths to (docs/adr/0171 §4):
/// the project asks for it (Uzunlukları projeksiyona indir, with a height)
/// and its system has one scale at a point; none otherwise.
pub fn survey_grid(settings: &ProjectSettings) -> Option<Grid> {
    if !settings.reduces_to_grid() || why_not_grid(settings).is_some() {
        return None;
    }
    Some(Grid {
        system: systems::own(settings)?.system?,
        height: settings.ground_height()?,
    })
}

/// Why Uzunlukları projeksiyona indir cannot be turned on (docs/adr/0171 §4).
pub const NEEDS_HEIGHT: &str = "Ortalama elipsoit yüksekliği yazılınca açılır.";
pub const NEEDS_SYSTEM: &str =
    "Projenin koordinat sistemi yok: uzunluklar projeksiyona indirilemez.";
pub const NEEDS_SCALE: &str = "Projenin sisteminde bir noktanın tek ölçeği yok (coğrafi sistem, Pseudo-Mercator ya da afinle bağlı yerel sistem): uzunluklar projeksiyona indirilemez.";

/// Why the survey windows cannot take lengths to the grid with these
/// settings, or none: a height, a system, and one scale at a point in it.
pub fn why_not_grid(settings: &ProjectSettings) -> Option<&'static str> {
    if settings.ground_height().is_none() {
        return Some(NEEDS_HEIGHT);
    }
    match systems::own(settings).and_then(|n| n.system) {
        None => Some(NEEDS_SYSTEM),
        Some(s) if !has_point_scale(&s) => Some(NEEDS_SCALE),
        Some(_) => None,
    }
}

/// What Kutupsal alım and Poligon hesabı say when they take lengths to the grid.
pub fn grid_note(height: f64) -> String {
    format!(
        "Ölçülen uzunluklar projeksiyona indirildi: ortalama elipsoit yüksekliği {} m, çizginin ölçeği ve yükseklik çarpanıyla (Proje ayarları › Ölçme).",
        trimmed(height)
    )
}

/// What Aplikasyon says when it gives the ground's distances.
pub fn stake_note(height: f64) -> String {
    format!(
        "Zemin uzunlukları da verildi: ortalama elipsoit yüksekliği {} m, çizginin ölçeği ve yükseklik çarpanıyla (Proje ayarları › Ölçme). Arazide zemindekini ölçün.",
        trimmed(height)
    )
}

/// A height as the form writes it: the display rule's four decimals without
/// trailing zeros or a bare point.
pub fn trimmed(v: f64) -> String {
    let s = fixed(v, 4);
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s.as_str()
    };
    if s.is_empty() || s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

/// The lines Mesafe ölç (`closed` false: the first ring, a path) and Alan
/// hesapla (the rings, the first the outer) say after their own: the
/// ellipsoid's, with the plane's scale against it, and the ground's, with
/// the height's factor; none in a project that names no height or has no
/// coordinate system.
pub fn lines(settings: &ProjectSettings, rings: &[Ring], closed: bool, f: &Format) -> Vec<String> {
    let Some(height) = settings.ground_height() else {
        return Vec::new();
    };
    let Some(system) = systems::own(settings).and_then(|n| n.system) else {
        return Vec::new();
    };
    let Ok(m) = ground_measures(&system, rings, closed, Some(height)) else {
        return vec![UNREACHED.to_owned()];
    };
    let what = |area: Option<f64>, length: f64| match area {
        Some(a) if closed => format!("Alan {}   Çevre {}", f.area(a), f.length(length)),
        _ => format!("Toplam uzunluk {}", f.length(length)),
    };
    let scale = m
        .scale
        .map(|k| format!("   Ölçek {}", fixed(k, 8)))
        .unwrap_or_default();
    let ellipsoid = format!(
        "Elipsoit üstünde: {}{scale}",
        what(m.ellipsoid_area, m.ellipsoid_length)
    );
    let factor = m
        .height_factor
        .map(|k| format!("   Yükseklik çarpanı {}", fixed(k, 8)))
        .unwrap_or_default();
    let ground = format!(
        "Zeminde (h = {} m): {}{factor}",
        trimmed(height),
        what(m.ground_area, m.ground_length.unwrap_or(0.0))
    );
    vec![ellipsoid, ground]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;

    fn settings(json: &str) -> ProjectSettings {
        serde_json::from_str(json).expect("settings")
    }

    fn square(o: Vec2, side: f64) -> Ring {
        Ring {
            pts: vec![
                o,
                Vec2::new(o.x + side, o.y),
                Vec2::new(o.x + side, o.y + side),
                Vec2::new(o.x, o.y + side),
            ],
            bulges: None,
        }
    }

    #[test]
    fn the_lines_name_the_ellipsoid_and_the_ground() {
        let tm30 = settings(
            r#"{"srid":5254,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"survey":{"groundHeight":850}}"#,
        );
        let f = Format::of(&tm30);
        let ring = square(Vec2::new(414000.0, 4540000.0), 40.0);
        let said = lines(&tm30, std::slice::from_ref(&ring), true, &f);
        assert_eq!(said.len(), 2);
        assert!(
            said[0].starts_with("Elipsoit üstünde: Alan 1599.")
                && said[0].contains("   Ölçek 1.0000"),
            "{said:?}"
        );
        assert!(
            said[1].starts_with("Zeminde (h = 850 m): Alan ")
                && said[1].contains("   Yükseklik çarpanı 0.99986"),
            "{said:?}"
        );
        // A path: its total length.
        let path = lines(&tm30, &[ring], false, &f);
        assert!(
            path[0].starts_with("Elipsoit üstünde: Toplam uzunluk 119.9"),
            "{path:?}"
        );
        // Without a height, nothing more: the measure's own line stays last.
        let no_height = settings(
            r#"{"srid":5254,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000}"#,
        );
        let ring = square(Vec2::new(414000.0, 4540000.0), 40.0);
        assert!(lines(&no_height, &[ring], true, &f).is_empty());
        // A local project has no ellipsoid, a height or none.
        let local = settings(
            r#"{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100,"survey":{"groundHeight":850}}"#,
        );
        assert!(lines(&local, &[square(Vec2::new(0.0, 0.0), 40.0)], true, &f).is_empty());
    }

    #[test]
    fn a_height_is_written_as_the_form_writes_it() {
        assert_eq!(trimmed(850.0), "850");
        assert_eq!(trimmed(850.25), "850.25");
        assert_eq!(trimmed(-27.123456), "-27.1235");
        assert_eq!(trimmed(-0.0), "0");
    }
}
