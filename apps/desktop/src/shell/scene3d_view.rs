//! 3B görünüm's look (DESIGN.md §7.20; docs/adr/8060 §9, docs/adr/8067 §9, 8119): the model is what the window is
//! for, so the scene takes all the room between two thin bands in the main window's own vocabulary. A building is the
//! model viewer's layout (DESIGN.md §7.21: the tree with its search at the left, the details at the right, the section's
//! card in the scene), Katlar ▾ and Ana pencereye al at its strip's end. A road's: over the scene one tool strip, the
//! camera (Görünüş ▾ with the four views, Sığdır), then the exaggeration and the terrain switches, then at its end what
//! the window does (glTF ver, Ana pencereye al). In the scene, the north arrow at the top right turns with the camera
//! and turns it north when pressed (DESIGN.md §8), and the legend folds at the bottom left. Under it, a status bar: the
//! point clicked and what it is on, how much the scene shows (its triangles and tiles in the tip), the mouse's help.
//!
//! It is always the whole of an operating-system window beside the plan, through the windows' contract ([`View3d`],
//! shell/windows/surface.rs, docs/adr/8060 §10, 8122): it is never brought over the drawing, so the plan and the model
//! are never one over the other. The sky is the drawing area's own ground (Görünüm › Çizim zemini) a little bluer at the zenith, so
//! the model reads as the plan's and its walls stand out from the ground.

use iced::widget::{Row, column, container, row, space, tooltip};
use iced::{Background, Border, Center, Color, Element, Fill, Task, Theme};
use kentos_interaction::shadow_tool::DAYS;
use kentos_model_view_ui::look;
use kentos_model_view_ui::scene::{self, Gesture, SceneView};
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::widget::{Dialog as Frame, Menu, StatusBar, Tip, tip};

use super::{CUT, EXAGGERATION, EXPORT, Event, FIT, FLAT, PARTS, SEE_THROUGH, TITLE, Window, msg};
use crate::app::{App, Dialog, Message};
use crate::scene3d_sun::{SunAt, SunChange};
use crate::style::strip;
use crate::traces::Control;
use crate::viewport::Canvas;
use crate::windows::{OwnWindow, Surface};

/// The surface this window is (shell/windows/).
const SURFACE: Surface = Surface::Scene3d;

/// 3B görünüm as a window of its own (shell/windows/surface.rs): it always opens beside the plan (docs/adr/8122); Esc
/// and its close button close it, Tab goes round its fields.
pub(crate) struct View3d;

impl OwnWindow for View3d {
    fn kept_as(&self) -> &'static str {
        "3b-gorunum"
    }

    /// The in-canvas window's width and the scene's room under its tool strip.
    fn sizes(&self) -> (iced::Size, iced::Size) {
        (
            iced::Size::new(1180.0, 760.0),
            iced::Size::new(720.0, 460.0),
        )
    }

    /// The model is looked at beside the plan, often on another screen.
    fn opens_apart(&self) -> bool {
        true
    }

    /// Never over the drawing (docs/adr/8122, the owner's choice of 18 November).
    fn always_apart(&self) -> bool {
        true
    }

    fn what(&self) -> &'static str {
        "3B görünümü"
    }

    fn apart_tip(&self) -> &'static str {
        "3B görünümü kendi penceresinde açar: pencereyi başka bir ekrana taşıyıp çizerken modeli izleyebilirsiniz. Sonraki açılışlar da ayrı pencerede olur."
    }

    fn return_tip(&self) -> &'static str {
        "3B görünümü yeniden ana pencerede, çizimin üstünde açar. Sonraki açılışlar da orada olur."
    }

    fn dialog(&self) -> Dialog {
        Dialog::Scene3d
    }

    fn is_open(&self, app: &App) -> bool {
        app.scene3d_window.is_some()
    }

    fn title(&self, _app: &App) -> String {
        TITLE.to_owned()
    }

    fn view<'a>(&self, app: &'a App) -> Element<'a, Message> {
        app.scene3d_window_view()
    }

    fn close(&self, app: &mut App) -> Task<Message> {
        app.scene3d_event(Event::Close)
    }

    fn trace_title(&self, _app: &App) -> Option<String> {
        Some(TITLE.to_owned())
    }

    fn trace_control(
        &self,
        app: &App,
        control: Control<'_>,
    ) -> Option<Result<Option<Message>, String>> {
        Some(app.scene3d_control(control))
    }
}

