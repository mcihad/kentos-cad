//! The sheet designer's work as catalog commands (docs/adr/9305): Komut ara and the command line run the messages the
//! Pafta tab, the sheets' list, the item tree and the designer's keys send, so a command that says “Sola hizala” does
//! what Hizala ▾ › Sola does. A command on the paper needs a sheet in front whose paper is not given over to the model;
//! without one it is off and Komut ara says what to do. Karelaj (`sheet.grid`) waits: the designer cannot put a grid
//! on a map yet.

use iced::Task;
use iced::keyboard::{Key, Modifiers, key::Named};
use kentos_sheet::kinds::ItemKind;
use kentos_sheet::model::Axis;
use kentos_sheet::ops::{
    AlignEdge, AlignTo, DistributeMode, Op, ReorderTo, SetFlag, SizeDimension,
};
use kentos_sheet::profile::ToolState;
use kentos_sheet_ui::save_template::SaveMessage;
use kentos_sheet_ui::variables::VariablesMessage;
use kentos_sheet_ui::{ExportKind, InspectorTab, Message as Sheet, Tool};

use crate::app::{App, Message};

/// The catalog's sheet commands the designer runs here, beside Yeni pafta, Şablondan pafta, Modele dön and Pafta
/// seti, which commands.rs ran before (they work with no sheet in front).
pub(crate) const COMMANDS: &[&str] = &[
    // The book: with or without a sheet in front.
    "sheet.open",
    "sheet.importKpafta",
    "sheet.undo",
    "sheet.redo",
    // The sheet in front.
    "sheet.pageSetup",
    "sheet.variables",
    "sheet.rename",
    "sheet.duplicate",
    "sheet.delete",
    "sheet.moveLeft",
    "sheet.moveRight",
    // The paper's tools.
    "sheet.tool.select",
    "sheet.tool.hand",
    // Düzen: the chosen items.
    "sheet.align.left",
    "sheet.align.center",
    "sheet.align.right",
    "sheet.align.top",
    "sheet.align.middle",
    "sheet.align.bottom",
    "sheet.alignTo.selection",
    "sheet.alignTo.page",
    "sheet.alignTo.margins",
    "sheet.distribute.hCenters",
    "sheet.distribute.hGaps",
    "sheet.distribute.vCenters",
    "sheet.distribute.vGaps",
    "sheet.matchSize.width",
    "sheet.matchSize.height",
    "sheet.order.front",
    "sheet.order.forward",
    "sheet.order.backward",
    "sheet.order.back",
    "sheet.group",
    "sheet.ungroup",
    "sheet.atlas",
    "sheet.renameItem",
    "sheet.duplicateItems",
    "sheet.deleteItems",
    "sheet.hideItems",
    "sheet.lockItems",
    "sheet.nudge",
    "sheet.selectAll",
    "sheet.escape",
    // The paper's view.
    "sheet.zoomPage",
    "sheet.zoomReal",
    "sheet.zoomSelection",
    "sheet.zoomIn",
    "sheet.zoomOut",
    // Çıktı.
    "sheet.preflight",
    "sheet.print",
    "sheet.export.pdf",
    "sheet.export.svg",
    "sheet.export.png",
    "sheet.export.kpafta",
    "sheet.saveTemplate",
];

/// Why a command on the paper is off with no drawing open.
const NO_DRAWING: &str = "Açık çizim yok.";
/// With no sheet in the project. The reasons are short enough for Komut ara's one line.
const NO_SHEETS: &str = "Projede pafta yok: Çıktı › Yeni pafta ile bir pafta açın.";
/// With the drawing in front: the sheets' tabs are under it.
const NOT_IN_FRONT: &str = "Önde pafta yok: alttaki sekmelerden bir paftayı açın.";
/// While a map's paper is given over to the model.
const MODEL_SCOPE: &str = "Model düzenleniyor: önce Kâğıda dön'e basın.";
const ONE: &str = "Önce paftada bir öğe seçin.";
const TWO: &str = "En az iki öğe seçin.";
const THREE: &str = "En az üç öğe seçin.";
/// What Kaydır says: the designer's arrow keys move the chosen items, a command has no direction.
const NUDGE: &str =
    "Paftada seçili öğeleri ok tuşları kaydırır: 1 mm, Shift ile 10 mm, Alt ile 0.1 mm.";

