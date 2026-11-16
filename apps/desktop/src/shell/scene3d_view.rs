//! 3B görünüm's look (DESIGN.md §7.20; docs/adr/8060 §9, docs/adr/8067 §9): the model is what the window is for, so
//! the scene takes all the room between two thin bands in the main window's own vocabulary. Over it, one tool strip:
//! the camera (Görünüş ▾ with the four views, Sığdır), then what shows (a building's Katlar ▾ and Mahaller; a road's
//! exaggeration and terrain switches), then at its end what the window does (glTF ver, Ana pencereye al). In the
//! scene, the north arrow at the top right turns with the camera and turns it north when pressed (DESIGN.md §8), and
//! the legend folds at the bottom left. Under it, a status bar: the point clicked and what it is on, how much the
//! scene shows (its triangles and tiles in the tip), the objects the check found something about, the mouse's help.
//!
//! Beside the plan it is the whole of an operating-system window (shell/windows/, docs/adr/8060); over the drawing
//! it is a window like the others, its header carrying Ayrı pencerede aç. The strip, the scene and the bar are the
//! same in both. The sky is the drawing area's own ground (Görünüm › Çizim zemini) a little bluer at the zenith, so
//! the model reads as the plan's and its walls stand out from the ground.

use iced::keyboard::key::Named;
use iced::widget::{Row, canvas, column, container, mouse_area, row, shader, space, stack, tooltip};
use iced::{Background, Border, Center, Color, Element, Fill, Gradient, Radians, Task, Theme, gradient, mouse};
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::legend::{self, Symbol};
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::widget::{Compass, Dialog as Frame, Legend, Menu, StatusBar, Tip, tip};

use super::{
    APART, CUT, EXAGGERATION, EXPORT, Event, FIT, FLAT, LEGEND, Marker, NORTH, PARTS, Program, RETURN, SEE_THROUGH,
    TITLE, VIEWS, Window, msg,
};
use crate::app::{App, Dialog, Message};
use crate::keys::KeyPress;
use crate::style::strip;
use crate::viewport::Canvas;
use crate::windows::{Focus, Surface};

/// The surface this window is (shell/windows/).
const SURFACE: Surface = Surface::Scene3d;
/// The legend's width, at 12 px text: its longest name (Bulgulu mahal) and its symbol.
const LEGEND_WIDTH: f32 = 150.0;

impl App {
    /// Whether 3B görünüm opens in a window of its own: the user's last choice, else beside the plan.
    pub(crate) fn scene3d_is_apart(&self) -> bool {
        self.windows.memory.apart(SURFACE)
    }

    /// Whether its own window has something to show: apart, and open.
    pub(crate) fn scene3d_window_wanted(&self) -> bool {
        self.scene3d_is_apart() && self.scene3d_window.is_some()
    }

    /// Its own window's title, before the program's name.
    pub(crate) fn scene3d_window_title(&self) -> String {
        TITLE.to_owned()
    }

    /// The window just made, shown: in its own window (brought to the front when it is open already, made by the
    /// windows following the state otherwise), or over the drawing.
    pub(crate) fn show_scene3d(&mut self) {
        if self.scene3d_is_apart() {
            if self.dialog == Some(Dialog::Scene3d) {
                self.dialog = None;
            }
            self.windows.ask_focus(Focus::Surface(SURFACE));
        } else {
            self.dialog = Some(Dialog::Scene3d);
        }
    }

    /// Into a window of its own, or back over the drawing; the choice is kept for its next opening, in the next run
    /// too. Back over the drawing waits while another window is open there.
    pub(super) fn scene3d_set_apart(&mut self, apart: bool) {
        if !apart && self.dialog.is_some_and(|d| d != Dialog::Scene3d) {
            self.warn("Ana pencerede açık bir pencere var: önce onu kapatın, sonra 3B görünümü ana pencereye alın.");
            return;
        }
        self.windows.memory.set_apart(SURFACE, apart);
        if apart {
            if self.dialog == Some(Dialog::Scene3d) {
                self.dialog = None;
            }
        } else if self.scene3d_window.is_some() {
            self.dialog = Some(Dialog::Scene3d);
            self.windows.ask_focus(Focus::Main);
        }
    }

