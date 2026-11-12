//! Sembolü karardan ver (docs/adr/4240; `cad.zoning.symbolize`), opened from İmar planı denetimi. The decision areas'
//! MPYY symbols come from their decisions: one row per plan level and use the drawing's decision areas have, with the
//! symbol KentOS's table gives it (the level's area symbol named as the use) or the one the user chose, how many areas
//! it holds and how many change. A use without a symbol is left open until the user chooses one for the row pressed,
//! among the level's area symbols; KentOS does not guess. Sembolleri ver writes the symbols in one undo step; the
//! decisions (the attributes) are never written.
//!
//! docs/adr/5040: the window opens with the project's kept choices; Seçimleri projede sakla (on) keeps the window's
//! choices in the project in the same undo step, so the next run and the zoning check read them; Seçimi kaldır takes
//! the pressed row's choice back to KentOS's table.

use crate::map;
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, ZONING_PROFILE, ZoningSymbolPick, ZoningSymbolize, ZoningSymbolized, ZoningSymbols, builtin_profile,
    builtin_zoning_symbols,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, zoning_symbolize};
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::{Dialog as Frame, Form};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet_chosen};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Sembolü karardan ver";
/// İmar planı denetimi's button that opens it.
pub const OPEN: &str = "Sembolleri karardan ver…";
const CLOSE: &str = "Kapat";
pub const WRITE: &str = "Sembolleri ver";
pub const SYMBOL: &str = "Sembol";
pub const KEEP: &str = "Seçimleri projede sakla";
pub const UNPICK: &str = "Seçimi kaldır";

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The user's symbols, by level and use; the project's kept ones when the window opens.
    pub picks: Vec<ZoningSymbolPick>,
    /// The project's kept choices when the window opened.
    pub kept: Vec<ZoningSymbolPick>,
    /// Whether Sembolleri ver keeps the choices in the project.
    pub keep: bool,
    /// The command's plan for every decision area.
    pub answer: Option<Result<ZoningSymbolized, String>>,
    /// The row pressed (a level and use).
    pub chosen: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Choose(usize),
    /// The chosen row's symbol, by its library id.
    Pick(String),
    /// The chosen row's choice taken back.
    Unpick,
    Keep(bool),
    Write,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Map(map::Event::ZoningSymbols(e))
}

/// A level and use the drawing's decision areas have: its symbol and where it comes from, its areas and how many change.
#[derive(Clone, Debug, PartialEq)]
pub struct Pair {
    pub level: String,
    pub usage: String,
    pub symbol: Option<String>,
    pub from: Option<String>,
    pub areas: usize,
    pub set: usize,
}

/// The rows: every level and use with a symbol or without one (the areas left for no plan, level or use are not rows),
/// in the order the areas first show them.
pub fn pairs(r: &ZoningSymbolized) -> Vec<Pair> {
    let mut out: Vec<Pair> = Vec::new();
    for a in r.areas.iter().filter(|a| matches!(a.state.as_str(), "set" | "same" | "unmapped")) {
        let at = match out.iter().position(|p| p.level == a.level && p.usage == a.usage) {
            Some(at) => at,
            None => {
                out.push(Pair {
                    level: a.level.clone(),
                    usage: a.usage.clone(),
                    symbol: a.symbol.clone(),
                    from: a.from.clone(),
                    areas: 0,
                    set: 0,
                });
                out.len() - 1
            }
        };
        out[at].areas += 1;
        out[at].set += usize::from(a.state == "set");
    }
    out
}

fn label(domain: &str, code: &str) -> String {
    builtin_profile(ZONING_PROFILE)
        .and_then(|p| p.domains.into_iter().find(|d| d.id == domain))
        .and_then(|d| d.values)
        .and_then(|v| v.into_iter().find(|c| c.code == code))
        .map_or_else(|| code.to_owned(), |c| c.label)
}

fn symbol_name(t: &ZoningSymbols, level: &str, id: &str) -> String {
    t.level(level).and_then(|l| l.choices.iter().find(|c| c.id == id)).map_or_else(|| id.to_owned(), |c| c.name.clone())
}

