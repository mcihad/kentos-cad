//! Katman değişiklikleri (docs/adr/9123, plan/veri.md VY-22): a project's save and its data layers' saves told apart.
//!
//! - Kaydet writes the project: its layers, looks and settings. A data layer's rows reach their source (a GeoPackage's
//!   table, a database's) only by Kaynağa yaz, and only then do other users and programs see them.
//! - While a data layer holds rows its source does not have, the status bar says how many, in how many layers, in the
//!   warning colour; after a write its source refused, in the danger colour, with the reason. Clicked, this window
//!   opens.
//! - The window lists the layers by name and source, and the chosen layer's rows: new, changed field by field (as the
//!   row came from the source and as it is on this device), deleted. Kaynağa yaz writes one layer or all of them,
//!   Değişiklikleri geri al lets one layer's go after a second press that says how many rows.
//! - It says where the rows wait meanwhile: a saved project keeps them on this device (data_journal.rs,
//!   docs/adr/9111 §3), and a copy of the project file taken to another computer does not carry them.

use std::cell::RefCell;
use std::collections::BTreeMap;

use iced::widget::{column, container, row};
use iced::{Element, Length, Task};
use kentos_ui::widget::app_menu::Item;
use kentos_ui::widget::list_details::ListDetails;
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::widget::{Dialog, Tip, overlay, table};
use kentos_ui::{label, theme::Tokens};

use crate::app::{App, Dialog as AppDialog, Message};
use crate::exchange::words;

/// A data layer with rows its source does not have yet.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Waiting {
    pub layer: String,
    pub name: String,
    /// Where its rows are, as Katmanlar's tip says it: the file, or the database and table.
    pub place: String,
    pub added: usize,
    pub changed: usize,
    pub deleted: usize,
    /// Why the last write to its source did not go, while its changes wait.
    pub failed: Option<String>,
    /// A write of it whose commit's answer was lost (docs/adr/9126): whether a receipt went in with it (the source can
    /// be asked).
    pub uncertain: Option<bool>,
}

impl Waiting {
    pub fn rows(&self) -> usize {
        self.added + self.changed + self.deleted
    }
}

/// The window's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct PendingWindow {
    /// The layer shown on the right; the first one when none is chosen.
    chosen: Option<String>,
    /// Değişiklikleri geri al… pressed for this layer: its rows go at the second press.
    rolling_back: Option<String>,
    /// Yeniden gönder… pressed for this layer (an uncertain write without a receipt): sent at the second press.
    resending: Option<String>,
    /// Whether a database layer's source keeps write receipts, by layer, as last asked; why it could not be asked.
    receipts: BTreeMap<String, Result<bool, String>>,
}

/// What the window and the status bar's cell say.
#[derive(Debug, Clone)]
pub enum Event {
    /// The window, on this layer or the first waiting one.
    Open(Option<String>),
    Choose(String),
    /// Kaynağa yaz: these layers, every waiting one when none is named.
    Save(Vec<String>),
    /// Değişiklikleri geri al…: asks.
    Rollback(String),
    RollbackConfirmed(String),
    RollbackCancelled,
    /// Yeniden gönder…: asks; the second press sends the uncertain write's changes again (docs/adr/9126).
    Resend(String),
    ResendConfirmed(String),
    ResendCancelled,
    /// Whether the layer's source keeps write receipts, as asked off the interface's thread.
    Receipts(String, Result<bool, String>),
    /// Yazım makbuzlarını kur: the receipts' table made in the layer's source.
    SetUpReceipts(String),
    SetUp(String, Result<(), String>),
    Close,
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::DataPending(e))
}

/// The status bar cell's message.
pub(crate) fn open() -> Message {
    message(Event::Open(None))
}

/// A layer menu's Değişiklikleri incele…: the window on that layer.
pub(crate) fn open_at(layer: String) -> Message {
    message(Event::Open(Some(layer)))
}

/// The window on this layer, or the first waiting one.
pub(crate) fn open_at_message(layer: Option<String>) -> Message {
    message(Event::Open(layer))
}

/// “1 katmanda 3 satır”.
fn amount(layers: usize, rows: usize) -> String {
    format!("{layers} katmanda {rows} satır")
}

