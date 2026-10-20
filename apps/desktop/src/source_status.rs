//! Kaynak durumu (docs/adr/0543; plan/gis.md CBS-40): a window with every bound layer's state as `cad.sources.describe`
//! says it (the source, whether it can be written, how much of the table the layer holds, its system, what waits on
//! either side) and the one way out the state asks for (`next`): Kaynağa yaz, Kaynaktan oku, Uzlaştır… or Yeniden
//! bağla…, run by the same events as Katmanlar's Kaynak ▸. The states are read again after each.

use iced::widget::{Column, container, row, text};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CommandResult, SourceKind, SourceNext, SourceState, SourcesDescribe};
use kentos_native_application::ExecutionContext;
use kentos_native_application::sources::describe;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided, VirtualList, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::bound::Event as Bound;
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const STATUS_TITLE: &str = "Kaynak durumu";
const REFRESH: &str = "Yenile";
const CLOSE: &str = "Kapat";
const ROW: f32 = 30.0;

#[derive(Debug, Clone)]
pub struct Window {
    states: Result<Vec<SourceState>, String>,
    /// The row pressed: its layer chosen in Katmanlar, its whole state said under the table.
    chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Pick(usize),
    /// The row's way out (its `next`).
    Act(usize),
    Refresh,
    Close,
}

fn msg(event: Event) -> Message {
    Message::SourceStatus(event)
}

/// The way out's button words; none when nothing waits.
pub fn act_words(next: SourceNext, kind: SourceKind) -> Option<&'static str> {
    Some(match next {
        SourceNext::None => return None,
        SourceNext::Write => "Kaynağa yaz",
        SourceNext::Read => "Kaynaktan oku",
        SourceNext::Reconcile => "Uzlaştır…",
        SourceNext::Relink if kind == SourceKind::Geopackage => "Yeniden bağla…",
        SourceNext::Relink => "Adresi denetle",
    })
}

/// The event that does the one thing a state asks for (`next`), the same for this window and for Öznitelik
/// tablosu's source line (docs/adr/0546 §3); none when nothing waits.
pub fn way_out(next: SourceNext, layer: Option<String>) -> Option<Bound> {
    Some(match next {
        SourceNext::None => return None,
        SourceNext::Write => Bound::Write(layer),
        SourceNext::Read => Bound::Read(layer),
        SourceNext::Relink => Bound::Relink(layer),
        SourceNext::Reconcile => Bound::Reconcile(layer),
    })
}

impl App {
    /// Kaynak durumu (`source.status`): the window over the project's bound layers.
    pub(crate) fn open_source_status(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.source_status = Some(Window {
            states: Err(String::new()),
            chosen: None,
        });
        self.dialog = Some(Dialog::SourceStatus);
        self.refresh_source_status();
    }

    /// The states read again (after a write, a read, a relink), while the window is open.
    pub(crate) fn refresh_source_status(&mut self) {
        if self.source_status.is_none() {
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let states = match describe::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SourcesDescribe::default(),
        ) {
            CommandResult::Completed { output, .. } => Ok(output.links),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Kaynakların durumu okunamadı.".to_owned()),
        };
        if let Some(w) = self.source_status.as_mut() {
            let count = states.as_ref().map_or(0, Vec::len);
            w.chosen = w.chosen.filter(|i| *i < count);
            w.states = states;
        }
    }