impl App {
    /// A sheet command of the catalog run by the designer; none for any other command. One that cannot run now says
    /// why, as its tip does.
    pub(crate) fn sheet_command(&mut self, id: &str) -> Option<Task<Message>> {
        if !COMMANDS.contains(&id) {
            return None;
        }
        if let Some(why) = self.sheet_why(id) {
            self.warn(why);
            return Some(Task::none());
        }
        let message = match id {
            "sheet.nudge" => {
                self.output(NUDGE);
                return Some(Task::none());
            }
            "sheet.hideItems" => {
                self.flag_chosen(false);
                return Some(Task::none());
            }
            "sheet.lockItems" => {
                self.flag_chosen(true);
                return Some(Task::none());
            }
            _ => self.designer_message(id)?,
        };
        Some(self.sheet_message(message))
    }

    /// The designer's own message for a command: what its button, list, tree or key sends.
    fn designer_message(&self, id: &str) -> Option<Sheet> {
        let open = self.sheets.open_sheet().map(|s| s.id.clone());
        // The sheet's tab: the model's is 0.
        let tab = open
            .as_deref()
            .and_then(|s| self.sheets.book().sheet_index(s))
            .map(|i| i + 1);
        Some(match id {
            // The first sheet, as the web's PAFTA.
            "sheet.open" => Sheet::Tab(1),
            "sheet.importKpafta" => Sheet::ImportKpafta,
            "sheet.undo" => Sheet::Undo,
            "sheet.redo" => Sheet::Redo,
            // Pafta › Sayfa: the inspector's Sayfa tab (paper, orientation, margins, grid).
            "sheet.pageSetup" => Sheet::InspectorTab(InspectorTab::Page),
            "sheet.variables" => Sheet::Variables(VariablesMessage::Open),
            // The sheets' list's Ad ver: the name is typed in the list.
            "sheet.rename" => Sheet::StartRenameSheet(open?),
            "sheet.duplicate" => Sheet::DuplicateSheet(open?),
            // As the tab's ×: said, and Geri al brings it back.
            "sheet.delete" => Sheet::CloseTab(tab?),
            "sheet.moveLeft" => Sheet::MoveTab(tab?, tab? - 1),
            "sheet.moveRight" => Sheet::MoveTab(tab?, tab? + 1),
            "sheet.tool.select" => Sheet::Tool(Tool::Select),
            "sheet.tool.hand" => Sheet::Tool(Tool::Hand),
            "sheet.align.left" => Sheet::Align(AlignEdge::Left),
            "sheet.align.center" => Sheet::Align(AlignEdge::Center),
            "sheet.align.right" => Sheet::Align(AlignEdge::Right),
            "sheet.align.top" => Sheet::Align(AlignEdge::Top),
            "sheet.align.middle" => Sheet::Align(AlignEdge::Middle),
            "sheet.align.bottom" => Sheet::Align(AlignEdge::Bottom),
            // Hizala ▾'s Sayfanın ortasına and Kenar boşluklarının ortasına; the selection's middle likewise.
            "sheet.alignTo.selection" => Sheet::AlignTo(AlignTo::Selection),
            "sheet.alignTo.page" => Sheet::AlignTo(AlignTo::Page),
            "sheet.alignTo.margins" => Sheet::AlignTo(AlignTo::Margins),
            "sheet.distribute.hCenters" => Sheet::Distribute(Axis::X, DistributeMode::Centers),
            "sheet.distribute.hGaps" => Sheet::Distribute(Axis::X, DistributeMode::Gaps),
            "sheet.distribute.vCenters" => Sheet::Distribute(Axis::Y, DistributeMode::Centers),
            "sheet.distribute.vGaps" => Sheet::Distribute(Axis::Y, DistributeMode::Gaps),
            "sheet.matchSize.width" => Sheet::MatchSize(SizeDimension::Width),
            "sheet.matchSize.height" => Sheet::MatchSize(SizeDimension::Height),
            "sheet.order.front" => Sheet::Order(ReorderTo::Front),
            "sheet.order.forward" => Sheet::Order(ReorderTo::Forward),
            "sheet.order.backward" => Sheet::Order(ReorderTo::Backward),
            "sheet.order.back" => Sheet::Order(ReorderTo::Back),
            "sheet.group" => Sheet::Group,
            "sheet.ungroup" => Sheet::Ungroup,
            // Ekle › Atlas: the atlas's window.
            "sheet.atlas" => Sheet::Atlas,
            // F2: the one chosen item's name is typed in the item tree.
            "sheet.renameItem" => Sheet::StartRenameItem(self.sheets.selection().first()?.clone()),
            "sheet.duplicateItems" => Sheet::Duplicate,
            "sheet.deleteItems" => Sheet::Delete,
            // Ctrl+A and Esc as the designer takes them.
            "sheet.selectAll" => self
                .sheets
                .key(&Key::Character("a".into()), Modifiers::COMMAND)?,
            "sheet.escape" => self
                .sheets
                .key(&Key::Named(Named::Escape), Modifiers::empty())?,
            "sheet.zoomPage" => Sheet::ZoomPage,
            "sheet.zoomReal" => Sheet::ZoomReal,
            "sheet.zoomSelection" => Sheet::ZoomSelection,
            "sheet.zoomIn" => Sheet::ZoomIn,
            "sheet.zoomOut" => Sheet::ZoomOut,
            // Çıktı › Ön denetim: the inspector's Ön denetim tab.
            "sheet.preflight" => Sheet::InspectorTab(InspectorTab::Preflight),
            "sheet.print" => Sheet::Print,
            "sheet.export.pdf" => Sheet::Export(ExportKind::Pdf),
            "sheet.export.svg" => Sheet::Export(ExportKind::Svg),
            "sheet.export.png" => Sheet::Export(ExportKind::Png),
            "sheet.export.kpafta" => Sheet::Export(ExportKind::Kpafta),
            "sheet.saveTemplate" => Sheet::SaveTemplate(SaveMessage::Open),
            _ => return None,
        })
    }

