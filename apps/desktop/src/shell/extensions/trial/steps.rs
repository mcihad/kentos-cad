//! The trial's commands and map tools as the agent asks for them (plan/eklenti.md ADIM 21; docs/adr/8057 §52): read
//! from the call and checked against the installed manifest before anything runs; broker.rs runs them.
//!
//! - **As the user starts them:** a command as from the ribbon (`params` is its `ctx.params`, empty when not given),
//!   or as a panel's button or changed input (`panel`, `button`: the panel opened, the agent's values set in it and
//!   checked as the button checks them, its answer put on the panel as the user would see it); a map tool with the
//!   shape the user would draw, checked against the one its manifest asks for and given as the drawing gives a drawn
//!   one (runner.rs `shape_json`).
//! - **With what the user granted:** the rights are the installed extension's (`Rights`), its panels' definitions read
//!   from its package as a panel is when it opens.

use std::collections::BTreeMap;

use kentos_extensions::manifest::{Capability, Capture, Manifest, MapToolEntry};
use kentos_extensions::ui_panel::{Component, PanelDefinition};
use serde_json::{Value, json};

use super::super::audits::Refusal;
use super::super::tools::PoolJob;
use crate::app::App;

/// The most dialogs a step's answers are given for.
const MOST_DIALOGS: usize = 10;

/// A step as the agent asked for it, checked.
#[derive(Clone, Debug)]
pub(crate) struct Spec {
    /// `ext.<eklenti>.<komut>` (or the map tool's), as the report names it.
    pub(crate) id: String,
    pub(crate) map_tool: bool,
    pub(crate) title: String,
    /// What its code is given (`ctx.params`; a map tool's `{"shape": …}`); for a panel's button, the values set in the
    /// panel before they are checked.
    pub(crate) params: Value,
    /// Started from a panel: which, by which button (or changed input), and the button's own output.
    pub(crate) panel: Option<Origin>,
    /// The objects selected while it runs, by their place in the agent's list.
    pub(crate) selection: Vec<usize>,
    /// The answers to its dialogs, in the order its code asks them: values by field, or null for İptal.
    pub(crate) dialogs: Vec<Value>,
}

/// The panel a step starts from, as a button press does (panels.rs `ext_panel_press`).
#[derive(Clone, Debug)]
pub(crate) struct Origin {
    pub(crate) panel: String,
    pub(crate) button: String,
    /// The button's `table`; none: the panel's first output (`OpenPanel::table_of`).
    pub(crate) table: Option<String>,
}

/// A step and what runs its code.
pub(crate) struct Step {
    pub(crate) spec: Spec,
    pub(crate) job: PoolJob,
}

/// What a trial's steps run with: the extension's id, what the user granted it, the panels its manifest declares.
#[derive(Clone, Debug, Default)]
pub(crate) struct Rights {
    pub(crate) ext: String,
    pub(crate) capabilities: Vec<Capability>,
    /// The extensions it lists as dependencies: their tools it may run.
    pub(crate) depends: Vec<String>,
    /// Its panels by id, each definition as read from its package (none: unreadable).
    pub(crate) panels: BTreeMap<String, Option<PanelDefinition>>,
}

impl Rights {
    /// Reading the drawing: `document.read`, or `document.write` that holds it.
    pub(crate) fn read(&self) -> bool {
        self.capabilities.contains(&Capability::DocumentRead) || self.write()
    }

    pub(crate) fn write(&self) -> bool {
        self.capabilities.contains(&Capability::DocumentWrite)
    }
}

fn invalid(words: String) -> Refusal {
    ("invalid_input", words)
}

