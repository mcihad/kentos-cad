//! Pictures of the sheet mode on the desktop for the owner to look at: the
//! window as the desktop shows it (the ribbon with its contextual Pafta tab,
//! the sheets and items, the paper, the inspector, the tabs and the status
//! bar), in the Pafta (light) and Grafit (dark) themes, 1440 and 1100 pixels
//! wide. Maps are the made-up town of `DemoMaps`. Not a test of correctness
//! and not run by default; writes PNGs to `.run/shots/sheet-desktop/`:
//!
//! ```text
//! cargo test -p kentos-sheet-ui screens -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use iced::widget::{Stack, column};
use iced::{Element, Fill, Size, Theme};
use kentos_sheet::kinds::GroundPoint;
use kentos_sheet::profile::Capabilities;
use kentos_sheet_ui::gallery::GalleryMessage;
use kentos_sheet_ui::message::FrameField;
use kentos_sheet_ui::{Context, DemoMaps, Designer, InspectorTab, Message, StageEvent};
use kentos_ui::theme::{self, Accent, Mode};
use kentos_ui::widget::StatusBar;
use kentos_ui::widget::ribbon::{AppButton, Ribbon};

struct Window {
    d: Designer,
    mode: Mode,
}

/// The desktop's tabs before the contextual one (its Giriş … Yardım).
const TABS: [&str; 7] = [
    "Giriş",
    "Çiz",
    "Değiştir",
    "Açıklama",
    "Analiz",
    "Görünüm",
    "Yardım",
];

impl Window {
    fn view(&self) -> Element<'_, Message> {
        let mut ribbon = Ribbon::new().application(AppButton::new("Kent").tail("OS"));
        for t in TABS {
            ribbon = ribbon.tab(t, false, Message::Model);
        }
        ribbon = ribbon.contextual_tab(
            kentos_sheet_ui::ribbon::TAB,
            self.d.book().sheets.len().to_string(),
            true,
            Message::ZoomPage,
        );
        for g in self.d.ribbon_groups(|m| m) {
            ribbon = ribbon.group(g);
        }
        let mut status = StatusBar::new();
        for c in self.d.status_cells() {
            status = status.push(c);
        }
        let base = column![
            ribbon,
            self.d.body(std::rc::Rc::new(DemoMaps)),
            self.d.tabs(),
            status
        ]
        .width(Fill)
        .height(Fill);
        match self.d.window(std::rc::Rc::new(DemoMaps)) {
            Some(w) => Stack::with_children([base.into(), w]).into(),
            None => base.into(),
        }
    }

    fn theme(&self) -> Theme {
        theme::theme(self.mode, Accent::Blue)
    }
}

fn shot(w: &mut Window, size: Size, out: &Path, name: &str) {
    // The software renderer: the same pixels on every machine, no GPU needed (a busy
    // gallery overflowed the GPU snapshot's staging buffer: iced_wgpu's headless frame).
    let mut snapshot = kentos_ui::snapshot::Snapshot::software(size).expect("a renderer");
    let mut update = |w: &mut Window, m: Message| {
        let _ = w.d.update(m);
    };
    snapshot.settle(w, Window::view, &mut update);
    let _ = snapshot.render(w.view(), &w.theme());
    let file = out.join(format!("{name}.png"));
    snapshot
        .render(w.view(), &w.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
}

fn context(workspace: Option<kentos_contracts::Workspace>) -> Context {
    Context {
        workspace,
        capabilities: Capabilities {
            georeferenced: true,
            attribute_layers: true,
            plot_scale: Some(1000),
        },
        project: kentos_sheet::display::ProjectInfo {
            name: "Kızılay Mahallesi Uygulama İmar Planı".into(),
            user: "Ayşe Yılmaz".into(),
            date: "2026-10-02".into(),
            crs_name: "TUREF / TM33".into(),
        },
        center: Some(GroundPoint {
            x: 484_250.0,
            y: 4_418_600.0,
        }),
        ..Context::default()
    }
}

/// A designer with a sheet of `template` in front, the item named `chosen` chosen.
fn with(
    template: &str,
    workspace: Option<kentos_contracts::Workspace>,
    chosen: Option<&str>,
) -> Designer {
    let mut d = Designer::new(context(workspace));
    let t = kentos_sheet::template::system_template(template)
        .expect("a system template")
        .clone();
    assert!(d.use_template(&t, None));
    if let Some(name) = chosen {
        let id = d
            .open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| i.name == name)
            .map(|i| i.id.clone())
            .expect("the item");
        d.update(Message::Select(vec![id]));
    }
    d
}

