//! PostGIS'e bağlan (docs/adr/0505, plan/gis.md CBS-04): a database's address (no password in it: PGPASSWORD or
//! ~/.pgpass), its tables listed off the interface's thread (`cad.sources.describe`), the chosen ones bound to new
//! layers with `cad.sources.edit`, a table in another system only with Projenin sistemine çevir ticked.

use iced::widget::{Column, text_input};
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, SourceTable, SourcesDescribe, SourcesDescribed, SourcesEdit, SourcesEditOp,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_ui::style;
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message};

#[derive(Debug, Default)]
pub struct State {
    pub address: String,
    pub tables: Vec<SourceTable>,
    pub chosen: Vec<bool>,
    pub reproject: bool,
    pub listing: bool,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Address(String),
    List,
    Listed(Result<Vec<SourceTable>, String>),
    Toggle(usize),
    Reproject,
    Run,
}

/// An empty drawing for the listing: describing a database reads no drawing.
fn listing_document() -> Result<kentos_domain::Document, String> {
    let empty = r#"{"format":"kentos.document","version":1,"name":"PostGIS","settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},
        "origin":{"x":0,"y":0},"activeLayer":"0","layers":[{"id":"0","name":"0","type":"layer","visible":true,
        "locked":false,"expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.25},"children":[]}],
        "entities":[],"styles":{"items":[],"categories":[]}}"#;
    kentos_commands::DocumentSnapshotV1::from_json(empty)
        .map_err(|e| e.to_string())
        .and_then(kentos_domain::Document::from_snapshot)
}

/// A database's tables through `cad.sources.describe`, or why not.
pub(crate) fn list_tables(address: String) -> Result<Vec<SourceTable>, String> {
    let mut doc = listing_document()?;
    let answer = kentos_native_application::sources::describe::execute(
        &mut ExecutionContext::new(&mut doc),
        SourcesDescribe {
            path: Some(address),
        },
    );
    match answer {
        CommandResult::Completed {
            output: SourcesDescribed { tables, .. },
            ..
        } => Ok(tables),
        CommandResult::Failed { error } | CommandResult::Conflict { error } => Err(error.message),
        _ => Err("tablolar okunamadı".into()),
    }
}

fn event(e: Event) -> Message {
    message(Exchange::Postgis(e))
}

impl App {
    pub(super) fn postgis_open(&mut self) -> Task<Message> {
        self.open_window(Window::Postgis(State::default()));
        Task::none()
    }

    pub(super) fn postgis_event(&mut self, e: Event) -> Task<Message> {
        let Some(Window::Postgis(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Address(a) => {
                s.address = a;
                s.status = None;
            }
            Event::List => {
                s.listing = true;
                s.status = None;
                let address = s.address.trim().to_owned();
                // A document of its own: listing reads no drawing.
                return off_thread(
                    move || list_tables(address),
                    |r| Exchange::Postgis(Event::Listed(r)),
                );
            }
            Event::Listed(r) => {
                s.listing = false;
                match r {
                    Ok(tables) => {
                        s.chosen = vec![false; tables.len()];
                        s.status = tables.is_empty().then(|| {
                            (
                                true,
                                "Veritabanında tam sayı anahtarlı tablo yok.".to_owned(),
                            )
                        });
                        s.tables = tables;
                    }
                    Err(why) => {
                        s.tables.clear();
                        s.chosen.clear();
                        s.status = Some((true, why));
                    }
                }
            }
            Event::Toggle(i) => {
                if let Some(c) = s.chosen.get_mut(i) {
                    *c = !*c;
                }
            }
            Event::Reproject => s.reproject = !s.reproject,
            Event::Run => self.postgis_bind(),
        }
        Task::none()
    }

    fn postgis_bind(&mut self) {
        let Some(Window::Postgis(s)) = &self.exchange else {
            return;
        };
        let address = s.address.trim().to_owned();
        let reproject = s.reproject;
        let tables: Vec<String> = s
            .tables
            .iter()
            .zip(&s.chosen)
            .filter(|(_, c)| **c)
            .map(|(t, _)| t.name.clone())
            .collect();
        let Some(doc) = &mut self.document else {
            return;
        };
        let first = doc.model.next_slot();
        let (mut bound, mut objects, mut warnings) = (Vec::new(), 0u32, Vec::new());
        for table in &tables {
            let input = SourcesEdit {
                edit: SourcesEditOp::Link {
                    extent: None,
                    path: address.clone(),
                    table: table.clone(),
                    layer: None,
                    reproject,
                    key: None,
                },
            };
            match kentos_native_application::sources::edit::execute(
                &mut ExecutionContext::new(&mut doc.model),
                input,
            ) {
                CommandResult::Completed {
                    output,
                    warnings: w,
                } => {
                    objects += output.objects;
                    bound.push(table.clone());
                    warnings.extend(w);
                }
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    if let Some(Window::Postgis(s)) = &mut self.exchange {
                        s.status = Some((
                            true,
                            if bound.is_empty() {
                                error.message
                            } else {
                                format!(
                                    "{} bağlandı; “{table}” bağlanamadı: {}",
                                    bound.join(", "),
                                    error.message
                                )
                            },
                        ));
                    }
                    return;
                }
                _ => return,
            }
        }
        let last = doc.model.next_slot();
        let slots: Vec<Slot> = (first..last)
            .filter_map(|n| u32::try_from(n).ok())
            .map(Slot)
            .filter(|s| doc.model.get(*s).is_some())
            .collect();
        self.close_exchange();
        self.zoom_to(&slots);
        self.announce(
            if warnings.is_empty() { Level::Success } else { Level::Warn },
            format!(
                "{} PostGIS tablosu bağlı katman oldu, {objects} nesne. Değişiklikleri Kaynağa yaz ile veritabanına yazın.",
                bound.len()
            ),
        );
        for w in warnings {
            self.say(Level::Warn, w.message);
        }
    }

    pub(super) fn postgis_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let project = self.document.as_ref().map_or(0, |d| d.settings().srid);
        let address = kentos_ui::widget::focus_ring(
            text_input("postgresql://kullanıcı@sunucu:5432/veritabanı", &s.address)
                .on_input(|t| event(Event::Address(t)))
                .on_submit(event(Event::List))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(style::field::input),
        );
        let mut body = Column::new().spacing(12).push(words::field(
            "Adres",
            address,
            Some("Parola adrese yazılmaz: PGPASSWORD ortam değişkeninden ya da ~/.pgpass dosyasından okunur.".to_owned()),
        ));
        if !s.tables.is_empty() {
            let mut list = Column::new().spacing(6);
            for (i, t) in s.tables.iter().enumerate() {
                let kind = match (&t.geometry, t.srid) {
                    (Some(g), Some(srid)) => format!("{}, EPSG:{srid}", g.to_lowercase()),
                    (Some(g), None) => g.to_lowercase(),
                    _ => "öznitelik tablosu".into(),
                };
                let label = format!(
                    "{} — {kind}, {} satır{}",
                    t.name,
                    t.rows,
                    if t.capabilities.update {
                        ""
                    } else {
                        ", salt okunur"
                    }
                );
                list = list.push(words::check(
                    s.chosen[i],
                    label,
                    Some(event(Event::Toggle(i))),
                ));
            }
            body = body.push(words::field("Tablolar", list, None));
        }
        let other = s
            .tables
            .iter()
            .zip(&s.chosen)
            .any(|(t, c)| *c && t.srid.is_some_and(|srid| srid != project));
        if other {
            body = body.push(words::field(
                "Koordinat sistemi",
                words::check(
                    s.reproject,
                    format!(
                        "Projenin sistemine çevir (EPSG:{project}); tablo kendi sisteminde kalır"
                    ),
                    Some(event(Event::Reproject)),
                ),
                None,
            ));
        }
        if s.listing {
            body = body.push(words::text_line(Line::Info, "Tablolar okunuyor…"));
        }
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        let chosen = s.chosen.iter().filter(|c| **c).count();
        let ready = chosen > 0 && (!other || s.reproject);
        overlay::blocking(
            Dialog::new("PostGIS'e bağlan")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::secondary(
                    "Tabloları listele",
                    (!s.address.trim().is_empty() && !s.listing).then(|| event(Event::List)),
                ))
                .action(words::primary("Bağla", ready.then(|| event(Event::Run))))
                .width(640.0)
                .max_height(720.0),
        )
    }
}
