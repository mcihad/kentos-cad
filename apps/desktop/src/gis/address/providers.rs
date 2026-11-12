//! Adres sağlayıcıları (`address.providers`, docs/adr/0229 §10, §13, §16):
//! the ready online providers (Nominatim, Photon, ArcGIS World, Google) and
//! the servers of one's own (Sunucu ekle, Sil) on the left; the chosen
//! one's card on the right: what is sent and to whom, its limits, whether its
//! results may be written into a drawing, its cache and its attribution, its
//! terms and privacy links, the box that accepts the card's version and
//! Kullan; its key (hidden, shown on demand), its country, ArcGIS's
//! “Sonuçları sakla”, a server's kind, address and pace. Dene asks one
//! address and says how long it took; Önbelleği temizle says how many
//! answers went. Kaydet writes the device's file and keys; nothing goes
//! into a drawing.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use iced::futures::channel::oneshot;
use iced::widget::{Column, button, column, container, row, scrollable};
use iced::{Center, Element, Fill, Length, Task};
use kentos_cloud::fetch::{self, Method, Request};
use kentos_services::geocode::providers::{
    KeyRule, Provider, ProviderSetting, Providers, StoreRule, table,
};
use kentos_services::geocode::read;
use kentos_services::geocode::request;
use kentos_services::query::encode;
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use kentos_processing::address::AddressHost;

use super::Event as Address;
use super::online::online;
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind as Say};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title.
pub const TITLE: &str = "Adres sağlayıcıları";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const ADD: &str = "Sunucu ekle";
const REMOVE: &str = "Sil";
const TRY: &str = "Dene";
const CLEAR: &str = "Önbelleği temizle";
const USE: &str = "Kullan";
/// The address Dene asks.
const TRY_ADDRESS: &str = "Kızılay, Çankaya, Ankara";

#[derive(Clone, Debug)]
pub struct Window {
    /// The device's settings when the window opened: Kaydet waits for a change.
    opened: Providers,
    settings: Providers,
    /// Keys as typed, by provider id; a provider not here keeps its own.
    keys: HashMap<String, String>,
    shown: HashSet<String>,
    /// Servers' pace as typed: requests a second and at once.
    paces: HashMap<String, (String, String)>,
    chosen: usize,
    said: Option<(Say, String)>,
    trying: Option<u64>,
    tried: Option<(Say, String)>,
    tickets: u64,
    cache: (usize, u64),
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(usize),
    Accept(bool),
    Use(bool),
    Key(String),
    ShowKey,
    Country(String),
    Store(bool),
    AddServer,
    RemoveServer,
    ServerKind(usize),
    ServerName(String),
    ServerUrl(String),
    ServerRate(String),
    ServerParallel(String),
    Try,
    Tried(u64, Result<String, String>),
    Link(String),
    ClearCache,
    Save,
    Cancel,
}

fn msg(e: Event) -> Message {
    Message::Address(Address::Providers(e))
}

/// Now as RFC 3339, UTC (when the card was accepted).
fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

/// Bytes in the user's words.
fn size_words(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.0} kB", (bytes as f64 / 1024.0).ceil())
    }
}

impl Window {
    fn providers(&self) -> Vec<Provider> {
        self.settings.resolved()
    }

    /// The chosen provider's setting, made when it has none yet.
    fn setting(&mut self, id: &str) -> &mut ProviderSetting {
        if let Some(i) = self.settings.providers.iter().position(|p| p.id == id) {
            return &mut self.settings.providers[i];
        }
        self.settings.providers.push(ProviderSetting {
            id: id.to_owned(),
            ..ProviderSetting::default()
        });
        let last = self.settings.providers.len() - 1;
        &mut self.settings.providers[last]
    }

    /// A provider's key as the window has it: as typed, else the device's.
    fn key(&self, id: &str) -> String {
        self.keys
            .get(id)
            .cloned()
            .or_else(|| online().key(id))
            .unwrap_or_default()
    }