impl App {
    /// The commands and map tools `params` asks for (`commands`, `mapTools`), checked against `manifest` (`ext`'s, as
    /// installed) for a drawing of `objects` objects, and the rights they run with; why not, the call's refusal.
    pub(crate) fn trial_steps(
        &self,
        ext: &str,
        manifest: &Manifest,
        params: &Value,
        objects: usize,
    ) -> Result<(Vec<Step>, Rights), Refusal> {
        let most = kentos_headless::agent_tools::TRIAL_MOST_COMMANDS;
        let list = |key: &str| match params.get(key) {
            None | Some(Value::Null) => Ok(Vec::new()),
            Some(Value::Array(l)) if l.len() <= most => Ok(l.clone()),
            Some(_) => Err(invalid(format!(
                "{key}: en çok {most} adımlık bir liste olmalı."
            ))),
        };
        let (commands, map_tools) = (list("commands")?, list("mapTools")?);
        let registry = self
            .extensions
            .registry
            .as_ref()
            .ok_or(("not_ready", "Eklenti kayıt defteri açık değil.".to_owned()))?;
        let grant = registry
            .grant(ext)
            .ok_or(("unknown_extension", "Eklenti kurulu değil.".to_owned()))?;
        let mut rights = Rights {
            ext: ext.to_owned(),
            capabilities: grant.capabilities.clone(),
            depends: manifest
                .dependencies
                .extensions
                .iter()
                .map(|d| d.id.clone())
                .collect(),
            panels: BTreeMap::new(),
        };
        if commands.is_empty() && map_tools.is_empty() {
            return Ok((Vec::new(), rights));
        }
        if !grant.python {
            return Err((
                "not_ready",
                "Eklentinin Python işlevleri kurulumda seçilmedi: komutları çalışmaz.".to_owned(),
            ));
        }
        // Its panels, read as a panel is when it opens (panels.rs): a step may start from one, its code may open one.
        for entry in &manifest.entrypoints.panels {
            let definition = registry
                .package_file(ext, &entry.definition)
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .and_then(|value| kentos_extensions::ui_panel::read(&value));
            rights.panels.insert(entry.id.clone(), definition);
        }
        let own = format!("ext.{ext}.");
        let local_of = |given: &str| given.strip_prefix(&own).unwrap_or(given).to_owned();
        let mut steps = Vec::new();
        for (i, given) in commands.iter().enumerate() {
            let at = format!("commands[{i}]");
            let (local, panel) = command_of(given, &at, &rights, &local_of)?;
            let entry = manifest
                .entrypoints
                .commands
                .iter()
                .find(|c| c.id == local)
                .ok_or_else(|| {
                    invalid(format!(
                        "{at}: “{local}” bu eklentinin bir komutu değil (entrypoints.commands)."
                    ))
                })?;
            let params = match given.get("params") {
                None | Some(Value::Null) => json!({}),
                Some(p @ Value::Object(_)) => p.clone(),
                Some(_) => return Err(invalid(format!("{at}.params bir sözlük olmalı."))),
            };
            let job = self
                .pool_job(ext, &local, &entry.handler)
                .map_err(|why| ("not_ready", why))?;
            let spec = Spec {
                id: format!("ext.{ext}.{local}"),
                map_tool: false,
                title: entry.title.clone(),
                params,
                panel,
                selection: places(given, &at, objects)?,
                dialogs: answers(given, &at)?,
            };
            steps.push(Step { spec, job });
        }
        for (i, given) in map_tools.iter().enumerate() {
            let at = format!("mapTools[{i}]");
            let local = given
                .get("mapTool")
                .and_then(Value::as_str)
                .map(&local_of)
                .ok_or_else(|| {
                    invalid(format!("{at}.mapTool: harita aracının kimliği gerekli."))
                })?;
            let entry = manifest
                .entrypoints
                .map_tools
                .iter()
                .find(|t| t.id == local)
                .ok_or_else(|| {
                    invalid(format!(
                        "{at}: “{local}” bu eklentinin bir harita aracı değil (entrypoints.mapTools)."
                    ))
                })?;
            let shape = shape_of(entry, given.get("shape").unwrap_or(&Value::Null))
                .map_err(|why| invalid(format!("{at}.shape: {why}")))?;
            let job = self
                .pool_job(ext, &local, &entry.handler)
                .map_err(|why| ("not_ready", why))?;
            let spec = Spec {
                id: format!("ext.{ext}.{local}"),
                map_tool: true,
                title: entry.title.clone(),
                params: json!({ "shape": shape }),
                panel: None,
                selection: Vec::new(),
                dialogs: answers(given, &at)?,
            };
            steps.push(Step { spec, job });
        }
        Ok((steps, rights))
    }
}

