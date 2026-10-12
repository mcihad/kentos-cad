//! Adres (docs/adr/0229 §13): the bottom panel's Adres tab, Konumdan adres's
//! answers, and the windows Adres verisi and Adres sağlayıcıları.
//!
//! - The tab looks for the address typed in its source: the project's
//!   address data (searched on the address thread, `engine.rs`; the index
//!   built again only when the definition, its layers' objects or their
//!   filters change) or an online provider of the device's (`online.rs`; its
//!   pace, key and cache kept, the request waited for on a thread of its
//!   own). The candidates are rows (Sıra, Adres, Tür, Puan, Mahalle, İlçe,
//!   Kaynak), the best first; a row's click or Git brings the view to it and
//!   marks it over the drawing (not saved), Nokta ekle writes a point there on
//!   the active layer with the match's attributes, Taşı takes the one
//!   selected point there; each is one step.
//! - Konumdan adres (`kentos_interaction::address_reverse`) asks the address
//!   of a place clicked: the answer is the tab's one row, its place marked.
//! - The status line says how many candidates, the best one's status, the
//!   provider's attribution and that the address went to its server.
//!
//! What a found address is and writes is `kentos_processing::address`'s, the
//! same as İşlemler's Adresleri eşle and Konumlara adres yaz. The view is
//! `view.rs`.

pub(crate) mod data;
pub(crate) mod engine;
pub(crate) mod online;
pub(crate) mod providers;
#[cfg(test)]
mod tests;
mod view;

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use iced::Task;
use iced::futures::channel::oneshot;
use kentos_contracts::{
    AddressLocator, CommandResult, EntitiesCreate, EntitiesSetProperties, EntitiesTransform,
    Entity, EntityGeometry, NewObject, PropertiesOperation, Transform, Vec2 as Wire,
};
use kentos_domain::{ChangeMark, Changes, Document};
use kentos_geometry_core::ops::geocode::{Status, parse};
use kentos_interaction::{Format, Level, Vec2};
use kentos_native_application::ExecutionContext;
use kentos_processing::address::{
    AddressHost, LOCATOR, Match, Matched, NO_SYSTEM, PROVIDER, Plane, SourceOption,
    match_attributes, online_matches, online_place, project_sources, provider_sources,
};
use kentos_services::geocode::Provider;
use kentos_services::geocode::read;
use kentos_services::geocode::request::{self, Bias, SEARCH_LIMIT};
use kentos_services::geocode::score::pick;

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::exchange::words::Kind as Say;
use crate::traces::Control;
use engine::{Answer, Asked, Built, Engine, Job, Reply};
use online::online;

/// The search box's id: the keyboard goes to it when the tab opens.
pub(crate) const ADDRESS_FIELD: &str = "address-box";

/// The panel's height Adres ara opens it to when it is shorter (logical pixels).
pub const PANEL_HEIGHT: f64 = 260.0;

/// The score under which an online provider's best candidate is said unmatched (§10, as a batch's).
pub const ONLINE_MIN_SCORE: u32 = kentos_contracts::ADDRESS_MIN_SCORE;

/// The table's columns.
pub const COLUMNS: [&str; 7] = ["Sıra", "Adres", "Tür", "Puan", "Mahalle", "İlçe", "Kaynak"];

