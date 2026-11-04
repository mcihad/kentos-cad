//! Poz metrajı (plan/road.md Y11.4, docs/adr/0909 §4, `payItems.editor`): the road project's pay item book and its
//! quantities. The book's items (code, name, unit, unit price) are the agency's and its source is required; KentOS
//! gives no item and no price. Its rules give each item KentOS's measures of the project's guardrails, culverts,
//! retaining walls and earthwork volumes, times a factor. The quantities are read as the book is typed
//! (`road.pay.edit`'s check and `road.pay.query`'s answer on a reading copy). Kaydet writes the book (one undo step
//! “Poz defteri”), Sil takes it off.
//!
//! ```text
//! ┌ Poz metrajı ─────────────────────────────────────────────────────────────────────────────────┐
//! │ Kaynak [                    ] Sürüm [      ] Para birimi [   ]                                 │
//! │ Pozlar                                   │ Kurallar                                            │
//! │ Numara Ad            Birim Fiyat  ×      │ Ölçü [▾]        Poz [▾] Çarpan Malzeme  ×            │
//! │ [Poz ekle]                               │ [Kural ekle]                                         │
//! │ Metraj: Numara Ad Birim Miktar Birim fiyat Tutar · toplam … · warnings                          │
//! │ [Sil]                                                                    [Kapat] [Kaydet]      │
//! └──────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::{Column, container, row, space};
use iced::{Element, Fill, Task, Theme};
use kentos_commands::{
    CommandWarning, PayEdit, PayItemDef, PayMeasure, PayQueried, PayRuleDef, RoadEditOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_pay_edit};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::road_dialog;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Poz metrajı";
const SAVE: &str = "Kaydet";
const CLOSE: &str = "Kapat";
const REMOVE: &str = "Sil";
const ADD_ITEM: &str = "Poz ekle";
const ADD_RULE: &str = "Kural ekle";
/// The book's own fields, in order.
pub(crate) const FIELDS: [&str; 3] = ["Kaynak", "Sürüm", "Para birimi"];

/// KentOS's measures, in the order a rule offers them, with their words.
pub(crate) const MEASURES: [(PayMeasure, &str); 11] = [
    (PayMeasure::GuardrailLength, "Otokorkuluk boyu (m)"),
    (PayMeasure::CulvertLength, "Menfez boyu (m)"),
    (PayMeasure::CulvertPipes, "Menfez borusu (m)"),
    (PayMeasure::CulvertConcrete, "Menfez betonu (m³)"),
    (PayMeasure::WallLength, "Duvar boyu (m)"),
    (PayMeasure::WallFace, "Duvar yüzü (m²)"),
    (PayMeasure::WallConcrete, "Duvar betonu (m³)"),
    (PayMeasure::VolumeCut, "Kazı (m³)"),
    (PayMeasure::VolumeFill, "Dolgu (m³)"),
    (PayMeasure::VolumeFactoredCut, "Kabarmış kazı (m³)"),
    (PayMeasure::VolumeFactoredFill, "Sıkışmış dolgu (m³)"),
];

/// The quantities as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) answer: Option<PayQueried>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
}

/// A rule as typed.
#[derive(Debug, Clone)]
pub(crate) struct RuleRow {
    pub(crate) measure: PayMeasure,
    pub(crate) item: String,
    pub(crate) factor: String,
    pub(crate) material: String,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) saved: bool,
    pub(crate) fields: [String; 3],
    /// Each item's code, name, unit and unit price as typed.
    pub(crate) items: Vec<[String; 4]>,
    pub(crate) rules: Vec<RuleRow>,
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Field(usize, String),
    Item(usize, usize, String),
    AddItem,
    DropItem(usize),
    Measure(usize, PayMeasure),
    RuleItem(usize, String),
    Factor(usize, String),
    Material(usize, String),
    AddRule,
    DropRule(usize),
    Save,
    Remove,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Pay(event)
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

impl Window {
    /// The edit as typed, or why it is not one.
    pub(crate) fn input(&self) -> Result<PayEdit, String> {
        let mut items = Vec::new();
        for (i, it) in self.items.iter().enumerate() {
            items.push(PayItemDef {
                code: it[0].trim().to_owned(),
                name: it[1].trim().to_owned(),
                unit: it[2].trim().to_owned(),
                price: number(&it[3], &format!("{}. pozun birim fiyatı", i + 1))?,
            });
        }
        let mut rules = Vec::new();
        for (i, r) in self.rules.iter().enumerate() {
            let material = r.material.trim();
            rules.push(PayRuleDef {
                measure: r.measure,
                item: r.item.clone(),
                factor: number(&r.factor, &format!("{}. kuralın çarpanı", i + 1))?.unwrap_or(1.0),
                material: (!material.is_empty()).then(|| material.to_owned()),
            });
        }
        let currency = self.fields[2].trim();
        Ok(PayEdit {
            operation: RoadEditOperation::Set,
            source: Some(self.fields[0].trim().to_owned()),
            version: Some(self.fields[1].trim().to_owned()),
            currency: Some((!currency.is_empty()).then(|| currency.to_owned())),
            items: Some(items),
            rules: Some(rules),
            expected_revision: None,
        })
    }
}

