//! Ölçeklerin özeti (plan/map-cad.md MAP-02, docs/adr/1231): the scales a map project works with, apart and in one
//! place, as Civil 3D's Drawing Settings keeps the drawing's scale (Units and Zone) apart from the grid and sea-level
//! factors (Transformation) and Trimble shows the combined factor: the drawing scale (the annotations' size on paper),
//! the screen's scale (zoom; it changes nothing), and at the screen's middle the projection's scale, the height's
//! factor and their product, and whether the survey windows take measured lengths to the grid. Read only; the
//! status bar's scale menu shows it.

use kentos_commands::ProjectSettings;
use kentos_geometry_core::crs::ground::{Grid, grid_factor, has_point_scale, point_scale};
use kentos_geometry_core::display::fixed;
use kentos_project::systems;
use kentos_project::wizard::scale_text;

use crate::Vec2;
use crate::ground::trimmed;

/// One row: what, its value, and what it does and does not change.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleRow {
    pub label: &'static str,
    pub value: String,
    pub detail: String,
}

fn row(label: &'static str, value: String, detail: impl Into<String>) -> ScaleRow {
    ScaleRow { label, value, detail: detail.into() }
}

/// A factor and its parts per million from 1: `0.99960000 (−400.0 ppm)`.
pub fn factor_text(k: f64) -> String {
    let ppm = (k - 1.0) * 1e6;
    let sign = if ppm < 0.0 { "−" } else { "+" };
    format!("{} ({sign}{} ppm)", fixed(k, 8), fixed(ppm.abs(), 1))
}