/// The tab's words.
pub mod texts {
    pub const SEARCH: &str = "Adres ara";
    pub const PLACEHOLDER: &str = "Adres: Kızılay Mah. Atatürk Blv. No:12 Çankaya/Ankara";
    pub const SEARCH_HINT: &str = "Mahalle, yol, kapı no, ilçe ve il tek satırda; kısaltmalar (Mah., Cad., Sk., Blv., No:) da okunur. Adres kodu (UAVT) da yazılabilir. Enter ya da Ara arar.";
    pub const FIND: &str = "Ara";
    pub const SOURCE: &str = "Kaynak";
    pub const REVERSE: &str = "Konumdan adres";
    pub const REVERSE_HINT: &str =
        "Çizimde tıklanan yerin adresini bu kaynaktan bulur; Esc'e dek sürer";
    pub const GO: &str = "Git";
    pub const GO_HINT: &str = "Görünümü seçili adaya getirir ve çizimde işaretler (kaydedilmez)";
    pub const ADD: &str = "Nokta ekle";
    pub const ADD_HINT: &str =
        "Seçili adayın yerine etkin katmana adres öznitelikleriyle bir nokta yazar";
    pub const MOVE: &str = "Taşı";
    pub const MOVE_HINT: &str =
        "Seçili tek noktayı adayın yerine götürür ve adres özniteliklerini yazar";
    pub const DATA: &str = "Adres verisi…";
    pub const PROVIDERS: &str = "Adres sağlayıcıları…";
    pub const NO_TEXT: &str = "Aranacak adresi yazın.";
    pub const NO_SOURCE: &str = "Adres kaynağı yok: projenin adres verisini Adres verisi penceresinden tanımlayın ya da Adres sağlayıcıları'ndan bir çevrimiçi sağlayıcıyı açın.";
    pub const SEARCHING: &str = "Aranıyor…";
    pub const NOT_FOUND: &str =
        "Aday yok: yazımı, kaynağı ve adres verisinin alanlarını denetleyin.";
    pub const NO_PLACE: &str = "Bu yerin yakınında adres yok.";
    pub const EMPTY: &str =
        "Bir adres yazıp Ara'ya basın ya da Konumdan adres ile çizimde bir yere tıklayın.";
    pub const BY_HAND: &str = "Elle seçildi";
    pub const PLACE: &str = "Tıklanan yerin adresi";
    /// The table's Puan column for a place's address.
    pub const DISTANCE: &str = "Uzaklık";
    pub const NOT_FOR_PLACE: &str = "Konumdan adres'in sonucu bir yerin adresidir; Nokta ekle ve Taşı aranan adreste çalışır. Nesnelere adres yazmak için Konumlara adres yaz'ı kullanın.";

    /// The status line's count.
    pub fn count(n: usize) -> String {
        format!("{n} aday")
    }

    /// Where an online address went (§10: said every time).
    pub fn sent(host: &str) -> String {
        format!("Adres {host} sunucusuna gönderildi")
    }

    /// Where an online place went.
    pub fn place_sent(host: &str) -> String {
        format!("Konum {host} sunucusuna gönderildi")
    }

    /// Why a provider's results are not written (§10).
    pub fn not_kept(name: &str) -> String {
        format!("{name} sonuçlarının saklanmasına izin vermez: Nokta ekle ve Taşı kapalıdır.")
    }
}

/// What the rows answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Question {
    /// An address typed.
    Address(String),
    /// A place clicked (Konumdan adres).
    Place(Vec2),
}

/// Who answered: the source's name, its attribution and the server the question went to.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Answerer {
    pub name: String,
    pub attribution: String,
    pub host: Option<String>,
    /// Why its results may not be written into the drawing (Google's, ArcGIS's without storing); none when they may.
    pub keeps_none: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// The box's words.
    Text(String),
    /// Kaynak's choice, by its id.
    Source(String),
    /// Ara, or Enter in the box.
    Find,
    /// The Konumdan adres button: the tool starts.
    Reverse,
    /// An answer from the address thread or a provider.
    Answered(Reply),
    /// A locator was built.
    Built(Built),
    /// A row pressed: it becomes the chosen one and the view goes to it.
    Press(usize),
    Go,
    AddPoint,
    MovePoint,
    OpenData,
    OpenProviders,
    /// Adres verisi's window.
    Data(data::Event),
    /// Adres sağlayıcıları's window.
    Providers(providers::Event),
}

/// What a locator was last built from: its definition, the drawing's change mark, the objects it read and its
/// layers' filters.
struct Stamp {
    def: AddressLocator,
    mark: ChangeMark,
    members: HashSet<u32>,
    filters: Vec<Option<kentos_contracts::LayerFilter>>,
}

/// The tab's state and the windows', kept for as long as the app lives.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) text: String,
    /// The source chosen, by id; none: the first that can be asked.
    pub(crate) source: Option<String>,
    pub(crate) rows: Vec<Match>,
    /// The best row's status (an address looked for).
    pub(crate) status: Option<Status>,
    pub(crate) question: Option<Question>,
    pub(crate) current: Option<usize>,
    waiting: Option<u64>,
    tickets: u64,
    pub(crate) said: Option<(Say, String)>,
    pub(crate) answerer: Option<Answerer>,
    /// The place marked and the drawing it is in (its session), with its words.
    mark: Option<(u64, Vec2, String)>,
    /// The tab opened: the keyboard goes to the box once.
    pub(crate) focus: bool,
    engine: Engine,
    built: HashMap<String, Stamp>,
    /// Konumdan adres's places asked during the update.
    pub(crate) wanted: Vec<Vec2>,
    pub(crate) data: Option<data::Window>,
    pub(crate) providers: Option<providers::Window>,
}

