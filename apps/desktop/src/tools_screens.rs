//! Pictures of the drawing and editing tools of docs/adr/0140, for the owner
//! to look at: the ribbon tabs that hold them, then each tool at work, in
//! the dark and the light theme, at 1440×900 and 1100×650. Not a test of
//! correctness and not run by default; writes `.run/shots/arac-*`:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=serit-degistir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! `KENTOS_SHOTS_ONLY` (comma separated) keeps only the named pictures.

use std::path::PathBuf;

use iced::{Point, Size};
use kentos_ui::snapshot::{Input, Snapshot};

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, find_text, find_texts};

/// One picture: its name and what brings the app to it.
pub(crate) type Scene = (&'static str, fn(&mut App));

/// What a click on the window opens over a scene (a menu).
pub(crate) type Pointing = fn(&mut Snapshot, &mut App);

/// A picture that needs the pointer at the window itself: what brings the app
/// to it, then what a click opens over it (a menu).
pub(crate) type Pointed = (&'static str, fn(&mut App), Pointing);

/// Any picture: its name, what brings the app to it, and the click over it, if any.
type Shot = (&'static str, fn(&mut App), Option<Pointing>);

/// Where a caption is drawn: its middle, or the point just right of it (a
/// small split button's arrow). A caption that is also the ribbon's (the layer
/// box says “Parsel”) is looked for in the docked panels first: the right of
/// the window, under the ribbon.
fn caption_at(snapshot: &mut Snapshot, app: &App, caption: &str, arrow: bool) -> Point {
    let docked = find_texts(snapshot, app, caption)
        .into_iter()
        .find(|at| at.x > 900.0 && at.y > 150.0);
    let at = docked
        .or_else(|| find_text(snapshot, app, caption))
        .unwrap_or_else(|| panic!("{caption} is on screen"));
    if arrow {
        Point::new(at.x + at.width + 10.0, at.center_y())
    } else {
        at.center()
    }
}

/// A split button's list opened, as its key tip opens it (ribbon_keys.rs): by the
/// button's own key, the tools' family or the tool with methods.
pub(crate) fn open_split(snapshot: &mut Snapshot, app: &mut App, key: &str) {
    open_menu_id(snapshot, app, crate::ribbon_keys::split_menu_id(key));
}

/// A ribbon drop-down's menu opened, as its key tip opens it (ribbon_keys.rs): by its label.
pub(crate) fn open_ribbon_menu(snapshot: &mut Snapshot, app: &mut App, label: &str) {
    open_menu_id(snapshot, app, crate::ribbon_keys::menu_id(label));
}

/// A folded panel's menu opened (a narrow window), as its key tip opens it: by the panel's title.
pub(crate) fn open_folded(snapshot: &mut Snapshot, app: &mut App, panel: &str) {
    open_menu_id(snapshot, app, crate::ribbon_keys::folded_id(panel));
}

/// Keys pressed over the window, one after another (a menu's arrows and Enter).
pub(crate) fn press_keys(
    snapshot: &mut Snapshot,
    app: &mut App,
    keys: &[iced::keyboard::key::Named],
) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    for key in keys {
        snapshot.input(app, App::view, &mut update, Input::Key(*key));
    }
}

/// A menu opened by its id, as the key tips open them.
fn open_menu_id(snapshot: &mut Snapshot, app: &mut App, id: iced::advanced::widget::Id) {
    use iced::futures::StreamExt as _;
    let task: iced::Task<Message> = kentos_ui::widget::context_menu::open_menu(id);
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(operation) = action {
                snapshot.operate(app.view(), operation);
            }
        }
    }
}

/// A click (or a right click) on a caption of the window.
pub(crate) fn press_caption(
    snapshot: &mut Snapshot,
    app: &mut App,
    caption: &str,
    right: bool,
    arrow: bool,
) {
    let at = caption_at(snapshot, app, caption, arrow);
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    let input = if right {
        Input::RightClick(at)
    } else {
        Input::Click(at)
    };
    snapshot.input(app, App::view, &mut update, input);
}

