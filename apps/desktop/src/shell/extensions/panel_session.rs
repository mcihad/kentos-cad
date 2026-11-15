//! An extension panel's session (plan/eklenti.md ADIM 22's "Tanım ve oturum", PY-UI-03 and 04, docs/adr/8057 §32):
//! the code a panel's button (or a changed input) started changes that panel as it runs: a field's value, a choice's
//! options, a text's words, what is shown and what can be used, a table's rows; it says a line under the form, puts the
//! cursor in a field, or closes the panel.
//!
//! - **Its own panels only:** the run's origin (panels.rs `PanelOrigin`), or one its manifest declares, named by the
//!   request (`panel`): `ctx.open_panel` opens it (or brings it in front) and holds it until the run ends. A run not
//!   started from a panel has no origin: its code's `ctx.panel` is None.
//! - **The user's typing wins:** an input the user changed while the command runs keeps what the user gave; the code's
//!   value is not put over it (`applied: false`). A change moves neither the focus, nor the text's cursor, nor the
//!   panel's scroll: the form is drawn from the panel's state, the widgets keep their own.
//! - **Checked as the panel's definition is:** an id the panel does not have, a property its component does not take,
//!   a value of the wrong kind or out of its bounds, a choice not among the options are the code's mistakes, its call's
//!   error (said to its code and its log, not put to the user); nothing of that call is applied.
//! - **A closed panel** answers `applied: false, closed: true`: the user closed it, not the code's mistake.

use kentos_extensions::ui_panel::{Choice, Component};
use kentos_headless::HeadlessError;
use serde_json::{Map, Value, json};

use super::panels::{OpenPanel, date_time_words, date_words, number_words};
use crate::app::App;
use crate::exchange::words;

/// The widget id of a panel's typed field: the code's `focus` puts the cursor there.
pub(crate) fn field_id(ext: &str, panel: &str, id: &str) -> iced::widget::Id {
    iced::widget::Id::from(format!("eklenti-paneli/{ext}/{panel}/{id}"))
}

fn refused(code: &'static str, message: impl Into<String>) -> HeadlessError {
    HeadlessError::new(code, message)
}

/// The panel's component `id`, or why not (the ids it has).
fn component<'a>(panel: &'a OpenPanel, id: &str) -> Result<&'a Component, HeadlessError> {
    panel
        .definition
        .body
        .iter()
        .find(|c| c.id() == Some(id))
        .ok_or_else(|| {
            let ids: Vec<&str> = panel
                .definition
                .body
                .iter()
                .filter_map(Component::id)
                .collect();
            refused(
                "unknown_component",
                format!(
                    "“{id}” panelde yok; kimliği olan bileşenler: {}.",
                    if ids.is_empty() {
                        "yok".to_owned()
                    } else {
                        ids.join(", ")
                    }
                ),
            )
        })
}

/// The kind a component is written with, for the code's words.
fn kind_of(c: &Component) -> &'static str {
    match c {
        Component::Text { .. } => "text",
        Component::TextField { .. } => "textField",
        Component::NumberField { .. } => "numberField",
        Component::Select { .. } => "select",
        Component::LayerPicker { .. } => "layerPicker",
        Component::FieldPicker { .. } => "fieldPicker",
        Component::FeaturePicker { .. } => "featurePicker",
        Component::MultiSelect { .. } => "multiSelect",
        Component::DateField { .. } => "dateField",
        Component::TimeField { .. } => "timeField",
        Component::DateTimeField { .. } => "dateTimeField",
        Component::Checkbox { .. } => "checkbox",
        Component::Button { .. } => "button",
        Component::Table { .. } => "table",
        Component::Tree { .. } => "tree",
        Component::Stats { .. } => "stats",
        Component::Chart { .. } => "chart",
        Component::Card { .. } => "card",
    }
}

/// Whether `c` is typed into (its cursor can be put there, it can be turned off).
fn typed(c: &Component) -> bool {
    matches!(
        c,
        Component::TextField { .. }
            | Component::NumberField { .. }
            | Component::DateField { .. }
            | Component::TimeField { .. }
            | Component::DateTimeField { .. }
    )
}