    /// What is wrong with the window's settings; none when they hold.
    fn problem(&self) -> Option<String> {
        for p in &self.settings.providers {
            if let Some(why) = p.problem() {
                return Some(why);
            }
        }
        for (id, (rate, parallel)) in &self.paces {
            let name = self
                .settings
                .providers
                .iter()
                .find(|p| &p.id == id)
                .and_then(|p| p.name.clone())
                .unwrap_or_else(|| id.clone());
            let t = table();
            let r = rate.trim().replace(',', ".").parse::<f64>().ok();
            if !r.is_some_and(|r| r >= t.servers.rates.0 && r <= t.servers.rates.1) {
                return Some(format!(
                    "“{name}” sunucusunun hızı saniyede {} ile {} istek arasında olmalı.",
                    t.servers.rates.0, t.servers.rates.1
                ));
            }
            let n = parallel.trim().parse::<u32>().ok();
            if !n.is_some_and(|n| (t.servers.parallels.0..=t.servers.parallels.1).contains(&n)) {
                return Some(format!(
                    "“{name}” sunucusuna aynı anda {} ile {} istek gider.",
                    t.servers.parallels.0, t.servers.parallels.1
                ));
            }
        }
        None
    }

    /// The settings with the servers' paces as typed.
    fn written(&self) -> Providers {
        let mut s = self.settings.clone();
        for p in &mut s.providers {
            if let Some((rate, parallel)) = self.paces.get(&p.id) {
                p.rate = rate.trim().replace(',', ".").parse::<f64>().ok();
                p.parallel = parallel.trim().parse::<u32>().ok();
            }
        }
        s
    }
}

impl App {
    /// `address.providers`: the window.
    pub(crate) fn open_address_providers(&mut self) -> Task<Message> {
        let o = online();
        let settings = o.settings();
        let paces = settings
            .providers
            .iter()
            .filter(|p| p.kind.is_some())
            .map(|p| {
                let t = table();
                (
                    p.id.clone(),
                    (
                        p.rate.unwrap_or(t.servers.rate).to_string(),
                        p.parallel.unwrap_or(t.servers.parallel).to_string(),
                    ),
                )
            })
            .collect();
        self.address.providers = Some(Window {
            opened: settings.clone(),
            settings,
            keys: HashMap::new(),
            shown: HashSet::new(),
            paces,
            chosen: 0,
            said: None,
            trying: None,
            tried: None,
            tickets: 0,
            cache: o.cache_size(),
        });
        self.dialog = Some(Dialog::AddressProviders);
        Task::none()
    }

