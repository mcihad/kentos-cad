//! The selection's commands of docs/adr/0187 on the desktop, as the web's
//! (`apps/web/src/app/selectionCommands.ts`): Önceki seçim (§3), Sıradakini
//! seç (§1, Shift+Boşluk; its chip is `selection_chip.rs`) and Seçim süzgeci
//! with its kinds (§5): the status bar's Süzgeç cell and its right-click
//! menu.

use kentos_domain::Slot;
use kentos_interaction::selectable::{self, KINDS};
use kentos_ui::widget::context_menu::Menu;
use serde_json::Value;

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::selecting::kind_title;

/// A kind's icon in the filter's menu and in Sıradakini seç's list: its
/// drawing tool's (the web's `ENTITY_KIND_ICON`).
pub(crate) fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "point" => "point",
        "line" => "line",
        "polyline" => "polyline",
        "polygon" => "polygon",
        "circle" => "circle",
        "arc" => "arc",
        "ellipse" => "ellipse",
        "spline" => "spline",
        "xline" => "xline",
        "ray" => "ray",
        "text" => "text",
        "dimension" => "dimension",
        "hatch" => "hatch",
        "insert" => "blockInsert",
        "leader" => "leader",
        "table" => "table",
        "image" => "imageInsert",
        _ => "more",
    }
}