/// Options as the code gives them: `[{"value": …, "label": …}]` or values alone.
fn options_of(id: &str, given: &Value) -> Result<Vec<Choice>, HeadlessError> {
    let wrong = || {
        refused(
            "invalid_input",
            format!(
                "“{id}”: options bir liste olmalı: [{{\"value\": …, \"label\": …}}] ya da değerler."
            ),
        )
    };
    let list = given.as_array().ok_or_else(wrong)?;
    let mut out: Vec<Choice> = Vec::with_capacity(list.len());
    for item in list {
        let (value, label) = match item {
            Value::String(v) => (v.clone(), v.clone()),
            Value::Object(o) => {
                let value = o.get("value").and_then(Value::as_str).ok_or_else(wrong)?;
                let label = o.get("label").and_then(Value::as_str).unwrap_or(value);
                (value.to_owned(), label.to_owned())
            }
            _ => return Err(wrong()),
        };
        // As Kapı 1 asks of a definition's options.
        if value.is_empty() || label.trim().is_empty() {
            return Err(refused(
                "invalid_input",
                format!("“{id}”: seçenek boş olmayan bir değer (value) ve ad (label) ister."),
            ));
        }
        if out.iter().any(|c| c.value == value) {
            return Err(refused(
                "invalid_input",
                format!("“{id}”: “{value}” seçeneklerde iki kez geçiyor."),
            ));
        }
        out.push(Choice { value, label });
    }
    Ok(out)
}

/// What a typed field holds for `value` (as the user would have typed it), or why `value` does not fit it.
fn typed_words(c: &Component, value: &Value) -> Result<String, String> {
    if value.is_null() {
        return Ok(String::new());
    }
    match c {
        Component::TextField { .. } => value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "bir metin olmalı".to_owned()),
        Component::NumberField { min, max, .. } => {
            let n = value.as_f64().ok_or_else(|| "bir sayı olmalı".to_owned())?;
            if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) {
                return Err(format!(
                    "{} ile {} arasında olmalı",
                    min.map_or_else(|| "−∞".to_owned(), number_words),
                    max.map_or_else(|| "∞".to_owned(), number_words)
                ));
            }
            Ok(number_words(n))
        }
        Component::DateField { .. } => value
            .as_str()
            .filter(|v| kentos_extensions::ui_panel::iso_date(v).is_some())
            .map(date_words)
            .ok_or_else(|| "bir ISO tarihi olmalı (YYYY-AA-GG)".to_owned()),
        Component::TimeField { .. } => value
            .as_str()
            .and_then(kentos_extensions::ui_panel::typed_time)
            .ok_or_else(|| "bir saat olmalı (SS:DD)".to_owned()),
        Component::DateTimeField { .. } => value
            .as_str()
            .filter(|v| kentos_extensions::ui_panel::iso_date_time(v).is_some())
            .map(date_time_words)
            .ok_or_else(|| "bir ISO tarih ve saati olmalı (YYYY-AA-GGTSS:DD)".to_owned()),
        _ => Err("bir değer tutmaz".to_owned()),
    }
}

/// The drawing's layer ids, for a layer picker's value; none without a drawing.
fn layer_ids(app: &App) -> Option<Vec<String>> {
    let doc = &app.document.as_ref()?.model;
    Some(
        doc.layers()
            .leaves()
            .into_iter()
            .map(|l| l.id.clone())
            .collect(),
    )
}