    pub(crate) fn address_providers_event(&mut self, e: Event) -> Task<Message> {
        let Some(w) = self.address.providers.as_mut() else {
            return Task::none();
        };
        w.said = None;
        let list = w.providers();
        let Some(p) = list.get(w.chosen).cloned() else {
            return Task::none();
        };
        match e {
            Event::Choose(i) => {
                w.chosen = i.min(list.len().saturating_sub(1));
                w.tried = None;
            }
            Event::Accept(on) => {
                let s = w.setting(&p.id);
                if on {
                    s.accepted = Some(p.terms);
                    s.accepted_at = Some(now_rfc3339());
                } else {
                    s.accepted = None;
                    s.accepted_at = None;
                    s.enabled = false;
                }
            }
            Event::Use(on) => {
                let s = w.setting(&p.id);
                s.enabled = on && s.accepted == Some(p.terms);
            }
            Event::Key(t) => {
                w.keys.insert(p.id.clone(), t);
            }
            Event::ShowKey => {
                if !w.shown.remove(&p.id) {
                    w.shown.insert(p.id.clone());
                }
            }
            Event::Country(t) => {
                let t = t.trim().to_uppercase();
                w.setting(&p.id).country = (!t.is_empty()).then_some(t);
            }
            Event::Store(on) => w.setting(&p.id).store = on,
            Event::AddServer => {
                let id = w.settings.next_server_id();
                let t = table();
                w.settings.providers.push(ProviderSetting {
                    id: id.clone(),
                    kind: t.servers.kinds.first().map(|k| k.kind),
                    name: Some("Kurum sunucusu".into()),
                    url: Some("https://".into()),
                    country: Some("TR".into()),
                    ..ProviderSetting::default()
                });
                w.paces.insert(
                    id.clone(),
                    (t.servers.rate.to_string(), t.servers.parallel.to_string()),
                );
                w.chosen = w.providers().iter().position(|p| p.id == id).unwrap_or(0);
            }
            Event::RemoveServer => {
                if p.server {
                    w.settings.providers.retain(|s| s.id != p.id);
                    w.paces.remove(&p.id);
                    w.keys.insert(p.id.clone(), String::new());
                    w.chosen = w.chosen.saturating_sub(1);
                }
            }
            Event::ServerKind(k) => {
                if let Some(kind) = table().servers.kinds.get(k) {
                    w.setting(&p.id).kind = Some(kind.kind);
                }
            }
            Event::ServerName(t) => w.setting(&p.id).name = Some(t),
            Event::ServerUrl(t) => w.setting(&p.id).url = Some(t),
            Event::ServerRate(t) => {
                w.paces.entry(p.id.clone()).or_default().0 = t;
            }
            Event::ServerParallel(t) => {
                w.paces.entry(p.id.clone()).or_default().1 = t;
            }
            Event::Try => return self.try_provider(),
            Event::Tried(ticket, answer) => {
                if w.trying == Some(ticket) {
                    w.trying = None;
                    w.tried = Some(match answer {
                        Ok(t) => (Say::Ok, t),
                        Err(t) => (Say::Error, t),
                    });
                }
            }
            Event::Link(url) => {
                if let Err(e) = crate::sheet_pdf::open_with_viewer(std::path::Path::new(&url)) {
                    w.said = Some((Say::Error, format!("Bağlantı açılamadı: {e}")));
                }
            }
            Event::ClearCache => {
                let (n, bytes) = online().clear_cache();
                w.cache = online().cache_size();
                w.said = Some((
                    Say::Ok,
                    format!("Önbellekten {n} yanıt ({}) silindi.", size_words(bytes)),
                ));
            }
            Event::Cancel => {
                self.address.providers = None;
                self.dialog = None;
            }
            Event::Save => {
                if let Some(why) = w.problem() {
                    w.said = Some((Say::Error, why));
                    return Task::none();
                }
                let settings = w.written();
                let keys = w.keys.clone();
                let o = online();
                let mut failed = o.save(settings).err();
                for (id, key) in keys {
                    if let Err(e) = o.set_key(&id, Some(&key)) {
                        failed.get_or_insert(e);
                    }
                }
                if let Some(e) = failed {
                    if let Some(w) = self.address.providers.as_mut() {
                        w.said = Some((Say::Error, e));
                    }
                    return Task::none();
                }
                self.address.providers = None;
                self.dialog = None;
                let on = o.providers().iter().filter(|p| p.usable()).count();
                self.say(
                    kentos_interaction::Level::Info,
                    format!("Adres sağlayıcıları kaydedildi: {on} sağlayıcı açık."),
                );
            }
        }
        Task::none()
    }