/// The command a step names, as given (`command`) or as a panel's button (or changed input) runs it (`panel`,
/// `button`), with the panel it starts from.
fn command_of(
    given: &Value,
    at: &str,
    rights: &Rights,
    local_of: &dyn Fn(&str) -> String,
) -> Result<(String, Option<Origin>), Refusal> {
    let command = given.get("command").and_then(Value::as_str);
    let panel = given.get("panel").and_then(Value::as_str);
    match (command, panel) {
        (Some(command), None) => Ok((local_of(command), None)),
        (None, Some(panel)) => {
            let declared: Vec<&str> = rights.panels.keys().map(String::as_str).collect();
            let definition = rights
                .panels
                .get(panel)
                .ok_or_else(|| {
                    invalid(format!(
                        "{at}.panel: “{panel}” eklentinin bir paneli değil; panelleri: {}.",
                        if declared.is_empty() {
                            "yok".to_owned()
                        } else {
                            declared.join(", ")
                        }
                    ))
                })?
                .as_ref()
                .ok_or_else(|| {
                    invalid(format!(
                        "{at}.panel: “{panel}” panelinin tanımı okunamadı (eklentinin günlüğünde)."
                    ))
                })?;
            let button = given.get("button").and_then(Value::as_str).ok_or_else(|| {
                invalid(format!(
                    "{at}.button: panelin düğmesinin (ya da onChange'i olan alanının) kimliği gerekli."
                ))
            })?;
            let (command, table) = match definition.body.iter().find(|c| c.id() == Some(button)) {
                Some(Component::Button { command, table, .. }) => (command.clone(), table.clone()),
                Some(c) => match c.on_change() {
                    Some(command) => (command.to_owned(), None),
                    None => {
                        return Err(invalid(format!(
                            "{at}.button: “{button}” bir düğme ya da onChange'i olan bir alan değil."
                        )));
                    }
                },
                None => {
                    return Err(invalid(format!(
                        "{at}.button: “{button}” “{panel}” panelinde yok."
                    )));
                }
            };
            let origin = Origin {
                panel: panel.to_owned(),
                button: button.to_owned(),
                table,
            };
            Ok((command, Some(origin)))
        }
        (Some(_), Some(_)) => Err(invalid(format!(
            "{at}: ya command ya da panel ile button verilir, ikisi birden değil."
        ))),
        (None, None) => Err(invalid(format!(
            "{at}: command (komutun kimliği) ya da panel ile button gerekli."
        ))),
    }
}

/// The objects a step selects, by their place in the agent's list of `objects`.
fn places(given: &Value, at: &str, objects: usize) -> Result<Vec<usize>, Refusal> {
    let wrong = || {
        invalid(if objects == 0 {
            format!("{at}.selection: features boş, seçilecek nesne yok.")
        } else {
            format!(
                "{at}.selection: features'taki sıralar olmalı (0 ile {} arası).",
                objects - 1
            )
        })
    };
    match given.get("selection") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| {
                v.as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|n| *n < objects)
                    .ok_or_else(wrong)
            })
            .collect(),
        Some(_) => Err(wrong()),
    }
}

/// The answers to a step's dialogs: each the fields' values or null (İptal).
fn answers(given: &Value, at: &str) -> Result<Vec<Value>, Refusal> {
    match given.get("dialogs") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(list))
            if list.len() <= MOST_DIALOGS && list.iter().all(|a| a.is_object() || a.is_null()) =>
        {
            Ok(list.clone())
        }
        Some(_) => Err(invalid(format!(
            "{at}.dialogs: en çok {MOST_DIALOGS} cevap; her biri alanların değerleri (sözlük) ya da null (İptal)."
        ))),
    }
}