/// A row's fields as the table shows a whole row: its filled values, `alan=değer`, in the fields' order.
fn row_words(entity: &kentos_commands::Entity) -> String {
    let words: Vec<String> = entity
        .base()
        .attrs
        .iter()
        .filter(|(_, v)| !v.trim().is_empty())
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    if words.is_empty() { "boş satır".to_owned() } else { words.join(", ") }
}

/// The shape of an object without its fields: whether the geometry changed.
fn shape_of(entity: &kentos_commands::Entity) -> kentos_commands::Entity {
    let mut shape = entity.clone();
    shape.base_mut().attrs.clear();
    shape
}

/// One line of the rows' table: what, the key, the field, as it came, on this device.
type Line = [String; 5];

/// A layer's waiting rows, line by line: new rows whole, changed rows field by field (and their shape), deleted rows
/// as they came.
fn lines_of(model: &kentos_domain::Document, layer: &str) -> Vec<Line> {
    let c = model.data_changes(layer);
    let mut out = Vec::new();
    for (_, entity) in &c.added {
        out.push(["Yeni".into(), "—".into(), "bütün satır".into(), "—".into(), row_words(entity)]);
    }
    for d in &c.changed {
        let (now, was) = (&d.now.base().attrs, &d.row.base().attrs);
        let mut fields: Vec<&String> = now.keys().chain(was.keys()).collect();
        fields.sort();
        fields.dedup();
        for f in fields {
            let (a, b) = (was.get(f).map_or("", String::as_str), now.get(f).map_or("", String::as_str));
            if a != b {
                let shown = |v: &str| if v.is_empty() { "boş".to_owned() } else { v.to_owned() };
                out.push(["Değişen".into(), d.key.to_string(), f.clone(), shown(a), shown(b)]);
            }
        }
        if shape_of(&d.now) != shape_of(&d.row) {
            out.push(["Değişen".into(), d.key.to_string(), "şekil".into(), "kaynaktaki".into(), "taşındı".into()]);
        }
    }
    for key in &c.deleted {
        let was = c.originals.get(key).map_or_else(|| "kaynaktaki satır".to_owned(), |(_, e)| row_words(e));
        out.push(["Silinen".into(), key.to_string(), "bütün satır".into(), was, "—".into()]);
    }
    out
}

thread_local! {
    /// The last count: the opened drawing (its session) and its generation, and its waiting layers without their
    /// failures. The status bar asks every frame; the rows are compared once for each change of the drawing.
    #[allow(clippy::type_complexity)]
    static LAST: RefCell<Option<((u64, u64), Vec<Waiting>)>> = const { RefCell::new(None) };
}

/// The waiting layers of a drawing, counted again only when it is another or changed.
fn counted(doc: &crate::document::Document) -> Vec<Waiting> {
    let key = (doc.session, doc.model.generation());
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        if let Some((at, w)) = &*last
            && *at == key
        {
            return w.clone();
        }
        let model = &doc.model;
        let edited = model.edited_layers();
        let waiting: Vec<Waiting> = model
            .layers()
            .leaves()
            .iter()
            .filter(|n| edited.contains(&n.id))
            .filter_map(|n| {
                let c = model.data_changes(&n.id);
                (!c.is_empty()).then(|| Waiting {
                    layer: n.id.clone(),
                    name: n.name.clone(),
                    place: n.data.as_ref().map(|d| d.place_words()).unwrap_or_default(),
                    added: c.added.len(),
                    changed: c.changed.len(),
                    deleted: c.deleted.len(),
                    failed: None,
                    uncertain: None,
                })
            })
            .collect();
        *last = Some((key, waiting.clone()));
        waiting
    })
}