    /// A key its own window's widgets did not take: Esc closes it, Tab goes round its fields.
    pub(crate) fn scene3d_window_key(&mut self, press: &KeyPress) -> Task<Message> {
        match press.named() {
            Some(Named::Tab) if !press.modifiers.control() && !press.modifiers.alt() => {
                kentos_ui::focus::next(press.modifiers.shift())
            }
            Some(Named::Escape) => self.scene3d_event(Event::Close),
            _ => Task::none(),
        }
    }

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
        Frame::new(TITLE)
            .header_button(
                crate::icons::from_web(Some("externalOpen")),
                Tip::new(APART).body(
                    "3B görünümü kendi penceresinde açar: pencereyi başka bir ekrana taşıyıp çizerken modeli izleyebilirsiniz. Sonraki açılışlar da ayrı pencerede olur.",
                ),
                msg(Event::Apart(true)),
            )
            .fill(viewer)
            .width(1180.0)
            .modal(msg(Event::Close))
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
        let (azimuth, elevation) = (w.azimuth, w.elevation);
        let face = VIEWS.iter().find(|v| same_view(azimuth, elevation, v.1, v.2)).map_or("Görünüş", |v| v.0);
        let mut left: Vec<Element<'a, Message>> = vec![
            strip::menu(
                crate::icons::from_web(Some("scene3dView")),
                face,
                "Görünüş: modele hangi yönden bakıldığı. Döndürünce bakış serbesttir; bir görünüş seçmek modeli o yönden sığdırır.",
                move || views_menu(azimuth, elevation),
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
            end = end.push(strip::icon_button(
                crate::icons::from_web(Some("windowReturn")),
                &format!("{RETURN}: 3B görünümü yeniden ana pencerede, çizimin üstünde açar. Sonraki açılışlar da orada olur."),
                Some(msg(Event::Apart(false))),
            ));
        }
        strip::surface(
            row![Row::with_children(left).spacing(1).align_y(Center).width(Fill).wrap().vertical_spacing(4), end]
                .spacing(6)
                .align_y(Center),
        )
    }

    /// The scene on its sky, the picked point ringed, the north arrow at the top right and the legend at the bottom
    /// left. The legend and the arrow take the pointer over themselves: a drag begun on them does not turn the model.
    fn scene3d_scene<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let program = Program {
            scene: w.scene.clone(),
            camera: w.camera(),
            exaggeration: w.exaggeration(),
            styles: w.styles(),
            light: w.light(),
            seen: w.seen.clone(),
            pending: w.pending.clone(),
        };
        let marker = canvas(Marker { camera: w.camera(), point: w.picked.as_ref().map(|p| w.exaggeration().shown(p.point)) })
            .width(Fill)
            .height(Fill);
        let north = tip(
            Compass::new(north_on_screen(w.azimuth)).size(typography::scaled(38.0)).on_press(msg(Event::North)),
            Tip::new("Kuzey").body(format!(
                "Model {} görülüyor, {:.0}° yukarıdan. Tıklayın: kamera kuzeye bakar.",
                seen_from(w.azimuth),
                w.elevation
            )),
            tooltip::Position::Left,
        );
        let legend = mouse_area(self.scene3d_legend(w)).interaction(mouse::Interaction::Idle);
        let sky = sky(self.canvas());
        container(stack![
            shader(program).width(Fill).height(Fill),
            marker,
            container(north).align_right(Fill).align_top(Fill).padding(10),
            container(legend).align_left(Fill).align_bottom(Fill).padding(10),
        ])
        .width(Fill)
        .height(Fill)
        .style(move |_: &Theme| container::Style { background: Some(sky), ..container::Style::default() })
        .into()
    }

    /// The legend: the parts the scene shows, each a face in its colour as a face toward the light shows it, its edge
    /// a shade darker (a see-through part: its tint inside its colour's edge); its heading folds it.
    fn scene3d_legend<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let swatches: Vec<(&str, [f32; 4])> = match &w.building {
            Some(b) => b.legend(),
            None => PARTS.iter().map(|(name, c)| (*name, [c[0], c[1], c[2], 1.0])).collect(),
        };
        let mut l = Legend::new().title(LEGEND).collapsible(!w.legend, msg(Event::Legend)).width(typography::scaled(LEGEND_WIDTH));
        for (name, c) in swatches {
            let shown = screen(c);
            let symbol = if c[3] < 1.0 {
                Symbol::Area { fill: Color { a: c[3].max(0.25), ..shown }, stroke: shown }
            } else {
                Symbol::Area { fill: shown, stroke: mix(shown, Color::BLACK, 0.35) }
            };
            l = l.item(symbol, name);
        }
        legend::frame(l)
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
            None => format!("{} yüzey ve koridor", w.scene.sources),
        };
        bar = bar.push(Readout::new(label::body(things.clone())).tip(Tip::new(things).body(format!(
            "Sahne {} üçgen, {} karodur; yalnız ekranda görünen karolar çizilir.",
            w.scene.triangles,
            w.scene.chunked.contents.len()
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
            Some(w) if self.scene3d_window.is_some() => self.window_view(w.id),
            _ => self.view(),
        }
    }
}

/// The view presets, the one the camera is at chosen, and Kuzeye bak.
fn views_menu(azimuth: f64, elevation: f64) -> Menu<Message> {
    let mut m = Menu::new();
    for (i, (name, a, e)) in VIEWS.iter().enumerate() {
        m = m.radio(*name, same_view(azimuth, elevation, *a, *e), msg(Event::View(i)));
    }
    m.separator().item(NORTH, msg(Event::North))
}

/// Whether the camera is at a preset's angles (to half a degree).
fn same_view(azimuth: f64, elevation: f64, a: f64, e: f64) -> bool {
    let d = (azimuth - a).rem_euclid(360.0);
    (d < 0.5 || d > 359.5) && (elevation - e).abs() < 0.5
}

/// Where north is on the screen for a camera at `azimuth` (degrees from east, counterclockwise, the camera's side of
/// its target): clockwise from up, as the north arrow takes it. The camera looks along −(cos a, sin a); north is
/// (cos a, −sin a) in the screen's right and ahead.
pub(super) fn north_on_screen(azimuth: f64) -> f32 {
    let a = azimuth.to_radians();
    a.cos().atan2(-a.sin()).to_degrees() as f32
}

/// The side the model is seen from, in words (“kuzeydoğudan”), from the camera's azimuth.
pub(super) fn seen_from(azimuth: f64) -> &'static str {
    const SIDES: [&str; 8] =
        ["doğudan", "kuzeydoğudan", "kuzeyden", "kuzeybatıdan", "batıdan", "güneybatıdan", "güneyden", "güneydoğudan"];
    let k = ((azimuth.rem_euclid(360.0) + 22.5) / 45.0).floor() as usize % 8;
    SIDES[k]
}