/// Pictures of menus the new commands are in (docs/adr/0141): the layer tree's, and Seç ▾.
fn pointed_scenes() -> Vec<Pointed> {
    let mut all = crate::text_scenes::pointed();
    all.extend(crate::style_scenes::pointed());
    all.extend(crate::leader_scenes::pointed());
    all.extend(crate::dimension_scenes::pointed());
    all.extend(crate::topology_scenes::pointed());
    all.extend(crate::hatch_scenes::pointed());
    all.extend(crate::selection_scenes::pointed());
    all.extend(crate::point_calc_scenes::pointed());
    all.extend(crate::raster_scenes::pointed());
    all.extend(crate::cloud_scenes::pointed());
    all.extend(crate::service_scenes::pointed());
    all.extend(crate::surface_scenes::pointed());
    all.extend(crate::interpolation_scenes::pointed());
    all.extend(crate::raster_ops_scenes::pointed());
    all.extend(crate::raster_vector_scenes::pointed());
    all.extend(crate::hydrology_scenes::pointed());
    all.extend(crate::distance_scenes::pointed());
    all.extend(crate::suitability_scenes::pointed());
    all.extend(crate::stats_scenes::pointed());
    all.extend(crate::chart_scenes::pointed());
    all.extend(crate::remote_scenes::pointed());
    all.extend(crate::multidim_scenes::pointed());
    all.extend(crate::label_scenes::pointed());
    all.extend(crate::renderer_scenes::pointed());
    all.extend(crate::attachment_scenes::pointed());
    all.extend(crate::address_scenes::pointed());
    all.extend(crate::history_scenes::pointed());
    all.extend(crate::method_scenes::pointed());
    all.extend(crate::alias_profile_scenes::pointed());
    let menus: Vec<Pointed> = vec![
        (
            "katman-menu-katman",
            |_| {},
            |s, app| {
                press_caption(s, app, "Parsel", true, false);
            },
        ),
        (
            "katman-menu-grup",
            |_| {},
            |s, app| {
                press_caption(s, app, "Kadastro", true, false);
            },
        ),
        (
            "secim-ailesi",
            |app| app.tab = "home",
            |s, app| open_split(s, app, "select"),
        ),
        // docs/adr/0142: Kot ver's three ways, the list of its split button (Düzenle, a CBS project's).
        (
            "kot-ver-yontemler",
            |app| app.tab = "edit",
            |s, app| open_split(s, app, "setElevation"),
        ),
    ];
    all.extend(menus);
    all
}

/// The ribbon tabs the new tools sit in: a CAD project's Değiştir, a CBS
/// project's Düzenle and Harita.
fn ribbon_scenes() -> Vec<Scene> {
    vec![
        ("serit-degistir", |app| {
            crate::files_testing::make_cad(app);
            app.tab = "modify";
        }),
        ("serit-duzenle", |app| app.tab = "edit"),
        // docs/adr/0165's 12 Oct addendum: a CAD project's axes stay at the bottom left with 0,0 in the view's middle.
        ("cad-eksen-simgesi", |app| {
            crate::files_testing::make_cad(app);
            app.viewport.camera.fit(
                &kentos_render_wgpu::Bounds {
                    min_x: -60.0,
                    min_y: -40.0,
                    max_x: 60.0,
                    max_y: 40.0,
                },
                24.0,
            );
        }),
        // docs/adr/0165's “ifadeler çizim biriminde” addendum: a label's `$alan` in mm² beside Öznitelikler.
        ("cad-mm-etiket", mm_labels),
        // The 13 Ekim addendum: Katman stili reads `$alan` in mm² too, and what it made draws the rooms.
        ("cad-mm-stil-kategorili", mm_style_categories),
        ("cad-mm-stil-aralikli", mm_style_classes),
        ("cad-mm-stil-kurallar", mm_style_rules),
        ("cad-mm-stil-cizim", mm_style_drawn),
        ("serit-harita", |app| app.tab = "map"),
        // docs/adr/0141: two zooms in and a step back, so that Önceki and Sonraki görünüm are both on.
        ("serit-gorunum", |app| {
            app.tab = "view";
            for id in ["view.zoomIn", "view.zoomIn", "view.previous"] {
                let _ = app.update(Message::Run(id));
            }
        }),
    ]
}