fn filters_of(def: &AddressLocator, doc: &Document) -> Vec<Option<kentos_contracts::LayerFilter>> {
    def.layers()
        .iter()
        .map(|l| doc.layers().get(l).and_then(|n| n.filter.clone()))
        .collect()
}

impl State {
    /// Whether locator `key` is built as `def` says over the drawing as it is now.
    fn fresh(&self, key: &str, def: &AddressLocator, doc: &Document) -> bool {
        let Some(s) = self.built.get(key) else {
            return false;
        };
        if s.def != *def || s.filters != filters_of(def, doc) {
            return false;
        }
        let layers: HashSet<&str> = def.layers().into_iter().collect();
        match doc.changes_since(s.mark) {
            Changes::All => false,
            Changes::Slots(slots) => slots.iter().all(|slot| {
                !s.members.contains(&slot.0)
                    && doc
                        .get(*slot)
                        .is_none_or(|e| !layers.contains(e.base().layer_id.as_str()))
            }),
        }
    }

    fn next_ticket(&mut self) -> u64 {
        self.tickets += 1;
        self.tickets
    }
}

fn failed(ticket: u64) -> Reply {
    Reply {
        ticket,
        answer: Answer::Failed("Adreslerin iş parçacığı yanıt vermedi.".into()),
    }
}

impl App {
    /// The sources Kaynak lists: the project's address data, then the device's providers, each that cannot be
    /// asked with why.
    pub(crate) fn address_sources(&self) -> Vec<SourceOption> {
        let mut out = self
            .document
            .as_ref()
            .map(|d| project_sources(&d.model))
            .unwrap_or_default();
        let o = online();
        let providers = o.providers();
        let mut list = provider_sources(&providers);
        for (s, p) in list.iter_mut().zip(&providers) {
            if s.why.is_none() {
                s.why = o.why_not(p);
            }
        }
        out.extend(list);
        out
    }

    /// The source asked: the one chosen when it can be asked, else the first that can.
    pub(crate) fn address_source(&self) -> Option<SourceOption> {
        let all = self.address_sources();
        self.address
            .source
            .as_ref()
            .and_then(|id| all.iter().find(|s| &s.id == id && s.why.is_none()).cloned())
            .or_else(|| all.into_iter().find(|s| s.why.is_none()))
    }

    /// `address.search`: the panel open on its Adres tab, grown when it is short, the keyboard in the box; on it
    /// already, the panel closes.
    pub(crate) fn open_address_search(&mut self) -> Task<Message> {
        if self.command_expanded && self.bottom_tab == BottomTab::Address {
            self.toggle_bottom();
            return Task::none();
        }
        self.show_bottom(BottomTab::Address);
        let bar = f64::from(kentos_ui::widget::tabs::height());
        if f64::from(self.bottom_log()) + bar < PANEL_HEIGHT {
            self.bottom_dragged(Some((PANEL_HEIGHT - bar) as f32), Instant::now());
        }
        self.address.focus = false;
        let id = iced::widget::Id::new(ADDRESS_FIELD);
        Task::batch([
            iced::widget::operation::focus(id.clone()),
            iced::widget::operation::select_all(id),
        ])
    }