/// The kinds a filter holds as the log says them (the web's `kindsText`).
fn kinds_text(mask: u32) -> String {
    if mask == selectable::ALL {
        return "bütün türler".to_owned();
    }
    if mask == 0 {
        return "hiçbir tür (hiçbir şey seçilmez)".to_owned();
    }
    KINDS
        .iter()
        .filter(|k| mask & selectable::bit(k) != 0)
        .map(|k| crate::layer_tree::lower_tr(kind_title(k)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Seçim süzgeci's list (the web's `SELECT_FILTER_KINDS`): the kinds in
/// their menu's order, then every kind or none at once.
fn select_filter_blocks() -> Vec<(&'static str, Vec<&'static str>)> {
    let kinds = KINDS
        .iter()
        .filter_map(|kind| {
            catalog()
                .commands()
                .iter()
                .find(|c| c.id.strip_prefix("edit.selectFilter.") == Some(kind))
                .map(|c| c.id)
        })
        .collect();
    vec![
        ("Seçilebilir türler", kinds),
        ("", vec!["edit.selectFilterAll", "edit.selectFilterNone"]),
    ]
}

impl App {
    /// Önceki seçim's objects still in the drawing and shown, in their order.
    pub(crate) fn previous_selection_ids(&self) -> Vec<Slot> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let layers = doc.model.layers();
        self.selection
            .previous()
            .iter()
            .copied()
            .filter(|s| {
                doc.model
                    .get(*s)
                    .is_some_and(|e| layers.is_visible(&e.base().layer_id))
            })
            .collect()
    }

    /// `edit.previousSelection`: the last selection replaced or cleared comes back.
    pub(crate) fn previous_selection(&mut self) {
        let ids = self.previous_selection_ids();
        if ids.is_empty() {
            self.warn("Geri getirilecek önceki seçim yok.");
            return;
        }
        let n = ids.len();
        self.selection.set(ids);
        self.output(format!("Önceki seçim geri geldi: {n} nesne."));
    }

    /// `edit.cycleSelection` (Shift+Boşluk): the chip's next candidate.
    pub(crate) fn cycle_selection(&mut self) {
        if let Some(index) = self.selection.cycle().map(|c| c.index) {
            self.selection.cycle_to(index + 1);
        }
    }

    /// `edit.selectFilter`: the filter on or off, its kinds kept.
    pub(crate) fn toggle_select_filter(&mut self) {
        let key = "drafting.selectFilter";
        let on = !self.settings.bool(key);
        let _ = self.settings.choose(&[(key, Value::Bool(on))]);
        self.apply_settings();
        self.output(if on {
            format!("Seçim süzgeci açık: {}.", kinds_text(self.select_kinds))
        } else {
            "Seçim süzgeci kapalı.".to_owned()
        });
    }

    /// `edit.selectFilter.<kind>`: the kind ticked or not; the filter on.
    pub(crate) fn toggle_select_kind(&mut self, kind: &str) {
        self.select_kinds ^= selectable::bit(kind);
        self.filter_on();
    }

    /// Bütün türler or Hiçbir tür from the cell's menu; the filter on.
    pub(crate) fn select_all_kinds(&mut self, on: bool) {
        self.select_kinds = if on { selectable::ALL } else { 0 };
        self.filter_on();
    }

    fn filter_on(&mut self) {
        let _ = self
            .settings
            .choose(&[("drafting.selectFilter", Value::Bool(true))]);
        self.apply_settings();
    }

    /// Whether a kind's command is ticked.
    pub(crate) fn select_kind_checked(&self, kind: &str) -> bool {
        self.select_kinds & selectable::bit(kind) != 0
    }

    /// The Süzgeç cell's right-click menu (the web's `selectFilterMenu`): the
    /// kinds by their short names and icons, each ticked or not, then every
    /// kind or none at once; it stays open as rows are ticked (docs/adr/0187
    /// §5). The ribbon's Seçim süzgeci ▾ shows the same list under its on/off
    /// row (`view::checklist_of`).
    pub(crate) fn select_filter_menu(&self) -> Menu<Message> {
        let blocks = select_filter_blocks();
        let checked: Vec<Option<bool>> = blocks
            .iter()
            .flat_map(|(_, ids)| ids.iter())
            .map(|id| self.checked(id))
            .collect();
        crate::view::checklist_of(&blocks, &checked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Item;

    fn app() -> App {
        App::boot(None).0
    }

    /// The ribbon's Seçim süzgeci ▾, as the catalog reads it from the web's layout.
    fn ribbon_menu() -> (Vec<(&'static str, Vec<&'static str>)>, bool) {
        catalog()
            .tabs()
            .flat_map(|tab| tab.panels.iter())
            .flat_map(|panel| panel.items.iter())
            .find_map(|item| match item {
                Item::Menu {
                    label: "Seçim süzgeci",
                    blocks,
                    checklist,
                    ..
                } => Some((blocks.clone(), *checklist)),
                _ => None,
            })
            .expect("Giriş › Seçim süzgeci ▾")
    }

    #[test]
    fn the_cell_s_menu_lists_seventeen_kinds_by_short_name_and_icon_and_stays_open() {
        let menu = format!("{:?}", app().select_filter_menu());
        assert!(menu.contains("Header(\"Seçilebilir türler\")"), "{menu}");
        for kind in KINDS {
            let name = kind_title(kind);
            assert!(menu.contains(&format!("label: \"{name}\"")), "{name}: {menu}");
        }
        assert!(menu.contains("label: \"Resim\""), "the seventeenth kind");
        // No long titles: the short names under the header, as the web's.
        assert!(!menu.contains("Seçim süzgecinde"), "{menu}");
        for row in ["Bütün türler", "Hiçbir tür"] {
            assert!(menu.contains(&format!("label: \"{row}\"")), "{row}");
        }
        // Every row leaves the menu open and keeps a ribbon opened over the drawing.
        assert_eq!(menu.matches("stay: true").count(), KINDS.len() + 2, "{menu}");
        assert!(menu.contains("RunKept(\"edit.selectFilter.image\")"));
        assert!(menu.contains("RunKept(\"edit.selectFilterNone\")"));
        assert!(!menu.contains("Run(\""), "{menu}");
    }

    #[test]
    fn the_ribbon_s_drop_down_is_the_same_checklist_under_its_on_off_row() {
        let (blocks, checklist) = ribbon_menu();
        assert!(checklist, "Seçim süzgeci ▾ is a checklist");
        let titles: Vec<&str> = blocks.iter().map(|(title, _)| *title).collect();
        assert_eq!(titles, ["", "Seçilebilir türler", ""]);
        assert_eq!(blocks[0].1, ["edit.selectFilter"]);
        // The cell's list, row for row.
        assert_eq!(blocks[1..], select_filter_blocks()[..]);
        let app = app();
        let ids: Vec<&str> = blocks.iter().flat_map(|(_, ids)| ids.iter().copied()).collect();
        let checked: Vec<Option<bool>> = ids.iter().map(|id| app.checked(id)).collect();
        let menu = format!("{:?}", crate::view::checklist_of(&blocks, &checked));
        // The on/off row says the cell's word, with its icon and its tick.
        assert!(menu.contains("label: \"Süzgeç\""), "{menu}");
        assert!(menu.contains("RunKept(\"edit.selectFilter\")"), "{menu}");
        assert_eq!(menu.matches("stay: true").count(), ids.len(), "{menu}");
    }

    #[test]
    fn hicbir_tur_then_ticks_make_the_filter_and_a_kept_run_leaves_the_ribbon_open() {
        let mut app = app();
        app.ribbon_peek = true;
        let _ = app.update(Message::RunKept("edit.selectFilterNone"));
        assert!(app.ribbon_peek, "a row that stays leaves the ribbon over the drawing");
        assert!(app.settings.bool("drafting.selectFilter"), "the filter is on");
        assert_eq!(app.select_kinds, 0);
        let _ = app.update(Message::RunKept("edit.selectFilter.text"));
        let _ = app.update(Message::RunKept("edit.selectFilter.image"));
        assert_eq!(
            app.select_kinds,
            selectable::bit("text") | selectable::bit("image")
        );
        assert_eq!(app.checked("edit.selectFilter.image"), Some(true));
        assert_eq!(app.checked("edit.selectFilter.point"), Some(false));
        let _ = app.update(Message::RunKept("edit.selectFilterAll"));
        assert_eq!(app.select_kinds, selectable::ALL);
        // A plain command still closes it.
        let _ = app.update(Message::Run("edit.selectFilter"));
        assert!(!app.ribbon_peek);
        assert!(!app.settings.bool("drafting.selectFilter"));
    }
}