impl App {
    /// Poz metrajı: the window on the road project's pay item book, or an empty one.
    pub(crate) fn open_pay(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Poz metrajı yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let window = match doc.model.road_pay() {
            Some(b) => Window {
                saved: true,
                fields: [b.source.clone(), b.version.clone(), b.currency.clone().unwrap_or_default()],
                items: b
                    .items
                    .iter()
                    .map(|i| [i.code.clone(), i.name.clone(), i.unit.clone(), i.price.map(trimmed).unwrap_or_default()])
                    .collect(),
                rules: b
                    .rules
                    .iter()
                    .map(|r| RuleRow {
                        measure: r.measure,
                        item: r.item.clone(),
                        factor: trimmed(r.factor),
                        material: r.material.clone().unwrap_or_default(),
                    })
                    .collect(),
                preview: Preview::default(),
            },
            // A new book: its source, items and prices are the agency's; nothing is filled in.
            None => Window { saved: false, fields: Default::default(), items: vec![Default::default()], rules: Vec::new(), preview: Preview::default() },
        };
        self.pay_window = Some(Box::new(window));
        self.dialog = Some(Dialog::Pay);
        self.refresh_pay();
    }

    fn refresh_pay(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.pay_window.as_mut() else { return };
        w.preview = match w.input().and_then(|q| road_pay_edit::preview(&doc.model, &q)) {
            Err(why) => Preview { why: Some(why), ..Preview::default() },
            Ok((answer, warnings)) => Preview { answer: Some(answer), warnings, why: None },
        };
    }

    pub(crate) fn pay_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.pay_window.as_mut() else { return Task::none() };
        match event {
            Event::Field(k, t) => w.fields[k.min(2)] = t,
            Event::Item(i, k, t) => {
                if let Some(it) = w.items.get_mut(i) {
                    // A code renamed takes its rules along.
                    if k == 0 {
                        for r in w.rules.iter_mut().filter(|r| r.item == it[0]) {
                            r.item = t.trim().to_owned();
                        }
                    }
                    it[k.min(3)] = t;
                }
            }
            Event::AddItem => w.items.push(Default::default()),
            Event::DropItem(i) => {
                if i < w.items.len() {
                    w.items.remove(i);
                }
            }
            Event::Measure(i, m) => {
                if let Some(r) = w.rules.get_mut(i) {
                    r.measure = m;
                }
            }
            Event::RuleItem(i, code) => {
                if let Some(r) = w.rules.get_mut(i) {
                    r.item = code;
                }
            }
            Event::Factor(i, t) => {
                if let Some(r) = w.rules.get_mut(i) {
                    r.factor = t;
                }
            }
            Event::Material(i, t) => {
                if let Some(r) = w.rules.get_mut(i) {
                    r.material = t;
                }
            }
            Event::AddRule => {
                let item = w.items.first().map(|i| i[0].trim().to_owned()).unwrap_or_default();
                w.rules.push(RuleRow { measure: PayMeasure::GuardrailLength, item, factor: "1".into(), material: String::new() });
            }
            Event::DropRule(i) => {
                if i < w.rules.len() {
                    w.rules.remove(i);
                }
            }
            Event::Save => return self.save_pay(),
            Event::Remove => return self.remove_pay(),
            Event::Close => {
                self.pay_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_pay();
        Task::none()
    }

    fn save_pay(&mut self) -> Task<Message> {
        let Some(w) = self.pay_window.as_ref() else { return Task::none() };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Poz defteri yazılamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_pay_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            if let Some(b) = &out.book {
                self.say(Level::Success, format!("Poz defteri yazıldı: {} pozla, {} kuralla.", b.items.len(), b.rules.len()));
            }
            if let Some(w) = self.pay_window.as_mut() {
                w.saved = true;
            }
            self.refresh_pay();
        }
        Task::none()
    }

