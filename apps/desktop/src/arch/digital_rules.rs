//! Dijital proje kuralları (plan/bim-plan.md BIM-01, plan/ifc-uyum.md IFC-01, docs/adr/8062; `arch.digitalRules`): the
//! 2026 digital project regulation's rules as KentOS keeps them — every row of the annexes with its annex, table, row
//! and PDF page (`kentos_ifc::rules`), KentOS's decisions on the open points, and the catalog's differences from the
//! IFC 4.3 schema with the decision that settles each. The window only reads: it changes no drawing. What a drawing's
//! model carries is the written file's to say (IFC-08), not this list's.
//!
//! ```text
//! ┌ Dijital proje kuralları ──────────────────────────────────────────────────────────────── × ┐
//! │ Profil ────────────────   [kural 1870] [gösterilen 312] [karar 32] [açık 13] [şema farkı 9] │
//! │  Temel profil 2026.1       ⓘ 13 nokta idarenin ya da Bakanlığın cevabını bekliyor          │
//! │ Süzgeç ────────────────   (Kurallar | Kararlar | Şema farkları)  table                       │
//! │  Ek [Hepsi ▾]              …                                                                │
//! │  Zorunluluk (Hepsi|…)      the chosen row: its words, its source, its decisions              │
//! │  ☑ Yalnız mimari model                                                                      │
//! │  Ara [          ]                                                                  [Kapat]  │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::sync::Arc;

use iced::widget::Column;
use iced::{Element, Fill, Task};
use kentos_digital_project::decision::Decision;
use kentos_digital_project::{Level, Source};
use kentos_ifc::catalog_schema::{self, Finding};
use kentos_ifc::rules::{self, Filter, Rule};
use kentos_ui::label;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form};

use crate::app::{App, Dialog, Message};
use crate::arch;
use crate::road_dialog::{self, RoadDialog, col};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Dijital proje kuralları";
const CLOSE: &str = "Kapat";

/// The annexes the Ek list offers, after Hepsi.
pub const ANNEXES: [(u8, &str); 9] = [
    (1, "EK-1 Proje kodu"),
    (2, "EK-2 Dosya adları"),
    (3, "EK-3 Mahal adları"),
    (4, "EK-4 CAD çizimi"),
    (5, "EK-5 IFC sınıfları"),
    (6, "EK-6 Öznitelik ve özellikler"),
    (7, "EK-7 Proje sınıfları"),
    (8, "EK-8 Gelişim seviyesi"),
    (9, "EK-9 Kontrol formu"),
];

/// The list under the figures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Listed {
    Rules,
    Decisions,
    Findings,
}

impl std::fmt::Display for Listed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Listed::Rules => "Kurallar",
            Listed::Decisions => "Kararlar",
            Listed::Findings => "Şema farkları",
        })
    }
}

/// Zorunluluk's choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    All,
    Mandatory,
    Optional,
}

impl std::fmt::Display for Asked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Asked::All => "Hepsi",
            Asked::Mandatory => "Zorunlu",
            Asked::Optional => "İsteğe bağlı",
        })
    }
}

/// Zorunluluk's choices in order.
const ASKED: [Asked; 3] = [Asked::All, Asked::Mandatory, Asked::Optional];

/// A schema difference as the window lists it: the finding and the decision that settles it.
#[derive(Clone, Debug)]
pub struct Difference {
    pub finding: Finding,
    pub decision: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Window {
    pub rules: Arc<Vec<Rule>>,
    pub decisions: Arc<Vec<&'static Decision>>,
    pub differences: Arc<Vec<Difference>>,
    pub filter: Filter,
    pub asked: Asked,
    pub list: Listed,
    /// The rules the filter keeps, by their places in `rules`.
    pub shown: Vec<usize>,
    /// The chosen row of the list shown, by its place in it.
    pub chosen: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// The Ek list's choice: 0 Hepsi, then [`ANNEXES`] in order.
    Annex(usize),
    Asked(Asked),
    Architecture(bool),
    Words(String),
    List(Listed),
    Choose(usize),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Arch(arch::Event::DigitalRules(e))
}

impl Window {
    /// The window as it opens: the architectural model's rules of every annex.
    pub fn new() -> Self {
        let rules = Arc::new(rules::all());
        let decisions = Arc::new(rules::decisions());
        let findings = catalog_schema::compare(kentos_ifc::catalog::base(), catalog_schema::dictionary());
        let differences = findings
            .into_iter()
            .map(|finding| {
                let key = finding.key();
                let decision = decisions.iter().find(|d| d.settles_key(&key)).map(|d| d.id.clone());
                Difference { finding, decision }
            })
            .collect();
        let mut w = Self {
            rules,
            decisions,
            differences: Arc::new(differences),
            filter: Filter {
                architecture: true,
                ..Filter::default()
            },
            asked: Asked::All,
            list: Listed::Rules,
            shown: Vec::new(),
            chosen: None,
        };
        w.refilter();
        w
    }

