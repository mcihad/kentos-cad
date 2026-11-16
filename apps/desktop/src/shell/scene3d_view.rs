//! 3B görünüm's look (DESIGN.md §7.20; docs/adr/8060 §9, docs/adr/8067 §9): the model is what the window is for, so
//! the scene takes all the room between two thin bands in the main window's own vocabulary. Over it, one tool strip:
//! the camera (Görünüş ▾ with the four views, Sığdır), then what shows (a building's Katlar ▾ and Mahaller; a road's
//! exaggeration and terrain switches), then at its end what the window does (glTF ver, Ana pencereye al). In the
//! scene, the north arrow at the top right turns with the camera and turns it north when pressed (DESIGN.md §8), and
//! the legend folds at the bottom left. Under it, a status bar: the point clicked and what it is on, how much the
//! scene shows (its triangles and tiles in the tip), the objects the check found something about, the mouse's help.
//!
//! Beside the plan it is the whole of an operating-system window, through the windows' contract ([`View3d`],
//! shell/windows/surface.rs, docs/adr/8060 §10); over the drawing it is a window like the others, its header carrying
//! Ayrı pencerede aç. The strip, the scene and the bar are the same in both. The sky is the drawing area's own ground (Görünüm › Çizim zemini) a little bluer at the zenith, so
//! the model reads as the plan's and its walls stand out from the ground.

use iced::widget::{Row, column, container, row, space, tooltip};
use iced::{Background, Border, Center, Color, Element, Fill, Task, Theme};
use kentos_model_view_ui::look;
use kentos_model_view_ui::scene::{self, Gesture, SceneView};
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::widget::{Dialog as Frame, StatusBar, Tip, tip};

use super::{CUT, EXAGGERATION, EXPORT, Event, FIT, FLAT, PARTS, SEE_THROUGH, TITLE, Window, msg};
use crate::app::{App, Dialog, Message};
use crate::style::strip;
use crate::traces::Control;
use crate::viewport::Canvas;
use crate::windows::{OwnWindow, Surface};

/// The surface this window is (shell/windows/).
const SURFACE: Surface = Surface::Scene3d;

/// 3B görünüm as a window of its own (shell/windows/surface.rs): it opens beside the plan until the user brings it
/// over the drawing; Esc and its close button close it, Tab goes round its fields.
pub(crate) struct View3d;

impl OwnWindow for View3d {
    fn kept_as(&self) -> &'static str {
        "3b-gorunum"
    }

    /// The in-canvas window's width and the scene's room under its tool strip.
    fn sizes(&self) -> (iced::Size, iced::Size) {
        (iced::Size::new(1180.0, 760.0), iced::Size::new(720.0, 460.0))
    }

    /// The model is looked at beside the plan, often on another screen.
    fn opens_apart(&self) -> bool {
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

    fn trace_control(&self, app: &App, control: Control<'_>) -> Option<Result<Option<Message>, String>> {
        Some(app.scene3d_control(control))
    }
}