    /// Gizle ya da göster, Kilitle ya da aç: the chosen items hidden (locked) unless all of them are, then all shown
    /// (unlocked), as one step of the designer's undo.
    fn flag_chosen(&mut self, lock: bool) {
        let book = self.sheets.book();
        let chosen: Vec<_> = self
            .sheets
            .selection()
            .iter()
            .filter_map(|id| book.item(id))
            .collect();
        let all = chosen
            .iter()
            .all(|i| if lock { i.locked } else { i.hidden });
        let flag = SetFlag {
            ids: chosen.iter().map(|i| i.id.clone()).collect(),
            value: !all,
        };
        let op = if lock { Op::Lock(flag) } else { Op::Hide(flag) };
        // The step named as the core names the tree's eye and lock (“Gizle: Harita”, “Kilidi aç: 3 öğe”).
        let label = kentos_sheet::ops::describe(book, &op);
        if !self.sheets.apply(vec![op], &label) {
            let why = self
                .sheets
                .error()
                .unwrap_or("Pafta çekirdeği işlemi yapmadı.")
                .to_owned();
            self.warn(why);
        }
    }

    /// Whether a sheet command of the catalog can run now; none for any other command.
    pub(crate) fn sheet_available(&self, id: &str) -> Option<bool> {
        COMMANDS.contains(&id).then(|| self.sheet_why(id).is_none())
    }

