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
    /// kinds, each ticked or not, then every kind or none at once.
    pub(crate) fn select_filter_menu(&self) -> Menu<Message> {
        let mut menu = Menu::new().header("Seçilebilir türler");
        for kind in KINDS {
            let id = catalog()
                .commands()
                .iter()
                .find(|c| c.id.strip_prefix("edit.selectFilter.") == Some(kind))
                .map(|c| c.id);
            let Some(id) = id else {
                continue;
            };
            menu = menu
                .check(
                    kind_title(kind),
                    self.select_kind_checked(kind),
                    Message::Run(id),
                )
                .icon(crate::icons::from_web(Some(kind_icon(kind))));
        }
        menu.separator()
            .item("Bütün türler", Message::SelectKinds(true))
            .icon(crate::icons::from_web(Some("selectAll")))
            .item("Hiçbir tür", Message::SelectKinds(false))
            .icon(crate::icons::from_web(Some("deselect")))
    }
}