    fn remove_pay(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = PayEdit {
            operation: RoadEditOperation::Remove,
            source: None,
            version: None,
            currency: None,
            items: None,
            rules: None,
            expected_revision: None,
        };
        let result = road_pay_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            self.pay_window = None;
            self.dialog = None;
            self.say(Level::Success, "Poz defteri kaldırıldı.");
        }
        Task::none()
    }

    pub(crate) fn pay_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.pay_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let field = |k: usize, hint: &'static str, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![field(0, "Listenin adı ve yılı", 360.0), field(1, "2026", 110.0), field(2, "TRY", 90.0)]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom);
        let fill = |title| road_dialog::Col { title, width: None, number: false };
        let items = &w.items;
        let items = iced::widget::column![
            road_dialog::grid_table(
                vec![road_dialog::col("Numara", 80.0), fill("Ad"), road_dialog::col("Birim", 64.0), road_dialog::col("Birim fiyat", 96.0), road_dialog::col("", 36.0)],
                items.len(),
                move |i, k| match k {
                    4 => road_dialog::drop_cell(msg(Event::DropItem(i))),
                    k => road_dialog::edit(&items[i][k], k == 3, move |t| msg(Event::Item(i, k, t))),
                },
                "Poz yok",
            ),
            words::secondary(ADD_ITEM, Some(msg(Event::AddItem))),
        ]
        .spacing(6);

        let codes: Vec<String> = w.items.iter().map(|i| i[0].trim().to_owned()).filter(|c| !c.is_empty()).collect();
        let rules = &w.rules;
        let rules = iced::widget::column![
            road_dialog::grid_table(
                vec![fill("Ölçü"), road_dialog::col("Poz", 84.0), road_dialog::col("Çarpan", 64.0), road_dialog::col("Malzeme", 96.0), road_dialog::col("", 36.0)],
                rules.len(),
                move |i, k| {
                    let r = &rules[i];
                    match k {
                        0 => road_dialog::pick(MEASURES.map(|m| m.1), MEASURES.iter().position(|m| m.0 == r.measure), move |k| {
                            msg(Event::Measure(i, MEASURES[k].0))
                        }),
                        1 => {
                            let list = codes.clone();
                            road_dialog::pick(codes.clone(), codes.iter().position(|c| *c == r.item), move |k| msg(Event::RuleItem(i, list[k].clone())))
                        }
                        2 => road_dialog::edit(&r.factor, true, move |t| msg(Event::Factor(i, t))),
                        3 => road_dialog::edit(&r.material, false, move |t| msg(Event::Material(i, t))),
                        _ => road_dialog::drop_cell(msg(Event::DropRule(i))),
                    }
                },
                "Kural yok",
            ),
            words::secondary(ADD_RULE, Some(msg(Event::AddRule))),
        ]
        .spacing(6);
        let book = row![container(items).width(Fill), container(rules).width(Fill)].spacing(12);
        let mut body = Column::new().spacing(10).push(top).push(
            row![label::caption("Pozlar").style(muted).width(Fill), label::caption("Kurallar").style(muted).width(Fill)].spacing(12),
        );
        body = body.push(book);
        match (&w.preview.why, &w.preview.answer) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(a)) => {
                let money = a.currency.clone().unwrap_or_default();
                use road_dialog::{col, grouped, num};
                let rows = a
                    .rows
                    .iter()
                    .map(|r| {
                        let sources = r
                            .lines
                            .iter()
                            .map(|l| format!("{}{}", l.name, if l.stale { " (eski)" } else { "" }))
                            .collect::<Vec<_>>()
                            .join(", ");
                        vec![
                            r.code.clone(),
                            r.name.clone(),
                            r.unit.clone(),
                            grouped(r.quantity, 2),
                            r.price.map_or_else(|| "—".into(), |p| grouped(p, 2)),
                            r.amount.map_or_else(|| "—".into(), |p| format!("{} {money}", grouped(p, 2))),
                            sources,
                        ]
                    })
                    .collect();
                let table = road_dialog::sheet(
                    vec![
                        col("Numara", 80.0),
                        col("Ad", 220.0),
                        col("Birim", 60.0),
                        num("Miktar", 110.0),
                        num("Birim fiyat", 110.0),
                        num("Tutar", 140.0),
                        road_dialog::Col { title: "Kaynaklar", width: None, number: false },
                    ],
                    rows,
                    "Poz tutarı yok",
                    240.0,
                );
                body = body.push(label::caption(format!(
                    "{} · sürüm {} · toplam {} {money}{}",
                    a.source,
                    a.version,
                    road_dialog::grouped(a.total, 2),
                    if a.unpriced > 0 { format!(" · {} pozun fiyatı yok", a.unpriced) } else { String::new() }
                )));
                if !w.preview.warnings.is_empty() {
                    let all = w.preview.warnings.iter().map(|x| x.message.as_str()).collect::<Vec<_>>().join(" ");
                    body = body.push(label::caption(all).style(warn));
                }
                body = body.push(table);
            }
            _ => {}
        }
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer =
            row![
                words::secondary(REMOVE, w.saved.then(|| msg(Event::Remove))),
                space::horizontal().width(Fill),
                words::secondary(CLOSE, Some(msg(Event::Close))),
                words::primary(SAVE, w.preview.answer.as_ref().map(|_| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center);
        Frame::new(TITLE).scroll(body).footer(footer).width(1080.0).modal(msg(Event::Close))
    }

    pub(crate) fn pay_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.pay_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADD_ITEM) => Some(msg(Event::AddItem)),
            Control::Press(ADD_RULE) => Some(msg(Event::AddRule)),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
