//! The services' part of the window (docs/adr/0208 §3): the credits' strip
//! at the drawing area's bottom right, under the scale bar, and the card it
//! opens, each credit with its links (opened in the browser; only `http`
//! and `https` addresses).

use iced::widget::{button, column, container, row, text as plain};
use iced::{Background, Border, Center, Element, Fill, Padding, Task, Theme};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::shape::{self, Level};
use kentos_ui::theme::{Tokens, typography};

use kentos_contracts::{ServiceKind, ServiceLayer};
use kentos_ui::widget::context_menu::Menu;

use super::overlay::Marks;
use crate::app::{App, Message};

#[derive(Clone, Debug)]
pub enum Event {
    /// The credits' card opened or closed.
    Credits(bool),
    /// A credit's link, opened in the browser.
    Open(String),
    /// A service layer's menu (docs/adr/0208 §14), by the layer's id:
    /// read again from its address (Yeniden yükle), its tiles forgotten on
    /// this device too (Önbelleği temizle), its extent shown, its opacity.
    Reload(String),
    ClearCache(String),
    ZoomTo(String),
    Opacity(String, f64),
    /// Bağlantılar's window (connections.rs).
    Connections(super::connections::Event),
    /// Harita servisi's window (window.rs).
    Window(super::window::Event),
    /// Servisten veri al's window (feed_window.rs).
    Feed(super::feed_window::Event),
    /// A feed layer's Yenile.
    FeedRefresh(String),
    /// Servis bilgisi's answers (info.rs).
    Info(Vec<super::info::Answer>),
    /// Servis ayarları from a layer's menu: Harita servisi on that layer.
    Settings(String),
}

/// Runs `work` on a thread of its own and brings its answer back as a message.
pub(crate) fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) -> Event + Send + 'static,
) -> Task<Message> {
    iced_runtime::task::blocking(
        move |mut out: iced::futures::channel::mpsc::Sender<Message>| {
            let answer = msg(done(work()));
            let _ =
                iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, answer));
        },
    )
}

/// The opacities the layer's menu offers.
pub const OPACITIES: [f64; 6] = [1.0, 0.9, 0.75, 0.6, 0.45, 0.3];

/// A service layer's icon: its ready basemap's, else its kind's.
pub fn service_icon(service: &ServiceLayer) -> &'static str {
    if let Some(p) = service
        .preset
        .as_deref()
        .and_then(kentos_services::presets::preset)
    {
        return match p.icon.as_str() {
            "basemapOsm" => "basemapOsm",
            "basemapTopo" => "basemapTopo",
            "basemapHot" => "basemapHot",
            "basemapCycle" => "basemapCycle",
            "basemapVector" => "basemapVector",
            "basemapVectorBright" => "basemapVectorBright",
            "basemapVectorLight" => "basemapVectorLight",
            "basemapImagery" => "basemapImagery",
            "basemapHgm" => "basemapHgm",
            "basemapHgmPhoto" => "basemapHgmPhoto",
            "basemapGoogleRoad" => "basemapGoogleRoad",
            "basemapGoogleSatellite" => "basemapGoogleSatellite",
            "basemapGoogleTerrain" => "basemapGoogleTerrain",
            "basemapGoogleHybrid" => "basemapGoogleHybrid",
            "basemapMaptiler" => "basemapMaptiler",
            "basemapMaptilerSatellite" => "basemapMaptilerSatellite",
            _ => "basemap",
        };
    }
    match service.kind {
        ServiceKind::Vector => "basemapVector",
        ServiceKind::Google => "basemapGoogleRoad",
        ServiceKind::Xyz => "basemap",
        ServiceKind::Wms | ServiceKind::Wmts | ServiceKind::OgcTiles | ServiceKind::Arcgis => {
            "serviceAdd"
        }
    }
}

/// A service's key from the drawing's settings.
fn key_in(settings: &kentos_contracts::ProjectSettings, service: &ServiceLayer) -> String {
    let connection = service
        .connection
        .as_ref()
        .and_then(|c| settings.connections.iter().find(|k| &k.id == c));
    super::key_of(service, connection)
}

fn msg(event: Event) -> Message {
    Message::Services(event)
}

/// The strip's longest text, characters: the rest is in the card.
const STRIP_MOST: usize = 96;
/// The strip's words' size, pixels at the interface's 100 %.
const STRIP_TEXT: f32 = 10.5;

/// How far the scale bar (18 px over the area's bottom) is lifted to stand
/// 3 px over the strip: its text's line, its padding and its gap below.
pub(crate) fn scale_bar_lift() -> f32 {
    let strip = typography::scaled(STRIP_TEXT) * 1.3 + 2.0 + 2.0;
    (strip + 3.0 - 18.0).max(0.0)
}