/// The table's rows; a choice the project keeps as it is says Projede.
pub fn rows(r: &ZoningSymbolized, t: &ZoningSymbols, kept: &[ZoningSymbolPick]) -> Vec<Vec<String>> {
    pairs(r)
        .into_iter()
        .map(|p| {
            let held = kept.iter().any(|k| k.level == p.level && k.usage == p.usage && Some(&k.symbol) == p.symbol.as_ref());
            vec![
                label("plan-seviye", &p.level),
                label("plan-kullanim", &p.usage),
                p.symbol.as_deref().map_or_else(|| "— seçilmedi".to_owned(), |s| symbol_name(t, &p.level, s)),
                match p.from.as_deref() {
                    Some("project") => "Projede".to_owned(),
                    Some("user") if held => "Projede".to_owned(),
                    Some("user") => "Seçildi".to_owned(),
                    Some(_) => "KentOS".to_owned(),
                    None => "—".to_owned(),
                },
                p.areas.to_string(),
                p.set.to_string(),
            ]
        })
        .collect()
}

impl App {
    fn zoning_symbols_input(&self) -> ZoningSymbolize {
        let s = &self.map.zoning_symbols;
        ZoningSymbolize { uids: None, symbols: s.picks.clone(), keep: s.keep, expected_revision: None }
    }

    /// Whether Sembolleri ver has something to write: a symbol, or a choice to keep.
    fn zoning_symbols_ready(&self) -> bool {
        let s = &self.map.zoning_symbols;
        matches!(&s.answer, Some(Ok(r)) if r.set > 0) || (s.keep && s.picks != s.kept && matches!(s.answer, Some(Ok(_))))
    }