    pub(crate) fn source_status_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.source_status.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Pick(i) => {
                w.chosen = Some(i);
                if let Some(s) = w.states.as_ref().ok().and_then(|s| s.get(i)) {
                    let id = s.link.layer.clone();
                    self.selected_layer = Some(id.clone());
                    self.layers_follow = false;
                    self.layer_reveal = Some(id);
                }
                Task::none()
            }
            Event::Act(i) => {
                let Some(s) = w.states.as_ref().ok().and_then(|s| s.get(i)).cloned() else {
                    return Task::none();
                };
                w.chosen = Some(i);
                let Some(event) = way_out(s.next, Some(s.link.layer.clone())) else {
                    return Task::none();
                };
                // Its own window takes the place of this one.
                if matches!(event, Bound::Reconcile(_)) {
                    self.source_status = None;
                }
                let task = self.bound_event(event);
                self.refresh_source_status();
                task
            }
            Event::Refresh => {
                self.refresh_source_status();
                Task::none()
            }
            Event::Close => {
                self.source_status = None;
                self.dialog = None;
                Task::none()
            }
        }
    }

    pub(crate) fn source_status_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.source_status else {
            return text("").into();
        };
        let srid = self
            .document
            .as_ref()
            .map_or(0, |d| d.model.settings().srid);
        let (lines, states): (Vec<Element<'_, Message>>, Vec<SourceState>) = match &w.states {
            Err(why) if why.is_empty() => (Vec::new(), Vec::new()),
            Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], Vec::new()),
            Ok(s) if s.is_empty() => (
                vec![words::text_line(
                    Kind::Info,
                    "Projede bağlı katman yok. GeoPackage'ı “Bağlı katman olarak aç” ile, PostGIS tablosunu “PostGIS'e bağlan” ile alın."
                        .to_owned(),
                )],
                Vec::new(),
            ),
            Ok(s) => (
                vec![words::text_line(
                    if s.iter().all(|x| x.next == SourceNext::None) { Kind::Ok } else { Kind::Info },
                    summary_words(s),
                )],
                s.clone(),
            ),
        };
        let names: Vec<String> = states.iter().map(|s| self.bound_name(&s.link.layer)).collect();
        let chosen = w.chosen;
        let head = row![
            container(label::caption("Katman")).width(typography::scaled(120.0)),
            container(label::caption("Kaynak (dosya › tablo)")).width(Fill),
            container(label::caption("Durum")).width(typography::scaled(170.0)),
            container(label::caption("Kapsam")).width(typography::scaled(150.0)),
            container(label::caption("Sistem")).width(typography::scaled(110.0)),
            container(label::caption("")).width(typography::scaled(116.0)),
        ]
        .spacing(8)
        .padding([4, 8]);
        let detail = chosen.and_then(|i| states.get(i)).map(|s| detail_words(s, srid));
        let count = states.len();
        let list = VirtualList::new(count, typography::scaled(ROW), move |i| {
            let s = &states[i];
            let cell = |t: String| {
                Elided::new(t)
                    .size(typography::caption())
                    .font(typography::ui())
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(Tokens::of(theme).text),
                    })
                    .width(Fill)
            };
            let act: Element<'_, Message> = match act_words(s.next, s.link.kind) {
                Some(words) => {
                    let blocked = s.next == SourceNext::Write && !s.link.writable;
                    iced::widget::button(label::caption(if blocked { "Salt okunur" } else { words }))
                        .on_press_maybe((!blocked).then(|| msg(Event::Act(i))))
                        .padding([2, 8])
                        .style(style::button::secondary)
                        .into()
                }
                None => label::caption("").into(),
            };
            let face = row![
                container(cell(names[i].clone())).width(typography::scaled(120.0)),
                cell(source_words(s)),
                container(cell(state_words(s))).width(typography::scaled(170.0)),
                container(cell(scope_words(s))).width(typography::scaled(150.0)),
                container(cell(system_words(s, srid))).width(typography::scaled(110.0)),
                container(act)
                    .width(typography::scaled(116.0))
                    .align_x(Center),
            ]
            .spacing(8)
            .padding([0, 8])
            .height(typography::scaled(ROW))
            .align_y(Center);
            iced::widget::button(face)
                .on_press(msg(Event::Pick(i)))
                .padding(0)
                .width(Fill)
                .style(style::button::row(chosen == Some(i)))
                .into()
        })
        .height(typography::scaled(150.0));
        let table = container(Column::new().push(head).push(list))
            .style(style::container::field_box)
            .width(Fill);
        let hint = detail.unwrap_or_else(|| {
            "Satıra basınca katmanı Katmanlar'da seçilir, durumunun tamamı burada yazar. Sağdaki düğme durumun istediği tek işi yapar."
                .to_owned()
        });
        let body = Column::new()
            .spacing(12)
            .push(words::summary(lines))
            .push(words::field("Bağlı katmanlar", table, Some(hint)));
        overlay::modal(
            Frame::new(STATUS_TITLE)
                .push(body)
                .action(words::secondary(CLOSE, Some(msg(Event::Close))))
                .action(words::primary(REFRESH, Some(msg(Event::Refresh))))
                .width(900.0),
            msg(Event::Close),
        )
    }

    /// The row of a layer, by its place in the window's table (the pictures and the tests press it).
    #[cfg(test)]
    pub(crate) fn source_status_row(&self, layer: &str) -> Option<usize> {
        let s = self.source_status.as_ref()?.states.as_ref().ok()?;
        s.iter().position(|x| x.link.layer == layer)
    }

    /// A row's way out.
    #[cfg(test)]
    pub(crate) fn source_status_next(&self, i: usize) -> SourceNext {
        self.source_status
            .as_ref()
            .and_then(|w| w.states.as_ref().ok()?.get(i).map(|s| s.next))
            .unwrap_or(SourceNext::None)
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn source_status_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.source_status.is_none() {
            return Err(format!("{STATUS_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(REFRESH) => Some(msg(Event::Refresh)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{STATUS_TITLE}” penceresinde {other} yok")),
        })
    }
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// The source in a few words: the file and its table, or the database's host and the table.
pub fn source_words(s: &SourceState) -> String {
    let place = match s.link.kind {
        SourceKind::Geopackage => format!("{} › {}", file_name(&s.link.path), s.link.table),
        _ => {
            let host = s
                .link
                .path
                .split_once("://")
                .map_or(s.link.path.as_str(), |(_, rest)| rest);
            format!("PostGIS: {host} › {}", s.link.table)
        }
    };
    if s.link.writable {
        place
    } else {
        format!("{place} (salt okunur)")
    }
}