    /// Why a sheet command of the catalog cannot run now, said in its tip and in Komut ara; none when it can, or for
    /// any other command.
    pub(crate) fn sheet_why(&self, id: &str) -> Option<String> {
        if !COMMANDS.contains(&id) {
            return None;
        }
        if self.document.is_none() {
            return Some(NO_DRAWING.to_owned());
        }
        let sheets = self.sheets.book().sheets.len();
        // The book's own commands: with the drawing in front too.
        match id {
            "sheet.importKpafta" => return None,
            "sheet.undo" => {
                return (!self.sheets.can_undo())
                    .then(|| "Geri alınacak pafta işlemi yok.".to_owned());
            }
            "sheet.redo" => {
                return (!self.sheets.can_redo())
                    .then(|| "Yinelenecek pafta işlemi yok.".to_owned());
            }
            "sheet.open" => return (sheets == 0).then(|| NO_SHEETS.to_owned()),
            _ => {}
        }
        if sheets == 0 {
            return Some(NO_SHEETS.to_owned());
        }
        let Some(open) = self.sheets.open_sheet() else {
            return Some(NOT_IN_FRONT.to_owned());
        };
        if self.sheets.model_scope().is_some() {
            return Some(MODEL_SCOPE.to_owned());
        }
        let book = self.sheets.book();
        let at = book.sheet_index(&open.id);
        let chosen: Vec<_> = self
            .sheets
            .selection()
            .iter()
            .filter_map(|i| book.item(i))
            .collect();
        let least = |n: usize, why: &str| (chosen.len() < n).then(|| why.to_owned());
        match id {
            "sheet.moveLeft" if at == Some(0) => Some("Pafta zaten en solda.".to_owned()),
            "sheet.moveRight" if at.is_some_and(|i| i + 1 == sheets) => {
                Some("Pafta zaten en sağda.".to_owned())
            }
            "sheet.alignTo.selection"
            | "sheet.matchSize.width"
            | "sheet.matchSize.height"
            | "sheet.group" => least(2, TWO),
            id if id.starts_with("sheet.distribute.") => least(3, THREE),
            id if id.starts_with("sheet.align.")
                || id.starts_with("sheet.alignTo.")
                || id.starts_with("sheet.order.") =>
            {
                least(1, ONE)
            }
            "sheet.duplicateItems"
            | "sheet.hideItems"
            | "sheet.lockItems"
            | "sheet.nudge"
            | "sheet.zoomSelection" => least(1, ONE),
            "sheet.deleteItems" => least(1, ONE).or_else(|| {
                chosen
                    .iter()
                    .all(|i| i.locked)
                    .then(|| "Seçili öğeler kilitli: önce kilitlerini açın.".to_owned())
            }),
            "sheet.ungroup" => (!chosen.iter().any(|i| matches!(i.kind, ItemKind::Group(_))))
                .then(|| "Önce bir grup seçin.".to_owned()),
            "sheet.renameItem" => (chosen.len() != 1).then(|| "Tek bir öğe seçin.".to_owned()),
            "sheet.atlas" => self.atlas_why(),
            _ => None,
        }
    }

    /// Why Atlas is off in this project's sheets: the designer's profile hides it or shows it off, with its reason.
    fn atlas_why(&self) -> Option<String> {
        let tool = self
            .sheets
            .profile()
            .groups
            .iter()
            .flat_map(|g| g.tools.iter())
            .find(|t| t.id == "atlas");
        match tool {
            None => Some("Bu proje türünün paftalarında atlas yok.".to_owned()),
            Some(t) if t.state != ToolState::Enabled => Some(
                t.reason
                    .clone()
                    .unwrap_or_else(|| "Bu projede atlas kullanılamaz.".to_owned()),
            ),
            Some(_) => None,
        }
    }

    /// Seç and El lit while they are the paper's tool; none for any other command.
    pub(crate) fn sheet_checked(&self, id: &str) -> Option<bool> {
        let tool = match id {
            "sheet.tool.select" => Tool::Select,
            "sheet.tool.hand" => Tool::Hand,
            _ => return None,
        };
        Some(self.sheets.is_active() && *self.sheets.tool() == tool)
    }
}