/// A local CAD project in millimetres: a 2 × 1.5 m parcel and a 1 × 1 m one,
/// their layer labelled with `$alan`, the first selected; its label and
/// Öznitelikler both read mm² (docs/adr/0165 §2, the 12 Ekim addendum).
fn mm_labels(app: &mut App) {
    crate::files_testing::make_cad(app);
    let doc = app.document.as_mut().expect("a drawing");
    let settings = kentos_contracts::ProjectSettings {
        srid: 0,
        drawing_unit: Some(kentos_contracts::DrawingUnit::Mm),
        // An architect's scale: the lines' paper widths stay thin around two-metre rooms.
        plot_scale: 50.0,
        ..doc.model.settings().clone()
    };
    doc.model.set_settings(settings);
    let layer = doc.model.layers().active().to_owned();
    let mut slots = Vec::new();
    for (x, w, h) in [(0.0, 2.0, 1.5), (2.5, 1.0, 1.0)] {
        let p = |x: f64, y: f64| kentos_contracts::Vec2 { x, y };
        let base = kentos_contracts::EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer.clone(),
            color: None,
            attrs: [(
                "Ad".to_owned(),
                if w > 1.5 { "Salon" } else { "Oda" }.to_owned(),
            )]
            .into(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        };
        slots.push(
            doc.model
                .add(kentos_contracts::Entity::Polygon(
                    kentos_contracts::PathEntity {
                        base,
                        pts: vec![p(x, 0.0), p(x + w, 0.0), p(x + w, h), p(x, h)],
                        ms: None,
                        bulges: None,
                        holes: None,
                        zs: None,
                        parts: None,
                        closed: false,
                        widths: None,
                    },
                ))
                .expect("a slot"),
        );
    }
    // A plain style: the sample's parcel band would hide two-metre rooms.
    let mut style = kentos_domain::default_style();
    style.label = Some(
        serde_json::from_str(
            r#"{"placement":"center","size":12,"text":"Ad || ' ' || $alan || ' mm²'"}"#,
        )
        .expect("a label style"),
    );
    doc.model.set_layer_style(&layer, style, "Etiketler");
    crate::elevation_scenes::select(app, &slots[..1]);
    crate::elevation_scenes::panel_room(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -0.5,
            min_y: -0.6,
            max_x: 4.0,
            max_y: 2.1,
        },
        24.0,
    );
}

/// A local CAD project in millimetres whose layer Odalar holds four rooms, a salon of 3 m²
/// (2 × 1.5 m), a kitchen of 1.5 m², a room of 1 m² and a closet of 0.5 m², each labelled with
/// its name and `$alan` in mm²: the layer's id (docs/adr/0165 §2, the 13 Ekim addendum). Katman
/// stili reads the same `$alan` in mm², so the classes it makes meet the values drawn.
fn mm_rooms(app: &mut App) -> String {
    use kentos_contracts::{DrawingUnit, Entity, EntityBase, PathEntity, ProjectSettings, Vec2};
    crate::files_testing::make_cad(app);
    let doc = app.document.as_mut().expect("a drawing");
    let settings = ProjectSettings {
        srid: 0,
        drawing_unit: Some(DrawingUnit::Mm),
        // An architect's scale: the lines' paper widths stay thin around rooms of a metre.
        plot_scale: 50.0,
        ..doc.model.settings().clone()
    };
    doc.model.set_settings(settings);
    let others: Vec<String> = doc
        .model
        .layers()
        .leaves()
        .into_iter()
        .map(|l| l.id.clone())
        .collect();
    let mut style = kentos_domain::default_style();
    style.label = Some(
        serde_json::from_str(
            r#"{"placement":"center","size":12,"text":"Ad || ' ' || $alan || ' mm²'"}"#,
        )
        .expect("a label style"),
    );
    let layer = doc
        .model
        .add_layer(kentos_domain::NewLayer::layer("Odalar"), None, true)
        .expect("a layer");
    doc.model.set_layer_style(&layer, style, "Etiketler");
    for id in others {
        doc.model.set_layer_visible(&id, false);
    }
    let p = |x: f64, y: f64| Vec2 { x, y };
    for (name, x, w, h) in [
        ("Salon", 0.0, 2.0, 1.5),
        ("Mutfak", 2.5, 1.5, 1.0),
        ("Oda", 4.5, 1.0, 1.0),
        ("Dolap", 6.0, 1.0, 0.5),
    ] {
        let base = EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer.clone(),
            color: None,
            attrs: [("Ad".to_owned(), name.to_owned())].into(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        };
        doc.model
            .add(Entity::Polygon(PathEntity {
                base,
                pts: vec![p(x, 0.0), p(x + w, 0.0), p(x + w, h), p(x, h)],
                ms: None,
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
                closed: false,
                widths: None,
            }))
            .expect("a room");
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -0.4,
            min_y: -0.5,
            max_x: 7.4,
            max_y: 2.0,
        },
        24.0,
    );
    layer
}

