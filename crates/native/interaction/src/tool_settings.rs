//! Ephemeral settings for the active architectural tool. Descriptors contain no geometry;
//! commits use the tool's existing parsers and validation. They never edit the selection.
use crate::format::Format;
use crate::tool::{Context, Memory, OptionChoice};
use crate::{Level, wall};
use kentos_commands::{WallStyleDef, fixed_width};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setting {
    pub key: &'static str,
    /// Its name: the tool's own words, or a block's parameter's (docs/adr/0726 §4).
    pub label: std::borrow::Cow<'static, str>,
    pub value: String,
    pub choices: Vec<OptionChoice>,
    pub editable: bool,
    /// A full-width wrapped explanation, never an editable cell.
    pub note: bool,
    /// Only actual warnings use the warning colour; explanatory hints remain quiet.
    pub warn: bool,
    /// A two-state setting shown directly rather than hidden in a choice menu.
    pub toggle: bool,
}
impl Setting {
    pub fn text(
        key: &'static str,
        label: impl Into<std::borrow::Cow<'static, str>>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            key,
            label: label.into(),
            value: value.into(),
            choices: Vec::new(),
            editable: true,
            note: false,
            warn: false,
            toggle: false,
        }
    }
    pub fn choice(
        key: &'static str,
        label: impl Into<std::borrow::Cow<'static, str>>,
        choices: Vec<OptionChoice>,
    ) -> Self {
        let value = choices
            .iter()
            .find(|c| c.checked)
            .map(|c| c.label.clone())
            .unwrap_or_default();
        Self {
            key,
            label: label.into(),
            value,
            choices,
            editable: true,
            note: false,
            warn: false,
            toggle: false,
        }
    }
    pub fn read_only(mut self) -> Self {
        self.editable = false;
        self
    }
    pub fn toggle(key: &'static str, label: impl Into<std::borrow::Cow<'static, str>>, on: bool) -> Self {
        Self {
            toggle: true,
            ..Self::choice(
                key,
                label,
                choices(
                    &[("yes", "Açık"), ("no", "Kapalı")],
                    if on { "yes" } else { "no" },
                ),
            )
        }
    }
    pub fn notice(
        key: &'static str,
        label: impl Into<std::borrow::Cow<'static, str>>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            note: true,
            warn: true,
            ..Self::text(key, label, value).read_only()
        }
    }
    pub fn hint(
        key: &'static str,
        label: impl Into<std::borrow::Cow<'static, str>>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            note: true,
            ..Self::text(key, label, value).read_only()
        }
    }
}
pub fn choices(items: &[(&str, &str)], selected: &str) -> Vec<OptionChoice> {
    items
        .iter()
        .map(|(typed, label)| OptionChoice {
            typed: (*typed).into(),
            label: (*label).into(),
            icon: None,
            preview: None,
            checked: *typed == selected,
            command: None,
        })
        .collect()
}

/// A separate suggestion row leaves the tool's original free-text field available.
pub fn suggestions(
    key: &'static str,
    label: &'static str,
    values: &[String],
    selected: Option<&str>,
) -> Setting {
    let mut row = Setting::choice(
        key,
        label,
        values
            .iter()
            .map(|value| OptionChoice {
                typed: value.clone(),
                label: value.clone(),
                checked: selected == Some(value.as_str()),
                icon: None,
                preview: None,
                command: None,
            })
            .collect(),
    );
    if row.value.is_empty() {
        row.value = "Listeden seç".into();
    }
    row
}

pub fn wall_rows(memory: &Memory, styles: &[WallStyleDef], f: &Format) -> Vec<Setting> {
    let style = wall::chosen_in(memory, styles);
    let mut width = Setting::text("E", "Kalınlık", f.length(wall::width(memory, style)));
    if style.and_then(|s| fixed_width(&s.components)).is_some() {
        width.editable = false;
    }
    vec![
        Setting::choice("S", "Stil", wall::choices("S", memory, styles)),
        width,
        Setting::choice("H", "Hiza", wall::choices("H", memory, styles)),
        Setting::text("Ö", "Öteleme", f.length(memory.wall_offset)),
    ]
}
pub fn wall_apply(key: &str, value: &str, cx: &mut Context<'_>) -> bool {
    match key {
        "S" | "H" => wall::choose(key, value, cx),
        "E" | "Ö" => {
            if key == "E"
                && wall::chosen(cx.memory, cx.doc.settings())
                    .as_ref()
                    .and_then(|s| fixed_width(&s.components))
                    .is_some()
            {
                cx.say(
                    Level::Warn,
                    "Bu stilin kalınlığı sabittir; başka bir stil seçin.",
                );
                return false;
            }
            let Some(v) = cx.typed_length(value).filter(|v| v.is_finite()) else {
                cx.say(
                    Level::Warn,
                    "Bir uzunluk yazın: 250mm, 0.25m veya 8\" gibi.",
                );
                return false;
            };
            let ask = if key == "E" {
                wall::Ask::Width
            } else {
                wall::Ask::Offset
            };
            let valid = if key == "E" {
                v > 0.0 && v <= 100.0
            } else {
                v.abs() <= 100.0
            };
            wall::typed(ask, v, cx);
            valid
        }
        _ => false,
    }
}