    /// Dene: one address asked of the chosen provider as the window has it, its time said.
    fn try_provider(&mut self) -> Task<Message> {
        let Some(w) = self.address.providers.as_mut() else {
            return Task::none();
        };
        let mut s = w.written();
        s.providers.retain(|p| p.problem().is_none());
        let Some(p) = s.resolved().into_iter().nth(w.chosen) else {
            return Task::none();
        };
        if p.accepted != Some(p.terms) {
            w.tried = Some((
                Say::Warn,
                "Önce koşulları onaylayın: Dene sağlayıcının sunucusuna bir istek gönderir.".into(),
            ));
            return Task::none();
        }
        let key = w.key(&p.id);
        if p.key == KeyRule::Required && key.trim().is_empty() {
            w.tried = Some((
                Say::Warn,
                format!("{} anahtar ister; anahtarı yazın.", p.name),
            ));
            return Task::none();
        }
        w.tickets += 1;
        let ticket = w.tickets;
        w.trying = Some(ticket);
        w.tried = Some((Say::Info, format!("“{TRY_ADDRESS}” soruluyor…")));
        let req = request::forward(&p, TRY_ADDRESS, request::BATCH_LIMIT, None);
        let (done, wait) = oneshot::channel();
        let _ = std::thread::Builder::new()
            .name("KentOS adres denemesi".into())
            .spawn(move || {
                let mut url = req.url.clone();
                if let Some(param) = &req.key_param
                    && !key.trim().is_empty()
                {
                    url.push(if url.contains('?') { '&' } else { '?' });
                    url.push_str(&format!("{param}={}", encode(key.trim())));
                }
                online().wait_turn(&p, &|| false);
                let t0 = Instant::now();
                let answer = fetch::wait(
                    Request {
                        method: Method::Get,
                        url,
                        headers: vec![("Accept".into(), "application/json".into())],
                        body: None,
                    },
                    1,
                )
                .map_err(|f| f.message)
                .and_then(|r| {
                    let ms = t0.elapsed().as_millis();
                    read::forward(p.kind, &r.body)
                        .map(|read| {
                            format!(
                                "{} aday, {ms} ms ({}, {}).",
                                read.candidates.len(),
                                p.host(),
                                r.status
                            )
                        })
                        .map_err(|why| format!("{why} ({}, {ms} ms)", r.status))
                });
                let _ = done.send(answer);
            });
        Task::perform(wait, move |r| {
            msg(Event::Tried(
                ticket,
                r.unwrap_or_else(|_| Err("Deneme yarıda kaldı.".into())),
            ))
        })
    }

