//! What a trace can do in an open window (the `dialog` step,
//! fixtures/interaction/README.md): read its title, and find the message a
//! control of it sends, by the words the window shows, as the web's player
//! finds the control in the page and uses it with the mouse and the keyboard.
//! A field is filled over what it held, a check box set, a button pressed; a
//! pressed button that is off sends nothing, as a click on it would. Only a
//! window that answers here can be played; another says so.

use crate::app::{App, Dialog, Message};

/// A control of the open window, by its words.
#[derive(Clone, Copy, Debug)]
pub enum Control<'a> {
    /// The field labelled so, its text replaced by this one.
    Fill(&'a str, &'a str),
    /// The check box with these words, set on or off.
    Check(&'a str, bool),
    /// The button with these words.
    Press(&'a str),
    /// The item with these words of the list named so (the `panel` step's `pick`).
    Pick(&'a str, &'a str),
    /// The table's header with these words, pressed.
    Sort(&'a str),
    /// The n-th row of the table, 1 first, pressed.
    Row(usize),
    /// A key of the box that has the keyboard (`Enter`, `Esc`).
    Key(&'a str),
}

impl std::fmt::Display for Control<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Control::Fill(label, _) => write!(f, "“{label}” alanı"),
            Control::Check(words, _) => write!(f, "“{words}” kutusu"),
            Control::Press(words) => write!(f, "“{words}” düğmesi"),
            Control::Pick(list, item) => write!(f, "“{item}” öğesi (“{list}” listesi)"),
            Control::Sort(header) => write!(f, "“{header}” başlığı"),
            Control::Row(n) => write!(f, "{n}. sonuç satırı"),
            Control::Key(key) => write!(f, "{key} tuşu"),
        }
    }
}

impl App {
    /// The open window's title, as it shows it; a window no trace answers
    /// yet goes by its name in the code.
    pub fn dialog_title(&self) -> Option<String> {
        self.dialog.map(|d| match d {
            Dialog::BlockDefine => crate::blocks::DEFINE_TITLE.to_owned(),
            Dialog::BlockAttributes => crate::block_attributes::ATTRIBUTES_TITLE.to_owned(),
            Dialog::AttributeValues => crate::attribute_values::VALUES_TITLE.to_owned(),
            Dialog::FindReplace => crate::find_replace::FIND_TITLE.to_owned(),
            Dialog::LayerMerge => crate::layer_merge::MERGE_TITLE.to_owned(),
            Dialog::LayerStates => crate::layer_states::STATES_TITLE.to_owned(),
            Dialog::AnnotationStyles => self.annotation_styles_title().to_owned(),
            Dialog::LayerPurge => crate::layer_purge::PURGE_TITLE.to_owned(),
            Dialog::LayerList => crate::layer_list::LIST_TITLE.to_owned(),
            Dialog::DataCompare => crate::data_compare::COMPARE_TITLE.to_owned(),
            Dialog::TableInsert => crate::tables::insert::INSERT_TITLE.to_owned(),
            Dialog::TableEditor => crate::tables::editor::EDITOR_TITLE.to_owned(),
            Dialog::Cogo => crate::cogo::COGO_TITLE.to_owned(),
            Dialog::Project if self.asking_type() => crate::project::TYPE_TITLE.to_owned(),
            other => format!("{other:?}"),
        })
    }

    /// The message `control` of the open window sends; `None` when it sends
    /// nothing (a button that is off, a box already so).
    pub fn dialog_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        match self.dialog {
            Some(Dialog::BlockDefine) => self.block_define_control(control),
            Some(Dialog::BlockAttributes) => self.block_attributes_control(control),
            Some(Dialog::AttributeValues) => self.attribute_values_control(control),
            Some(Dialog::FindReplace) => self.find_replace_control(control),
            Some(Dialog::LayerMerge) => self.layer_merge_control(control),
            Some(Dialog::LayerStates) => self.layer_states_control(control),
            Some(Dialog::AnnotationStyles) => self.annotation_styles_control(control),
            Some(Dialog::LayerPurge) => self.layer_purge_control(control),
            Some(Dialog::LayerList) => self.layer_list_control(control),
            Some(Dialog::DataCompare) => self.data_compare_control(control),
            Some(Dialog::Cogo) => self.cogo_control(control),
            Some(Dialog::TableInsert) => self.table_insert_control(control),
            Some(Dialog::TableEditor) => self.table_editor_control(control),
            Some(Dialog::Project) if self.asking_type() => self.project_type_control(control),
            Some(other) => Err(format!("{other:?} penceresi izden yanıtlanamıyor")),
            None => Err("açık pencere yok".to_owned()),
        }
    }
}