impl App {
    /// Over the drawing: a window like the others, its strip, scene and bar framed as one viewer. The scene takes
    /// what the application window leaves (between 300 and 640 scaled pixels; a building's panes between 420 and
    /// 760), all of the window's room when it was given a size or made full screen (docs/adr/0746).
    pub(crate) fn scene3d_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.scene3d_window else {
            return space().into();
        };
        let sized = kentos_ui::widget::sizable::sizes::get(TITLE).is_some();
        let (low, high) = if w.building.is_some() {
            (420.0, 760.0)
        } else {
            (300.0, 640.0)
        };
        let room = (self.window_size.height - typography::scaled(270.0))
            .clamp(typography::scaled(low), typography::scaled(high));
        let viewer = container(self.scene3d_body(w))
            .height(Fill)
            .max_height(if sized { f32::INFINITY } else { room })
            .style(|t: &Theme| container::Style {
                border: Border {
                    color: Tokens::of(t).border,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            });
        // A building's tree and details take 600 pixels of it: the scene keeps the road's room beside them.
        let width = if w.building.is_some() { 1280.0 } else { 1180.0 };
        crate::windows::placed(
            Frame::new(TITLE).fill(viewer).width(width),
            SURFACE,
            false,
            msg(Event::Close),
        )
    }