impl App {
    /// The data layers with rows their source does not have yet, in the tree's order, each with why its last write
    /// failed.
    pub(crate) fn data_waiting(&self) -> Vec<Waiting> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut waiting = counted(doc);
        for w in &mut waiting {
            w.failed = self.gis.data_edits.failed_of(&w.layer).map(str::to_owned);
            w.uncertain = self.gis.data_edits.uncertain.get(&w.layer).map(|u| u.receipt.is_some());
        }
        waiting
    }

    /// A layer's data source, when it is a data layer.
    fn data_of(&self, layer: &str) -> Option<kentos_commands::DataLayer> {
        self.document.as_ref()?.model.layers().get(layer)?.data.clone()
    }

    fn data_layer_name_of(&self, layer: &str) -> String {
        self.document
            .as_ref()
            .and_then(|d| d.model.layers().get(layer).map(|n| n.name.clone()))
            .unwrap_or_else(|| layer.to_owned())
    }

    /// Whether the waiting database layers' sources keep write receipts, asked off the interface's thread: a file's
    /// commit has a sure answer, a database's may be lost (docs/adr/9126).
    fn ask_receipts(&mut self) -> Task<Message> {
        let tasks: Vec<Task<Message>> = self
            .data_waiting()
            .into_iter()
            .filter_map(|w| Some((w.layer.clone(), self.data_of(&w.layer).filter(|d| d.driver == "postgis")?)))
            .map(|(layer, data)| {
                crate::db_connections::off_thread(
                    move || kentos_native_application::data_write::receipts_ready(&data),
                    move |known| message(Event::Receipts(layer, known)),
                )
            })
            .collect();
        Task::batch(tasks)
    }

    /// After a form's Kaydet on a data layer's record (docs/adr/9123): the record waits in its layer, and how many of
    /// the layer's rows its source does not have yet; none when the record's layer is not a data layer being edited.
    pub(crate) fn waiting_words_of(&self, uid: &str) -> Option<String> {
        let doc = self.document.as_ref()?;
        let uid = kentos_domain::Uuid::parse_str(uid).ok()?;
        let layer = doc.model.slot_of(uid).and_then(|s| doc.model.get(s)).map(|e| e.base().layer_id.clone())?;
        let w = self.data_waiting().into_iter().find(|w| w.layer == layer)?;
        Some(format!(
            "Değer “{}” katmanında bekliyor, kaynağa yazılmadı: {}. Kaynağa yazmak için durum çubuğundaki “Kaynağa \
             yazılmadı”ya ya da katmanın menüsünde Değişiklikleri kaydet'e tıklayın.",
            w.name,
            crate::data_edits::counts_words(w.added, w.changed, w.deleted)
        ))
    }

    /// Where the waiting rows are kept meanwhile, in a sentence.
    fn data_draft_words(&self) -> String {
        let Some(doc) = &self.document else {
            return String::new();
        };
        match crate::data_journal::place_of(doc) {
            _ if doc.cloud_source().is_some() => {
                "Bulut projesinde bekleyen satırlar bu cihazda saklanmaz; kapatmadan önce kaynağa yazın.".to_owned()
            }
            Some(_) if !doc.dirty() => "Proje kaydedildi: bekleyen satırlar bu cihazda taslak olarak saklanıyor ve proje burada \
                 açılınca geri gelir. Proje dosyasını başka bir bilgisayara kopyalamak onları taşımaz."
                .to_owned(),
            Some(_) => "Proje kaydedilince bekleyen satırlar bu cihazda taslak olarak saklanır; şimdilik yalnız bu oturumda."
                .to_owned(),
            None => "Proje henüz bir yere kaydedilmedi: bekleyen satırlar yalnız bu oturumda; kapatmadan önce kaynağa yazın \
                 ya da projeyi kaydedin."
                .to_owned(),
        }
    }

    /// The status bar's cell while a data layer has rows its source does not have: how many, in how many layers; after
    /// a refused write, in the danger colour. As the bar tightens (`fit`, its narrow-window step): the rows alone, then
    /// the number alone beside its icon; the tip keeps every word.
    pub(crate) fn data_pending_cell(&self, fit: u8) -> Option<Element<'_, Message>> {
        let (words, failed) = self.data_pending_words(fit)?;
        let waiting = self.data_waiting();
        let body = format!(
            "Kaydet projeyi yazar (katmanlar, görünüşler, ayarlar). Satırlar kaynağa Kaynağa yaz ile gider; başka \
             kullanıcılar ve programlar onları ancak o zaman görür. Bekleyen: {}. {} Tıklayın: satırları inceleyin, \
             kaynağa yazın ya da geri alın.",
            waiting
                .iter()
                .map(|w| format!("{} ({}): {}", w.name, w.place, crate::data_edits::counts_words(w.added, w.changed, w.deleted)))
                .collect::<Vec<_>>()
                .join("; "),
            self.data_draft_words()
        );
        let body = match waiting.iter().find_map(|w| w.failed.as_ref().map(|f| (w.name.clone(), f.clone()))) {
            Some((name, why)) => format!("Son yazma denemesi “{name}” için başarısız: {why} Değişiklikler duruyor. {body}"),
            None => body,
        };
        Some(
            Readout::new(label::strong(words).style(move |theme: &iced::Theme| iced::widget::text::Style {
                color: Some(if failed { Tokens::of(theme).danger } else { Tokens::of(theme).warning }),
            }))
            .icon(crate::icons::from_web(Some("save")))
            .tip(Tip::new("Katman değişiklikleri").body(body))
            .on_press(open())
            .into(),
        )
    }

    /// The cell's words at the bar's narrow-window step `fit` and whether a write failed; none while nothing waits.
    pub(crate) fn data_pending_words(&self, fit: u8) -> Option<(String, bool)> {
        let waiting = self.data_waiting();
        if waiting.is_empty() {
            return None;
        }
        let rows: usize = waiting.iter().map(Waiting::rows).sum();
        let failed = waiting.iter().any(|w| w.failed.is_some());
        let unsure = waiting.iter().any(|w| w.uncertain.is_some());
        let words = match (fit, failed, unsure) {
            (7.., _, _) => rows.to_string(),
            (4.., _, true) => format!("{rows} satırın sonucu bilinmiyor"),
            (4.., false, _) => format!("{rows} satır bekliyor"),
            (4.., true, _) => format!("{rows} satır yazılamadı"),
            (_, _, true) => format!("Yazımın sonucu bilinmiyor: {}", amount(waiting.len(), rows)),
            (_, false, _) => format!("Kaynağa yazılmadı: {}", amount(waiting.len(), rows)),
            (_, true, _) => format!("Kaynağa yazılamadı: {}", amount(waiting.len(), rows)),
        };
        Some((words, failed))
    }

    pub(crate) fn data_pending_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(layer) => {
                let first = self.data_waiting().first().map(|w| w.layer.clone());
                let w = self.gis.data_pending.get_or_insert_with(PendingWindow::default);
                if layer.is_some() || w.chosen.is_none() {
                    w.chosen = layer.or(first);
                }
                self.dialog = Some(AppDialog::Gis(crate::gis::GisDialogKind::DataPending));
                return self.ask_receipts();
            }
            Event::Receipts(layer, known) => {
                if let Some(w) = self.gis.data_pending.as_mut() {
                    w.receipts.insert(layer, known);
                }
            }
            Event::SetUpReceipts(layer) => {
                let Some(data) = self.data_of(&layer) else {
                    return Task::none();
                };
                return crate::db_connections::off_thread(
                    move || kentos_native_application::data_write::set_up_receipts(&data),
                    move |done| message(Event::SetUp(layer, done)),
                );
            }
            Event::SetUp(layer, done) => match done {
                Ok(()) => {
                    let name = self.data_layer_name_of(&layer);
                    self.say(
                        kentos_interaction::Level::Success,
                        format!(
                            "Yazım makbuzları “{name}” katmanının kaynağında kuruldu (kentos_writes tablosu; öbür tablolar \
                             değişmedi). Bundan sonraki yazımların sonucu bağlantı kopsa da kaynağa sorulabilir."
                        ),
                    );
                    if let Some(w) = self.gis.data_pending.as_mut() {
                        w.receipts.insert(layer, Ok(true));
                    }
                }
                Err(why) => self.warn(format!("Yazım makbuzları kurulamadı: {why}")),
            },
            Event::Resend(layer) => {
                if let Some(w) = self.gis.data_pending.as_mut() {
                    w.resending = Some(layer);
                }
            }
            Event::ResendCancelled => {
                if let Some(w) = self.gis.data_pending.as_mut() {
                    w.resending = None;
                }
            }
            Event::ResendConfirmed(layer) => {
                if let Some(w) = self.gis.data_pending.as_mut() {
                    w.resending = None;
                }
                return self.data_edits_event(crate::data_edits::Event::Resend(vec![layer]));
            }
            Event::Choose(layer) => {
                if let Some(w) = self.gis.data_pending.as_mut() {
                    w.chosen = Some(layer);
                    w.rolling_back = None;
                }
            }
            Event::Save(layers) => {
                let layers = if layers.is_empty() {
                    self.data_waiting().into_iter().map(|w| w.layer).collect()
                } else {
                    layers
                };
                return self.data_edits_event(crate::data_edits::Event::Save(layers));
            }
            Event::Rollback(layer) => {
                if let Some(w) = self.gis.data_pending.as_mut() {
                    w.rolling_back = Some(layer);
                }
            }
            Event::RollbackCancelled => {
                if let Some(w) = self.gis.data_pending.as_mut() {
                    w.rolling_back = None;
                }
            }
            Event::RollbackConfirmed(layer) => {
                if let Some(w) = self.gis.data_pending.as_mut() {
                    w.rolling_back = None;
                }
                return self.data_edits_event(crate::data_edits::Event::Rollback(vec![layer]));
            }
            Event::Close => {
                self.gis.data_pending = None;
                if matches!(self.dialog, Some(AppDialog::Gis(crate::gis::GisDialogKind::DataPending))) {
                    self.dialog = None;
                }
            }
        }
        Task::none()
    }

    /// The window: the waiting layers on the left, the chosen one's rows and actions on the right.
    pub(crate) fn data_pending_view(&self) -> Element<'_, Message> {
        let waiting = self.data_waiting();
        let state = self.gis.data_pending.clone().unwrap_or_default();
        let chosen = state
            .chosen
            .as_ref()
            .and_then(|c| waiting.iter().find(|w| &w.layer == c))
            .or_else(|| waiting.first())
            .cloned();
        let saving = self.data_saving();
        let mut dialog = Dialog::new("Katman değişiklikleri").key("data-pending");
        if waiting.is_empty() {
            dialog = dialog.push(
                container(words::text_line(
                    words::Kind::Ok,
                    "Bekleyen katman değişikliği yok: bütün satırlar kaynaklarında.",
                ))
                .padding([4, 16]),
            );
        } else {
            let rows: usize = waiting.iter().map(Waiting::rows).sum();
            dialog = dialog.push(
                container(words::text_line(
                    words::Kind::Info,
                    format!(
                        "{} kaynağa yazılmadı. Kaydet projeyi yazar; satırlar kaynağa Kaynağa yaz ile gider. {}",
                        amount(waiting.len(), rows),
                        self.data_draft_words()
                    ),
                ))
                .padding([0, 16]),
            );
            let list = waiting.iter().fold(column![].spacing(1), |c, w| {
                let mut subtitle = format!("{} · {}", w.place, crate::data_edits::counts_words(w.added, w.changed, w.deleted));
                if w.uncertain.is_some() {
                    subtitle.push_str(" · sonucu bilinmiyor");
                } else if w.failed.is_some() {
                    subtitle.push_str(" · yazılamadı");
                }
                c.push(
                    Item::new(w.name.clone())
                        .subtitle(subtitle)
                        .on_press(message(Event::Choose(w.layer.clone())))
                        .current(chosen.as_ref().is_some_and(|ch| ch.layer == w.layer)),
                )
            });
            let details = chosen.map_or_else(|| column![].into(), |w| self.data_pending_details(w, &state, saving));
            dialog = dialog.push(ListDetails::new(list, details).list_width(220.0).height(380.0));
        }
        let mut dialog = dialog.action(words::secondary("Kapat", Some(message(Event::Close))));
        if waiting.len() > 1 {
            dialog = dialog.action(words::primary(
                "Hepsini kaynağa yaz",
                (!saving).then(|| message(Event::Save(Vec::new()))),
            ));
        }
        if saving {
            dialog = dialog.status(label::caption("Kaynağa yazılıyor…"));
        }
        overlay::blocking(dialog.width(860.0).max_height(620.0).on_close(Some(message(Event::Close))))
    }

    /// The chosen layer: its source, its counts, why its last write failed, its rows and its two actions.
    fn data_pending_details(&self, w: Waiting, state: &PendingWindow, saving: bool) -> Element<'_, Message> {
        let lines = self.document.as_ref().map(|d| lines_of(&d.model, &w.layer)).unwrap_or_default();
        let mut c = column![
            label::heading(w.name.clone()),
            label::caption(format!(
                "{} · {} kaynağa yazılmadı",
                w.place,
                crate::data_edits::counts_words(w.added, w.changed, w.deleted)
            )),
        ]
        .spacing(4);
        match (w.uncertain, &w.failed) {
            (Some(true), _) => {
                c = c.push(words::text_line(
                    words::Kind::Warn,
                    "Önceki yazımın sonucu bilinmiyor: bağlantı yazım bitmek üzereyken koptu. Kaynağa yaz önce kaynağa bu \
                     yazımın makbuzunu sorar; yazılmışsa satırlar ikinci kez yazılmaz.",
                ));
            }
            (Some(false), _) => {
                c = c.push(words::text_line(
                    words::Kind::Error,
                    "Önceki yazımın sonucu bilinmiyor ve bu kaynak yazım makbuzu tutmuyor: kayıt kendiliğinden yeniden \
                     gönderilmez. Kaynakta satırları denetleyin; yazılmamışsa Yeniden gönder, yazılmışsa Değişiklikleri \
                     geri al.",
                ));
            }
            (None, Some(why)) => {
                c = c.push(words::text_line(
                    words::Kind::Error,
                    format!("Son yazma denemesi başarısız: {why} Değişiklikler duruyor; Kaynağa yaz yeniden dener."),
                ));
            }
            (None, None) => {}
        }
        // A database whose commit's answer may be lost: its receipts, or the way to set them up (docs/adr/9126).
        match state.receipts.get(&w.layer) {
            Some(Ok(false)) => {
                c = c.push(
                    row![
                        words::text_line(
                            words::Kind::Info,
                            "Bu veritabanı yazım makbuzu tutmuyor: bağlantı yazım sırasında koparsa sonucu sorulamaz.",
                        ),
                        words::secondary("Yazım makbuzlarını kur", Some(message(Event::SetUpReceipts(w.layer.clone())))),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                );
            }
            Some(Err(why)) => {
                c = c.push(words::text_line(words::Kind::Info, format!("Yazım makbuzları sorulamadı: {why}")));
            }
            _ => {}
        }
        let table = table::Table::new([
            table::Column::new("Durum").width(Length::Fixed(kentos_ui::theme::typography::scaled(64.0))),
            table::Column::new("Anahtar").width(Length::Fixed(kentos_ui::theme::typography::scaled(64.0))).align_right(),
            table::Column::new("Alan").width(Length::FillPortion(2)),
            table::Column::new("Kaynakta").width(Length::FillPortion(3)),
            table::Column::new("Bu cihazda").width(Length::FillPortion(3)),
        ])
        .extend(lines.into_iter().map(|l| {
            table::Row::new([
                table::text(l[0].clone()).into(),
                table::number(l[1].clone()).into(),
                table::text(l[2].clone()).into(),
                table::text(l[3].clone()).into(),
                table::text(l[4].clone()).into(),
            ])
        }))
        .height(Length::Fill);
        c = c.push(container(table).height(Length::Fill).style(kentos_ui::style::container::bordered));
        let actions: Element<'_, Message> = if state.resending.as_deref() == Some(w.layer.as_str()) {
            row![
                label::body(
                    "Satırlar kaynağa yeniden gönderilecek; önceki yazım tamamlanmışsa yeni satırlar ikinci kez eklenir."
                )
                .width(Length::Fill),
                words::secondary("Vazgeç", Some(message(Event::ResendCancelled))),
                words::danger("Yeniden gönder", Some(message(Event::ResendConfirmed(w.layer.clone())))),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        } else if w.uncertain == Some(false) {
            row![
                iced::widget::Space::new().width(Length::Fill),
                words::ghost("Değişiklikleri geri al…", (!saving).then(|| message(Event::Rollback(w.layer.clone())))),
                words::secondary("Yeniden gönder…", (!saving).then(|| message(Event::Resend(w.layer.clone())))),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        } else if state.rolling_back.as_deref() == Some(w.layer.as_str()) {
            row![
                label::body(format!(
                    "{} değişikliği bırakılacak; kaynak olduğu gibi kalır.",
                    crate::data_edits::counts_words(w.added, w.changed, w.deleted)
                ))
                .width(Length::Fill),
                words::secondary("Vazgeç", Some(message(Event::RollbackCancelled))),
                words::danger("Geri al", Some(message(Event::RollbackConfirmed(w.layer.clone())))),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        } else {
            row![
                iced::widget::Space::new().width(Length::Fill),
                words::ghost("Değişiklikleri geri al…", (!saving).then(|| message(Event::Rollback(w.layer.clone())))),
                words::primary("Kaynağa yaz", (!saving).then(|| message(Event::Save(vec![w.layer.clone()])))),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        };
        c.push(actions).height(Length::Fill).into()
    }
}
