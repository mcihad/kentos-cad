//! A layer's own snapping in the layer tree (docs/adr/0163 §4), as the web's
//! (`apps/web/src/ui/layers/layerSnap.ts`): the row's magnet (the general
//! kinds, off, or kinds of its own; a group's from its layers) and the row
//! menu's Kenet ▸. A group's magnet and menu write to all its layers (a group
//! keeps none). The document's `set_layer_snap` is an edit, not an undo step,
//! as a lock is.

use kentos_contracts::{LayerNode, LayerNodeType, LayerSnap};
use kentos_domain::LayerTree;
use kentos_ui::widget::Menu;
use kentos_ui::widget::tree_view::Toggle;

use crate::app::{App, Message};
use crate::catalog::catalog;

/// What a row's magnet shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SnapState {
    /// The general kinds.
    General,
    Off,
    /// Kinds of its own (a group's: some of its layers have, or they differ).
    Kinds,
}

/// The snap kinds a layer can keep to, in the Kenet menu's order (the web's `SNAP_KINDS`).
pub(crate) const SNAP_KINDS: [&str; 12] = [
    "endpoint",
    "midpoint",
    "intersection",
    "center",
    "perpendicular",
    "tangent",
    "node",
    "nearest",
    "centroid",
    "extension",
    "parallel",
    "grid",
];

/// A row's magnet: a layer's own state; a group's off when all its layers
/// are, general when all are, else own kinds.
pub(crate) fn snap_state(layers: &LayerTree, node: &LayerNode) -> SnapState {
    let of = |n: &LayerNode| match &n.snap {
        None => SnapState::General,
        Some(s) if s.off => SnapState::Off,
        Some(_) => SnapState::Kinds,
    };
    if node.kind == LayerNodeType::Layer {
        return of(node);
    }
    let states: Vec<SnapState> = layers.leaves_of(&node.id).into_iter().map(of).collect();
    match states.split_first() {
        None => SnapState::General,
        Some((first, rest)) if rest.iter().all(|s| s == first) => *first,
        Some(_) => SnapState::Kinds,
    }
}

/// The snapping a node's layers share; `None` when they differ.
pub(crate) fn common_snap(layers: &LayerTree, node: &LayerNode) -> Option<Option<LayerSnap>> {
    let leaves = layers.leaves_of(&node.id);
    let first = leaves.first().and_then(|l| l.snap.clone());
    leaves.iter().all(|l| l.snap == first).then_some(first)
}

/// A magnet's click: off goes back to the general kinds, anything else turns off.
pub(crate) fn toggled(layers: &LayerTree, node: &LayerNode) -> Option<LayerSnap> {
    (snap_state(layers, node) != SnapState::Off).then(off)
}

fn off() -> LayerSnap {
    LayerSnap {
        off: true,
        kinds: None,
    }
}

/// What a magnet says, by its state (the web's `SNAP_TIP`).
fn tip(state: SnapState) -> &'static str {
    match state {
        SnapState::General => "Kenet: genel türler; kapatmak için tıklayın",
        SnapState::Kinds => "Kenet: katmanın kendi türleri; kapatmak için tıklayın",
        SnapState::Off => "Kenet kapalı; açmak için tıklayın",
    }
}

fn message(id: &str, snap: Option<LayerSnap>) -> Message {
    Message::Layer(crate::layering::Event::Snap(id.to_owned(), snap))
}

impl App {
    /// The row's magnet.
    pub(crate) fn layer_snap_toggle(
        &self,
        layers: &LayerTree,
        node: &LayerNode,
    ) -> Toggle<Message> {
        let state = snap_state(layers, node);
        let on = crate::icons::from_web(Some(if state == SnapState::Kinds {
            "magnetKinds"
        } else {
            "magnet"
        }));
        Toggle::new(
            state != SnapState::Off,
            (on, crate::icons::from_web(Some("magnetOff"))),
            (tip(state), tip(SnapState::Off)),
            message(&node.id, toggled(layers, node)),
        )
        .pressed(state != SnapState::General)
    }