impl App {
    /// Over the drawing: a window like the others, its strip, scene and bar framed as one viewer. The scene takes
    /// what the application window leaves (between 300 and 640 scaled pixels), all of the window's room when it was
    /// given a size or made full screen (docs/adr/0746).
    pub(crate) fn scene3d_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.scene3d_window else {
            return space().into();
        };
        let sized = kentos_ui::widget::sizable::sizes::get(TITLE).is_some();
        let room =
            (self.window_size.height - typography::scaled(270.0)).clamp(typography::scaled(300.0), typography::scaled(640.0));
        let viewer = container(column![self.scene3d_strip(w, false), strip::rule(), self.scene3d_scene(w), self.scene3d_status(w)])
            .height(Fill)
            .max_height(if sized { f32::INFINITY } else { room })
            .style(|t: &Theme| container::Style {
                border: Border { color: Tokens::of(t).border, width: 1.0, radius: 0.0.into() },
                ..container::Style::default()
            });
        crate::windows::placed(Frame::new(TITLE).fill(viewer).width(1180.0), SURFACE, false, msg(Event::Close))
    }

    /// Beside the plan: the whole of its own window, the strip at the top, the bar at the bottom, the scene between.
    pub(crate) fn scene3d_window_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.scene3d_window else {
            return space().into();
        };
        container(column![self.scene3d_strip(w, true), strip::rule(), self.scene3d_scene(w), self.scene3d_status(w)])
            .width(Fill)
            .height(Fill)
            .style(|t: &Theme| container::Style {
                background: Some(Background::Color(Tokens::of(t).surface)),
                ..container::Style::default()
            })
            .into()
    }

    /// The tool strip: the camera, what shows, then at its end what the window does. A narrow window wraps the first
    /// two groups; the end stays at the right.
    fn scene3d_strip<'a>(&'a self, w: &'a Window, apart: bool) -> Element<'a, Message> {
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
        match &w.building {
            Some(b) => {
                left.push(b.storeys_menu());
                left.push(b.rooms_toggle());
            }
            None => {
                left.push(container(label::body(EXAGGERATION).style(muted)).padding([0, 6]).into());
                left.push(
                    tip(
                        kentos_ui::widget::text_field("1", &w.exaggeration)
                            .on_input(|t| msg(Event::Exaggeration(t)))
                            .width(typography::scaled(46.0)),
                        Tip::new(EXAGGERATION).body(
                            "Yükseklikler bu kat büyük gösterilir, sahnenin en alçak noktasından; yüzeylerin kotları değişmez.",
                        ),
                        tooltip::Position::Bottom,
                    ),
                );
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
            }
        }
        let mut end = Row::new().spacing(1).align_y(Center);
        if w.building.is_none() {
            end = end.push(strip::labelled(
                crate::icons::from_web(Some("gltfExport")),
                EXPORT,
                "glTF ver: arazi yüzeylerini ve koridorları bir glTF dosyasına yazar.",
                Some(msg(Event::Export)),
            ));
        }
        if apart {
            if w.building.is_none() {
                end = end.push(strip::divider());
            }
            end = end.push(crate::windows::return_button(SURFACE));
        }
        strip::surface(
            row![Row::with_children(left).spacing(1).align_y(Center).width(Fill).wrap().vertical_spacing(4), end]
                .spacing(6)
                .align_y(Center),
        )
    }

    /// The scene on its sky, the picked point ringed, the north arrow at the top right and the legend at the bottom
    /// left (crates/model-view-ui's look, the IFC viewer's too).
    fn scene3d_scene<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let view = SceneView {
            scene: w.scene.clone(),
            camera: w.camera(),
            exaggeration: w.exaggeration(),
            styles: w.styles(),
            light: w.light(),
            planes: Vec::new(),
            marked: w.picked.as_ref().map(|p| w.exaggeration().shown(p.point)),
            seen: w.seen.clone(),
            pending: w.pending.clone(),
        };
        let swatches: Vec<(&str, [f32; 4])> = match &w.building {
            Some(b) => b.legend(),
            None => PARTS.iter().map(|(name, c)| (*name, [c[0], c[1], c[2], 1.0])).collect(),
        };
        look::framed(
            scene::view(view, gesture),
            look::compass(&w.orbit, msg(Event::North)),
            look::legend(&swatches, w.legend, msg(Event::Legend)),
            sky(self.canvas()),
        )
    }

    /// The status bar: the point clicked and what it is on (else what a click does), then at the right how much the
    /// scene shows, the objects with findings and the mouse's help.
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
                Readout::new(label::body(if w.building.is_some() {
                    "Bir nesneye tıklayın: planda seçilir, adı ve yeri burada yazar."
                } else {
                    "Bir noktaya tıklayın: yeri ve neyin üstünde olduğu burada yazar."
                })
                .style(muted))
                .icon(Icon::Crosshair),
            ),
        };
        bar = bar.spacer();
        let things = match &w.building {
            Some(b) => b.counted(),
            None => format!("{} yüzey ve koridor", w.scene.sources()),
        };
        bar = bar.push(Readout::new(label::body(things.clone())).tip(Tip::new(things).body(format!(
            "Sahne {} üçgen, {} karodur; yalnız ekranda görünen karolar çizilir.",
            w.scene.triangles(),
            w.scene.contents()
        ))));
        if let Some(found) = w.building.as_ref().map(|b| b.found_shown()).filter(|n| *n > 0) {
            bar = bar.separator().push(
                Readout::new(label::body(format!("{found} nesnede bulgu")).style(|t: &Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(t).danger),
                }))
                .icon(Icon::Warning)
                .tip(Tip::new("Denetim bulguları").body(
                    "Denetim'de seçilen bulguların nesneleri kırmızıdır; satır seçilmemişse listelenen bütün bulguların. Çizim değişince bulgular Denetle'ye dek gösterilmez.",
                )),
            );
        }
        bar.separator()
            .push(Readout::new(label::body("Fare")).icon(Icon::Help).tip(Tip::new("Fareyle gezinme").body(
                "Sürükleyin: döndürür. Sağ tuşla ya da Shift ile sürükleyin: kaydırır. Tekerlek: yaklaştırır, uzaklaştırır. Tıklayın: noktayı ve nesneyi seçer.",
            )))
            .into()
    }
}

#[cfg(test)]
impl App {
    /// What a picture of 3B görünüm shows: its own window while it is open beside the plan, else the main window.
    pub(crate) fn scene3d_shown(&self) -> Element<'_, Message> {
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
    })
}

/// The sky over the drawing area's ground (Görünüm › Çizim zemini): towards the drawing area's own blue (its window
/// box's, the same on every ground) at the zenith; the high-contrast ground stays black. The IFC viewer's too.
pub(crate) fn sky(canvas: Canvas) -> Background {
    let [r, g, b, _] = crate::viewport::palette(canvas).background.0;
    let ground = Color::from_rgb8(r, g, b);
    look::sky(ground, (canvas != Canvas::Black).then(|| crate::viewport::mark_colors(canvas).window))
}

fn muted(t: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(Tokens::of(t).muted) }
}