/// The shape a map tool is given for `given` (GeoJSON; a circle's `center` and `radius`), as the drawing gives the
/// shape the user drew; why not, when it is not the shape the tool asks for (`entry.capture`).
fn shape_of(entry: &MapToolEntry, given: &Value) -> Result<Value, String> {
    use kentos_interaction::Vec2;
    use kentos_interaction::capture::Shape;
    let point = |v: &Value| -> Option<Vec2> {
        let xy = v.as_array()?;
        Some(Vec2::new(xy.first()?.as_f64()?, xy.get(1)?.as_f64()?))
    };
    let points = |v: &Value| -> Option<Vec<Vec2>> { v.as_array()?.iter().map(point).collect() };
    let typed = |kind: &str| given.get("type").and_then(Value::as_str) == Some(kind);
    let coordinates = given.get("coordinates").unwrap_or(&Value::Null);
    let shape = match entry.capture {
        Capture::Point => typed("Point")
            .then(|| point(coordinates).map(Shape::Point))
            .flatten()
            .ok_or("nokta isteyen araç: GeoJSON Point ({\"type\": \"Point\", \"coordinates\": [x, y]}).")?,
        Capture::Line => typed("LineString")
            .then(|| points(coordinates).filter(|p| p.len() >= 2).map(Shape::Line))
            .flatten()
            .ok_or("çizgi isteyen araç: en az iki noktalı bir GeoJSON LineString.")?,
        Capture::Polygon | Capture::Rectangle | Capture::OrientedRectangle => {
            let rectangle = entry.capture != Capture::Polygon;
            let mut ring = typed("Polygon")
                .then(|| coordinates.get(0).and_then(points))
                .flatten()
                .unwrap_or_default();
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            let fits = if rectangle { ring.len() == 4 } else { ring.len() >= 3 };
            if !fits {
                return Err(if rectangle {
                    "dikdörtgen isteyen araç: dört köşeli bir GeoJSON Polygon (şablonu varsa onun ölçüsünde)."
                } else {
                    "alan isteyen araç: en az üç köşeli bir GeoJSON Polygon."
                }
                .to_owned());
            }
            Shape::Polygon(ring)
        }
        Capture::Circle => {
            let center = given.get("center").and_then(point);
            let radius = given
                .get("radius")
                .and_then(Value::as_f64)
                .filter(|r| *r > 0.0);
            match (center, radius) {
                (Some(center), Some(radius)) => Shape::Circle { center, radius },
                _ => {
                    return Err(
                        "daire isteyen araç: {\"center\": [x, y], \"radius\": r} (r sıfırdan büyük)."
                            .to_owned(),
                    );
                }
            }
        }
    };
    Ok(super::super::runner::shape_json(&shape))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(capture: Capture) -> MapToolEntry {
        MapToolEntry {
            id: "yerlestir".into(),
            title: "Yerleştir".into(),
            icon: None,
            capture,
            template_width_m: None,
            template_length_m: None,
            snap_layers: Vec::new(),
            handler: "x:y".into(),
        }
    }

    /// A map tool is given the shape the user would draw, as the drawing gives it (a polygon's ring closed, a circle as
    /// its outline with its centre and radius); a shape that is not the one it asks for is said in its words.
    #[test]
    fn a_map_tool_is_given_the_shape_it_asks_for() {
        let point = shape_of(
            &tool(Capture::Point),
            &json!({ "type": "Point", "coordinates": [3.0, 4.0] }),
        )
        .expect("a point");
        assert_eq!(point, json!({ "type": "Point", "coordinates": [3.0, 4.0] }));
        let square = json!({ "type": "Polygon",
                             "coordinates": [[[0.0, 0.0], [10.0, 0.0], [10.0, 12.0], [0.0, 12.0], [0.0, 0.0]]] });
        let rect = shape_of(&tool(Capture::Rectangle), &square).expect("a rectangle");
        assert_eq!(
            rect["coordinates"][0].as_array().unwrap().len(),
            5,
            "closed as drawn"
        );
        let circle = shape_of(
            &tool(Capture::Circle),
            &json!({ "center": [5.0, 5.0], "radius": 2.0 }),
        )
        .expect("a circle");
        assert_eq!(circle["circle"]["radius"], 2.0);
        assert_eq!(circle["coordinates"][0].as_array().unwrap().len(), 65);
        // The wrong shape, a rectangle of three corners, a line of one point: said.
        let wrong = shape_of(&tool(Capture::Point), &square).unwrap_err();
        assert!(wrong.contains("Point"), "{wrong}");
        let three = json!({ "type": "Polygon", "coordinates": [[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]]] });
        assert!(
            shape_of(&tool(Capture::Rectangle), &three)
                .unwrap_err()
                .contains("dört")
        );
        assert!(shape_of(&tool(Capture::Polygon), &three).is_ok());
        let one = json!({ "type": "LineString", "coordinates": [[0.0, 0.0]] });
        assert!(shape_of(&tool(Capture::Line), &one).is_err());
    }

    /// The objects selected are places in the agent's list; the dialogs' answers are values or null.
    #[test]
    fn a_selection_is_of_the_agents_objects_and_an_answer_is_values_or_none() {
        let given = json!({ "selection": [0, 2], "dialogs": [{ "ilk": 3 }, null] });
        assert_eq!(places(&given, "commands[0]", 3).unwrap(), [0, 2]);
        let far = places(&given, "commands[0]", 2).unwrap_err();
        assert_eq!(far.0, "invalid_input");
        assert!(far.1.contains("0 ile 1 arası"), "{}", far.1);
        assert!(
            places(&given, "commands[0]", 0)
                .unwrap_err()
                .1
                .contains("features boş")
        );
        assert_eq!(answers(&given, "commands[0]").unwrap().len(), 2);
        let wrong = json!({ "dialogs": ["evet"] });
        assert_eq!(
            answers(&wrong, "commands[1]").unwrap_err().0,
            "invalid_input"
        );
    }

    /// A panel's button runs its command, a changed input its onChange; a step names a command or a panel's button,
    /// not both, and a panel or button that is not there is said.
    #[test]
    fn a_panels_button_names_the_command_it_runs() {
        let definition = kentos_extensions::ui_panel::read(&json!({
            "format": kentos_extensions::ui_panel::FORMAT, "schemaVersion": kentos_extensions::ui_panel::VERSION,
            "title": "Yapı ara",
            "body": [
                { "type": "textField", "id": "sorgu", "label": "Numara" },
                { "type": "checkbox", "id": "numarasiz", "label": "Numarasızlar", "onChange": "yapi-ara" },
                { "type": "button", "id": "ara", "label": "Ara", "command": "yapi-ara", "table": "sonuc" },
                { "type": "table", "id": "sonuc", "columns": ["no"] }
            ]
        }))
        .expect("a panel");
        let rights = Rights {
            ext: "tr.ornek.x".into(),
            panels: BTreeMap::from([
                ("yapi-paneli".to_owned(), Some(definition)),
                ("bozuk".to_owned(), None),
            ]),
            ..Rights::default()
        };
        let local_of = |g: &str| g.strip_prefix("ext.tr.ornek.x.").unwrap_or(g).to_owned();
        let (command, origin) = command_of(
            &json!({ "panel": "yapi-paneli", "button": "ara" }),
            "c",
            &rights,
            &local_of,
        )
        .unwrap();
        let origin = origin.expect("from the panel");
        assert_eq!(
            (command.as_str(), origin.table.as_deref()),
            ("yapi-ara", Some("sonuc"))
        );
        let (command, origin) = command_of(
            &json!({ "panel": "yapi-paneli", "button": "numarasiz" }),
            "c",
            &rights,
            &local_of,
        )
        .unwrap();
        assert_eq!(command, "yapi-ara");
        assert_eq!(
            origin.unwrap().table,
            None,
            "a changed input's rows go to the first output"
        );
        let (command, origin) = command_of(
            &json!({ "command": "ext.tr.ornek.x.katmanlari-kur" }),
            "c",
            &rights,
            &local_of,
        )
        .unwrap();
        assert_eq!(
            (command.as_str(), origin.is_none()),
            ("katmanlari-kur", true)
        );
        for (given, words) in [
            (
                json!({ "command": "a", "panel": "yapi-paneli" }),
                "ikisi birden",
            ),
            (json!({}), "gerekli"),
            (
                json!({ "panel": "yok", "button": "ara" }),
                "panelleri: bozuk, yapi-paneli",
            ),
            (json!({ "panel": "bozuk", "button": "ara" }), "okunamadı"),
            (
                json!({ "panel": "yapi-paneli", "button": "sorgu" }),
                "onChange",
            ),
            (
                json!({ "panel": "yapi-paneli", "button": "sil" }),
                "panelinde yok",
            ),
        ] {
            let said = command_of(&given, "c", &rights, &local_of).unwrap_err().1;
            assert!(said.contains(words), "{given}: {said}");
        }
    }
}
