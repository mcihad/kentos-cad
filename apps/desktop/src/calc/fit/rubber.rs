//! Kauçuk levha in Vektör oturtma (the web's `ui/calc/fitRubber.ts`,
//! docs/adr/0158 §5): the used pairs are the sheet's links, a pair whose
//! target is its source a fixed point (Sabit); the core says whether they
//! give a sheet (`ops::rubber`). The table's residuals are Helmert's: at
//! each link, the local correction the sheet makes beyond the best
//! similarity. The summary, the report and the reasons there is no sheet.

use kentos_contracts::RubberLink;
use kentos_geometry_core::ops::fit::{self as solver, FitPair};
use kentos_geometry_core::ops::rubber::{Link, RubberError, Sheet};
use kentos_interaction::fixed;

use crate::calc::traverse::mm_text;
use crate::exchange::words::Kind as Line;

pub(super) const HINT: &str = "Yerel düzeltme: her bağ hedefine tam oturur, çevresi yakınlığıyla kayar; en az 3 bağ. Köşeler taşınır, kenarlar doğru kalır.";

/// The least links a sheet takes.
const NEED: usize = 3;

/// The used pairs as links, how many are fixed points, and why they give
/// no sheet (none when they give one).
#[derive(Clone, Debug, Default)]
pub(super) struct Links {
    pub(super) links: Vec<RubberLink>,
    pub(super) fixed: usize,
    pub(super) error: Option<RubberError>,
}

impl Links {
    pub(super) fn of(pairs: &[FitPair]) -> Links {
        let core: Vec<Link> = pairs
            .iter()
            .filter(|p| p.used)
            .map(|p| Link {
                from: p.source,
                to: p.target,
            })
            .collect();
        let fixed = core.iter().filter(|l| l.from == l.to).count();
        let error = if core.len() < NEED {
            Some(RubberError::TooFew)
        } else {
            Sheet::solve(&core).err()
        };
        let links = core
            .iter()
            .map(|l| RubberLink {
                from: kentos_contracts::Vec2 {
                    x: l.from.x,
                    y: l.from.y,
                },
                to: kentos_contracts::Vec2 {
                    x: l.to.x,
                    y: l.to.y,
                },
            })
            .collect();
        Links {
            links,
            fixed,
            error,
        }
    }
}

/// The local corrections at the used links (Helmert's residuals): the
/// largest and whose, and their mean (metres).
#[derive(Clone, Debug)]
pub(super) struct Corrections {
    pub(super) worst: f64,
    pub(super) who: String,
    pub(super) mean: f64,
}

/// The summary under the table: the links and that the sheet meets them,
/// the local corrections and Helmert's m0; or why there is no sheet.
pub(super) fn summary(
    l: &Links,
    fit: Option<&solver::Fit>,
    corrections: Option<&Corrections>,
) -> Vec<(Line, String)> {
    match l.error {
        Some(RubberError::TooFew) => {
            return vec![(
                Line::Info,
                format!(
                    "Kauçuk levha için en az {NEED} kullanılan bağ gerekir; şimdi {}. Koordinatları yazın, yapıştırın ya da çizimden seçin.",
                    l.links.len()
                ),
            )];
        }
        Some(e) => return vec![(Line::Warn, failure(e).to_owned())],
        None => {}
    }
    let mut lines = vec![(
        Line::Ok,
        format!(
            "{} bağ ({} sabit nokta); levha her bağdan tam geçer.",
            l.links.len(),
            l.fixed
        ),
    )];
    if let (Some(m0), Some(c)) = (fit.and_then(|f| f.m0), corrections) {
        lines.push((
            Line::Info,
            format!(
                "Yerel düzeltme (Helmert'e göre) en çok {}: {}; ortalama {}, Helmert m0 = ±{}.",
                mm_text(c.worst),
                c.who,
                mm_text(c.mean),
                mm_text(m0)
            ),
        ));
        lines.push((
            Line::Info,
            "Levha hatalı bir bağı da tam geçer: ölçü hatası olan bağı Kullan'dan çıkarın."
                .to_owned(),
        ));
    }
    lines
}

/// Kauçuk levha's report, tab-separated: the method, the pairs with their
/// local corrections (`rows`, as the table has them), the links, the sheet
/// or why there is none, the corrections and Helmert's m0 and parameters.
pub(super) fn report(
    title: &str,
    rows: Vec<Vec<String>>,
    l: &Links,
    fit: Option<&solver::Fit>,
    corrections: Option<&Corrections>,
    parameters: Option<String>,
) -> Vec<Vec<String>> {
    let strings = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
    let mut lines = vec![
        strings(&[title, "kauçuk levha"]),
        strings(&[
            "Yöntem",
            "İnce plaka eğrisi: levha her bağdan tam geçer; vY, vX ve v Helmert'e göre yerel düzeltmedir.",
        ]),
        strings(&[
            "Kullan", "Ad", "Kaynak Y", "Kaynak X", "Hedef Y", "Hedef X", "vY (mm)", "vX (mm)",
            "v (mm)",
        ]),
    ];
    lines.extend(rows);
    lines.push(vec![
        "Bağ".to_owned(),
        l.links.len().to_string(),
        "Sabit nokta".to_owned(),
        l.fixed.to_string(),
    ]);
    match l.error {
        Some(RubberError::TooFew) => lines.push(vec![
            "Levha".to_owned(),
            format!("En az {NEED} kullanılan bağ gerekir."),
        ]),
        Some(e) => lines.push(vec!["Levha".to_owned(), failure(e).to_owned()]),
        None => {}
    }
    if let Some(c) = corrections {
        lines.push(vec![
            "En büyük yerel düzeltme (mm)".to_owned(),
            fixed(c.worst * 1000.0, 1),
            c.who.clone(),
        ]);
        lines.push(vec![
            "Ortalama yerel düzeltme (mm)".to_owned(),
            fixed(c.mean * 1000.0, 1),
        ]);
    }
    if let Some(fit) = fit {
        lines.push(vec![
            "Helmert m0 (mm)".to_owned(),
            fit.m0
                .map_or_else(|| "—".to_owned(), |m0| fixed(m0 * 1000.0, 2)),
        ]);
        if let Some(p) = parameters {
            lines.push(vec!["Helmert".to_owned(), p]);
        }
    }
    lines
}

fn failure(e: RubberError) -> &'static str {
    match e {
        RubberError::TooFew => "",
        RubberError::Duplicate => "İki bağın kaynağı aynı nokta. Birini Kullan'dan çıkarın.",
        RubberError::Collinear => {
            "Bağların kaynakları bir doğru üstünde; levha kurulamaz. Doğrunun dışında bir bağ ekleyin."
        }
        RubberError::Singular => {
            "Bağların denklem takımının tek çözümü yok. Birbirine çok yakın kaynakları birleştirin."
        }
        RubberError::TooMany => "En çok 1000 bağ alınır. Bağları azaltın.",
    }
}