    /// Kenet ▸: the general kinds, off, or only some kinds. A kind ticked
    /// makes the layer's own list from what it takes now (the general kinds
    /// when it has none, none when it is off); the last one unticked turns it
    /// off.
    pub(crate) fn layer_snap_menu(&self, layers: &LayerTree, node: &LayerNode) -> Menu<Message> {
        let common = common_snap(layers, node);
        let general: Vec<&str> = SNAP_KINDS
            .into_iter()
            .filter(|k| self.settings.bool(&format!("snap.{k}")))
            .collect();
        let ticked: Vec<&str> = match &common {
            None => Vec::new(),
            Some(None) => general,
            Some(Some(s)) => s.kinds.iter().flatten().map(String::as_str).collect(),
        };
        let is_off = matches!(&common, Some(Some(s)) if s.off);
        let mut menu = Menu::new()
            .radio(
                "Genel türler",
                common == Some(None),
                message(&node.id, None),
            )
            .radio("Kapalı", is_off, message(&node.id, Some(off())))
            .separator()
            .header("Yalnız bu türler");
        for k in SNAP_KINDS {
            let on = ticked.contains(&k);
            let kinds: Vec<String> = SNAP_KINDS
                .into_iter()
                .filter(|x| if *x == k { !on } else { ticked.contains(x) })
                .map(str::to_owned)
                .collect();
            let snap = if kinds.is_empty() {
                off()
            } else {
                LayerSnap {
                    off: false,
                    kinds: Some(kinds),
                }
            };
            let command = catalog().get(&format!("draft.snap.{k}"));
            menu = menu
                .check(
                    command.map_or(k, |c| c.short),
                    on,
                    message(&node.id, Some(snap)),
                )
                .icon(command.map_or(kentos_ui::icon::Icon::Button, |c| c.icon));
        }
        menu
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    fn snap(app: &App, id: &str) -> Option<LayerSnap> {
        app.document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .and_then(|n| n.snap.clone())
    }

    fn kinds(k: &[&str]) -> Option<LayerSnap> {
        Some(LayerSnap {
            off: false,
            kinds: Some(k.iter().map(|s| (*s).to_owned()).collect()),
        })
    }

    /// The magnet's click and Kenet ▸ write through the document, as an edit
    /// (the web's `layerSnap.test.ts`).
    #[test]
    fn the_magnet_turns_off_and_back_and_the_menu_makes_a_list_of_its_own() {
        let mut app = app_with_drawing();
        let layers = app.document.as_ref().expect("open").model.layers().clone();
        let bina = layers.get("bina").expect("Bina").clone();
        assert_eq!(snap_state(&layers, &bina), SnapState::General);
        assert_eq!(toggled(&layers, &bina), Some(off()));
        let _ = app.update(message("bina", Some(off())));
        assert_eq!(snap(&app, "bina"), Some(off()));
        assert!(
            app.document.as_ref().expect("open").model.is_dirty(),
            "an edit"
        );
        let layers = app.document.as_ref().expect("open").model.layers().clone();
        assert_eq!(
            snap_state(&layers, layers.get("bina").expect("Bina")),
            SnapState::Off
        );
        assert_eq!(toggled(&layers, layers.get("bina").expect("Bina")), None);
        // Kenet ▸ on an off layer: a kind ticked is the list alone.
        let menu = format!(
            "{:?}",
            app.layer_snap_menu(&layers, layers.get("bina").expect("Bina"))
        );
        for name in [
            "Genel türler",
            "Kapalı",
            "Yalnız bu türler",
            "Uç nokta",
            "Karelaj",
        ] {
            assert!(menu.contains(name), "{name} on the menu");
        }
        let _ = app.update(message("bina", kinds(&["grid"])));
        assert_eq!(snap(&app, "bina"), kinds(&["grid"]));
        let layers = app.document.as_ref().expect("open").model.layers().clone();
        assert_eq!(
            snap_state(&layers, layers.get("bina").expect("Bina")),
            SnapState::Kinds
        );
    }

    /// The magnets in their three states and Kenet ▸ open on Bina's row, for
    /// the owner, in `.run/shots/katman-kenet-*`:
    ///
    /// ```text
    /// cargo test -p kentos-desktop layer_snap::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::Size;
        use iced::keyboard::key::Named;
        use kentos_ui::snapshot::{Input, Snapshot};

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(message("parsel", Some(off())));
                let _ = app.update(message("bina", kinds(&["endpoint", "intersection"])));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                // The Bina row, found by its name.
                let bina = crate::files_testing::find_text(&mut snapshot, &app, "Bina")
                    .expect("the Bina row")
                    .center();
                snapshot.input(&mut app, App::view, &mut update, Input::RightClick(bina));
                // Etkin katman yap, Gizle, Kilitle, Kenet ▸.
                for k in [
                    Named::ArrowDown,
                    Named::ArrowDown,
                    Named::ArrowDown,
                    Named::ArrowDown,
                    Named::ArrowRight,
                ] {
                    snapshot.input(&mut app, App::view, &mut update, Input::Key(k));
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("katman-kenet-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