/// Katman stili as the owner met it: Kategorili, `$alan`, Değerlerden sınıfla.
fn mm_style_categories(app: &mut App) {
    use crate::style::layer_style::{Event, Field, Kind};
    let layer = mm_rooms(app);
    app.open_layer_style(Some(layer));
    for e in [
        Event::Kind(Kind::Categorized),
        Event::Expr(Field::Categories, "$alan".into()),
        Event::Classify,
    ] {
        let _ = app.update(Message::LayerStyle(e));
    }
}

/// Aralıklı, `$alan`, three classes along the blues, Sınıfla.
fn mm_style_classes(app: &mut App) {
    use crate::style::layer_style::{Event, Field, Kind};
    let layer = mm_rooms(app);
    app.open_layer_style(Some(layer));
    for e in [
        Event::Kind(Kind::Graduated),
        Event::Expr(Field::Classes, "$alan".into()),
        Event::Count("3".into()),
        Event::Ramp("maviler"),
        Event::Graduate,
    ] {
        let _ = app.update(Message::LayerStyle(e));
    }
}

/// Kurallar: larger than 1 200 000 mm² (the salon and the kitchen), and what the rule left.
fn mm_style_rules(app: &mut App) {
    use crate::style::layer_style::{Event, Field, Kind, RuleEdit};
    let layer = mm_rooms(app);
    app.open_layer_style(Some(layer));
    for e in [
        Event::Kind(Kind::Rules),
        Event::Rule(vec![0], RuleEdit::Label("Büyük odalar".into())),
        Event::Expr(Field::Rule(vec![0]), "$alan > 1200000".into()),
        Event::AddElse,
    ] {
        let _ = app.update(Message::LayerStyle(e));
    }
}

/// The rooms drawn by the classes Katman stili made, the window closed: the salon in the darkest
/// of three blues, the kitchen in the middle one, the room and the closet in the lightest.
fn mm_style_drawn(app: &mut App) {
    use crate::style::layer_style::{Event, Field, Kind};
    let layer = mm_rooms(app);
    app.open_layer_style(Some(layer));
    for e in [
        Event::Kind(Kind::Graduated),
        Event::Expr(Field::Classes, "$alan".into()),
        Event::Count("3".into()),
        Event::Ramp("maviler"),
        Event::Graduate,
        Event::Done,
    ] {
        let _ = app.update(Message::LayerStyle(e));
    }
}