/// The rows for a project at the screen's middle `center` (the drawing's coordinates, metres east and north) with the
/// screen at 1:`screen` (none when not known): drawing scale, screen, then the projection's.
pub fn rows(settings: &ProjectSettings, center: Vec2, screen: Option<f64>) -> Vec<ScaleRow> {
    let mut out = vec![row(
        "Çizim ölçeği",
        scale_text(settings.plot_scale),
        "Yazıların ve ölçülerin kâğıttaki boyu; geometri gerçek boyunda kalır.",
    )];
    if let Some(s) = screen.filter(|s| s.is_finite() && *s > 0.0) {
        out.push(row(
            "Ekran",
            scale_text(s.round().max(1.0)),
            "Yakınlaştırma: geometriyi, ölçüleri ve kaydı değiştirmez.",
        ));
    }
    let Some(named) = systems::own(settings) else {
        out.push(row(
            "Koordinat sistemi",
            "Yok (yerel)".to_owned(),
            "Projeksiyon ölçeği yok: uzunluklar çizimin düzleminde ölçülür.",
        ));
        return out;
    };
    let Some(system) = named.system.filter(has_point_scale) else {
        out.push(row(
            "Projeksiyon ölçeği",
            "Tek değer yok".to_owned(),
            format!(
                "{}: bu sistemde bir noktanın tek ölçeği yok (coğrafi, Pseudo-Mercator ya da afinle bağlı yerel sistem).",
                named.title
            ),
        ));
        return out;
    };
    let Some(k) = point_scale(&system, center) else {
        out.push(row(
            "Projeksiyon ölçeği",
            "Ulaşılmıyor".to_owned(),
            format!("Ekranın ortası {} sisteminin ulaştığı yerin dışında.", named.title),
        ));
        return out;
    };
    out.push(row(
        "Projeksiyon ölçeği",
        factor_text(k),
        format!(
            "{}, ekranın ortasında: haritadaki uzunluk = elipsoitteki × bu ölçek. Çizimi değiştirmez; yalnız ölçme hesaplarına girer.",
            named.title
        ),
    ));
    if let Some(h) = settings.ground_height() {
        let grid = Grid { system, height: h };
        // Along the east: the height's factor takes the radius of curvature in a direction (Euler's).
        let east = Vec2::new(center.x + 1.0, center.y);
        if let Ok(f) = grid_factor(&grid, center, east) {
            out.push(row(
                "Yükseklik çarpanı",
                factor_text(f.height_factor),
                format!("Ortalama elipsoit yüksekliği {} m (Proje ayarları › Ölçme): R / (R + h), doğu yönünde.", trimmed(h)),
            ));
            out.push(row(
                "Birleşik çarpan",
                factor_text(f.combined()),
                "Zemindeki uzunluk × birleşik çarpan = haritadaki uzunluk.",
            ));
        }
    }
    out.push(if settings.reduces_to_grid() {
        row(
            "Ölçülen uzunluklar",
            "Projeksiyona indirilir".to_owned(),
            "Kutupsal alım, poligon ve ağ pencereleri ölçülen uzunluğu çarpanla haritaya indirir (Proje ayarları › Ölçme).",
        )
    } else {
        row(
            "Ölçülen uzunluklar",
            "Düzlemde kalır".to_owned(),
            "Ölçme pencereleri uzunluğu olduğu gibi kullanır; indirmek için Proje ayarları › Ölçme.",
        )
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(json: &str) -> ProjectSettings {
        serde_json::from_str(json).expect("settings")
    }

    fn value(rows: &[ScaleRow], label: &str) -> Option<String> {
        rows.iter().find(|r| r.label == label).map(|r| r.value.clone())
    }

    /// The number before the parentheses.
    fn number(text: &str) -> f64 {
        text.split_whitespace().next().and_then(|n| n.parse().ok()).expect("a number")
    }

    #[test]
    fn a_map_projects_scales_apart_and_against_proj() {
        // TUREF / TM30 (EPSG:5254), 50 km east of the central meridian, h = 850 m. The references, not KentOS:
        //   echo "550000 4540000" | invproj +proj=tmerc +lon_0=30 +k=1 +x_0=500000 +ellps=GRS80
        //     → 30.594220068865 40.993306194487;  proj -V … → Parallel scale (k): 1.00003076
        //   N = a/√(1 − e² sin²φ) = 6 387 343.2508 m (GRS80), N/(N + 850) = 0.9998669420 (Python, by hand)
        let tm30 = settings(
            r#"{"srid":5254,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"survey":{"groundHeight":850}}"#,
        );
        let rows = rows(&tm30, Vec2::new(550_000.0, 4_540_000.0), Some(2345.4));
        assert_eq!(value(&rows, "Çizim ölçeği").as_deref(), Some("1:1.000"));
        assert_eq!(value(&rows, "Ekran").as_deref(), Some("1:2.345"));
        let k = number(&value(&rows, "Projeksiyon ölçeği").expect("k"));
        assert!((k - 1.000_030_76).abs() <= 1e-8, "{k}");
        assert!(value(&rows, "Projeksiyon ölçeği").expect("k").ends_with("(+30.8 ppm)"));
        let hf = number(&value(&rows, "Yükseklik çarpanı").expect("height factor"));
        assert!((hf - 0.999_866_942_0).abs() <= 1e-8, "{hf}");
        assert!(value(&rows, "Yükseklik çarpanı").expect("hf").ends_with("(−133.1 ppm)"));
        let c = number(&value(&rows, "Birleşik çarpan").expect("combined"));
        assert!((c - 1.000_030_76 * 0.999_866_942_0).abs() <= 2e-8, "{c}");
        assert_eq!(value(&rows, "Ölçülen uzunluklar").as_deref(), Some("Düzlemde kalır"));
        // The labels in their order: what changes the drawing comes first, what only measures last.
        let labels: Vec<&str> = rows.iter().map(|r| r.label).collect();
        assert_eq!(
            labels,
            ["Çizim ölçeği", "Ekran", "Projeksiyon ölçeği", "Yükseklik çarpanı", "Birleşik çarpan", "Ölçülen uzunluklar"]
        );
    }

    #[test]
    fn without_a_height_a_system_or_a_point_scale_less_is_said() {
        let no_height = settings(
            r#"{"srid":5254,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":500}"#,
        );
        let r = rows(&no_height, Vec2::new(500_000.0, 4_540_000.0), None);
        let labels: Vec<&str> = r.iter().map(|r| r.label).collect();
        assert_eq!(labels, ["Çizim ölçeği", "Projeksiyon ölçeği", "Ölçülen uzunluklar"]);
        // On the central meridian a TM30 with k₀ = 1 has scale 1.
        assert_eq!(value(&r, "Projeksiyon ölçeği").as_deref(), Some("1.00000000 (+0.0 ppm)"));
        let local = settings(r#"{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100}"#);
        let r = rows(&local, Vec2::new(0.0, 0.0), Some(50.0));
        assert_eq!(value(&r, "Koordinat sistemi").as_deref(), Some("Yok (yerel)"));
        let geographic = settings(r#"{"srid":4326,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":1000}"#);
        let r = rows(&geographic, Vec2::new(30.0, 40.0), None);
        assert_eq!(value(&r, "Projeksiyon ölçeği").as_deref(), Some("Tek değer yok"));
    }

    #[test]
    fn a_factor_says_its_parts_per_million() {
        assert_eq!(factor_text(0.9996), "0.99960000 (−400.0 ppm)");
        assert_eq!(factor_text(1.0), "1.00000000 (+0.0 ppm)");
    }
}