    fn zoning_symbols_run(&mut self) {
        let input = self.zoning_symbols_input();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        self.map.zoning_symbols.answer = Some(
            match zoning_symbolize::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Önizleme alınamadı.".to_owned()),
            },
        );
    }

    /// Sembolü karardan ver from İmar planı denetimi.
    pub(crate) fn open_zoning_symbols(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        let kept = self
            .document
            .as_ref()
            .map(|d| zoning_symbolize::kept(&d.model))
            .unwrap_or_default();
        self.map.zoning_symbols = State {
            picks: kept.clone(),
            kept,
            keep: true,
            ..State::default()
        };
        self.zoning_symbols_run();
        self.dialog = Some(Dialog::Map(map::MapDialogKind::ZoningSymbols));
        Task::none()
    }

    pub(crate) fn zoning_symbols_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => return self.open_zoning_symbols(),
            Event::Choose(i) => self.map.zoning_symbols.chosen = Some(i),
            Event::Pick(id) => {
                let Some(Ok(r)) = &self.map.zoning_symbols.answer else {
                    return Task::none();
                };
                let Some(p) = self
                    .map
                    .zoning_symbols
                    .chosen
                    .and_then(|i| pairs(r).into_iter().nth(i))
                else {
                    return Task::none();
                };
                let picks = &mut self.map.zoning_symbols.picks;
                picks.retain(|k| !(k.level == p.level && k.usage == p.usage));
                picks.push(ZoningSymbolPick { level: p.level, usage: p.usage, symbol: id });
                self.zoning_symbols_run();
            }
            Event::Unpick => {
                let Some(Ok(r)) = &self.map.zoning_symbols.answer else {
                    return Task::none();
                };
                let Some(p) = self
                    .map
                    .zoning_symbols
                    .chosen
                    .and_then(|i| pairs(r).into_iter().nth(i))
                else {
                    return Task::none();
                };
                self.map
                    .zoning_symbols
                    .picks
                    .retain(|k| !(k.level == p.level && k.usage == p.usage));
                self.zoning_symbols_run();
            }
            Event::Keep(on) => {
                self.map.zoning_symbols.keep = on;
                self.zoning_symbols_run();
            }
            Event::Write => {
                let input = self.zoning_symbols_input();
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match zoning_symbolize::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, .. } => {
                        let kept = output.kept.as_ref().map_or_else(String::new, |k| {
                            format!(" Projede {} sembol seçimi saklanıyor.", k.len())
                        });
                        self.say(
                            Level::Success,
                            format!(
                                "{TITLE}: {} karar alanının sembolü kararından verildi (tek adımda geri alınır); kararlar değişmedi.{kept}",
                                output.set
                            ),
                        );
                        self.map.zoning_symbols = State::default();
                        self.dialog = None;
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
            }
            Event::Close => {
                self.map.zoning_symbols = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn zoning_symbols_view(&self) -> Element<'_, Message> {
        let s = &self.map.zoning_symbols;
        let table = builtin_zoning_symbols();
        let mut lines = vec![words::text_line(
            Kind::Info,
            "Sembol karar değildir: karar, alanın Kullanım değeridir; sembol ondan ve planın kademesinden seçilir, sembol değişikliği kararı değiştirmez. KentOS yalnız adı kullanımla aynı yazılan MPYY alan sembolünü eşler; öbürlerini siz seçersiniz.",
        )];
        let mut table_rows = Vec::new();
        let mut ready = false;
        let mut picker: Option<Element<'_, Message>> = None;
        let mut picked = false;
        match (&s.answer, &table) {
            (Some(Ok(r)), Some(t)) => {
                ready = self.zoning_symbols_ready();
                let open = pairs(r).iter().filter(|p| p.symbol.is_none()).map(|p| p.areas).sum::<usize>();
                lines.push(words::text_line(
                    if r.set > 0 { Kind::Ok } else { Kind::Info },
                    format!(
                        "{} karar alanı: {} alanın sembolü verilir, {} alan zaten kararının sembolünde; {open} alanın kullanımının sembolü seçilmedi, {} alan atlanır (planı, kademesi ya da kullanımı yok).",
                        r.areas.len(),
                        r.set,
                        r.same,
                        r.skipped
                    ),
                ));
                table_rows = rows(r, t, &s.kept);
                if !s.kept.is_empty() {
                    lines.push(words::text_line(
                        Kind::Info,
                        format!("Projede {} sembol seçimi saklı; KentOS'un tablosundan önce okunur.", s.kept.len()),
                    ));
                }
                if let Some(p) = s.chosen.and_then(|i| pairs(r).into_iter().nth(i)) {
                    picked = s.picks.iter().any(|k| k.level == p.level && k.usage == p.usage);
                    picker = t.level(&p.level).map(|lv| {
                        let options: Vec<(String, String)> = lv.choices.iter().map(|c| (c.id.clone(), c.name.clone())).collect();
                        fields::select_owned(options, p.symbol.as_deref().unwrap_or(""), |id| msg(Event::Pick(id)))
                    });
                    if picker.is_none() {
                        lines.push(words::text_line(
                            Kind::Warn,
                            format!("{} kademesinin MPYY alan sembolleri yok; KentOS uygulama ve nazım imar planlarınınkini bilir.", label("plan-seviye", &p.level)),
                        ));
                    }
                }
            }
            (Some(Err(why)), _) => lines.push(words::text_line(Kind::Error, why.clone())),
            _ => {}
        }
        let cols = vec![
            col("Kademe", 160.0),
            col("Kullanım", 170.0),
            col("Sembol", 230.0),
            col("Kaynak", 80.0),
            num("Alan", 60.0),
            num("Verilecek", 80.0),
        ];
        let sheet = sheet_chosen(cols, table_rows, "Karar alanı yok.", 260.0, s.chosen, |i| msg(Event::Choose(i)));
        let mut form = Form::new().label_width(150.0).section("Seçili satır");
        form = match picker {
            Some(p) => form.field(SYMBOL, p).help("Kademenin MPYY alan sembolleri; seçim bu kademe ve kullanımın bütün alanlarına verilir."),
            None => form.field(SYMBOL, words::text_line(Kind::Info, "Tabloda bir satır seçin.")),
        };
        let form = form
            .row(words::secondary(UNPICK, picked.then(|| msg(Event::Unpick))))
            .section("Ver")
            .field(KEEP, Switch::new(s.keep, |on| msg(Event::Keep(on))))
            .help("Açıkken seçimler projede saklanır: sonraki çalıştırma ve imar denetimi onları okur.")
            .row(words::primary(WRITE, ready.then(|| msg(Event::Write))));
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(words::summary(lines))
            .push_fit(sheet, kentos_ui::theme::typography::scaled(120.0))
            .push(form);
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .width(880.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn zoning_symbols_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(WRITE) => self.zoning_symbols_ready().then(|| msg(Event::Write)),
            Control::Press(UNPICK) => Some(msg(Event::Unpick)),
            Control::Check(KEEP, on) => Some(msg(Event::Keep(on))),
            Control::Row(n) if n > 0 => Some(msg(Event::Choose(n - 1))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