/// `panel.set` on `panel`: every property checked first, then applied; whether the value went in (not when the user
/// changed that input while the command ran).
fn set(
    panel: &mut OpenPanel,
    params: &Value,
    layers: Option<&[String]>,
) -> Result<bool, HeadlessError> {
    let id = params
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| refused("invalid_input", "id gerekli: bileşenin kimliği."))?
        .to_owned();
    let c = component(panel, &id)?.clone();
    let props: &Map<String, Value> = params
        .as_object()
        .ok_or_else(|| refused("invalid_input", "Yama bir sözlük olmalı."))?;
    let not_taken = |prop: &str, only: &str| {
        refused(
            "invalid_input",
            format!("“{id}” ({}) {prop} almaz: {only}.", kind_of(&c)),
        )
    };
    let wrong = |why: String| refused("invalid_input", format!("“{id}” ({}): {why}.", kind_of(&c)));
    // Checked whole first: a wrong part applies nothing.
    let options = match props.get("options").filter(|v| !v.is_null()) {
        None => None,
        Some(given) => match &c {
            Component::Select { .. } | Component::MultiSelect { .. } => {
                Some(options_of(&id, given)?)
            }
            _ => {
                return Err(not_taken(
                    "options",
                    "yalnız select ve multiSelect seçenek alır",
                ));
            }
        },
    };
    let words = match props.get("text").filter(|v| !v.is_null()) {
        None => None,
        Some(given) => match &c {
            Component::Text { .. } => Some(
                given
                    .as_str()
                    .ok_or_else(|| wrong("text bir metin olmalı".to_owned()))?
                    .chars()
                    .take(2000)
                    .collect::<String>(),
            ),
            _ => return Err(not_taken("text", "yalnız text bileşeninin yazısı değişir")),
        },
    };
    let enabled = match props.get("enabled").filter(|v| !v.is_null()) {
        None => None,
        Some(given) => {
            let on = given
                .as_bool()
                .ok_or_else(|| wrong("enabled true ya da false olmalı".to_owned()))?;
            if !(typed(&c) || matches!(c, Component::Button { .. })) {
                return Err(not_taken(
                    "enabled",
                    "düğmeler ile metin, sayı, tarih ve saat alanları kapatılır",
                ));
            }
            Some(on)
        }
    };
    let visible = match props.get("visible").filter(|v| !v.is_null()) {
        None => None,
        Some(given) => Some(
            given
                .as_bool()
                .ok_or_else(|| wrong("visible true ya da false olmalı".to_owned()))?,
        ),
    };
    // The value, against the options it will have.
    enum Given {
        Words(String),
        Check(bool),
        Choice(String),
        Chosen(Vec<String>),
        Picks(Vec<String>),
    }
    let value = match props.get("value") {
        None => None,
        Some(given) => Some(match &c {
            c if typed(c) => Given::Words(typed_words(c, given).map_err(wrong)?),
            Component::Checkbox { .. } => Given::Check(
                given
                    .as_bool()
                    .ok_or_else(|| wrong("value true ya da false olmalı".to_owned()))?,
            ),
            Component::Select {
                options: defined, ..
            } => {
                let choice = given
                    .as_str()
                    .ok_or_else(|| wrong("value seçeneklerden birinin değeri olmalı".to_owned()))?;
                let now = options
                    .as_deref()
                    .unwrap_or_else(|| panel.options_of(&id, defined));
                if !now.iter().any(|o| o.value == choice) {
                    return Err(wrong(format!("“{choice}” seçeneklerde yok")));
                }
                Given::Choice(choice.to_owned())
            }
            Component::MultiSelect {
                options: defined, ..
            } => {
                let list = given
                    .as_array()
                    .ok_or_else(|| wrong("value seçenek değerlerinin listesi olmalı".to_owned()))?;
                let now = options
                    .as_deref()
                    .unwrap_or_else(|| panel.options_of(&id, defined));
                let mut chosen = Vec::with_capacity(list.len());
                for v in list {
                    let v = v.as_str().ok_or_else(|| {
                        wrong("value seçenek değerlerinin listesi olmalı".to_owned())
                    })?;
                    if !now.iter().any(|o| o.value == v) {
                        return Err(wrong(format!("“{v}” seçeneklerde yok")));
                    }
                    chosen.push(v.to_owned());
                }
                Given::Chosen(chosen)
            }
            Component::LayerPicker { .. } => {
                let layer = given.as_str().unwrap_or_default();
                if !given.is_null() && !given.is_string() {
                    return Err(wrong("value bir katman kimliği olmalı".to_owned()));
                }
                if !layer.is_empty()
                    && let Some(layers) = layers
                    && !layers.iter().any(|l| l == layer)
                {
                    return Err(wrong(format!("“{layer}” çizimde bir katman değil")));
                }
                Given::Choice(layer.to_owned())
            }
            Component::FieldPicker { .. } => {
                if !given.is_null() && !given.is_string() {
                    return Err(wrong("value bir alan adı olmalı".to_owned()));
                }
                Given::Choice(given.as_str().unwrap_or_default().to_owned())
            }
            Component::FeaturePicker { .. } => {
                let uids: Vec<String> = match given {
                    Value::Null => Vec::new(),
                    Value::String(u) => vec![u.clone()],
                    Value::Array(list) => list
                        .iter()
                        .map(|u| u.as_str().map(str::to_owned))
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| {
                            wrong("value nesnelerin kalıcı kimlikleri olmalı".to_owned())
                        })?,
                    _ => {
                        return Err(wrong(
                            "value nesnelerin kalıcı kimlikleri olmalı".to_owned(),
                        ));
                    }
                };
                if let Some(bad) = uids
                    .iter()
                    .find(|u| kentos_domain::Uuid::parse_str(u).is_err())
                {
                    return Err(wrong(format!("“{bad}” bir kalıcı kimlik değil")));
                }
                Given::Picks(uids)
            }
            _ => {
                return Err(not_taken(
                    "value",
                    "değer tutan bileşenlerin değeri değişir",
                ));
            }
        }),
    };
    // Applied: the options before the value they hold.
    if let Some(options) = options {
        match &c {
            Component::Select { .. } => {
                let kept = panel
                    .choices
                    .get(&id)
                    .is_some_and(|v| options.iter().any(|o| o.value == *v));
                if !kept {
                    let first = options.first().map(|o| o.value.clone()).unwrap_or_default();
                    panel.choices.insert(id.clone(), first);
                }
            }
            _ => {
                if let Some(chosen) = panel.multi.get_mut(&id) {
                    chosen.retain(|v| options.iter().any(|o| o.value == *v));
                }
            }
        }
        panel.options.insert(id.clone(), options);
    }
    if let Some(words) = words {
        panel.words.insert(id.clone(), words);
    }
    if let Some(on) = enabled {
        if on {
            panel.disabled.remove(&id);
        } else {
            panel.disabled.insert(id.clone());
        }
    }
    if let Some(shown) = visible {
        if shown {
            panel.hidden.remove(&id);
        } else {
            panel.hidden.insert(id.clone());
        }
    }
    let Some(value) = value else {
        return Ok(true);
    };
    // The user's own value stays.
    if panel.touched.contains(&id) {
        return Ok(false);
    }
    match value {
        Given::Words(words) => {
            panel.texts.insert(id.clone(), words);
        }
        Given::Check(on) => {
            panel.checks.insert(id.clone(), on);
        }
        Given::Choice(choice) => panel.choose(id.clone(), choice),
        Given::Chosen(chosen) => {
            panel.multi.insert(id.clone(), chosen);
        }
        Given::Picks(uids) => {
            panel.picks.insert(id.clone(), uids);
        }
    }
    // A value the code put right is not wrong any more.
    if panel.invalid.as_ref().is_some_and(|(at, _)| *at == id) {
        panel.invalid = None;
    }
    Ok(true)
}