fn out_dir() -> PathBuf {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    out
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let fonts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/assets/fonts/drawing");
    assert!(
        kentos_sheet_ui::fonts::load_dir(&fonts) > 0,
        "the drawing's faces in {}",
        fonts.display()
    );
    kentos_ui::theme::motion::set_reduced(true);
    let out = out_dir();
    // The Pafta (light) theme first, then Grafit (dark).
    for (mode, tag) in [(Mode::Light, "pafta"), (Mode::Dark, "grafit")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 720.0)] {
            let size = Size::new(width, height);
            let px = width as u32;
            // 1. The general sheet, its title chosen: handles, the inspector's Öğe tab.
            let mut w = Window {
                d: with("sys:genel-a3-yatay", None, Some("Başlık")),
                mode,
            };
            shot(&mut w, size, &out, &format!("{tag}-{px}-1-pafta"));
            // 2. A drag under way: the item where it goes, the snapping lines and distances.
            let title =
                w.d.open_sheet()
                    .unwrap()
                    .items
                    .iter()
                    .find(|i| i.name == "Alt başlık")
                    .unwrap()
                    .clone();
            let c = title.frame.center();
            let stage = Size::new(width - 252.0 - 316.0 - 2.0, height - 220.0);
            w.d.update(Message::Stage(StageEvent::Press {
                at: c,
                px: iced::Point::ORIGIN,
                size: stage,
                shift: false,
                ctrl: false,
                alt: false,
            }));
            for k in 1..=3 {
                let at = [
                    c[0] - 2_000.0 * f64::from(k),
                    c[1] + 7_400.0 * f64::from(k) / 3.0,
                ];
                w.d.update(Message::Stage(StageEvent::Move {
                    at,
                    size: stage,
                    shift: false,
                    ctrl: false,
                    alt: false,
                }));
            }
            shot(&mut w, size, &out, &format!("{tag}-{px}-2-surukle"));
            // 3. The page: paper, margins, the layout variant in force.
            let mut w = Window {
                d: with("sys:genel-a3-yatay", None, None),
                mode,
            };
            w.d.update(Message::InspectorTab(InspectorTab::Page));
            shot(&mut w, size, &out, &format!("{tag}-{px}-3-sayfa"));
            // 4. The zoning plan on its A1, the preflight's findings.
            let mut w = Window {
                d: with("sys:imar-plani", None, None),
                mode,
            };
            w.d.update(Message::InspectorTab(InspectorTab::Preflight));
            shot(&mut w, size, &out, &format!("{tag}-{px}-4-on-denetim"));
            // 5. A CAD project: the technical sheet, the mode's own names and tools; the map chosen.
            let mut w = Window {
                d: with(
                    "sys:cad-teknik",
                    Some(kentos_contracts::Workspace::Cad),
                    Some("Görünüm penceresi"),
                ),
                mode,
            };
            shot(&mut w, size, &out, &format!("{tag}-{px}-5-cad"));
            // 6. The template gallery.
            let mut w = Window {
                d: with("sys:genel-a3-yatay", None, None),
                mode,
            };
            w.d.update(Message::Gallery(GalleryMessage::Open));
            w.d.update(Message::Gallery(GalleryMessage::Pick(
                "sys:imar-plani".into(),
            )));
            shot(&mut w, size, &out, &format!("{tag}-{px}-6-galeri"));
            // 7. A turned map frame: its town, its karelaj and its frame cut to the turned box.
            let mut w = Window {
                d: with("sys:genel-a3-yatay", None, Some("Harita")),
                mode,
            };
            w.d.update(Message::Frame(FrameField::Rotation, 20.0));
            shot(&mut w, size, &out, &format!("{tag}-{px}-7-donuk-harita"));
        }
    }
}

