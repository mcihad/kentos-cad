//! A layer's right-click menu by module and by layer kind (layer_entries.rs, docs/adr/9195 §5): every entry on every
//! kind of layer (çizim, açıklama, veri tablosu, bağlı, servis, raster, nokta bulutu, grup) in every module, against
//! the ADR's table, in kentos-cad following the project and in the module's own program; kentos-cad with a project
//! no module is decided for shows what each kind has. The entries that run a ribbon command follow the module's
//! command set, İstatistik and Grafik the bottom panel's tabs. Then the pictures of each module's menu on each kind.
//! Test code only.

use iced::{Point, Size};
use kentos_launch::Module;
use kentos_ui::snapshot::Input;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::catalog::catalog;
use crate::files_testing::{app_with_drawing, drive};
use crate::layer_entries::{LayerEntry, LayerKind};
use crate::tools_scenes::{Objects, layer, open_edited};

/// The table of docs/adr/9195 §5: each entry, the words its presence is read by, the modules that have it (M Mimari,
/// H Harita, Y Yol, C CBS) and the kinds of layer it is about (d çizim, a açıklama, v veri tablosu, b bağlı,
/// s servis, r raster, n nokta bulutu, g grup).
const TABLE: [(LayerEntry, &str, &str, &str); 43] = [
    (LayerEntry::Activate, "Etkin katman yap", "MHYC", "davbrn"),
    (LayerEntry::EditSession, "Düzenlemeyi başlat", "C", "v"),
    (
        LayerEntry::Visibility,
        "Yalnızca bunu göster",
        "MHYC",
        "davbsrng",
    ),
    (LayerEntry::Lock, "Kilitle", "MHYC", "davbrng"),
    (LayerEntry::Freeze, "Bütün katmanları çöz", "MHYC", "dabrng"),
    (LayerEntry::Plot, "Yazdırma", "MHYC", "davbsrng"),
    (LayerEntry::Mobile, "Mobilde", "MHYC", "davbsrng"),
    (LayerEntry::Order, "Yukarı taşı", "MHYC", "davbsrng"),
    (LayerEntry::Service, "Servis ayarları…", "HYC", "s"),
    (LayerEntry::Refresh, "Yenile", "HYC", "d"),
    (
        LayerEntry::SelectObjects,
        "Nesnelerini seç",
        "MHYC",
        "dabrng",
    ),
    (LayerEntry::ZoomTo, "Katmana yakınlaştır", "MHYC", "davbrng"),
    (LayerEntry::Export, "Dışa aktar", "HYC", "dvb"),
    (LayerEntry::Migrate, "Veri kaynağına taşı…", "C", "d"),
    (LayerEntry::Counts, "Nesne sayısını göster", "C", "dvb"),
    (LayerEntry::Color, "Renk", "MHYC", "davb"),
    (LayerEntry::LineType, "Çizgi tipi", "MHYC", "dab"),
    (LayerEntry::LineWeight, "Kalınlık", "MHYC", "dab"),
    (LayerEntry::Style, "Katman stili…", "MHYC", "dvb"),
    (LayerEntry::Labels, "Etiketler…", "HYC", "dvb"),
    (LayerEntry::StyleFiles, "Stil yükle…", "HC", "dvb"),
    (LayerEntry::Sld, "Görünüşü SLD olarak kaydet…", "HC", "dvb"),
    (LayerEntry::Qml, "QML'den görünüş al…", "HC", "dvb"),
    (
        LayerEntry::MapLibre,
        "MapLibre stilinden görünüş al…",
        "HC",
        "dvb",
    ),
    (LayerEntry::Impact, "Değişikliğin etkisi…", "MHYC", "davb"),
    (LayerEntry::Fields, "Alanlar…", "HC", "dvb"),
    (LayerEntry::Forms, "Formlar…", "HC", "dvb"),
    (LayerEntry::Info, "Bilgi kartı ve eylemler…", "HC", "dvb"),
    (LayerEntry::Dataset, "Veri kümesi bilgisi…", "HYC", "dvb"),
    (LayerEntry::Filter, "Süzgeç…", "HYC", "dvb"),
    (LayerEntry::Time, "Zaman ayarları…", "C", "dvb"),
    (LayerEntry::Statistics, "İstatistik", "HC", "dvb"),
    (LayerEntry::Chart, "Grafik", "C", "dvb"),
    (LayerEntry::Kind, "Katman türü", "C", "da"),
    (LayerEntry::Crs, "Koordinat sistemi", "HYC", "dab"),
    (LayerEntry::Source, "Kaynağa yaz", "HYC", "b"),
    (LayerEntry::Snap, "Kenet", "MHYC", "davbg"),
    (LayerEntry::Scenario, "Senaryo", "C", "g"),
    (LayerEntry::Rename, "Yeniden adlandır", "MHYC", "davbsrng"),
    (LayerEntry::Add, "Yanına yeni katman", "MHYC", "davbrng"),
    (LayerEntry::Duplicate, "Kopyasını oluştur", "MHYC", "dabrn"),
    (
        LayerEntry::Merge,
        "Başka katmanlarla birleştir…",
        "MHYC",
        "dabrn",
    ),
    (LayerEntry::Remove, "Sil", "MHYC", "davbsrng"),
];