/// `panel.rows`: the rows of a table, a tree, cards, figures or a chart, as a command's report gives them (`rows`, and
/// `columns` in their order, else in the order the rows first give them); the component's id.
fn rows(panel: &mut OpenPanel, params: &Value) -> Result<String, HeadlessError> {
    let id = params
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| refused("invalid_input", "id gerekli: tablonun kimliği."))?
        .to_owned();
    let c = component(panel, &id)?;
    if !matches!(
        c,
        Component::Table { .. }
            | Component::Tree { .. }
            | Component::Stats { .. }
            | Component::Chart { .. }
            | Component::Card { .. }
    ) {
        return Err(refused(
            "invalid_input",
            format!(
                "“{id}” ({}) satır almaz: satırlar table, tree, card, stats ya da chart'a gider.",
                kind_of(c)
            ),
        ));
    }
    let list = params
        .get("rows")
        .and_then(Value::as_array)
        .filter(|rows| rows.iter().all(Value::is_object))
        .ok_or_else(|| {
            refused(
                "invalid_input",
                format!("“{id}”: rows bir satır listesi olmalı (her biri sütun → değer sözlüğü)."),
            )
        })?;
    let mut columns: Vec<String> = params
        .get("columns")
        .and_then(Value::as_array)
        .map(|c| {
            c.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    if columns.is_empty() {
        for row in list {
            for key in row.as_object().into_iter().flat_map(|o| o.keys()) {
                if !columns.contains(key) {
                    columns.push(key.clone());
                }
            }
        }
    }
    panel.tables.insert(id.clone(), (columns, list.clone()));
    Ok(id)
}

impl App {
    /// `method` (`panel.open`, `panel.set`, `panel.rows`, `panel.say`, `panel.focus`, `panel.close`) asked by `ext`'s
    /// running code of one of its panels (`panel`, else the one its run started from): what it applied, or why not, the
    /// call's error.
    pub(crate) fn ext_panel_session(
        &mut self,
        ext: &str,
        method: &str,
        params: &Value,
    ) -> Result<Value, HeadlessError> {
        let run = self
            .extensions
            .runners
            .get(ext)
            .and_then(|r| r.run.as_ref())
            .ok_or_else(|| refused("busy", "Eklentinin çalışan bir işlevi yok."))?;
        let origin = run.origin.clone();
        let local = match params.get("panel").and_then(Value::as_str) {
            // One of its own, as its installed manifest declares them.
            Some(local) => {
                let declared: Vec<String> = self
                    .extensions
                    .registry
                    .as_ref()
                    .and_then(|r| r.manifest(ext))
                    .map(|m| m.entrypoints.panels.iter().map(|p| p.id.clone()).collect())
                    .unwrap_or_default();
                if !declared.iter().any(|p| p == local) {
                    return Err(refused(
                        "unknown_panel",
                        format!(
                            "“{local}” eklentinin bir paneli değil; panelleri: {}.",
                            if declared.is_empty() {
                                "yok".to_owned()
                            } else {
                                declared.join(", ")
                            }
                        ),
                    ));
                }
                local.to_owned()
            }
            None => origin.map(|o| o.panel).ok_or_else(|| {
                refused(
                    "no_panel",
                    "Bu çalıştırma bir panelden başlamadı: ctx.panel yalnız bir panelin düğmesinin (ya da değişen \
                     alanının) komutunda vardır; başka bir panel için ctx.open_panel.",
                )
            })?,
        };
        let open = |app: &App| {
            app.extensions
                .panels
                .iter()
                .position(|p| p.ext == ext && p.local == local)
        };
        // Opened (or brought in front) and held until the run ends.
        if method == "panel.open" {
            let task = self.ext_panel_open(ext, &local);
            self.extensions.later.push(task);
            let at = open(self).ok_or_else(|| {
                refused(
                    "panel_invalid",
                    format!("“{local}” paneli açılamadı: tanım dosyası okunamadı (eklentinin günlüğünde)."),
                )
            })?;
            self.extensions.panels[at].held = true;
            return Ok(json!({ "applied": true }));
        }
        let Some(at) = open(self) else {
            return Ok(json!({ "applied": false, "closed": true }));
        };
        let origin = super::panels::PanelOrigin {
            ext: ext.to_owned(),
            panel: local.clone(),
            button: String::new(),
            table: None,
        };
        match method {
            "panel.set" => {
                let layers = layer_ids(self);
                let applied = set(&mut self.extensions.panels[at], params, layers.as_deref())?;
                self.ext_panel_seen(at);
                Ok(json!({ "applied": applied }))
            }
            "panel.rows" => {
                let id = rows(&mut self.extensions.panels[at], params)?;
                // The charts drawing them, worked out again (panel_chart.rs).
                self.ext_panel_charts(&origin.ext, &origin.panel, &id);
                self.ext_panel_seen(at);
                Ok(json!({ "applied": true }))
            }
            "panel.say" => {
                let text = params
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .ok_or_else(|| {
                        refused(
                            "invalid_input",
                            "text gerekli: panelin altında söylenecek söz.",
                        )
                    })?;
                let kind = match params.get("kind").and_then(Value::as_str).unwrap_or("info") {
                    "info" => words::Kind::Info,
                    "success" | "ok" => words::Kind::Ok,
                    "warning" => words::Kind::Warn,
                    "error" => words::Kind::Error,
                    other => {
                        return Err(refused(
                            "invalid_input",
                            format!("kind “{other}” olamaz: info, success, warning ya da error."),
                        ));
                    }
                };
                let panel = &mut self.extensions.panels[at];
                panel.said = Some((kind, text.chars().take(500).collect()));
                panel.spoke = true;
                self.ext_panel_seen(at);
                Ok(json!({ "applied": true }))
            }
            "panel.focus" => {
                let id = params
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| refused("invalid_input", "id gerekli: alanın kimliği."))?;
                let panel = &self.extensions.panels[at];
                let c = component(panel, id)?;
                if !typed(c) {
                    return Err(refused(
                        "invalid_input",
                        format!(
                            "“{id}” ({}) odak almaz: imleç metin, sayı, tarih ve saat alanlarına konur.",
                            kind_of(c)
                        ),
                    ));
                }
                // Only in the panel shown, a field shown and in use: the code does not bring its tab in front of what
                // the user looks at.
                if at != self.extensions.active_panel
                    || panel.hidden.contains(id)
                    || panel.disabled.contains(id)
                {
                    return Ok(json!({ "applied": false }));
                }
                self.extensions
                    .later
                    .push(iced::widget::operation::focus(field_id(
                        &origin.ext,
                        &origin.panel,
                        id,
                    )));
                Ok(json!({ "applied": true }))
            }
            "panel.close" => {
                let task = self.ext_panel_close(at);
                self.extensions.later.push(task);
                Ok(json!({ "applied": true }))
            }
            other => Err(refused(
                "unknown_method",
                format!(
                    "“{other}” bir panel isteği değil: panel.open, panel.set, panel.rows, panel.say, panel.focus ya da \
                     panel.close."
                ),
            )),
        }
    }

    /// `ext`'s run ended (or its process closed): the panels its code held are the user's again.
    pub(crate) fn ext_panels_released(&mut self, ext: &str) {
        for panel in self.extensions.panels.iter_mut().filter(|p| p.ext == ext) {
            panel.held = false;
            if panel.running.is_none() {
                panel.touched.clear();
            }
        }
    }

    /// A panel changed by its code while another was shown: its tab marks it until it is shown.
    fn ext_panel_seen(&mut self, at: usize) {
        if at != self.extensions.active_panel
            && let Some(panel) = self.extensions.panels.get_mut(at)
        {
            panel.unseen = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;

    use super::*;

    fn panel() -> OpenPanel {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Deneme",
            "body": [
                { "type": "text", "id": "durum", "text": "Hazır." },
                { "type": "textField", "id": "ad", "label": "Ad", "required": true },
                { "type": "numberField", "id": "adet", "label": "Adet", "min": 0, "max": 10 },
                { "type": "select", "id": "mahalle", "label": "Mahalle",
                  "options": [{ "value": "a", "label": "A" }, { "value": "b", "label": "B" }] },
                { "type": "multiSelect", "id": "bilgi", "label": "Bilgi", "value": ["x", "z"],
                  "options": [{ "value": "x", "label": "X" }, { "value": "y", "label": "Y" },
                              { "value": "z", "label": "Z" }] },
                { "type": "checkbox", "id": "tamam", "label": "Tamam" },
                { "type": "dateField", "id": "gun", "label": "Gün" },
                { "type": "button", "id": "ara", "label": "Ara", "command": "ara" },
                { "type": "table", "id": "liste" }
            ] });
        OpenPanel::new(
            "tr.ornek.deneme",
            "deneme",
            ui_panel::read(&value).expect("a panel"),
        )
    }

    fn code(r: Result<bool, HeadlessError>) -> String {
        match r {
            Ok(applied) => format!("ok {applied}"),
            Err(e) => format!("{}: {}", e.code, e.message),
        }
    }

    /// docs/adr/8057 §32: values, options, words, what is shown and in use, set as the definition allows; a wrong part
    /// applies nothing and says what is wrong where.
    #[test]
    fn the_code_sets_its_panels_values_options_and_state_as_its_definition_allows() {
        let mut p = panel();
        assert_eq!(
            code(set(&mut p, &json!({ "id": "ad", "value": "Ayşe" }), None)),
            "ok true"
        );
        assert_eq!(p.texts["ad"], "Ayşe");
        assert_eq!(
            code(set(&mut p, &json!({ "id": "adet", "value": 4.5 }), None)),
            "ok true"
        );
        assert_eq!(p.texts["adet"], "4,5", "as the user writes it");
        assert_eq!(
            code(set(&mut p, &json!({ "id": "adet", "value": 12 }), None)),
            "invalid_input: “adet” (numberField): 0 ile 10 arasında olmalı."
        );
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "gun", "value": "2026-11-15" }),
                None
            )),
            "ok true"
        );
        assert_eq!(p.texts["gun"], "15.11.2026");
        assert!(
            code(set(&mut p, &json!({ "id": "gun", "value": "15/11" }), None))
                .starts_with("invalid_input")
        );
        // A choice's options replaced: the value set against them, one not among them refused.
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "mahalle", "options": ["Merkez", { "value": "yeni", "label": "Yeni Mahalle" }],
                         "value": "yeni" }),
                None
            )),
            "ok true"
        );
        assert_eq!(p.choices["mahalle"], "yeni");
        assert_eq!(p.options_of("mahalle", &[]).len(), 2);
        assert_eq!(
            code(set(&mut p, &json!({ "id": "mahalle", "value": "a" }), None)),
            "invalid_input: “mahalle” (select): “a” seçeneklerde yok."
        );
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "mahalle", "options": [""] }),
                None
            )),
            "invalid_input: “mahalle”: seçenek boş olmayan bir değer (value) ve ad (label) ister."
        );
        // A multiple choice's chosen values not among its new options let go.
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "bilgi", "options": ["y", "z"] }),
                None
            )),
            "ok true"
        );
        assert_eq!(p.multi["bilgi"], ["z"]);
        let read = p.check_params().unwrap();
        assert_eq!(read["bilgi"], json!(["z"]));
        // Words, use and sight.
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "durum", "text": "3 kayıt." }),
                None
            )),
            "ok true"
        );
        assert_eq!(p.words["durum"], "3 kayıt.");
        assert_eq!(
            code(set(&mut p, &json!({ "id": "ara", "enabled": false }), None)),
            "ok true"
        );
        assert!(p.disabled.contains("ara"));
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "liste", "enabled": false }),
                None
            )),
            "invalid_input: “liste” (table) enabled almaz: düğmeler ile metin, sayı, tarih ve saat alanları kapatılır."
        );
        assert_eq!(
            code(set(&mut p, &json!({ "id": "tamam", "text": "x" }), None)),
            "invalid_input: “tamam” (checkbox) text almaz: yalnız text bileşeninin yazısı değişir."
        );
        // A hidden required field is not asked of the user.
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "ad", "value": null, "visible": false }),
                None
            )),
            "ok true"
        );
        assert!(p.check_params().is_ok(), "hidden: not asked");
        // An id the panel has not: said with the ids it has; a wrong part applies nothing.
        let e = code(set(&mut p, &json!({ "id": "yok", "value": 1 }), None));
        assert!(
            e.starts_with(
                "unknown_component: “yok” panelde yok; kimliği olan bileşenler: durum, ad"
            ),
            "{e}"
        );
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "tamam", "value": "evet", "visible": false }),
                None
            )),
            "invalid_input: “tamam” (checkbox): value true ya da false olmalı."
        );
        assert!(
            !p.hidden.contains("tamam"),
            "nothing of a wrong call applied"
        );
    }

    /// PY-UI-04: an input the user changed while the command runs keeps the user's value; the call's other parts go.
    #[test]
    fn the_users_typing_wins_over_the_codes_value() {
        let mut p = panel();
        p.running = Some("ara".into());
        p.forget_invalid(&super::super::panels::PanelEvent::Text(
            "ad".into(),
            "Ali".into(),
        ));
        p.texts.insert("ad".into(), "Ali".into());
        assert_eq!(
            code(set(
                &mut p,
                &json!({ "id": "ad", "value": "Veli", "enabled": false }),
                None
            )),
            "ok false"
        );
        assert_eq!(p.texts["ad"], "Ali", "the user's");
        assert!(p.disabled.contains("ad"), "the rest applied");
        assert_eq!(
            code(set(&mut p, &json!({ "id": "tamam", "value": true }), None)),
            "ok true"
        );
    }

    /// Rows to a table as a report gives them, their columns in the order given or first met; not to a button.
    #[test]
    fn rows_go_to_a_table_not_a_button() {
        let mut p = panel();
        let id = rows(
            &mut p,
            &json!({ "id": "liste", "rows": [{ "ad": "A" }, { "alan": 2, "ad": "B" }] }),
        )
        .unwrap();
        assert_eq!(id, "liste");
        assert_eq!(p.tables["liste"].0, ["ad", "alan"]);
        assert_eq!(p.tables["liste"].1.len(), 2);
        let e = rows(&mut p, &json!({ "id": "ara", "rows": [] })).unwrap_err();
        assert_eq!(e.code, "invalid_input");
        assert!(e.message.contains("satır almaz"), "{}", e.message);
    }
}
