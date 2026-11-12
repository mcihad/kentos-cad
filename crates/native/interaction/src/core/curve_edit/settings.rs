//! Presentation of the existing curve-edit commands. Choosing a display
//! mode changes no geometry; every edit still goes through cad.curves.edit.
use super::{Ask, PointView, points_of};
use crate::tool::Context;
use crate::tool_settings::{Setting, choices};
use kentos_commands::Entity;

fn actions(
    key: &'static str,
    label: &'static str,
    items: &[(&str, &str)],
    active: &str,
) -> Setting {
    let mut row = Setting::choice(key, label, choices(items, active));
    if row.value.is_empty() {
        row.value = "İşlem seç".into();
    }
    row
}

pub(super) fn rows(
    e: Option<&Entity>,
    ask: Option<Ask>,
    view: PointView,
    cx: &Context<'_>,
) -> Vec<Setting> {
    let Some(e) = e else {
        return vec![Setting::hint(
            "pick",
            "Eğri seçin",
            "Bir çoklu çizgi veya NURBS eğrisi seçin. Mevcut eğri, işlem seçilene kadar değişmez.",
        )];
    };
    let (fit, ctrl) = points_of(e);
    let selected = match ask {
        Some(Ask::AddFit) => "E",
        Some(Ask::MoveFit(_)) => "T",
        Some(Ask::RemoveFit) => "S",
        Some(Ask::Tangents(_)) => "G",
        Some(Ask::MoveControl(_)) => "D",
        Some(Ask::Weight(_)) => "L",
        Some(Ask::RemoveControl) => "İ",
        Some(Ask::Knot) => "Ü",
        Some(Ask::Tolerance) => "Ç",
        _ => "",
    };
    match e {
        Entity::Polyline(p) => {
            let mut items = vec![if p.closed {
                ("A", "Aç")
            } else {
                ("K", "Kapat")
            }];
            if p.bulges
                .as_ref()
                .is_some_and(|b| b.iter().any(|v| *v != 0.))
            {
                items.push(("D", "Yayları düzleştir"));
            }
            items.extend([
                ("E", "Geçiş noktalarıyla eğriye çevir"),
                ("N", "Denetim noktalarıyla eğriye çevir"),
            ]);
            vec![
                Setting::hint(
                    "source",
                    "Seçili çizgi",
                    format!(
                        "{} çoklu çizgi · {} köşe",
                        if p.closed { "Kapalı" } else { "Açık" },
                        p.pts.len()
                    ),
                ),
                actions("curve-action", "İşlem", &items, selected),
                Setting::text(
                    "degree",
                    "Yeni eğri derecesi",
                    cx.memory.nurbs_degree.to_string(),
                ),
                Setting::hint(
                    "conversion",
                    "Dönüşüm",
                    "Eğriye çevir, şekli yeniden kurar. Yayları düzleştir, yay kenarlarını kiriş yapar. Her işlem tek adımda geri alınır.",
                ),
            ]
        }
        Entity::Nurbs(n) => {
            let mut rows = vec![Setting::hint(
                "source",
                "Seçili eğri",
                format!(
                    "{} NURBS · derece {}\n{} geçiş noktası · {} denetim noktası",
                    if n.closed { "Kapalı" } else { "Açık" },
                    n.degree,
                    fit.len(),
                    ctrl.len()
                ),
            )];
            let mut modes = vec![("control", "Denetim noktaları")];
            if !fit.is_empty() {
                modes = vec![
                    ("fit", "Geçiş noktaları"),
                    ("control", "Denetim noktaları"),
                    ("both", "İkisini göster"),
                ];
            }
            rows.push(Setting::choice(
                "point-view",
                "Noktalar",
                choices(
                    &modes,
                    match view {
                        PointView::Fit => "fit",
                        PointView::Control => "control",
                        PointView::Both => "both",
                    },
                ),
            ));
            if !fit.is_empty() && view != PointView::Control {
                let mut items = vec![
                    ("E", "Geçiş noktası ekle"),
                    ("T", "Geçiş noktasını taşı"),
                    ("S", "Geçiş noktasını sil"),
                ];
                if !n.closed {
                    items.push(("G", "Uç teğetlerini ayarla"));
                }
                rows.push(actions("fit-edit", "Geçiş işlemi", &items, selected));
            }
            if view != PointView::Fit {
                let mut items = vec![
                    ("D", "Denetim noktasını taşı"),
                    ("L", "Nokta ağırlığını değiştir"),
                    ("İ", "Denetim noktasını sil"),
                ];
                if !n.closed {
                    items.push(("Ü", "Düğüm ekle · şekli korur"));
                }
                rows.push(actions("control-edit", "Denetim işlemi", &items, selected));
                if n.fit.is_some() {
                    rows.push(Setting::hint("authority", "Geçiş verisi", "Denetim, ağırlık veya düğüm düzenlemesi geçiş verisini bırakır. Geçiş noktalarıyla sürdürmek için nokta görünümünü değiştirin."));
                }
            }
            if let Some(Ask::MoveFit(Some(i)) | Ask::MoveControl(Some(i)) | Ask::Weight(Some(i))) =
                ask
            {
                rows.push(Setting::hint(
                    "picked-point",
                    "Seçili nokta",
                    format!("{}. nokta", i + 1),
                ));
                if matches!(ask, Some(Ask::Weight(_))) {
                    let weight = n
                        .weights
                        .as_ref()
                        .and_then(|w| w.get(i))
                        .copied()
                        .unwrap_or(1.);
                    rows.push(Setting::text("weight", "Ağırlık", weight.to_string()));
                }
            }
            rows.push(actions(
                "curve-action",
                "Biçim",
                &[
                    if n.closed {
                        ("A", "Aç · yeniden kurar")
                    } else {
                        ("K", "Kapat · yeniden kurar")
                    },
                    ("Ç", "Çoklu çizgiye çevir…"),
                ],
                selected,
            ));
            if matches!(ask, Some(Ask::Tolerance)) {
                rows.push(Setting::text(
                    "tolerance",
                    "Kiriş toleransı",
                    cx.format().length(cx.memory.curve_tolerance),
                ));
                rows.push(Setting::notice("approximation", "Yaklaşık dönüşüm", "Toleransı onayladığınızda NURBS yerine çoklu çizgi yazılır. Esc dönüşümü iptal eder."));
            }
            rows.push(Setting::hint("continuity", "Süreklilik", if matches!(ask, Some(Ask::Tangents(_))) {
                "Uç teğeti yönünü gösterin; Enter o ucu serbest bırakır. Yön ayarı G2 eğrilik eşlemesi sağlamaz. Önizleme onaydan önce hesaplanır."
            } else {
                "Düğüm ekleme şekli korur. Aç/Kapat yeniden kurar. Uç teğeti yönü, G2 eğrilik eşlemesi sağlamaz."
            }));
            rows
        }
        _ => Vec::new(),
    }
}