    /// Beside the plan: the whole of its own window, the strip at the top, the bar at the bottom, the scene between.
    pub(crate) fn scene3d_window_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.scene3d_window else {
            return space().into();
        };
        container(self.scene3d_body(w))
            .width(Fill)
            .height(Fill)
            .style(|t: &Theme| container::Style {
                background: Some(Background::Color(Tokens::of(t).surface)),
                ..container::Style::default()
            })
            .into()
    }

    /// What the window holds: a building's viewer, else the road's strip, scene and bar.
    fn scene3d_body<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        match &w.building {
            Some(b) => {
                let mut items = vec![b.storeys_menu()];
                items.extend(sun_items(b.sun));
                items.push(crate::scene3d_sheet::sheet_button());
                b.viewer
                    .layout(self.viewer_host(|m| msg(Event::Viewer(m)), items))
            }
            None => column![
                self.scene3d_strip(w),
                strip::rule(),
                self.scene3d_scene(w),
                self.scene3d_status(w)
            ]
            .into(),
        }
    }

    /// The road's tool strip: the camera, what shows, then at its end what the window does. A narrow window wraps the
    /// first two groups; the end stays at the right.
    fn scene3d_strip<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let orbit = w.orbit;
        let mut left: Vec<Element<'a, Message>> = vec![
            strip::menu(
                crate::icons::from_web(Some("scene3dView")),
                look::view_name(&orbit),
                "Görünüş: modele hangi yönden bakıldığı. Döndürünce bakış serbesttir; bir görünüş seçmek modeli o yönden sığdırır.",
                move || look::views_menu(&orbit, |i| msg(Event::View(i)), msg(Event::North)),
            ),
            strip::labelled(
                Icon::ZoomExtents,
                FIT,
                "Sığdır: bütün modeli bu bakışla görünüme sığdırır.",
                Some(msg(Event::Fit)),
            ),
            strip::divider(),
        ];
        left.push(
            container(label::body(EXAGGERATION).style(muted))
                .padding([0, 6])
                .into(),
        );
        left.push(tip(
            kentos_ui::widget::text_field("1", &w.exaggeration)
                .on_input(|t| msg(Event::Exaggeration(t)))
                .width(typography::scaled(46.0)),
            Tip::new(EXAGGERATION)
                .body("Yükseklikler bu kat büyük gösterilir, sahnenin en alçak noktasından; yüzeylerin kotları değişmez."),
            tooltip::Position::Bottom,
        ));
        left.push(space().width(4).into());
        left.push(strip::divider());
        left.push(strip::toggle(
            None,
            CUT,
            "Araziyi kes: koridorların kazdığı ve doldurduğu yerde araziyi çizmez; kapalıyken arazi koridorun üstünden geçer.",
            w.cut,
            msg(Event::Cut),
        ));
        left.push(strip::toggle(
            None,
            FLAT,
            "Düz yüzler: araziyi üçgenleriyle gösterir; kapalıyken arazi yumuşak gölgelenir.",
            w.flat,
            msg(Event::Flat),
        ));
        left.push(strip::toggle(
            None,
            SEE_THROUGH,
            "Arazi saydam: koridorların ve şevlerin altındaki araziyi saydam gösterir.",
            w.see_through,
            msg(Event::SeeThrough),
        ));
        let end = Row::new().spacing(1).align_y(Center).push(strip::labelled(
            crate::icons::from_web(Some("gltfExport")),
            EXPORT,
            "glTF ver: arazi yüzeylerini ve koridorları bir glTF dosyasına yazar.",
            Some(msg(Event::Export)),
        ));
        strip::surface(
            row![
                Row::with_children(left)
                    .spacing(1)
                    .align_y(Center)
                    .width(Fill)
                    .wrap()
                    .vertical_spacing(4),
                end
            ]
            .spacing(6)
            .align_y(Center),
        )
    }

    /// The road's scene on its sky, the picked point ringed, the north arrow at the top right and the legend at the
    /// bottom left (crates/model-view-ui's look, the IFC viewer's too).
    fn scene3d_scene<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let view = SceneView {
            scene: w.scene.clone(),
            caps: None,
            camera: w.camera(),
            exaggeration: w.exaggeration(),
            styles: w.styles(),
            light: w.light(),
            // The road's light as it was: the sun alone (docs/adr/8120 leaves its look to its own work).
            sky: 0.0,
            fill: 0.0,
            quality: self.scene3d_quality(),
            marks: Vec::new(),
            hover: false,
            handles: Vec::new(),
            active: None,
            planes: Vec::new(),
            marked: w.picked.as_ref().map(|p| w.exaggeration().shown(p.point)),
            seen: w.seen.clone(),
            pending: w.pending.clone(),
            sun: false,
        };
        let swatches: Vec<(&str, [f32; 4])> = PARTS
            .iter()
            .map(|(name, c)| (*name, [c[0], c[1], c[2], 1.0]))
            .collect();
        look::framed(
            scene::view(view, gesture),
            look::compass(&w.orbit, msg(Event::North)),
            look::legend(&swatches, w.legend, msg(Event::Legend)),
            None,
            None,
            sky(self.canvas()),
        )
    }

    /// The road's status bar: the point clicked and what it is on (else what a click does), then at the right how much
    /// the scene shows and the mouse's help.
    fn scene3d_status<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let mut bar = StatusBar::new();
        bar = match &w.picked {
            Some(p) => {
                let f = self.format();
                let at = format!(
                    "{} {}   {} {}   Z {}",
                    f.east_label(),
                    f.coord(p.point[0]),
                    f.north_label(),
                    f.coord(p.point[1]),
                    f.coord(p.point[2])
                );
                bar.push(
                    Readout::new(row![label::body(p.what.clone()), label::mono(at)].spacing(12).align_y(Center))
                        .icon(Icon::Crosshair)
                        .tip(Tip::new("Seçilen nokta").body(format!(
                            "{} sağa (doğu), {} yukarı (kuzey), Z kot; düşey abartma gösterimdir, kot gerçektir.",
                            f.east_label(),
                            f.north_label()
                        ))),
                )
            }
            None => bar.push(
                Readout::new(
                    label::body("Bir noktaya tıklayın: yeri ve neyin üstünde olduğu burada yazar.")
                        .style(muted),
                )
                .icon(Icon::Crosshair),
            ),
        };
        bar = bar.spacer();
        let things = format!("{} yüzey ve koridor", w.scene.sources());
        bar = bar.push(
            Readout::new(label::body(things.clone())).tip(Tip::new(things).body(format!(
                "Sahne {} üçgen, {} karodur; yalnız ekranda görünen karolar çizilir.",
                w.scene.triangles(),
                w.scene.contents()
            ))),
        );
        bar.separator()
            .push(Readout::new(label::body("Fare")).icon(Icon::Help).tip(Tip::new("Fareyle gezinme").body(
                "Sürükleyin: döndürür. Sağ tuşla ya da Shift ile sürükleyin: kaydırır. Tekerlek: yaklaştırır, uzaklaştırır. Tıklayın: noktayı ve nesneyi seçer.",
            )))
            .into()
    }
}

