//! The layer tree's pictures for the owner, run by hand.

use super::*;

/// Pictures for the owner: the layer tree following the selection, one
/// layer (its group opened) and several; `.run/shots/katman-secimi-*`. The
/// last two show the tree after the panel's own changes went through the
/// layer commands (docs/adr/0248 §6): a layer added beside Bina, coloured,
/// dashed, locked and renamed, a group added; and Katman durumları with a
/// state saved and one applied.
/// `cargo test -p kentos-desktop layering::screens -- --ignored --nocapture`
/// The SLD the screens' Parsel takes: QGIS's fill and dashed outline.
#[cfg(test)]
pub(super) const SLD_SCENE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor xmlns="http://www.opengis.net/sld" version="1.1.0" xmlns:se="http://www.opengis.net/se">
 <NamedLayer><se:Name>parsel</se:Name><UserStyle><se:FeatureTypeStyle><se:Rule><se:Name>Parsel</se:Name>
  <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill">#7570b3</se:SvgParameter><se:SvgParameter name="fill-opacity">0.6</se:SvgParameter></se:Fill>
   <se:Stroke><se:SvgParameter name="stroke">#d95f02</se:SvgParameter><se:SvgParameter name="stroke-width">3</se:SvgParameter><se:SvgParameter name="stroke-dasharray">12 6</se:SvgParameter></se:Stroke>
  </se:PolygonSymbolizer></se:Rule></se:FeatureTypeStyle></UserStyle></NamedLayer></StyledLayerDescriptor>"##;