/// docs/adr/0206: a map's Çizim ölçeğini al, Görünüme sığdır and Görünümden
/// al (the project at 1:2500, the map at 1:1000); the aplikasyon sketch's
/// coordinate list reading a parcel taken from the drawing's choice, what it
/// gives said, its headings given and drawn so. Tall windows, so that the
/// inspector shows them; `.run/shots/sheet-desktop/*-0206-*`:
///
/// ```text
/// cargo test -p kentos-sheet-ui --test screens adr_0206 -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn adr_0206_screens() {
    use kentos_sheet::display::{CoordPoint, CoordinateInput};
    let fonts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/assets/fonts/drawing");
    assert!(kentos_sheet_ui::fonts::load_dir(&fonts) > 0);
    kentos_ui::theme::motion::set_reduced(true);
    let out = out_dir();
    for (mode, tag) in [(Mode::Light, "pafta"), (Mode::Dark, "grafit")] {
        let size = Size::new(1440.0, 1300.0);
        // The map at 1:1000 in a project now at 1:2500, the drawing area 1 200 × 600 m.
        let mut d = with("sys:genel-a3-yatay", None, Some("Harita"));
        d.set_context(Context {
            capabilities: Capabilities {
                plot_scale: Some(2500),
                ..context(None).capabilities
            },
            view_size: Some([1_200.0, 600.0]),
            ..context(None)
        });
        let mut w = Window { d, mode };
        shot(&mut w, size, &out, &format!("{tag}-0206-1-harita-olcek"));
        // The coordinate list: a parcel's four corners, its area, its headings.
        let mut d = Designer::new(Context {
            layers: vec![
                ("parsel".into(), "Parsel".into()),
                ("bina".into(), "Bina".into()),
            ],
            ..context(Some(kentos_contracts::Workspace::Cad))
        });
        let t = kentos_sheet::template::system_template("sys:aplikasyon-krokisi")
            .expect("a system template")
            .clone();
        assert!(d.use_template(&t, None));
        let list = d
            .open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| i.name == "Koordinat listesi")
            .map(|i| i.id.clone())
            .expect("the list");
        d.update(Message::Select(vec![list.clone()]));
        d.update(Message::CoordObjects(vec![
            "0192f6a0-0000-7000-8000-000000000101".into(),
        ]));
        for (key, text) in [
            ("point", "Nokta No"),
            ("east", "Sağa (Y)"),
            ("north", "Yukarı (X)"),
        ] {
            d.update(Message::CoordHeading(key, text.into()));
        }
        let corners = [
            (484_210.25, 4_418_570.5),
            (484_262.75, 4_418_574.0),
            (484_258.5, 4_418_628.25),
            (484_206.0, 4_418_624.75),
        ];
        let mut inputs = kentos_sheet_ui::empty_inputs();
        inputs.coordinates.push(CoordinateInput {
            item: list,
            points: corners
                .iter()
                .enumerate()
                .map(|(i, (x, y))| CoordPoint {
                    name: Some(format!("{}", 101 + i)),
                    x: *x,
                    y: *y,
                    z: None,
                })
                .collect(),
            closed: true,
            area: Some(2_826.94),
            objects: Some(1),
            missing: None,
        });
        d.set_data(inputs);
        let mut w = Window { d, mode };
        shot(
            &mut w,
            size,
            &out,
            &format!("{tag}-0206-2-koordinat-listesi"),
        );
    }
}