    pub(crate) fn address_providers_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.address.providers else {
            return iced::widget::text("").into();
        };
        let list = w.providers();
        let mut items = Column::new().spacing(2);
        for (i, p) in list.iter().enumerate() {
            let state = if p.enabled && p.accepted == Some(p.terms) {
                "Açık"
            } else if p.accepted == Some(p.terms) {
                "Kapalı"
            } else {
                "Koşulları onaylanmadı"
            };
            let kind = if p.server { "Kurum sunucusu" } else { "Hazır" };
            items = items.push(
                button(
                    column![
                        label::body(p.name.clone()),
                        label::caption(format!("{kind} · {state}"))
                    ]
                    .spacing(1),
                )
                .width(Fill)
                .padding([4, 8])
                .on_press(msg(Event::Choose(i)))
                .style(style::button::table_row(w.chosen == i, false)),
            );
        }
        let tool = |glyph: &str, words: &'static str, on: Option<Message>| {
            button(
                row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                    .spacing(6)
                    .align_y(Center),
            )
            .style(style::button::secondary)
            .padding(kentos_ui::theme::metrics::action())
            .on_press_maybe(on)
        };
        let chosen = list.get(w.chosen);
        let servers = w
            .settings
            .providers
            .iter()
            .filter(|p| p.kind.is_some())
            .count();
        let left = column![
            container(scrollable(items).height(Fill))
                .height(Fill)
                .style(style::container::field_box)
                .width(Fill),
            row![
                tool(
                    "plus",
                    ADD,
                    (servers < kentos_services::geocode::providers::SERVER_LIMIT)
                        .then(|| msg(Event::AddServer))
                ),
                tool(
                    "erase",
                    REMOVE,
                    chosen
                        .filter(|p| p.server)
                        .map(|_| msg(Event::RemoveServer))
                ),
            ]
            .spacing(8),
        ]
        .spacing(8)
        .height(Fill)
        .width(Length::Fixed(typography::scaled(230.0)));
        let right: Element<'_, Message> = match chosen {
            Some(p) => container(scrollable(self.provider_card(w, p)).height(Fill))
                .height(Fill)
                .width(Fill)
                .into(),
            None => container(label::muted("Sağlayıcı yok.")).padding(8).into(),
        };
        let problem = w.problem();
        let line = match (&w.said, &problem) {
            (Some((kind, t)), _) => words::text_line(*kind, t.clone()),
            (None, Some(p)) => words::text_line(Say::Error, p.clone()),
            (None, None) => words::text_line(
                Say::Info,
                format!(
                    "Ayarlar ve anahtarlar bu cihazdadır, çizime yazılmaz. Önbellek: {} yanıt, {}.",
                    w.cache.0,
                    size_words(w.cache.1)
                ),
            ),
        };
        Frame::new(TITLE)
            .fill(row![left, right].spacing(16).height(Fill))
            .push(words::summary(vec![line]))
            .action(words::secondary(CLEAR, Some(msg(Event::ClearCache))))
            .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .action(words::primary(
                SAVE,
                (problem.is_none() && (w.written() != w.opened || !w.keys.is_empty()))
                    .then(|| msg(Event::Save)),
            ))
            .width(1000.0)
            .max_height(660.0)
            .modal(msg(Event::Cancel))
    }

    /// The chosen provider's card and settings.
    fn provider_card<'a>(&'a self, w: &'a Window, p: &Provider) -> Element<'a, Message> {
        let labelled = |name: &'a str, body: Element<'a, Message>| -> Element<'a, Message> {
            row![
                container(label::body(name)).width(Length::Fixed(typography::scaled(130.0))),
                body
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let field = |value: String, on: fn(String) -> Event, width: f32| -> Element<'a, Message> {
            container(kentos_ui::widget::text_field("", &value).on_input(move |t| msg(on(t))))
                .width(Length::Fixed(typography::scaled(width)))
                .into()
        };
        let mut card = Column::new().spacing(8).padding([0, 8]);
        card = card.push(label::strong(p.name.clone()));
        card = card.push(label::muted(p.host()));
        // The card: what is sent and to whom, the limits, writing, the cache, the attribution.
        let mut lines = Column::new().spacing(4);
        for l in &p.lines {
            lines =
                lines.push(row![label::body("•"), label::body(l.clone()).width(Fill)].spacing(6));
        }
        let storing = match p.store {
            StoreRule::Allowed => "Sonuçlar çizime yazılabilir.",
            StoreRule::WithOption => "Sonuçlar yalnız “Sonuçları sakla” açıkken çizime yazılır.",
            StoreRule::Never => "Sonuçlar çizime yazılmaz.",
        };
        lines = lines.push(label::caption(format!(
            "Atıf: {} · {storing}",
            p.attribution
        )));
        card = card.push(
            container(lines)
                .padding([8, 10])
                .width(Fill)
                .style(style::container::field_box),
        );
        let mut links = row![].spacing(8);
        if !p.terms_url.is_empty() {
            links = links.push(words::ghost(
                "Kullanım koşulları",
                Some(msg(Event::Link(p.terms_url.clone()))),
            ));
        }
        if !p.privacy_url.is_empty() {
            links = links.push(words::ghost(
                "Gizlilik ilkesi",
                Some(msg(Event::Link(p.privacy_url.clone()))),
            ));
        }
        card = card.push(links);
        let accepted = p.accepted == Some(p.terms);
        card = card.push(words::check(
            accepted,
            format!(
                "Bu kartı ve sağlayıcının koşullarını okudum, kabul ediyorum (sürüm {})",
                p.terms
            ),
            Some(msg(Event::Accept(!accepted))),
        ));
        card = card.push(words::check(
            p.enabled && accepted,
            USE,
            accepted.then(|| msg(Event::Use(!p.enabled))),
        ));
        if p.key != KeyRule::None {
            let key = w.key(&p.id);
            let shown = w.shown.contains(&p.id);
            let input = kentos_ui::widget::text_field(
                if p.key == KeyRule::Required {
                    "Anahtar (gerekli)"
                } else {
                    "Anahtar (isteğe bağlı)"
                },
                &key,
            )
            .secure(!shown)
            .on_input(|t| msg(Event::Key(t)));
            card = card.push(labelled(
                "Anahtar",
                row![
                    container(input).width(Length::Fixed(typography::scaled(300.0))),
                    words::ghost(
                        if shown { "Gizle" } else { "Göster" },
                        Some(msg(Event::ShowKey))
                    ),
                ]
                .spacing(8)
                .align_y(Center)
                .into(),
            ));
        }
        card = card.push(labelled(
            "Ülke",
            row![
                field(p.country.clone().unwrap_or_default(), Event::Country, 60.0),
                label::muted("İki harfli ülke kodu (TR); boşsa bütün dünyada aranır."),
            ]
            .spacing(8)
            .align_y(Center)
            .into(),
        ));
        if p.store == StoreRule::WithOption {
            card = card.push(words::check(
                p.storing,
                "Sonuçları sakla (forStorage=true; her istek kredi harcar)",
                Some(msg(Event::Store(!p.storing))),
            ));
        }
        if p.server {
            let s = w.settings.providers.iter().find(|s| s.id == p.id);
            let kinds = &table().servers.kinds;
            card = card.push(labelled(
                "Tür",
                container(Select::new(
                    kinds.iter().map(|k| Choice::new(k.name.clone())),
                    kinds.iter().position(|k| k.kind == p.kind),
                    |k| msg(Event::ServerKind(k)),
                ))
                .width(Length::Fixed(typography::scaled(240.0)))
                .into(),
            ));
            card = card.push(labelled(
                "Ad",
                field(
                    s.and_then(|s| s.name.clone()).unwrap_or_default(),
                    Event::ServerName,
                    300.0,
                ),
            ));
            card = card.push(labelled(
                "Adres",
                field(
                    s.and_then(|s| s.url.clone()).unwrap_or_default(),
                    Event::ServerUrl,
                    420.0,
                ),
            ));
            let (rate, parallel) = w.paces.get(&p.id).cloned().unwrap_or_default();
            card = card.push(labelled(
                "İstek / s",
                row![
                    field(rate, Event::ServerRate, 80.0),
                    label::body("Aynı anda"),
                    field(parallel, Event::ServerParallel, 60.0)
                ]
                .spacing(8)
                .align_y(Center)
                .into(),
            ));
        }
        card = card.push(
            row![
                words::secondary(
                    TRY,
                    (accepted && w.trying.is_none()).then(|| msg(Event::Try))
                ),
                label::muted(format!("“{TRY_ADDRESS}” aranır ve süresi söylenir.")),
            ]
            .spacing(8)
            .align_y(Center),
        );
        if let Some((kind, t)) = &w.tried {
            card = card.push(words::text_line(*kind, t.clone()));
        }
        card.into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn address_providers_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let w = self
            .address
            .providers
            .as_ref()
            .ok_or("Adres sağlayıcıları açık değil")?;
        let list = w.providers();
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(ADD) => Some(msg(Event::AddServer)),
            Control::Press(CLEAR) => Some(msg(Event::ClearCache)),
            Control::Check(USE, on) => Some(msg(Event::Use(on))),
            Control::Check(words, on) if words.starts_with("Bu kartı") => {
                Some(msg(Event::Accept(on)))
            }
            Control::Pick(TITLE, item) => {
                let i = list
                    .iter()
                    .position(|p| p.name == item)
                    .ok_or_else(|| format!("“{item}” sağlayıcısı yok"))?;
                Some(msg(Event::Choose(i)))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