#[cfg(any(test, feature = "testing"))]
impl App {
    /// What a picture of 3B görünüm shows: its own window while it is open beside the plan, else the main window.
    pub fn scene3d_shown(&self) -> Element<'_, Message> {
        match self.windows.of(SURFACE) {
            Some(w) if self.surface_wanted(SURFACE) => self.window_view(w.id),
            _ => self.view(),
        }
    }
}

/// What a gesture on the scene does to the window.
fn gesture(g: Gesture) -> Message {
    msg(match g {
        Gesture::Orbit(dx, dy) => Event::Orbit(dx, dy),
        Gesture::Pan(dx, dy) => Event::Pan(dx, dy),
        Gesture::Dolly(steps) => Event::Dolly(steps),
        Gesture::Pick(x, y, width, height) => Event::Pick(x, y, width, height),
        Gesture::Size(width, height) => Event::Sized(width, height),
        // The road measures nothing yet: its scene never asks for the pointer's moves.
        Gesture::Hover(_, width, height) => Event::Sized(width, height),
        // A section's handles are a building's viewer's (the road's scene has none).
        g @ (Gesture::Handle(..) | Gesture::HandleDone(_)) => {
            Event::Viewer(kentos_model_view_ui::Message::Gesture(g))
        }
    })
}

/// The sky over the drawing area's ground (Görünüm › Çizim zemini): towards the drawing area's own blue (its window
/// box's, the same on every ground) at the zenith; the high-contrast ground stays black. The IFC viewer's too.
pub(crate) fn sky(canvas: Canvas) -> Background {
    let [r, g, b, _] = crate::viewport::palette(canvas).background.0;
    let ground = Color::from_rgb8(r, g, b);
    look::sky(
        ground,
        (canvas != Canvas::Black).then(|| crate::viewport::mark_colors(canvas).window),
    )
}

fn muted(t: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(t).muted),
    }
}

/// Güneş's strip items (scene3d_sun.rs): its switch, and while it is on its day and its hour with the steps either side.
fn sun_items<'a>(at: Option<SunAt>) -> Vec<Element<'a, Message>> {
    let mut items = vec![strip::toggle(
        Some(crate::icons::from_web(Some("sun"))),
        "Güneş",
        "Güneş: binayı seçilen gün ve saatin güneşiyle aydınlatır, gölgelerini düşürür. Konum projenin koordinat \
         sisteminden, yoksa Gölge (GL) aracında yazılan enlem ve boylamdan.",
        at.is_some(),
        Message::Scene3d(Event::Sun(SunChange::Toggle)),
    )];
    let Some(at) = at else { return items };
    items.push(strip::menu(
        Icon::Calendar,
        DAYS[at.day].1,
        "Tarih: güneşin günü.",
        move || {
            DAYS.iter().enumerate().fold(Menu::new(), |m, (i, d)| {
                m.radio(
                    d.1.to_owned(),
                    i == at.day,
                    Message::Scene3d(Event::Sun(SunChange::Day(i))),
                )
            })
        },
    ));
    let step = |by: i8| Some(Message::Scene3d(Event::Sun(SunChange::Hour(by))));
    items.push(strip::icon_button(
        Icon::ChevronLeft,
        "Bir saat önce",
        step(-1),
    ));
    items.push(strip::readout(
        kentos_native_application::shadows::clock(at.hour),
        "Saat: yerel saat (Türkiye UTC+3).",
        44.0,
    ));
    items.push(strip::icon_button(
        Icon::ChevronRight,
        "Bir saat sonra",
        step(1),
    ));
    items
}