/// What waits, in a few words.
pub fn state_words(s: &SourceState) -> String {
    if !s.found {
        return "Yerinde yok".to_owned();
    }
    if s.rows.is_none() {
        return "Tablo okunamadı".to_owned();
    }
    let differences = s.added + s.modified + s.removed;
    if differences == 0 {
        return "Aynı".to_owned();
    }
    let side = match (s.edited, s.changed) {
        (true, false) => "Katmanda yazılmamış",
        (false, true) => "Dosyada yeni",
        (true, true) => "İki yan da değişmiş",
        (false, false) => "Fark var",
    };
    format!("{side}: {differences} nesne")
}

/// How much of the table the layer holds.
pub fn scope_words(s: &SourceState) -> String {
    match (s.rows, s.total) {
        (Some(rows), Some(total)) if s.link.extent.is_some() => {
            format!("{} nesne · alan {rows} / {total}", s.objects)
        }
        (Some(rows), Some(total)) if rows < total => format!("{} nesne · {rows} / {total}", s.objects),
        (_, Some(total)) => format!("{} nesne · {total} satır", s.objects),
        _ => format!("{} nesne", s.objects),
    }
}

/// The table's system against the project's.
pub fn system_words(s: &SourceState, project: u32) -> String {
    match s.link.srid {
        None => "Tanımsız".to_owned(),
        Some(code) if code == project => format!("EPSG:{code}"),
        Some(code) => format!("EPSG:{code} → proje"),
    }
}

/// The whole state of a row, under the table.
pub fn detail_words(s: &SourceState, project: u32) -> String {
    let kind = match s.link.kind {
        SourceKind::Geopackage => "GeoPackage",
        _ => "PostGIS",
    };
    let mut parts = vec![format!("{kind} {}.", s.link.path)];
    if !s.found {
        parts.push(match s.link.kind {
            SourceKind::Geopackage => "Dosya yerinde yok; nesneler çizimde. Yeniden bağla… ile dosyanın yeni yerini gösterin.".to_owned(),
            _ => "Veritabanı cevap vermedi; adresi ve ağı denetleyin, nesneler çizimde.".to_owned(),
        });
    } else if s.rows.is_none() {
        parts.push(format!("“{}” tablosu okunamadı; dosyanın tablolarından birine yeniden bağlayın.", s.link.table));
    } else {
        if s.added + s.modified + s.removed > 0 {
            parts.push(format!(
                "Tabloda olmayan {}, farklı {}, katmanda olmayan {} nesne.",
                s.added, s.modified, s.removed
            ));
        }
        parts.push(match s.next {
            SourceNext::None => "Katman ile tablo aynı.".to_owned(),
            SourceNext::Write => "Katmandaki değişiklikler yazılmadı: Kaynağa yaz onları tabloya tek işlemde yazar.".to_owned(),
            SourceNext::Read => "Dosya katman onunla eşleştiğinden beri değişti: Kaynaktan oku tablonun hâlini alır.".to_owned(),
            SourceNext::Reconcile => "Katman da dosya da değişti: Uzlaştır… iki yanın değişikliklerini ve çatışmaları gösterir.".to_owned(),
            SourceNext::Relink => String::new(),
        });
        if let Some(e) = s.link.extent {
            parts.push(format!(
                "Çalışma kopyası tablonun bir alanı ({:.0}, {:.0} – {:.0}, {:.0}); alanın dışındaki satırlara dokunulmaz.",
                e[0], e[1], e[2], e[3]
            ));
        }
        if s.link.srid.is_some_and(|c| c != project) {
            parts.push("Tablo kendi sisteminde kalır; okuma ve yazma projenin sistemine çevirerek yapılır.".to_owned());
        }
    }
    parts.retain(|p| !p.is_empty());
    parts.join(" ")
}

/// The states in a sentence.
fn summary_words(states: &[SourceState]) -> String {
    let waiting = states.iter().filter(|s| s.next != SourceNext::None).count();
    let n = states.len();
    match waiting {
        0 => format!("{n} bağlı katmanın hepsi kaynağıyla aynı."),
        w => format!("{n} bağlı katmandan {w} tanesi bir iş bekliyor."),
    }
}