    fn refilter(&mut self) {
        self.filter.level = match self.asked {
            Asked::All => None,
            Asked::Mandatory => Some(Level::Mandatory),
            Asked::Optional => Some(Level::Optional),
        };
        self.shown = self.filter.apply(&self.rules);
        self.chosen = None;
    }

    /// The decisions the filter keeps (its annex and words), by their places.
    pub fn decisions_shown(&self) -> Vec<usize> {
        let words = rules::fold(self.filter.words.trim());
        self.decisions
            .iter()
            .enumerate()
            .filter(|(_, d)| self.filter.annex.is_none_or(|a| d.sources.iter().any(|s| s.annex == a)))
            .filter(|(_, d)| {
                let hay = rules::fold(&format!("{} {} {} {}", d.id, d.title, d.text, d.rows.join(" ")));
                words.split_whitespace().all(|w| hay.contains(w))
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// The schema differences the filter keeps (the annex of their rows, its words), by their places.
    pub fn differences_shown(&self) -> Vec<usize> {
        let words = rules::fold(self.filter.words.trim());
        self.differences
            .iter()
            .enumerate()
            .filter(|(_, x)| {
                self.filter.annex.is_none_or(|a| {
                    x.finding.rows().iter().any(|r| annex_of_row(r) == Some(a))
                })
            })
            .filter(|(_, x)| {
                let hay = rules::fold(&format!("{} {}", x.finding.message(), x.finding.rows().join(" ")));
                words.split_whitespace().all(|w| hay.contains(w))
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// How many rows the list shown keeps.
    pub fn shown_count(&self) -> usize {
        match self.list {
            Listed::Rules => self.shown.len(),
            Listed::Decisions => self.decisions_shown().len(),
            Listed::Findings => self.differences_shown().len(),
        }
    }

    /// A row's source as the list cites it, by its id; the id itself where the catalog has no such row.
    fn cite_of(&self, id: &str) -> String {
        self.rules.iter().find(|r| r.id == id).map_or_else(|| id.to_owned(), |r| r.source.cite())
    }

    /// The decisions still waiting for an answer.
    pub fn open_decisions(&self) -> usize {
        self.decisions.iter().filter(|d| d.open()).count()
    }

    /// The rows of the list shown, as the table writes them.
    pub fn rows(&self) -> Vec<Vec<String>> {
        match self.list {
            Listed::Rules => self
                .shown
                .iter()
                .map(|&i| {
                    let r = &self.rules[i];
                    vec![
                        r.title.clone(),
                        match (&r.class_tr, &r.class) {
                            (Some(tr), Some(c)) => format!("{tr} ({c})"),
                            (None, Some(c)) => c.clone(),
                            _ => String::new(),
                        },
                        r.path.clone(),
                        r.level.map_or_else(String::new, |l| l.named().to_owned()),
                        r.source.cite(),
                    ]
                })
                .collect(),
            Listed::Decisions => self
                .decisions_shown()
                .into_iter()
                .map(|i| &self.decisions[i])
                .map(|d| {
                    vec![
                        d.id.clone(),
                        d.title.clone(),
                        d.kind.named().to_owned(),
                        value_words(d),
                        d.sources.first().map(Source::cite).unwrap_or_default(),
                    ]
                })
                .collect(),
            Listed::Findings => self
                .differences_shown()
                .into_iter()
                .map(|i| &self.differences[i])
                .map(|x| {
                    vec![
                        x.finding.message(),
                        x.finding.rows().first().map(|r| self.cite_of(r)).unwrap_or_default(),
                        x.decision.clone().unwrap_or_else(|| "karar yok".to_owned()),
                    ]
                })
                .collect(),
        }
    }
}

/// The annex a catalog row id names: “T6.92/…” 6, “M5.5.4” 5.
fn annex_of_row(id: &str) -> Option<u8> {
    id.trim_start_matches(['T', 'M', 'S']).split(['.', '/']).next()?.parse().ok()
}

/// A decision's parameter as the list says it: its value, “açık” with KentOS's default, or nothing; a name pattern is
/// said as such and written out under the table.
fn value_words(d: &Decision) -> String {
    match &d.param {
        Some(p) => match (&p.value, &p.default) {
            (Some(v), _) if v.starts_with('^') => "ad kalıbı".to_owned(),
            (Some(v), _) => v.clone(),
            (None, Some(def)) => format!("açık (KentOS: {def})"),
            (None, None) => "açık".to_owned(),
        },
        None if d.open() => "açık".to_owned(),
        None => String::new(),
    }
}

impl App {
    /// Dijital proje kuralları (`arch.digitalRules`): the base profile's rules, the architectural model's first.
    pub(crate) fn open_digital_rules(&mut self) {
        self.arch.digital_rules = Some(Window::new());
        self.dialog = Some(Dialog::Arch(arch::ArchDialogKind::DigitalRules));
    }

    pub(crate) fn digital_rules_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Close = event {
            self.arch.digital_rules = None;
            self.dialog = None;
            return Task::none();
        }
        let Some(w) = self.arch.digital_rules.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Annex(k) => {
                w.filter.annex = k.checked_sub(1).and_then(|i| ANNEXES.get(i)).map(|a| a.0);
                w.refilter();
            }
            Event::Asked(a) => {
                w.asked = a;
                w.refilter();
            }
            Event::Architecture(on) => {
                w.filter.architecture = on;
                w.refilter();
            }
            Event::Words(t) => {
                w.filter.words = t;
                w.refilter();
            }
            Event::List(l) => {
                w.list = l;
                w.chosen = None;
            }
            Event::Choose(k) => w.chosen = Some(k),
            Event::Close => {}
        }
        Task::none()
    }

    pub(crate) fn digital_rules_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.arch.digital_rules else {
            return iced::widget::text("").into();
        };
        let profile = &kentos_ifc::catalog::base().profile;
        let annex_at = w.filter.annex.and_then(|a| ANNEXES.iter().position(|x| x.0 == a)).map_or(0, |i| i + 1);
        let form = Form::new()
            .wide(label::caption("Yönetmeliğin eklerindeki kurallar; her satır ekini, tablosunu ve PDF sayfasını söyler."))
            .section("Profil")
            .field("Profil", label::body(format!("{} {}", profile.name, profile.version)))
            .field("Yayım", label::body(format!("RG {}", profile.gazette)))
            .field("Ekler", label::body(format!("{} sayfa, taranmış PDF", profile.annexes_pdf.pages)))
            .section("Süzgeç")
            .field(
                "Ek",
                Select::new(
                    std::iter::once(Choice::new("Hepsi")).chain(ANNEXES.iter().map(|a| Choice::new(a.1))),
                    Some(annex_at),
                    |k| msg(Event::Annex(k)),
                ),
            )
            .field(
                "Zorunluluk",
                Select::new(
                    ASKED.iter().map(|a| Choice::new(a.to_string())),
                    ASKED.iter().position(|a| *a == w.asked),
                    |k| msg(Event::Asked(ASKED.get(k).copied().unwrap_or(Asked::All))),
                ),
            )
            .row(crate::exchange::words::check(
                w.filter.architecture,
                "Yalnız mimari modelin kuralları",
                Some(msg(Event::Architecture(!w.filter.architecture))),
            ))
            .field(
                "Ara",
                kentos_ui::widget::text_field("sınıf, özellik, kod, sayfa…", &w.filter.words).on_input(|t| msg(Event::Words(t))),
            );
        let open = w.open_decisions();
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close))
            .form_width(310.0)
            .width(1240.0)
            .figure("Kural", w.rules.len().to_string())
            .figure("Gösterilen", w.shown_count().to_string())
            .figure("Karar", w.decisions.len().to_string())
            .figure("Açık karar", open.to_string())
            .figure("Şema farkı", w.differences.len().to_string());
        if open > 0 {
            d = d.note(Banner::info(format!(
                "{open} nokta idarenin ya da Bakanlığın cevabını bekliyor; değeri boş kalır, tahminle doldurulmaz."
            )));
        }
        let unsettled = w.differences.iter().filter(|x| x.decision.is_none()).count();
        if unsettled > 0 {
            d = d.note(Banner::warning(format!("{unsettled} şema farkının kararı yok.")));
        }
        let (cols, empty) = match w.list {
            Listed::Rules => (
                vec![col("Kural", 220.0), col("Sınıf", 130.0), col("Yol / kod", 190.0), col("Zorunluluk", 76.0), col("Kaynak", 250.0)],
                "Süzgece uyan kural yok.",
            ),
            Listed::Decisions => (
                vec![col("No", 40.0), col("Karar", 290.0), col("Nasıl", 120.0), col("Değer", 150.0), col("Kaynak", 200.0)],
                "Süzgece uyan karar yok.",
            ),
            Listed::Findings => (vec![col("Fark", 500.0), col("Satır", 250.0), col("Karar", 60.0)], "Süzgece uyan şema farkı yok."),
        };
        let table = road_dialog::sheet_chosen(cols, w.rows(), empty, 330.0, w.chosen, |k| msg(Event::Choose(k)));
        d = d.detail(road_dialog::detail_choice(
            [Listed::Rules, Listed::Decisions, Listed::Findings],
            w.list,
            |l| msg(Event::List(l)),
            table,
        ));
        if let Some(tail) = self.digital_rules_chosen(w) {
            d = d.tail(tail);
        }
        d.view()
    }

    /// The chosen row's words, source and decisions under the table.
    fn digital_rules_chosen<'a>(&self, w: &'a Window) -> Option<Element<'a, Message>> {
        let k = w.chosen?;
        let mut col = Column::new().spacing(4).width(Fill);
        match w.list {
            Listed::Rules => {
                let r = &w.rules[*w.shown.get(k)?];
                col = col.push(label::strong(format!("{} · {}", r.title, r.source.cite())));
                if !r.text.is_empty() {
                    col = col.push(label::body(r.text.clone()));
                }
                for id in &r.decisions {
                    if let Some(dec) = w.decisions.iter().find(|x| &x.id == id) {
                        col = col.push(label::caption(format!("Karar {} — {}: {}", dec.id, dec.kind.named(), dec.text)));
                    }
                }
            }
            Listed::Decisions => {
                let dec = w.decisions.get(*w.decisions_shown().get(k)?)?;
                col = col.push(label::strong(format!("{} · {} · {}", dec.id, dec.title, dec.kind.named())));
                col = col.push(label::body(dec.text.clone()));
                let sources: Vec<String> = dec.sources.iter().map(Source::cite).collect();
                col = col.push(label::caption(format!("Kaynak: {}", sources.join("; "))));
                if let Some(p) = &dec.param {
                    let value = p.value.clone().unwrap_or_else(|| "açık".to_owned());
                    col = col.push(label::caption(format!("Değer ({}): {value}", p.key)));
                }
                if let Some(who) = &dec.asked {
                    col = col.push(label::caption(format!("Cevaplayacak: {who}")));
                }
            }
            Listed::Findings => {
                let x = w.differences.get(*w.differences_shown().get(k)?)?;
                col = col.push(label::strong(x.finding.message()));
                let cited: Vec<String> = x.finding.rows().iter().map(|r| w.cite_of(r)).collect();
                col = col.push(label::caption(format!("Satırlar: {}", cited.join("; "))));
                if let Some(dec) = x.decision.as_ref().and_then(|id| w.decisions.iter().find(|d| &d.id == id)) {
                    col = col.push(label::body(format!("Karar {} — {}: {}", dec.id, dec.kind.named(), dec.text)));
                }
            }
        }
        Some(
            iced::widget::container(col)
                .padding([8, 10])
                .width(Fill)
                .style(kentos_ui::style::container::field_box)
                .into(),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn digital_rules_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.arch.digital_rules.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press("Kurallar") => Some(msg(Event::List(Listed::Rules))),
            Control::Press("Kararlar") => Some(msg(Event::List(Listed::Decisions))),
            Control::Press("Şema farkları") => Some(msg(Event::List(Listed::Findings))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