/// The project's layers: one of each kind, its id, its name in the tree and the kind it is.
const LAYERS: [(&str, &str, LayerKind); 8] = [
    ("cizim", "Çizim katmanı", LayerKind::Drawing),
    ("aciklama", "Açıklama katmanı", LayerKind::Annotation),
    ("veri", "Veri katmanı", LayerKind::Data),
    ("bagli", "Bağlı katman", LayerKind::Bound),
    ("servis", "Servis katmanı", LayerKind::Service),
    ("raster", "Raster katmanı", LayerKind::Raster),
    ("bulut", "Nokta bulutu katmanı", LayerKind::PointCloud),
    ("grup", "Grup", LayerKind::Group),
];

fn module_letter(module: Option<Module>) -> Option<char> {
    module.map(|m| match m {
        Module::Architecture => 'M',
        Module::Map => 'H',
        Module::Road => 'Y',
        Module::Gis => 'C',
    })
}

fn kind_letter(kind: LayerKind) -> char {
    match kind {
        LayerKind::Drawing => 'd',
        LayerKind::Annotation => 'a',
        LayerKind::Data => 'v',
        LayerKind::Bound => 'b',
        LayerKind::Service => 's',
        LayerKind::Raster => 'r',
        LayerKind::PointCloud => 'n',
        LayerKind::Group => 'g',
    }
}