/// A part's colour as a face toward the light shows it: the renderer writes the part's colour times the face's light
/// to the screen as it is (the program draws with the web's colours, no gamma step), so a face lit fully is the colour
/// itself.
fn screen(c: [f32; 4]) -> Color {
    Color::from_rgb(c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0))
}

/// The sky over the drawing area's ground: a little bluer at the zenith than at the foot, darker above on a dark
/// ground and a light one alike; the high-contrast ground stays black.
fn sky(canvas: Canvas) -> Background {
    let [r, g, b, _] = crate::viewport::palette(canvas).background.0;
    let ground = Color::from_rgb8(r, g, b);
    if canvas == Canvas::Black {
        return Background::Color(ground);
    }
    // The drawing area's blue (its window box's), the same on every ground.
    let blue = crate::viewport::mark_colors(canvas).window;
    let light = ground.r + ground.g + ground.b > 1.5;
    let (zenith, foot) = if light { (0.16, 0.03) } else { (0.03, 0.12) };
    // From the top (the stop at 0) down: the gradient's angle is a half turn from upward.
    Background::Gradient(Gradient::Linear(
        gradient::Linear::new(Radians(std::f32::consts::PI)).add_stop(0.0, mix(ground, blue, zenith)).add_stop(1.0, mix(ground, blue, foot)),
    ))
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::from_rgb(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t)
}

fn muted(t: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(Tokens::of(t).muted) }
}

#[cfg(test)]
mod tests {
    use super::{north_on_screen, seen_from};

    #[test]
    fn the_north_arrow_and_the_side_follow_the_camera() {
        // From the north-east the model is seen with north behind and to the right: down right on the screen.
        assert!((north_on_screen(45.0) - 135.0).abs() < 1e-4);
        // From the south, looking north (Üst, Kuzeye bak): north is up.
        assert!(north_on_screen(-90.0).abs() < 1e-4);
        // From the south-west, looking north-east: north is up and to the left.
        assert!((north_on_screen(225.0) + 45.0).abs() < 1e-4);
        assert_eq!(seen_from(45.0), "kuzeydoğudan");
        assert_eq!(seen_from(225.0), "güneybatıdan");
        assert_eq!(seen_from(-90.0), "güneyden");
        assert_eq!(seen_from(359.0), "doğudan");
    }
}