    pub(crate) fn address_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Text(t) => self.address.text = t,
            Event::Source(id) => {
                self.address.source = Some(id);
                self.address.said = None;
            }
            Event::Find => return self.address_find(),
            Event::Reverse => return self.run("address.reverse"),
            Event::Answered(reply) => self.address_answered(reply),
            Event::Built(b) => {
                if b.key.starts_with('#') {
                    return Task::none();
                }
                if let Some(w) = self.address.data.as_mut() {
                    w.built(&b);
                }
            }
            Event::Press(i) => {
                if i < self.address.rows.len() {
                    self.address.current = Some(i);
                    self.address_go();
                }
            }
            Event::Go => self.address_go(),
            Event::AddPoint => self.address_add_point(),
            Event::MovePoint => self.address_move_point(),
            Event::OpenData => return self.open_address_data(),
            Event::OpenProviders => return self.open_address_providers(),
            Event::Data(e) => return self.address_data_event(e),
            Event::Providers(e) => return self.address_providers_event(e),
        }
        Task::none()
    }

    /// Builds locator `key` as `def` says when it is not built so already: the drawing read here, the index on
    /// the address thread.
    pub(crate) fn address_build(&mut self, key: &str, def: &AddressLocator) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let model = &doc.model;
        if self.address.fresh(key, def, model) {
            return Task::none();
        }
        let input = kentos_native_application::addresses::input(model, def);
        let members: HashSet<u32> = def
            .layers()
            .iter()
            .flat_map(|l| model.by_layer(l).map(|e| e.base().id))
            .collect();
        self.address.built.insert(
            key.to_owned(),
            Stamp {
                def: def.clone(),
                mark: model.change_mark(),
                members,
                filters: filters_of(def, model),
            },
        );
        let (done, wait) = oneshot::channel();
        self.address.engine.send(Job::Build {
            key: key.to_owned(),
            input: Box::new(input),
            done,
        });
        Task::perform(wait, |r| {
            Message::Address(Event::Built(r.unwrap_or(Built {
                key: String::new(),
                doors: 0,
                millis: 0.0,
            })))
        })
    }

    /// Sends a question to the address thread, its locator built first when it must be.
    pub(crate) fn address_ask(
        &mut self,
        key: &str,
        def: &AddressLocator,
        job: impl FnOnce(oneshot::Sender<Reply>) -> Job,
        ticket: u64,
    ) -> Task<Message> {
        let build = self.address_build(key, def);
        let (done, wait) = oneshot::channel();
        self.address.engine.send(job(done));
        Task::batch([
            build,
            Task::perform(wait, move |r| {
                Message::Address(Event::Answered(r.unwrap_or_else(|_| failed(ticket))))
            }),
        ])
    }

    /// The view's middle and box in WGS 84, a provider's search's bias.
    fn address_bias(&self, plane: &Plane) -> Option<Bias> {
        let b = self.viewport.camera.visible_bounds();
        let c = Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
        let (lon, lat) = plane.to_wgs84(c)?;
        let (west, south) = plane.to_wgs84(Vec2::new(b.min_x, b.min_y))?;
        let (east, north) = plane.to_wgs84(Vec2::new(b.max_x, b.max_y))?;
        Some(Bias {
            lon,
            lat,
            west,
            south,
            east,
            north,
        })
    }

    /// The provider of a source, with the project's plane; why not when it cannot be asked.
    fn address_provider(&self, id: &str) -> Result<(Provider, Plane), String> {
        let p = online()
            .providers()
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("“{id}” sağlayıcısı bu cihazda yok."))?;
        let plane = self
            .document
            .as_ref()
            .and_then(|d| Plane::of(&d.model))
            .ok_or_else(|| NO_SYSTEM.to_owned())?;
        Ok((p, plane))
    }

    /// Ara: the address typed, looked for in the source.
    fn address_find(&mut self) -> Task<Message> {
        let text = self.address.text.trim().to_owned();
        if text.is_empty() {
            self.address.said = Some((Say::Warn, texts::NO_TEXT.into()));
            return Task::none();
        }
        let Some(source) = self.address_source() else {
            self.address.said = Some((Say::Warn, texts::NO_SOURCE.into()));
            return Task::none();
        };
        let ticket = self.address.next_ticket();
        self.address.waiting = Some(ticket);
        self.address.question = Some(Question::Address(text.clone()));
        self.address.said = Some((Say::Info, texts::SEARCHING.into()));
        if let Some(id) = source.id.strip_prefix(LOCATOR) {
            let Some(def) = self
                .document
                .as_ref()
                .and_then(|d| kentos_native_application::addresses::locator(&d.model, id).cloned())
            else {
                return Task::none();
            };
            self.address.answerer = Some(Answerer {
                name: def.name.clone(),
                ..Answerer::default()
            });
            let key = id.to_owned();
            let min_score = def.min_score;
            return self.address_ask(
                id,
                &def,
                move |done| Job::Find {
                    key,
                    asked: Asked::Line(text),
                    min_score,
                    ticket,
                    done,
                },
                ticket,
            );
        }
        let id = source
            .id
            .strip_prefix(PROVIDER)
            .unwrap_or_default()
            .to_owned();
        let (p, plane) = match self.address_provider(&id) {
            Ok(x) => x,
            Err(why) => {
                self.address.waiting = None;
                self.address.said = Some((Say::Warn, why));
                return Task::none();
            }
        };
        let bias = self.address_bias(&plane);
        self.address.answerer = Some(Answerer {
            name: p.name.clone(),
            attribution: p.attribution.clone(),
            host: Some(p.host()),
            keeps_none: (!p.may_store()).then(|| texts::not_kept(&p.name)),
        });
        let req = request::forward(&p, &text, SEARCH_LIMIT, bias.as_ref());
        let (done, wait) = oneshot::channel();
        let asked = std::thread::Builder::new()
            .name("KentOS adres sağlayıcısı".into())
            .spawn(move || {
                let answer = online()
                    .ask(&p, &req, &|| false)
                    .and_then(|body| read::forward(p.kind, &body))
                    .map_or_else(Answer::Failed, |r| {
                        let q = parse(&text);
                        let status =
                            pick(&q, &r.candidates, ONLINE_MIN_SCORE).map(|(_, _, _, s)| s);
                        Answer::Found {
                            rows: online_matches(&q, &r.candidates, &plane),
                            status,
                        }
                    });
                let _ = done.send(Reply { ticket, answer });
            });
        if asked.is_err() {
            self.address.waiting = None;
            self.address.said = Some((
                Say::Error,
                "Sağlayıcıya sorulamadı: iş parçacığı açılmadı.".into(),
            ));
            return Task::none();
        }
        Task::perform(wait, move |r| {
            Message::Address(Event::Answered(r.unwrap_or_else(|_| failed(ticket))))
        })
    }

    /// Konumdan adres's place: its address looked for in the source, the tab open on it.
    pub(crate) fn address_at(&mut self, at: Vec2) -> Task<Message> {
        if !(self.command_expanded && self.bottom_tab == BottomTab::Address) {
            self.show_bottom(BottomTab::Address);
            let bar = f64::from(kentos_ui::widget::tabs::height());
            if f64::from(self.bottom_log()) + bar < PANEL_HEIGHT {
                self.bottom_dragged(Some((PANEL_HEIGHT - bar) as f32), Instant::now());
            }
        }
        let Some(source) = self.address_source() else {
            self.address.said = Some((Say::Warn, texts::NO_SOURCE.into()));
            return Task::none();
        };
        let ticket = self.address.next_ticket();
        self.address.waiting = Some(ticket);
        self.address.question = Some(Question::Place(at));
        self.address.said = Some((Say::Info, texts::SEARCHING.into()));
        if let Some(id) = source.id.strip_prefix(LOCATOR) {
            let Some(def) = self
                .document
                .as_ref()
                .and_then(|d| kentos_native_application::addresses::locator(&d.model, id).cloned())
            else {
                return Task::none();
            };
            self.address.answerer = Some(Answerer {
                name: def.name.clone(),
                ..Answerer::default()
            });
            let key = id.to_owned();
            let reach = def.reach;
            return self.address_ask(
                id,
                &def,
                move |done| Job::Reverse {
                    key,
                    at,
                    reach,
                    ticket,
                    done,
                },
                ticket,
            );
        }
        let id = source
            .id
            .strip_prefix(PROVIDER)
            .unwrap_or_default()
            .to_owned();
        let (p, plane) = match self.address_provider(&id) {
            Ok(x) => x,
            Err(why) => {
                self.address.waiting = None;
                self.address.said = Some((Say::Warn, why));
                return Task::none();
            }
        };
        let Some((lon, lat)) = plane.to_wgs84(at) else {
            self.address.waiting = None;
            self.address.said = Some((Say::Warn, "Bu yer WGS 84'e çevrilemedi.".into()));
            return Task::none();
        };
        self.address.answerer = Some(Answerer {
            name: p.name.clone(),
            attribution: p.attribution.clone(),
            host: Some(p.host()),
            keeps_none: (!p.may_store()).then(|| texts::not_kept(&p.name)),
        });
        let per_metre = self.document.as_ref().map_or(1.0, |d| {
            kentos_native_application::addresses::per_metre(&d.model)
        });
        let req = request::reverse(&p, lon, lat);
        let (done, wait) = oneshot::channel();
        let asked = std::thread::Builder::new()
            .name("KentOS adres sağlayıcısı".into())
            .spawn(move || {
                let answer = online()
                    .ask(&p, &req, &|| false)
                    .and_then(|body| read::reverse(p.kind, &body))
                    .map_or_else(Answer::Failed, |c| {
                        Answer::Place(
                            c.and_then(|c| online_place(&c, &plane, at, per_metre))
                                .map(Box::new),
                        )
                    });
                let _ = done.send(Reply { ticket, answer });
            });
        if asked.is_err() {
            self.address.waiting = None;
            self.address.said = Some((
                Say::Error,
                "Sağlayıcıya sorulamadı: iş parçacığı açılmadı.".into(),
            ));
            return Task::none();
        }
        Task::perform(wait, move |r| {
            Message::Address(Event::Answered(r.unwrap_or_else(|_| failed(ticket))))
        })
    }

    /// The tool's places, asked after the update.
    pub(crate) fn address_tasks(&mut self) -> Task<Message> {
        let asked = std::mem::take(&mut self.address.wanted);
        Task::batch(
            asked
                .into_iter()
                .map(|p| self.address_at(p))
                .collect::<Vec<_>>(),
        )
    }

    fn address_answered(&mut self, reply: Reply) {
        if self
            .address
            .data
            .as_ref()
            .is_some_and(|w| w.waits_for(reply.ticket))
        {
            if let Some(w) = self.address.data.as_mut() {
                w.answered(reply);
            }
            return;
        }
        if self.address.waiting != Some(reply.ticket) {
            // A later question was asked: this answer is old.
            return;
        }
        self.address.waiting = None;
        let place = matches!(self.address.question, Some(Question::Place(_)));
        match reply.answer {
            Answer::Found { rows, status } => {
                self.address.status = status;
                self.address.current = (!rows.is_empty()).then_some(0);
                self.address.said = rows
                    .is_empty()
                    .then(|| (Say::Warn, texts::NOT_FOUND.to_owned()));
                self.address.rows = rows;
            }
            Answer::Place(found) => {
                self.address.status = None;
                self.address.said = found
                    .is_none()
                    .then(|| (Say::Warn, texts::NO_PLACE.to_owned()));
                self.address.rows = found.into_iter().map(|m| *m).collect();
                self.address.current = (!self.address.rows.is_empty()).then_some(0);
            }
            Answer::Checked(_) => {}
            Answer::Failed(why) => {
                self.address.rows.clear();
                self.address.current = None;
                self.address.status = None;
                self.address.said = Some((Say::Error, why));
            }
        }
        // A place's address is marked where it is; an address looked for, when Git or a row asks.
        if place {
            self.address_mark_current();
        }
    }

    /// The chosen row marked over the drawing.
    fn address_mark_current(&mut self) {
        let (Some(m), Some(session)) = (
            self.address.current.and_then(|i| self.address.rows.get(i)),
            self.document.as_ref().map(|d| d.session),
        ) else {
            return;
        };
        self.address.mark = Some((session, m.at, m.label.clone()));
        self.search_unmark();
    }

    /// Git: the view comes to the chosen row, which is marked.
    fn address_go(&mut self) {
        let Some(m) = self
            .address
            .current
            .and_then(|i| self.address.rows.get(i))
            .cloned()
        else {
            return;
        };
        self.address_mark_current();
        self.navigating(|app| app.viewport.camera.center_on(m.at));
    }

    /// The place marked, as the drawing shows it.
    pub(crate) fn address_mark_label(&self) -> Option<(Vec2, String)> {
        let doc = self.document.as_ref()?;
        self.address
            .mark
            .as_ref()
            .filter(|(session, _, _)| *session == doc.session)
            .map(|(_, p, words)| (*p, words.clone()))
    }

    /// İşareti kaldır, and a new drawing: the mark goes.
    pub(crate) fn address_unmark(&mut self) {
        self.address.mark = None;
    }

    /// What a point written at the chosen row says of it; none for a place's address.
    fn address_attributes(
        &self,
        base: &std::collections::BTreeMap<String, String>,
    ) -> Result<(Match, std::collections::BTreeMap<String, String>), String> {
        let i = self.address.current.ok_or("Bir aday seçin.")?;
        let m = self.address.rows.get(i).cloned().ok_or("Bir aday seçin.")?;
        let Some(Question::Address(input)) = &self.address.question else {
            return Err(texts::NOT_FOR_PLACE.into());
        };
        let answerer = self.address.answerer.clone().unwrap_or_default();
        if let Some(why) = &answerer.keeps_none {
            return Err(why.clone());
        }
        let status = match (i, self.address.status) {
            (0, Some(s)) => s.word(),
            _ => texts::BY_HAND,
        };
        let mut attrs = match_attributes(
            base,
            &Matched {
                status: self.address.status.unwrap_or(Status::Matched),
                found: &m,
                source: &answerer.name,
                attribution: &answerer.attribution,
                input,
            },
        );
        attrs.insert("Durum".into(), status.into());
        Ok((m, attrs))
    }

    /// Nokta ekle: a point at the chosen row on the active layer, with the match's attributes; one step.
    fn address_add_point(&mut self) {
        let (m, attrs) = match self.address_attributes(&Default::default()) {
            Ok(x) => x,
            Err(why) => {
                self.address.said = Some((Say::Warn, why));
                return;
            }
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let layer = doc.model.layers().active().to_owned();
        let input = EntitiesCreate {
            layer_id: layer,
            objects: vec![NewObject {
                geometry: EntityGeometry::Point {
                    p: Wire {
                        x: m.at.x,
                        y: m.at.y,
                    },
                    z: None,
                    parts: None,
                },
                color: None,
                line_weight: None,
                attrs: Some(attrs),
                label: None,
                symbol: None,
                label_of: None,
                label_scale: None,
            }],
            operation: None,
            expected_revision: None,
        };
        let model = &mut doc.model;
        let step = model.begin_group(texts::ADD);
        match kentos_native_application::create::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed { warnings, .. } => {
                model.end_group(step);
                for w in warnings {
                    self.warn(w.message);
                }
                self.say(
                    Level::Success,
                    format!("Adres noktası yazıldı: {}.", m.label),
                );
            }
            other => {
                model.cancel_group(step);
                self.address.said = Some((Say::Error, refusal(other, "Nokta yazılamadı.")));
            }
        }
    }

    /// Taşı: the one point selected goes to the chosen row, its address attributes written; one step.
    fn address_move_point(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let point = match self.selection.ids() {
            [slot] => doc.model.get(*slot).and_then(|e| match e {
                Entity::Point(p) => Some((*slot, p.p, e.base().attrs.clone())),
                _ => None,
            }),
            _ => None,
        };
        let Some((slot, from, base)) = point else {
            self.address.said = Some((Say::Warn, "Taşınacak tek bir nokta seçin.".into()));
            return;
        };
        let (m, attrs) = match self.address_attributes(&base) {
            Ok(x) => x,
            Err(why) => {
                self.address.said = Some((Say::Warn, why));
                return;
            }
        };
        let Some(uid) = doc.model.uid(slot).map(|u| u.to_string()) else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let model = &mut doc.model;
        let step = model.begin_group(texts::MOVE);
        let moved = kentos_native_application::transform::execute(
            &mut ExecutionContext::new(model),
            EntitiesTransform {
                uids: vec![uid.clone()],
                transform: Transform::Move {
                    dx: m.at.x - from.x,
                    dy: m.at.y - from.y,
                },
                copy: None,
                expected_revision: None,
            },
        );
        if !matches!(moved, CommandResult::Completed { .. }) {
            model.cancel_group(step);
            self.address.said = Some((Say::Error, refusal(moved, "Nokta taşınamadı.")));
            return;
        }
        let set = kentos_native_application::set::execute(
            &mut ExecutionContext::new(model),
            EntitiesSetProperties {
                uids: vec![uid],
                layer_id: None,
                color: None,
                line_weight: None,
                symbol: None,
                attrs: Some(attrs.into_iter().map(|(k, v)| (k, Some(v))).collect()),
                label: None,
                attachments: None,
                unlink: false,
                operation: PropertiesOperation::Attributes,
                expected_revision: None,
            },
        );
        match set {
            CommandResult::Completed { .. } => {
                model.end_group(step);
                self.say(
                    Level::Success,
                    format!("Nokta adresine taşındı: {}.", m.label),
                );
            }
            other => {
                model.cancel_group(step);
                self.address.said = Some((Say::Error, refusal(other, "Nokta taşınamadı.")));
            }
        }
    }

    /// A new drawing: what was built and found for the last one goes.
    pub(crate) fn address_reset(&mut self) {
        for key in std::mem::take(&mut self.address.built).into_keys() {
            self.address.engine.send(Job::Drop(key));
        }
        self.address.rows.clear();
        self.address.current = None;
        self.address.status = None;
        self.address.question = None;
        self.address.waiting = None;
        self.address.said = None;
        self.address.mark = None;
    }

    /// The device learns whether a Google basemap is shown (Google's results only beside its map).
    pub(crate) fn follow_address(&self) {
        let shown = self.document.as_ref().is_some_and(|d| {
            crate::services::overlay::shown(&d.model)
                .iter()
                .any(|(_, s)| s.kind == kentos_contracts::ServiceKind::Google)
        });
        online().set_google_shown(shown);
    }

    /// A row's words as the table shows them: Adres, Tür, Puan (a place's distance), Mahalle, İlçe, Kaynak.
    pub(crate) fn address_cells(&self, m: &Match, format: &Format) -> [String; 6] {
        let points = match (m.score, m.distance) {
            (Some(s), _) => s.to_string(),
            (None, Some(d)) => format.length(d),
            (None, None) => "—".to_owned(),
        };
        let source = self
            .address
            .answerer
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default();
        [
            m.label.clone(),
            m.kind.clone(),
            points,
            m.neighbourhood().to_owned(),
            m.district().to_owned(),
            source,
        ]
    }

    /// The status line: how many, the best one's status, the attribution and where the address went.
    pub(crate) fn address_status_line(&self) -> String {
        let a = &self.address;
        if a.waiting.is_some() {
            return texts::SEARCHING.to_owned();
        }
        if a.question.is_none() {
            return String::new();
        }
        let mut parts: Vec<String> = Vec::new();
        if matches!(a.question, Some(Question::Place(_))) && !a.rows.is_empty() {
            parts.push(texts::PLACE.to_owned());
        }
        if matches!(a.question, Some(Question::Address(_))) {
            parts.push(texts::count(a.rows.len()));
            if let (Some(s), Some(best)) = (a.status, a.rows.first()) {
                let points = best.score.map(|p| format!(" ({p})")).unwrap_or_default();
                parts.push(format!("En iyisi: {}{points}", s.word()));
            }
        }
        if let Some(who) = &a.answerer {
            if !who.attribution.is_empty() {
                parts.push(who.attribution.clone());
            }
            if let Some(host) = &who.host {
                parts.push(match a.question {
                    Some(Question::Place(_)) => texts::place_sent(host),
                    _ => texts::sent(host),
                });
            }
        }
        parts.join(" · ")
    }

    /// The tab's controls by their words (a trace's `panel` step).
    pub(crate) fn address_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let msg = |e| Some(Message::Address(e));
        Ok(match control {
            Control::Fill(texts::SEARCH, t) => msg(Event::Text(t.to_owned())),
            Control::Press(texts::FIND) | Control::Key("Enter") => msg(Event::Find),
            Control::Press(texts::REVERSE) => msg(Event::Reverse),
            Control::Press(texts::GO) => self.address.current.map(|_| Message::Address(Event::Go)),
            Control::Press(texts::ADD) => self
                .address
                .current
                .map(|_| Message::Address(Event::AddPoint)),
            Control::Press(texts::MOVE) => self
                .address
                .current
                .map(|_| Message::Address(Event::MovePoint)),
            Control::Press(texts::DATA) => msg(Event::OpenData),
            Control::Press(texts::PROVIDERS) => msg(Event::OpenProviders),
            Control::Pick(texts::SOURCE, item) => {
                let s = self
                    .address_sources()
                    .into_iter()
                    .find(|s| s.name == item)
                    .ok_or_else(|| format!("“{item}” öğesi (Kaynak) yok"))?;
                if let Some(why) = s.why {
                    return Err(format!("“{item}” seçilemez: {why}"));
                }
                msg(Event::Source(s.id))
            }
            Control::Row(n) => {
                if n == 0 || n > self.address.rows.len() {
                    return Err(format!("{n}. aday satırı yok"));
                }
                msg(Event::Press(n - 1))
            }
            other => return Err(format!("“Adres” sekmesinde {other} yok")),
        })
    }

    /// What a trace's `address` expectation reads: the status line and the rows (Adres, Tür, Puan, Mahalle, İlçe,
    /// joined by “ | ”), as the tab shows them.
    pub(crate) fn address_seen(&self) -> Option<(String, Vec<String>)> {
        if !self.command_expanded || self.bottom_tab != BottomTab::Address {
            return None;
        }
        let format = self.format();
        let rows = self
            .address
            .rows
            .iter()
            .map(|m| {
                let c = self.address_cells(m, &format);
                c[..5].join(" | ")
            })
            .collect();
        // What the tab's status line shows: what was said, else the answer's line.
        let line = self
            .address
            .said
            .as_ref()
            .map_or_else(|| self.address_status_line(), |(_, t)| t.clone());
        Some((line, rows))
    }
}

/// A refusal's words.
fn refusal<T>(result: CommandResult<T>, otherwise: &str) -> String {
    match result {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => error.message,
        _ => otherwise.to_owned(),
    }
}