/// The words an entry is read by on `kind`: a group's zoom and new layer say so.
fn probe(entry: LayerEntry, words: &'static str, kind: LayerKind) -> &'static str {
    match (entry, kind) {
        (LayerEntry::ZoomTo, LayerKind::Group) => "Gruba yakınlaştır",
        (LayerEntry::Add, LayerKind::Group) => "İçine yeni katman",
        _ => words,
    }
}

/// Whether a menu's labels hold one starting with `label` (“Alanlar… (3)” for `Alanlar…`).
fn has(menu: &str, label: &str) -> bool {
    menu.contains(&format!("label: \"{label}"))
}

/// A layer of the tree with `extra` fields (its data, service, feed, type).
fn node(id: &str, name: &str, extra: Value) -> Value {
    let mut n = layer(id, name, "fg", 0.25);
    if let (Some(n), Some(extra)) = (n.as_object_mut(), extra.as_object()) {
        n.extend(extra.clone());
    }
    n
}

/// A project of every kind of layer, `module`'s (none: one no module is decided for), opened in `program` (none:
/// kentos-cad): a drawing layer fed by a service (Yenile) with a line, a CBS annotation layer with a text, a data
/// layer of a GeoPackage's parcels, a layer bound to a table, a service's, rasters, a point cloud, and a scenario
/// group holding a layer. No file is read: the menu asks none.
fn every_kind(module: Option<Module>, program: Option<Module>) -> App {
    let mut app = app_with_drawing();
    app.program = program;
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let mut o = Objects::new();
    o.line("cizim", [0.0, 0.0], [20.0, 0.0]);
    o.text("aciklama", [2.0, 4.0], "Not", 2.5, 0.0);
    o.line("bagli", [0.0, 10.0], [20.0, 10.0]);
    o.line("grup-ic", [0.0, 20.0], [20.0, 20.0]);
    o.push(
        "raster",
        json!({
            "kind": "raster", "affine": [487000.0, 1.0, 0.0, 4420030.0, 0.0, -1.0], "width": 10, "height": 10,
            "bands": 1, "sample": "f32", "file": "yok/dem.tif", "srid": 5256,
            "style": { "render": "rampShade", "bands": [1], "stretch": "minMax", "ramp": "Arazi" }
        }),
    );
    o.push(
        "bulut",
        json!({
            "kind": "pointcloud",
            "sources": [{ "file": "yok/bulut.laz", "format": "laz", "count": 10,
                "bounds": [487030.0, 4420000.0, 0.0, 487031.0, 4420001.0, 1.0] }],
            "bounds": [487030.0, 4420000.0, 0.0, 487031.0, 4420001.0, 1.0], "count": 10, "srid": 5256,
            "style": { "render": "rgb", "size": 2 }
        }),
    );
    open_edited(&mut app, o, |d| {
        let mut group = node("grup", "Grup", json!({ "type": "group", "scenario": {} }));
        group["children"] = json!([layer("grup-ic", "Gruptaki katman", "fg", 0.25)]);
        d["layers"] = json!([
            node(
                "cizim",
                "Çizim katmanı",
                json!({ "feed": { "kind": "wfs", "url": "https://ornek.org/wfs", "name": "parsel" } })
            ),
            node(
                "aciklama",
                "Açıklama katmanı",
                json!({ "geometry": "annotation" })
            ),
            node(
                "veri",
                "Veri katmanı",
                json!({ "data": {
                "driver": "gpkg", "address": "/yok/veri.gpkg", "table": "parseller", "key": "fid",
                "geometry": { "column": "geom", "typeName": "POLYGON", "srid": 5256 }
            } })
            ),
            node("bagli", "Bağlı katman", json!({})),
            node(
                "servis",
                "Servis katmanı",
                json!({ "service": { "kind": "xyz", "url": "https://ornek.org/{z}/{x}/{y}.png" } })
            ),
            node("raster", "Raster katmanı", json!({})),
            node("bulut", "Nokta bulutu katmanı", json!({})),
            group,
        ]);
        d["activeLayer"] = json!("cizim");
        d["settings"]["sources"] = json!([{
            "id": "kaynak-1", "kind": "geopackage", "path": "/yok/bagli.gpkg", "table": "bagli", "layer": "bagli",
            "revision": "0".repeat(64), "digest": "0".repeat(64), "writable": true
        }]);
    });
    let doc = app.document.as_mut().expect("the project");
    let mut s = doc.model.settings().clone();
    s.workspace = module.map(crate::program::type_of);
    s.cad_work = module.and_then(|m| crate::program::works_of(m).first().copied());
    doc.model.set_settings(s);
    let task = app.update(Message::Swallowed);
    drive(&mut app, task);
    app
}

/// Every layer's menu in `module`'s project opened in `program` against the table: the cells that differ, said.
#[inline(never)]
fn check(module: Option<Module>, program: Option<Module>) -> Vec<String> {
    let app = every_kind(module, program);
    assert_eq!(app.active_module(), module, "{module:?} in {program:?}");
    let letter = module_letter(module);
    let doc = app.document.as_ref().expect("the project");
    let mut wrong = Vec::new();
    for (id, _, kind) in LAYERS {
        let node = doc.model.layers().get(id).expect("the layer").clone();
        assert_eq!(app.layer_kind(&node), kind, "{id}");
        let menu = format!("{:?}", app.layer_menu(&node, false));
        for (entry, words, modules, kinds) in TABLE {
            let placed =
                letter.is_none_or(|l| modules.contains(l)) && kinds.contains(kind_letter(kind));
            if app.layer_entry_on(entry, kind) != placed {
                wrong.push(format!(
                    "{module:?} {kind:?} {entry:?}: the table says {}",
                    !placed
                ));
            }
            // What the layer's state asks besides: Veri kaynağına taşı… moves a CBS project's objects alone.
            let want = placed && (entry != LayerEntry::Migrate || module == Some(Module::Gis));
            let label = probe(entry, words, kind);
            if has(&menu, label) != want {
                wrong.push(format!(
                    "{module:?} in {program:?}, {kind:?}: “{label}” ({entry:?}) shown {}",
                    !want
                ));
            }
        }
    }
    wrong
}

/// Mobilde (docs/adr/9198): every node may be shown on the phone or not; to edit is a CBS project's data layer's alone.
/// The choice is `cad.layers.set`'s, kept on the node; shown, the default, keeps no field.
#[test]
fn the_phone_edits_a_cbs_data_layer_alone() {
    use kentos_commands::LayerMobile;
    let item = |menu: &str, label: &str| menu.contains(&format!("label: \"{label}\""));
    for (module, edits) in [(Some(Module::Gis), true), (Some(Module::Map), false), (Some(Module::Architecture), false)] {
        let mut app = every_kind(module, None);
        let mobile = |app: &App, id: &str| app.document.as_ref().and_then(|d| d.model.layers().get(id)).and_then(|n| n.mobile);
        for (id, _, kind) in LAYERS {
            let node = app.document.as_ref().and_then(|d| d.model.layers().get(id)).cloned().expect("the layer");
            let menu = format!("{:?}", app.layer_menu(&node, false));
            assert!(item(&menu, "Gösterme") && item(&menu, "Göster"), "{module:?} {kind:?}");
            let editable = edits && kind == LayerKind::Data;
            assert_eq!(item(&menu, "Düzenle"), editable, "{module:?} {kind:?}");
        }
        // Gösterme leaves the data layer out of the phone; Düzenle is its rows' writing in CBS alone; Göster takes the
        // field away again.
        app.set_layer_mobile("veri", LayerMobile::Hide);
        assert_eq!(mobile(&app, "veri"), Some(LayerMobile::Hide));
        app.set_layer_mobile("veri", LayerMobile::Edit);
        assert_eq!(mobile(&app, "veri"), Some(if edits { LayerMobile::Edit } else { LayerMobile::Hide }), "{module:?}");
        if !edits {
            let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
            assert!(said.iter().any(|t| t.contains("mobilde düzenlenemez: mobil uygulama CAD projesini yazmaz")), "{said:?}");
        }
        app.set_layer_mobile("veri", LayerMobile::Show);
        assert_eq!(mobile(&app, "veri"), None);
        // A group hidden hides what it holds.
        app.set_layer_mobile("grup", LayerMobile::Hide);
        assert_eq!(mobile(&app, "grup"), Some(LayerMobile::Hide));
    }
}

#[test]
fn each_modules_layer_menu_has_its_entries_on_each_kind_of_layer() {
    // The table lists every entry once, in the menu's order.
    assert_eq!(TABLE.map(|(entry, ..)| entry), LayerEntry::ALL);
    let mut wrong = Vec::new();
    for module in Module::ALL {
        wrong.extend(check(Some(module), None));
        wrong.extend(check(Some(module), Some(module)));
    }
    // kentos-cad with a project no module is decided for: what each kind has.
    wrong.extend(check(None, None));
    assert!(
        wrong.is_empty(),
        "{} cells differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The entries that run a ribbon command are where the module's command set has it; İstatistik and Grafik where the
/// bottom panel has their tab; each kind's entries are the table's.
#[test]
fn the_layer_menu_follows_the_command_set_and_the_tabs() {
    let commands = [
        (LayerEntry::Activate, "tool.layerMakeActive"),
        (LayerEntry::Visibility, "tool.layerIsolate"),
        (LayerEntry::Lock, "tool.layerLock"),
        (LayerEntry::Freeze, "tool.layerFreeze"),
        (LayerEntry::Merge, "layer.merge"),
        (LayerEntry::Style, "style.layerStyle"),
        (LayerEntry::LineType, "lineTypes.manage"),
        (LayerEntry::Dataset, "layer.dataset"),
        (LayerEntry::Impact, "layer.impact"),
        (LayerEntry::Crs, "layer.crs"),
        (LayerEntry::Info, "layer.info"),
        (LayerEntry::Filter, "layer.filter"),
        (LayerEntry::Labels, "layer.labels"),
        (LayerEntry::Fields, "layer.fields"),
        (LayerEntry::Forms, "data.forms"),
        (LayerEntry::Statistics, "data.statistics"),
        (LayerEntry::Chart, "data.chart"),
        (LayerEntry::Source, "source.read"),
    ];
    let mut wrong = Vec::new();
    for module in Module::ALL {
        let profile = crate::module_profile::profile(module);
        for (entry, command) in commands {
            let offered = catalog().module_has(module, command);
            if profile.layer_entry(entry) != offered {
                wrong.push(format!("{module:?} {entry:?}: {command} offered {offered}"));
            }
        }
        for (entry, tab) in [
            (LayerEntry::Statistics, BottomTab::Statistics),
            (LayerEntry::Chart, BottomTab::Chart),
        ] {
            if profile.layer_entry(entry) != profile.bottom(tab) {
                wrong.push(format!("{module:?} {entry:?}: the tab {tab:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    for (entry, _, _, kinds) in TABLE {
        let listed: Vec<char> = entry.kinds().iter().map(|k| kind_letter(*k)).collect();
        let mut want: Vec<char> = kinds.chars().collect();
        let mut got = listed.clone();
        want.sort_unstable();
        got.sort_unstable();
        assert_eq!(got, want, "{entry:?}");
    }
}

/// The pictures of docs/adr/9195 §5: each module's menu on each kind of layer, in kentos-cad with the module's project
/// on screen, light at 1440×900, into `.run/gui-evidence/kullanim-9195/menu-<modül>-<tür>.png`. For the owner, run by
/// hand; the folder is not committed.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn each_modules_layer_menu_pictures() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/kullanim-9195");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    for module in Module::ALL {
        for (_, name, kind) in LAYERS {
            picture(module, name, kind, None, size, &out);
        }
    }
    // Görünüş ▸ and Veri ▸ opened: a drawing's in KentOS Mimari and Harita, a data layer's in KentOS GIS.
    picture(
        Module::Architecture,
        "Çizim katmanı",
        LayerKind::Drawing,
        Some("Görünüş"),
        size,
        &out,
    );
    picture(
        Module::Map,
        "Çizim katmanı",
        LayerKind::Drawing,
        Some("Veri"),
        size,
        &out,
    );
    picture(
        Module::Gis,
        "Veri katmanı",
        LayerKind::Data,
        Some("Görünüş"),
        size,
        &out,
    );
    picture(
        Module::Gis,
        "Veri katmanı",
        LayerKind::Data,
        Some("Veri"),
        size,
        &out,
    );
}

/// The pictures of docs/adr/9198: Görünürlük ▸ Mobilde ▸ on a CBS project's data layer (Göster, Düzenle, Gösterme) and
/// on a map's drawing layer (Göster, Gösterme), light at 1440×900, into `.run/gui-evidence/kullanim-9198/`. For the
/// owner, run by hand; the folder is not committed.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn the_phone_menu_pictures() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-9198");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    for (module, name, kind) in [
        (Module::Gis, "Veri katmanı", LayerKind::Data),
        (Module::Map, "Çizim katmanı", LayerKind::Drawing),
    ] {
        let mut app = every_kind(Some(module), None);
        let _ = app.settings.choose(&[("appearance.theme", Value::from("light"))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = kentos_ui::snapshot::Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        crate::tools_screens::press_caption(&mut snapshot, &mut app, name, true, false);
        snapshot.settle(&mut app, App::view, &mut update);
        for caption in ["Görünürlük", "Mobilde"] {
            let item = crate::tools_screens::caption_at(&mut snapshot, &app, caption, false);
            snapshot.input(&mut app, App::view, &mut update, Input::Click(item));
            snapshot.settle(&mut app, App::view, &mut update);
            snapshot.input(&mut app, App::view, &mut update, Input::Move(Point::new(item.x + 4.0, item.y)));
            snapshot.settle(&mut app, App::view, &mut update);
        }
        let file = out.join(format!("mobilde-{}-{}.png", module.id(), kind.title().replace(' ', "-")));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

#[inline(never)]
fn picture(
    module: Module,
    name: &str,
    kind: LayerKind,
    submenu: Option<&str>,
    size: Size,
    out: &std::path::Path,
) {
    let mut app = every_kind(Some(module), None);
    let _ = app
        .settings
        .choose(&[("appearance.theme", Value::from("light"))]);
    app.apply_settings();
    let _ = app.update(Message::WindowResized(size));
    let mut snapshot = kentos_ui::snapshot::Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    crate::tools_screens::press_caption(&mut snapshot, &mut app, name, true, false);
    snapshot.settle(&mut app, App::view, &mut update);
    if let Some(caption) = submenu {
        let item = crate::tools_screens::caption_at(&mut snapshot, &app, caption, false);
        snapshot.input(&mut app, App::view, &mut update, Input::Click(item));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Move(Point::new(item.x + 4.0, item.y)),
        );
        snapshot.settle(&mut app, App::view, &mut update);
    }
    let file = out.join(format!(
        "menu-{}-{}{}.png",
        module.id(),
        kind.title().replace(' ', "-"),
        submenu
            .map(|c| format!("-{}", c.to_lowercase()))
            .unwrap_or_default()
    ));
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes");
    println!("{}", file.display());
}