/// Every scene of the new tools: the ribbon's, then each tool's (the tool
/// modules add theirs here as they are built).
fn scenes() -> Vec<Scene> {
    let mut all = ribbon_scenes();
    all.extend(crate::tools_scenes::scenes());
    all.extend(crate::elevation_scenes::scenes());
    all.extend(crate::parts_scenes::scenes());
    all.extend(crate::text_scenes::scenes());
    all.extend(crate::style_scenes::scenes());
    all.extend(crate::leader_scenes::scenes());
    all.extend(crate::dimension_scenes::scenes());
    all.extend(crate::topology_scenes::scenes());
    all.extend(crate::table_scenes::scenes());
    all.extend(crate::coordinate_scenes::scenes());
    all.extend(crate::hatch_scenes::scenes());
    all.extend(crate::hatch_boundary_scenes::scenes());
    all.extend(crate::selection_scenes::scenes());
    all.extend(crate::point_calc_scenes::scenes());
    all.extend(crate::stationing_scenes::scenes());
    all.extend(crate::centerline_scenes::scenes());
    all.extend(crate::edge_shift_scenes::scenes());
    all.extend(crate::offset_scenes::scenes());
    all.extend(crate::stretch_scenes::scenes());
    all.extend(crate::lengthen_scenes::scenes());
    all.extend(crate::divide_scenes::scenes());
    all.extend(crate::assoc_array_scenes::scenes());
    all.extend(crate::polyline_scenes::scenes());
    all.extend(crate::nurbs_scenes::scenes());
    all.extend(crate::curve_edit_scenes::scenes());
    all.extend(crate::audit_scenes::scenes());
    all.extend(crate::multipart_scenes::scenes());
    all.extend(crate::explode_scenes::scenes());
    all.extend(crate::clipboard_scenes::scenes());
    all.extend(crate::wipeout_scenes::scenes());
    all.extend(crate::layer_style_scenes::scenes());
    all.extend(crate::standard_scenes::scenes());
    all.extend(crate::line_type_scenes::scenes());
    all.extend(crate::impact_scenes::scenes());
    all.extend(crate::block_scenes::scenes());
    all.extend(crate::block_attr_scenes::scenes());
    all.extend(crate::block_dynamic_scenes::scenes());
    all.extend(crate::dimension_follow_scenes::scenes());
    all.extend(crate::dimension_option_scenes::scenes());
    all.extend(crate::dimension_mark_scenes::scenes());
    all.extend(crate::annotative_scenes::scenes());
    all.extend(crate::paragraph_layout_scenes::scenes());
    all.extend(crate::text_field_scenes::scenes());
    all.extend(crate::multileader_scenes::scenes());
    all.extend(crate::table_cells_scenes::scenes());
    all.extend(crate::style_override_scenes::scenes());
    all.extend(crate::match_scenes::scenes());
    all.extend(crate::cleanup_scenes::scenes());
    all.extend(crate::look_scenes::scenes());
    all.extend(crate::ucs_scenes::scenes());
    all.extend(crate::imperial_scenes::scenes());
    all.extend(crate::subref_scenes::scenes());
    all.extend(crate::input_scenes::scenes());
    all.extend(crate::view_scenes::scenes());
    all.extend(crate::method_scenes::scenes());
    all.extend(crate::segment_scenes::scenes());
    all.extend(crate::curve_scenes::scenes());
    all.extend(crate::transform_scenes::scenes());
    all.extend(crate::mirror_scenes::scenes());
    all.extend(crate::image_scenes::scenes());
    all.extend(crate::temporal_scenes::scenes());
    all.extend(crate::attachment_scenes::scenes());
    all
}

fn wanted(name: &str) -> bool {
    std::env::var("KENTOS_SHOTS_ONLY")
        .map_or(true, |only| only.split(',').any(|o| o.trim() == name))
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let all: Vec<Shot> = scenes()
                .into_iter()
                .map(|(name, bring)| (name, bring, None))
                .chain(
                    pointed_scenes()
                        .into_iter()
                        .map(|(name, bring, point)| (name, bring, Some(point))),
                )
                .collect();
            for (name, bring, point) in all {
                if !wanted(name) {
                    continue;
                }
                let mut app = app_with_drawing();
                // The window's size, as a resize tells it: what fits the window (a dialog's grid) follows it.
                app.window_size = Size::new(width, height);
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                // The command history open, so the picture shows what the tool said; the
                // area settles at its size before the drawing opens and is fitted to it.
                app.command_expanded = !name.starts_with("serit");
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message: Message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                bring(&mut app);
                snapshot.settle(&mut app, App::view, &mut update);
                if let Some(point) = point {
                    point(&mut snapshot, &mut app);
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("arac-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