#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "tek",
                "cok",
                "menu",
                "renk",
                "ad",
                "sil",
                "komutlar",
                "durumlar",
                "sld",
                "qml",
                "maplibre",
                "maplibre-yaz",
                "menu-gorunus",
                "stil-yukle",
            ]
            .into_iter()
            .filter(|n| {
                std::env::var("KENTOS_SHOTS_ONLY")
                    .map_or(true, |o| o.split(',').any(|w| n.contains(w)))
            }) {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let model = &app.document.as_ref().expect("open").model;
                let slot_on = |layer: &str| {
                    model
                        .entities()
                        .find(|e| e.base().layer_id == layer)
                        .map(|e| kentos_domain::Slot(e.base().id))
                };
                let picked: Vec<_> = match name {
                    "cok" => ["parsel", "bina", "cizim"]
                        .iter()
                        .filter_map(|layer| slot_on(layer))
                        .collect(),
                    _ => slot_on("bina").into_iter().collect(),
                };
                // The group closed first: following the selection opens it.
                app.document
                    .as_mut()
                    .expect("open")
                    .model
                    .set_layer_expanded("layer-g", false);
                app.selection.set(picked);
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                // The Bina row: the panel's third row (below Kadastro and Parsel), its swatch after the caret and indent.
                let panel = width - app.docks.size(kentos_ui::widget::docking::Side::Right);
                let bina = iced::Point::new(panel + 120.0, 303.0);
                let swatch = iced::Point::new(panel + 21.0, 303.0);
                use kentos_ui::snapshot::Input;
                match name {
                    "menu" => {
                        snapshot.input(&mut app, App::view, &mut update, Input::RightClick(bina))
                    }
                    "renk" => {
                        snapshot.input(&mut app, App::view, &mut update, Input::Click(swatch))
                    }
                    // The menu's Görünüş submenu: SLD, QML and MapLibre out and in (docs/adr/0516, 0555, 0560).
                    "menu-gorunus" => {
                        snapshot.input(&mut app, App::view, &mut update, Input::RightClick(bina));
                        let at = if width > 1200.0 {
                            iced::Point::new(1095.0, 537.0)
                        } else {
                            iced::Point::new(741.0, 238.0)
                        };
                        snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
                    }
                    "ad" => {
                        let _ = app.update(Message::Layer(Event::Rename("bina".into())));
                        let _ = app
                            .update(Message::Layer(Event::RenameInput("Bina ve yapılar".into())));
                    }
                    "sil" => {
                        let _ = app.update(Message::Layer(Event::Remove("cizim".into())));
                    }
                    // SLD'den görünüş al (docs/adr/0516): QGIS 4.2's rule-based look on Parsel, its Diğer purple.
                    "sld" => {
                        let sld = std::env::temp_dir().join("kentos-katman-sld.sld");
                        std::fs::write(&sld, SLD_SCENE).expect("written");
                        let _ = app.update(Message::Layer(Event::SldImportFrom(
                            "parsel".into(),
                            Some(sld),
                        )));
                        let model = &app.document.as_ref().expect("open").model;
                        let renderer = model
                            .layers()
                            .get("parsel")
                            .and_then(|n| n.style.renderer.clone());
                        assert_eq!(
                            renderer.as_ref().map(|r| r["type"].clone()),
                            Some("single".into())
                        );
                        app.selection.set(Vec::new());
                        // Katman stili shows the look taken.
                        app.open_layer_style(Some("parsel".into()));
                    }
                    // QML'den görünüş al (docs/adr/0555): QGIS 4.2's fill with a star at each parcel's centroid.
                    "qml" => {
                        let qml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("../../fixtures/qml/v1/qgis-merkez.qml");
                        let _ = app.update(Message::Layer(Event::QmlImportFrom(
                            "parsel".into(),
                            Some(qml),
                        )));
                        let model = &app.document.as_ref().expect("open").model;
                        let renderer = model
                            .layers()
                            .get("parsel")
                            .and_then(|n| n.style.renderer.clone());
                        assert_eq!(
                            renderer.as_ref().map(|r| r["type"].clone()),
                            Some("single".into())
                        );
                        app.selection.set(Vec::new());
                        // Katman stili shows the look taken: the fill, its outline and the star.
                        app.open_layer_style(Some("parsel".into()));
                    }
                    // MapLibre stilinden görünüş al (docs/adr/0560): a third party's categorized style on Parsel.
                    "maplibre" => {
                        let style = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("../../fixtures/maplibre/v1/kategorili.json");
                        let _ = app.update(Message::Layer(Event::MaplibreImportFrom(
                            Some("parsel".into()),
                            Some(style),
                        )));
                        let model = &app.document.as_ref().expect("open").model;
                        let renderer = model
                            .layers()
                            .get("parsel")
                            .and_then(|n| n.style.renderer.clone());
                        assert_eq!(
                            renderer.as_ref().map(|r| r["type"].clone()),
                            Some("categorized".into())
                        );
                        app.selection.set(Vec::new());
                        app.open_layer_style(Some("parsel".into()));
                    }
                    // Stil yükle (docs/adr/0635 §9): the file's extension chooses the format, here QGIS's QML.
                    "stil-yukle" => {
                        let qml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("../../fixtures/qml/v1/qgis-merkez.qml");
                        let _ = app.update(Message::Layer(Event::StyleLoadFrom(
                            "parsel".into(),
                            Some(qml),
                        )));
                        let model = &app.document.as_ref().expect("open").model;
                        assert!(
                            model
                                .layers()
                                .get("parsel")
                                .and_then(|n| n.style.renderer.as_ref())
                                .is_some(),
                            "the look is taken"
                        );
                        let back = std::env::temp_dir()
                            .join(format!("kentos-stil-{}.sld", std::process::id()));
                        let _ = app.update(Message::Layer(Event::StyleSaveTo(
                            "parsel".into(),
                            Some(back.clone()),
                        )));
                        let written = std::fs::read_to_string(&back).expect("the SLD is written");
                        assert!(written.contains("StyledLayerDescriptor"));
                        let _ = std::fs::remove_file(&back);
                        app.selection.set(Vec::new());
                        app.open_layer_style(Some("parsel".into()));
                    }
                    // Görünüşü MapLibre stili olarak kaydet (docs/adr/0560): the visible layers as one style.
                    "maplibre-yaz" => {
                        let dir = std::env::temp_dir()
                            .join(format!("kentos-maplibre-{}", std::process::id()));
                        let _ = std::fs::remove_dir_all(&dir);
                        std::fs::create_dir_all(&dir).expect("a folder");
                        let style = dir.join("harita.json");
                        let _ = app.update(Message::Layer(Event::MaplibreExportTo(
                            None,
                            Some(style.clone()),
                        )));
                        assert!(style.exists(), "the style is written");
                        let _ = std::fs::remove_dir_all(&dir);
                        app.selection.set(Vec::new());
                    }
                    "komutlar" => {
                        let _ = app.update(Message::Layer(Event::AddBeside("bina".into())));
                        let id = app
                            .document
                            .as_ref()
                            .expect("open")
                            .model
                            .layers()
                            .active()
                            .to_owned();
                        let _ =
                            app.update(Message::Layer(Event::Color(id.clone(), "#E5484D".into())));
                        let _ = app.update(Message::Layer(Event::LineType(
                            id.clone(),
                            LineType::Dashed,
                        )));
                        let _ = app.update(Message::Layer(Event::LineWeight(id.clone(), 0.5)));
                        let _ = app.update(Message::Layer(Event::Rename(id.clone())));
                        let _ = app.update(Message::Layer(Event::RenameInput("Duvar".into())));
                        let _ = app.update(Message::Layer(Event::RenameDone));
                        let _ = app.update(Message::LayerLocked(id));
                        let _ = app.update(Message::Run("layer.newGroup"));
                        app.selection.set(Vec::new());
                    }
                    "durumlar" => {
                        let parts = kentos_domain::layer_states::LayerStateParts {
                            locks: true,
                            styles: true,
                        };
                        // The sample drawing's Çizim is hidden: the first state keeps it so.
                        app.save_layer_state("Çizim gizli", parts);
                        let _ = app.update(Message::LayerVisible("cizim".into()));
                        app.save_layer_state("Çizim açık", parts);
                        app.apply_layer_state("durum-1");
                        let _ = app.update(Message::Run("layer.states"));
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("katman-secimi-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