impl App {
    /// A layer's service and its key, when the layer is drawn from one.
    fn served(&self, id: &str) -> Option<(String, String, ServiceLayer)> {
        let doc = self.document.as_ref()?;
        let node = doc.model.layers().get(id)?;
        let service = node.service.clone()?;
        Some((
            key_in(doc.model.settings(), &service),
            node.name.clone(),
            service,
        ))
    }

    /// Why a layer's service shows nothing, when it failed (the layer tree's badge).
    pub(crate) fn service_failure(&self, service: &ServiceLayer) -> Option<String> {
        let doc = self.document.as_ref()?;
        if !super::in_use() {
            return None;
        }
        super::hub().failure(&key_in(doc.model.settings(), service))
    }

    pub(crate) fn services_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Reload(id) => {
                if let Some((key, name, _)) = self.served(&id) {
                    super::hub().reload(&key);
                    self.say(
                        kentos_interaction::Level::Info,
                        format!("“{name}” servisi yeniden okunuyor."),
                    );
                }
            }
            Event::ClearCache(id) => {
                if let Some((key, name, _)) = self.served(&id) {
                    let gone = super::hub().forget(&key);
                    self.say(
                        kentos_interaction::Level::Success,
                        format!("“{name}” servisinin önbelleği temizlendi: bu cihazda {gone} yanıt silindi."),
                    );
                }
            }
            Event::ZoomTo(id) => self.zoom_to_service(&id),
            Event::Connections(e) => return self.connections_event(e),
            Event::Window(e) => return self.service_window_event(e),
            Event::Feed(e) => return self.feed_event(e),
            Event::FeedRefresh(id) => return self.refresh_feed(&id),
            Event::Info(answers) => self.service_info_answered(answers),
            Event::Settings(id) => self.open_service_window(Some(id)),
            Event::Opacity(id, opacity) => self.service_opacity(&id, opacity),
            Event::Credits(open) => self.service_credits = open,
            Event::Open(url) => {
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    return Task::none();
                }
                if let Err(e) = crate::sheet_pdf::open_with_viewer(std::path::Path::new(&url)) {
                    self.warn(format!("Adres tarayıcıda açılamadı ({e}): {url}"));
                }
            }
        }
        Task::none()
    }

    /// Shows a service's extent (its capabilities' box in WGS 84 degrees, in the project's system).
    fn zoom_to_service(&mut self, id: &str) {
        let Some((_, name, service)) = self.served(id) else {
            return;
        };
        let Some([w, s, e, n]) = service.bbox else {
            self.warn(format!(
                "“{name}” servisinin kapsamı bilinmiyor: servisi Harita servisi penceresinden yeniden ekleyin."
            ));
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let project = super::systems::ProjectSystem::of(doc.model.settings());
        let pair = match super::systems::pair(&project, 4326) {
            Ok(p) => p,
            Err(why) => {
                self.warn(format!("“{name}”: {why}"));
                return;
            }
        };
        // The box's edges, five points each, into the project's system.
        let mut b = kentos_render_wgpu::Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for i in 0..=4 {
            let t = f64::from(i) / 4.0;
            for (x, y) in [
                (w + (e - w) * t, s),
                (w + (e - w) * t, n),
                (w, s + (n - s) * t),
                (e, s + (n - s) * t),
            ] {
                if let Some((px, py)) = (pair.to_project)(x, y) {
                    b.min_x = b.min_x.min(px);
                    b.min_y = b.min_y.min(py);
                    b.max_x = b.max_x.max(px);
                    b.max_y = b.max_y.max(py);
                }
            }
        }
        if !(b.min_x.is_finite() && b.max_x > b.min_x && b.max_y > b.min_y) {
            self.warn(format!(
                "“{name}” servisinin kapsamı projenin sisteminde gösterilemiyor."
            ));
            return;
        }
        self.navigating(|app| app.viewport.camera.fit(&b, 24.0));
    }

    /// A service layer's opacity, written by `cad.layers.service` as one undo step.
    fn service_opacity(&mut self, id: &str, opacity: f64) {
        let Some((_, _, service)) = self.served(id) else {
            return;
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let input = kentos_contracts::LayersService {
            operation: kentos_contracts::LayerServiceOperation::Update,
            layer: Some(id.to_owned()),
            name: None,
            parent: None,
            index: None,
            service: Some(ServiceLayer {
                opacity: (opacity < 1.0).then_some(opacity),
                ..service
            }),
            feed: None,
            fields: None,
            connections: None,
            expected_revision: None,
        };
        let result = kentos_native_application::layers_service::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            input,
        );
        if let kentos_contracts::CommandResult::Failed { error }
        | kentos_contracts::CommandResult::Conflict { error }
        | kentos_contracts::CommandResult::NeedsInput { error } = result
        {
            self.warn(error.message);
        }
    }

    /// A service layer's own items in its menu (docs/adr/0208 §14).
    pub(crate) fn service_menu(
        &self,
        node: &kentos_contracts::LayerNode,
        menu: Menu<Message>,
    ) -> Menu<Message> {
        let Some(service) = &node.service else {
            return menu;
        };
        let id = node.id.clone();
        let now = service.opacity.unwrap_or(1.0);
        let opacities = OPACITIES.iter().fold(Menu::new(), |m, o| {
            m.radio(
                format!("% {}", (o * 100.0).round()),
                (now - o).abs() < 1e-9,
                msg(Event::Opacity(id.clone(), *o)),
            )
        });
        menu.item("Servis ayarları…", msg(Event::Settings(id.clone())))
            .icon(crate::icons::from_web(Some("serviceSettings")))
            .item("Yeniden yükle", msg(Event::Reload(id.clone())))
            .icon(crate::icons::from_web(Some("serviceReload")))
            .item("Önbelleği temizle", msg(Event::ClearCache(id.clone())))
            .icon(crate::icons::from_web(Some("serviceCache")))
            .item(
                "Servisin kapsamına yakınlaştır",
                service
                    .bbox
                    .is_some()
                    .then(|| msg(Event::ZoomTo(id.clone()))),
            )
            .icon(crate::icons::from_web(Some("serviceExtent")))
            .submenu("Saydamlık", opacities)
            .icon(crate::icons::from_web(Some("basemap")))
            .separator()
    }

    /// The strip and, when open, the card, over the drawing area.
    pub(crate) fn credits_view(&self, marks: &Marks) -> Vec<Element<'_, Message>> {
        let words = marks.strip();
        if words.is_empty() {
            return Vec::new();
        }
        let shown: String = if words.chars().count() > STRIP_MOST {
            let mut s: String = words.chars().take(STRIP_MOST - 1).collect();
            s.push('…');
            s
        } else {
            words
        };
        let strip = button(
            plain(shown)
                .font(typography::ui())
                .size(typography::scaled(STRIP_TEXT)),
        )
        .padding([1, 6])
        .on_press(msg(Event::Credits(!self.service_credits)))
        .style(|theme: &Theme, status| {
            let t = Tokens::of(theme);
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(t.surface.scale_alpha(if hovered {
                    0.95
                } else {
                    0.78
                }))),
                text_color: if hovered { t.text } else { t.muted },
                border: Border {
                    radius: shape::sm().into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        });
        let mut out: Vec<Element<'_, Message>> = vec![
            container(strip)
                .align_right(Fill)
                .align_bottom(Fill)
                .padding(Padding {
                    top: 0.0,
                    right: 4.0,
                    bottom: 2.0,
                    left: 0.0,
                })
                .into(),
        ];
        if self.service_credits {
            out.push(self.credits_card(marks));
        }
        out
    }

    fn credits_card(&self, marks: &Marks) -> Element<'_, Message> {
        let mut body = column![].spacing(8);
        for c in &marks.credits {
            let mut item = column![label::body(c.text.clone()).width(Fill)].spacing(2);
            for (words, url) in &c.links {
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    continue;
                }
                item = item.push(
                    button(
                        row![
                            icon(Icon::Link).size(12.0).tone(Tone::Accent),
                            plain(words.clone())
                                .font(typography::ui())
                                .size(typography::caption()),
                        ]
                        .spacing(4)
                        .align_y(Center),
                    )
                    .padding([1, 2])
                    .on_press(msg(Event::Open(url.clone())))
                    .style(style::button::ghost),
                );
            }
            body = body.push(item);
        }
        let head = row![
            container(label::strong("Atıflar")).width(Fill),
            button(icon(Icon::Close).size(10.0).tone(Tone::Muted))
                .on_press(msg(Event::Credits(false)))
                .padding(4)
                .style(style::button::ghost),
        ]
        .align_y(Center);
        let card = container(
            column![
                head,
                label::caption("Görünen servislerin atıfları; bağlantılar tarayıcıda açılır."),
                body
            ]
            .spacing(6),
        )
        .width(340)
        .padding(10)
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(Background::Color(t.popover)),
                border: Border {
                    color: t.border_strong(),
                    width: 1.0,
                    radius: shape::md().into(),
                },
                shadow: shape::shadow(Level::Float, &t),
                ..container::Style::default()
            }
        });
        container(card)
            .align_right(Fill)
            .align_bottom(Fill)
            .padding(Padding {
                top: 0.0,
                right: 6.0,
                bottom: 24.0,
                left: 0.0,
            })
            .into()
    }
}
