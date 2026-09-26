//! Uygulama ayarları on the desktop (`tools.options`, Ctrl+,; docs/adr/0023):
//! the drafting aids the tool session uses, the drawing area's graphics and
//! the theme, from the typed schema, with the settings file's actions. Like
//! the web's window it works on a draft: nothing changes until Kaydet; Esc,
//! × and Vazgeç leave everything as it was (DESIGN.md §7.9, §7.10).

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use iced::widget::{button, container, row, scrollable, text};
use iced::{Center, Element, Fill, Shrink, Task};
use serde_json::Value;

use kentos_contracts::{ResolvedSetting, SettingErrorCode, SettingScope, same_value};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::number::Unit;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Dialog, Form, NumberInput, Segmented, Switch, overlay};

use crate::app::{App, Message};
use crate::settings::schema;

/// The settings the window shows, in its order.
pub const KEYS: [&str; 23] = [
    "drafting.ortho",
    "drafting.polar",
    "drafting.polarIncrement",
    "drafting.snapAperture",
    "drafting.pickAperture",
    "drafting.cursorInput",
    "drafting.commandBar",
    "drafting.snap",
    "snap.endpoint",
    "snap.midpoint",
    "snap.center",
    "snap.node",
    "snap.intersection",
    "snap.perpendicular",
    "snap.tangent",
    "snap.nearest",
    "graphics.msaa",
    "graphics.hiDpi",
    "appearance.theme",
    "appearance.startScreen",
    "newProjects.srid",
    "newProjects.workspace",
    "newProjects.drawingFont",
];

/// The snap kinds, in the web's order (docs/adr/0029).
const SNAP_KINDS: [&str; 8] = [
    "snap.endpoint",
    "snap.midpoint",
    "snap.center",
    "snap.node",
    "snap.intersection",
    "snap.perpendicular",
    "snap.tangent",
    "snap.nearest",
];

const PX: &[Unit] = &[Unit::new("px", 1.0)];

/// The window's draft: the values asked for, and a pending import or reset.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsDraft {
    pub values: BTreeMap<&'static str, Value>,
    initial: BTreeMap<&'static str, Value>,
    /// A settings file read with İçe aktar: its name and text, taken on Kaydet.
    pub import: Option<(String, String)>,
    /// “Varsayılanlara döndür”: the stored values are forgotten on Kaydet.
    pub reset: bool,
    /// What the last file action said.
    pub note: Option<(bool, String)>,
}

impl SettingsDraft {
    fn changed(&self) -> bool {
        self.values != self.initial || self.import.is_some() || self.reset
    }
}

/// What the window asks the app to do.
#[derive(Debug, Clone)]
pub enum Edit {
    Value(&'static str, Value),
    Preset(&'static str),
    Save,
    Export,
    Exported(Option<Result<PathBuf, String>>),
    Import,
    Imported(Option<Result<(String, String), String>>),
    Reset,
}

impl App {
    /// Opens the window on the values asked for now.
    pub(crate) fn open_settings(&mut self) {
        let values: BTreeMap<&'static str, Value> = KEYS
            .iter()
            .map(|&k| (k, self.settings.requested(k)))
            .collect();
        self.settings_draft = Some(SettingsDraft {
            initial: values.clone(),
            values,
            import: None,
            reset: false,
            note: None,
        });
        self.dialog = Some(crate::app::Dialog::Settings);
    }

    pub(crate) fn settings_edit(&mut self, edit: Edit) -> Task<Message> {
        let Some(draft) = &mut self.settings_draft else {
            return Task::none();
        };
        match edit {
            Edit::Value(key, value) => {
                draft.values.insert(key, value);
            }
            Edit::Preset(id) => {
                if let Some(preset) = schema().preset(id) {
                    for (key, value) in &preset.values {
                        if let Some(&k) = KEYS.iter().find(|k| **k == key.as_str()) {
                            draft.values.insert(k, value.clone());
                        }
                    }
                }
            }
            Edit::Reset => {
                for key in KEYS {
                    if let Some(d) = schema().get(key) {
                        draft.values.insert(key, d.default.clone());
                    }
                }
                draft.import = None;
                draft.reset = true;
                draft.note = Some((
                    false,
                    "Bütün ayarlar varsayılanlarında; Kaydet ile saklanan değerler silinir.".into(),
                ));
            }
            Edit::Export => {
                let text = self.settings.export_text();
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Ayarları dışa aktar")
                            .add_filter("KentOS ayarları (.json)", &["json"])
                            .set_file_name("kentos-ayarlar.json")
                            .save_file()
                            .await?;
                        let path = file.path().to_path_buf();
                        Some(
                            std::fs::write(&path, text)
                                .map(|()| path.clone())
                                .map_err(|e| format!("{}: {e}", path.display())),
                        )
                    },
                    |done| Message::Settings(Edit::Exported(done)),
                );
            }
            Edit::Exported(None) | Edit::Imported(None) => {}
            Edit::Exported(Some(Ok(path))) => {
                draft.note = Some((
                    false,
                    format!("Ayarlar dışa aktarıldı: {}.", path.display()),
                ));
            }
            Edit::Exported(Some(Err(error))) | Edit::Imported(Some(Err(error))) => {
                draft.note = Some((
                    true,
                    format!("Dosya işlenemedi: {error}. Başka bir yer seçip yeniden deneyin."),
                ));
            }
            Edit::Import => {
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Ayarları içe aktar")
                            .add_filter("KentOS ayarları (.json)", &["json"])
                            .pick_file()
                            .await?;
                        let name = file.file_name();
                        Some(
                            std::fs::read(file.path())
                                .map(|bytes| (name, String::from_utf8_lossy(&bytes).into_owned()))
                                .map_err(|e| e.to_string()),
                        )
                    },
                    |read| Message::Settings(Edit::Imported(read)),
                );
            }
            Edit::Imported(Some(Ok((name, body)))) => return self.take_import(name, body),
            Edit::Save => return self.save_settings(),
        }
        Task::none()
    }

    /// A settings file's values into the draft (checked by the shared rules first).
    fn take_import(&mut self, name: String, body: String) -> Task<Message> {
        let Some(draft) = &mut self.settings_draft else {
            return Task::none();
        };
        match kentos_contracts::SettingsFile::from_json(&body, schema()) {
            Err(code) => {
                draft.note = Some((true, format!("“{name}” alınamadı. {}", code.message())));
            }
            Ok((file, diagnostics)) => {
                let layers = kentos_contracts::SettingsLayers {
                    user: file.user,
                    device: file.device,
                    ..kentos_contracts::SettingsLayers::default()
                };
                let resolved = kentos_contracts::resolve(
                    schema(),
                    &layers,
                    &kentos_contracts::SettingsPolicy::default(),
                    &BTreeMap::new(),
                );
                for key in KEYS {
                    // Session values are not in a file: the window keeps them.
                    let session = schema()
                        .get(key)
                        .is_some_and(|d| d.scope == SettingScope::Session);
                    if let (false, Some(r)) = (session, resolved.get(key)) {
                        draft.values.insert(key, r.requested.clone());
                    }
                }
                let left: Vec<String> = diagnostics
                    .iter()
                    .filter(|d| d.code != SettingErrorCode::UnknownKey)
                    .map(|d| d.key.clone())
                    .collect();
                draft.note = Some(if left.is_empty() {
                    (
                        false,
                        format!("“{name}” okundu. Değerleri pencerede; Kaydet ile uygulanır."),
                    )
                } else {
                    (
                        true,
                        format!(
                            "“{name}” okundu; geçersiz {} değer alınmadı: {}.",
                            left.len(),
                            left.join(", ")
                        ),
                    )
                });
                draft.import = Some((name, body));
                draft.reset = false;
            }
        }
        Task::none()
    }

    /// Kaydet: an import or a reset first, then every changed value; the tool session and the drawing area follow at once.
    fn save_settings(&mut self) -> Task<Message> {
        let Some(draft) = self.settings_draft.take() else {
            return Task::none();
        };
        self.dialog = None;
        if let Some((_, body)) = &draft.import {
            let _ = self.settings.import_text(body);
        } else if draft.reset {
            self.settings.reset();
        }
        let changes: Vec<(&str, Value)> = draft
            .values
            .iter()
            .filter(|(key, value)| !same_value(&self.settings.requested(key), value))
            .map(|(key, value)| (*key, value.clone()))
            .collect();
        let refused = self.settings.choose(&changes);
        self.apply_settings();
        if let Some(error) = &self.settings.write_error {
            self.warn(format!("Ayarlar dosyaya yazılamadı ({error}); bu oturumda geçerli. Klasörün yazma iznini denetleyin."));
        }
        if refused.is_empty() {
            self.say(
                kentos_interaction::Level::Success,
                "Uygulama ayarları kaydedildi.",
            );
        } else {
            let keys: Vec<&str> = refused.iter().map(|(k, _)| k.as_str()).collect();
            self.warn(format!(
                "Kaydedilemeyen ayar: {}. Değerleri denetleyip yeniden deneyin.",
                keys.join(", ")
            ));
        }
        Task::none()
    }

    /// The window.
    pub(crate) fn settings_dialog(&self) -> Element<'_, Message> {
        let Some(draft) = &self.settings_draft else {
            return text("").into();
        };
        let value = |key: &str| draft.values.get(key).cloned().unwrap_or(Value::Null);
        let help = |key: &str| schema().get(key).map_or("", |d| d.description.as_str());
        let title = |key: &str| schema().get(key).map_or("", |d| d.title.as_str());
        // A switch the organisation's policy fixes shows the value in use and cannot be turned.
        let switch = |key: &'static str, caption: Option<&'static str>| {
            let control = match self.settings.resolved(key).filter(|r| r.locked) {
                Some(r) => Switch::disabled(r.effective.as_bool().unwrap_or(false))
                    .label("Kurum politikası sabitliyor"),
                None => Switch::new(value(key).as_bool().unwrap_or(false), move |v| {
                    Message::Settings(Edit::Value(key, Value::Bool(v)))
                }),
            };
            match caption {
                Some(caption) if self.settings.resolved(key).is_some_and(|r| !r.locked) => {
                    control.label(caption)
                }
                _ => control,
            }
        };

        // Two columns: the drafting aids and the theme; the graphics and the settings file.
        let drafting = Form::new()
            .label_width(150.0)
            .section("Çizim yardımcıları")
            .field(
                title("drafting.ortho"),
                switch("drafting.ortho", Some("Bu oturum")),
            )
            .help(help("drafting.ortho"))
            .field(
                title("drafting.polar"),
                switch("drafting.polar", Some("Bu oturum")),
            )
            .help(help("drafting.polar"))
            .field(
                title("drafting.polarIncrement"),
                choices("drafting.polarIncrement", &value("drafting.polarIncrement")),
            )
            .help(help("drafting.polarIncrement"))
            .field(
                title("drafting.snapAperture"),
                NumberInput::new(
                    value("drafting.snapAperture").as_f64().unwrap_or(11.0),
                    |v| {
                        Message::Settings(Edit::Value(
                            "drafting.snapAperture",
                            Value::from(v.round() as i64),
                        ))
                    },
                )
                .units(PX)
                .range(range("drafting.snapAperture"))
                .step(1.0)
                .decimals(0)
                .width(120),
            )
            .help(help("drafting.snapAperture"))
            .field(
                title("drafting.pickAperture"),
                NumberInput::new(
                    value("drafting.pickAperture").as_f64().unwrap_or(5.0),
                    |v| {
                        Message::Settings(Edit::Value(
                            "drafting.pickAperture",
                            Value::from(v.round() as i64),
                        ))
                    },
                )
                .units(PX)
                .range(range("drafting.pickAperture"))
                .step(1.0)
                .decimals(0)
                .width(120),
            )
            .help(help("drafting.pickAperture"))
            .field(
                title("drafting.cursorInput"),
                switch("drafting.cursorInput", None),
            )
            .help(help("drafting.cursorInput"))
            .field(
                title("drafting.commandBar"),
                switch("drafting.commandBar", None),
            )
            .help(help("drafting.commandBar"))
            .section("Kenetleme")
            .field(
                title("drafting.snap"),
                switch("drafting.snap", Some("Bu oturum")),
            )
            .help(help("drafting.snap"));
        let drafting = SNAP_KINDS
            .iter()
            .fold(drafting, |form, &key| {
                form.field(title(key), switch(key, None)).help(help(key))
            })
            .section("Görünüm")
            .field(
                title("appearance.theme"),
                choices("appearance.theme", &value("appearance.theme")),
            )
            .help(help("appearance.theme"))
            .field(
                title("appearance.startScreen"),
                switch("appearance.startScreen", None),
            )
            .help(help("appearance.startScreen"))
            .section("Yeni projeler")
            .field(
                title("newProjects.srid"),
                crs_choice(value("newProjects.srid").as_u64().unwrap_or(5256) as u32),
            )
            .help(help("newProjects.srid"))
            .field(
                title("newProjects.workspace"),
                choices("newProjects.workspace", &value("newProjects.workspace")),
            )
            .help(help("newProjects.workspace"))
            .field(
                title("newProjects.drawingFont"),
                listed("newProjects.drawingFont", &value("newProjects.drawingFont")),
            )
            .help(help("newProjects.drawingFont"));
        let mut graphics = Form::new()
            .label_width(150.0)
            .section("Grafik (bu cihaz)")
            .field("Hazır ayar", presets(draft))
            .help("Hızlı, Dengeli ve Kaliteli yalnız aşağıdaki iki değeri doldurur; her biri ayrıca değiştirilebilir. Çizimin kaydını ve hassasiyetini değiştirmez.")
            .field(title("graphics.msaa"), choices("graphics.msaa", &value("graphics.msaa")))
            .help(help("graphics.msaa"))
            .row(self.effective_note("graphics.msaa", &value("graphics.msaa")))
            .field(title("graphics.hiDpi"), switch("graphics.hiDpi", None))
            .help(help("graphics.hiDpi"))
            .section("Ayar dosyası")
            .row(
                row![
                    action("Dışa aktar…", Edit::Export),
                    action("İçe aktar…", Edit::Import),
                    action("Varsayılanlara döndür", Edit::Reset),
                ]
                .spacing(6),
            );
        if let Some((warn, note)) = &draft.note {
            graphics = graphics.row(if *warn {
                Banner::warning(note.as_str())
            } else {
                Banner::info(note.as_str())
            });
        }
        let graphics = graphics.row(label::caption(self.where_kept()));
        let body = row![
            container(drafting).width(Fill),
            container(graphics).width(Fill)
        ]
        .spacing(28);

        let save = button(label::body("Kaydet"))
            .on_press_maybe(draft.changed().then_some(Message::Settings(Edit::Save)))
            .padding([5, 16])
            .style(style::button::primary);
        let cancel = button(label::body("Vazgeç"))
            .on_press(Message::DialogClosed)
            .padding([5, 16])
            .style(style::button::secondary);
        overlay::modal(
            Dialog::new("Uygulama ayarları")
                .hint("Ctrl+,")
                .push(label::muted(
                    "Çizim yardımcıları, kenet türleri ve görünüm sizin tercihinizdir; grafik ayarları bu cihaza özgüdür; Orto, Kutupsal izleme ve Kenetleme bu oturum içindir.",
                ))
                .push(scrollable(body).height(Shrink))
                .action(cancel)
                .action(save)
                .width(1080.0),
            Message::DialogClosed,
        )
    }

    /// The value in use against the one asked for, with the device's reason when they differ (SET-03).
    fn effective_note(&self, key: &str, draft: &Value) -> Element<'_, Message> {
        let Some(r) = self.settings.preview(key, draft) else {
            return text("").into();
        };
        let status = self.viewport.status();
        let supported = status
            .supported
            .iter()
            .map(|&n| samples_label(n))
            .collect::<Vec<_>>()
            .join(", ");
        let memory = status.stats.target_bytes.div_ceil(1 << 20);
        let about = if supported.is_empty() {
            String::new()
        } else {
            format!(" Desteklenenler: {supported}.")
        };
        let now = if memory > 0 {
            format!(" Çizim hedefleri şu an yaklaşık {memory} MB.")
        } else {
            " Şu an çizim doğrudan pencereye yapılıyor; ek hedef yok.".to_owned()
        };
        effective_banner(&r, about, now).into()
    }

    /// Where the settings are kept, and what opening them did.
    fn where_kept(&self) -> String {
        let mut out = match self.settings.path() {
            Some(path) => format!("Ayarlar {} dosyasında saklanır.", path.display()),
            None => "Ayarlar bu oturum için bellekte; dosyaya yazılmaz.".to_owned(),
        };
        for m in self.settings.migrations() {
            out.push_str(&format!(
                " {}: {} dosyasından {} değer alındı; o dosya olduğu gibi duruyor.",
                m.at, m.from, m.moved
            ));
        }
        if let Some(r) = &self.settings.report.recovered {
            out.push_str(&format!(
                " Açılışta dosya okunamadı ya da geçersiz değer içeriyordu; eski metin {} olarak saklandı.",
                r.backup.display()
            ));
        }
        out
    }
}

fn effective_banner<'a>(r: &ResolvedSetting, about: String, now: String) -> Banner<'a, Message> {
    let label = |v: &Value| {
        v.as_u64()
            .map_or_else(|| v.to_string(), |n| samples_label(n as u32))
    };
    if same_value(&r.effective, &r.requested) {
        Banner::info(format!("Kullanılan: {}.{about}{now}", label(&r.effective)))
    } else {
        let why = r.reason.map_or("", |reason| reason.message());
        let detail = r.detail.as_deref().unwrap_or("");
        Banner::warning(format!(
            "İstenen {}, kullanılan {}. {why} {detail}{now}",
            label(&r.requested),
            label(&r.effective)
        ))
    }
}

/// A sample count as the schema labels it (“Kapalı”, “4×”).
pub fn samples_label(n: u32) -> String {
    schema()
        .get("graphics.msaa")
        .and_then(|d| {
            d.choices
                .iter()
                .find(|c| c.value.as_u64() == Some(u64::from(n)))
        })
        .map_or_else(|| format!("{n}×"), |c| c.label.clone())
}

fn range(key: &str) -> std::ops::RangeInclusive<f64> {
    let d = schema().get(key);
    d.and_then(|d| d.min).unwrap_or(0.0)..=d.and_then(|d| d.max).unwrap_or(100.0)
}

fn action(label_text: &'static str, edit: Edit) -> Element<'static, Message> {
    button(label::body(label_text))
        .on_press(Message::Settings(edit))
        .padding([4, 12])
        .style(style::button::secondary)
        .into()
}

/// One of a setting's choices, as a segment.
#[derive(Clone, Copy)]
struct Pick {
    key: &'static str,
    index: usize,
}

impl Pick {
    fn choice(self) -> Option<&'static kentos_contracts::SettingChoice> {
        schema()
            .get(self.key)
            .and_then(|d| d.choices.get(self.index))
    }
}

impl PartialEq for Pick {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.index == other.index
    }
}

impl fmt::Display for Pick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.choice().map_or("", |c| c.label.as_str()))
    }
}

/// A setting's choices as a segmented control, the draft's value selected.
/// A setting with many choices as a list (the drawing typefaces).
fn listed(key: &'static str, current: &Value) -> Element<'static, Message> {
    let choices: Vec<kentos_contracts::SettingChoice> = schema()
        .get(key)
        .map(|d| d.choices.clone())
        .unwrap_or_default();
    let selected = choices.iter().position(|c| same_value(&c.value, current));
    let labels = choices.iter().map(|c| Choice::new(c.label.clone()));
    let values: Vec<Value> = choices.iter().map(|c| c.value.clone()).collect();
    Select::new(labels, selected, move |i| {
        Message::Settings(Edit::Value(
            key,
            values.get(i).cloned().unwrap_or(Value::Null),
        ))
    })
    .searchable(false)
    .into()
}

/// The coordinate system new projects are offered with: the registry's list.
fn crs_choice(srid: u32) -> Element<'static, Message> {
    let systems = crate::crs::systems();
    let labels = systems.iter().map(|s| {
        Choice::new(format!("{} (EPSG:{})", s.name, s.srid))
            .detail(crate::crs::datum_label(&s.datum))
    });
    let selected = systems.iter().position(|s| s.srid == srid);
    Select::new(labels, selected, move |i| {
        Message::Settings(Edit::Value(
            "newProjects.srid",
            Value::from(systems.get(i).map_or(5256, |s| s.srid)),
        ))
    })
    .into()
}

fn choices(key: &'static str, current: &Value) -> Element<'static, Message> {
    let count = schema().get(key).map_or(0, |d| d.choices.len());
    let picks = (0..count).map(move |index| Pick { key, index });
    let selected = picks
        .clone()
        .find(|p| p.choice().is_some_and(|c| same_value(&c.value, current)))
        .unwrap_or(Pick {
            key,
            index: usize::MAX,
        });
    Segmented::new(picks, selected, move |p: Pick| {
        Message::Settings(Edit::Value(
            key,
            p.choice().map_or(Value::Null, |c| c.value.clone()),
        ))
    })
    .into()
}

/// A graphics preset, as a segment.
#[derive(Clone, Copy, PartialEq)]
struct PresetPick(&'static str, &'static str);

impl fmt::Display for PresetPick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.1)
    }
}

/// The graphics presets; none selected (and “Özel” beside them) when the values match none.
fn presets(draft: &SettingsDraft) -> Element<'static, Message> {
    let picks: Vec<PresetPick> = schema()
        .presets
        .iter()
        .filter(|p| p.group == "graphics")
        .map(|p| PresetPick(p.id.as_str(), p.title.as_str()))
        .collect();
    let matching = schema().matching_preset("graphics", |key| draft.values.get(key).cloned());
    let selected = matching.map_or(PresetPick("", ""), |p| {
        PresetPick(p.id.as_str(), p.title.as_str())
    });
    let control = Segmented::new(picks, selected, |p: PresetPick| {
        Message::Settings(Edit::Preset(p.0))
    });
    if matching.is_some() {
        control.into()
    } else {
        row![control, label::caption("Özel")]
            .spacing(8)
            .align_y(Center)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Dialog;
    use kentos_render_wgpu::SampleFailure;

    fn edit(app: &mut App, edit: Edit) {
        let _ = app.update(Message::Settings(edit));
    }

    #[test]
    fn the_window_works_on_a_draft_and_kaydet_reaches_the_tool_session() {
        let (mut app, _) = App::boot(None);
        let _ = app.run("tools.options");
        assert_eq!(app.dialog, Some(Dialog::Settings));
        edit(
            &mut app,
            Edit::Value("drafting.snapAperture", Value::from(18)),
        );
        edit(&mut app, Edit::Value("drafting.polar", Value::Bool(true)));
        edit(
            &mut app,
            Edit::Value("drafting.polarIncrement", Value::from(30)),
        );
        edit(
            &mut app,
            Edit::Value("drafting.cursorInput", Value::Bool(false)),
        );
        // Nothing is used before Kaydet.
        assert_eq!(app.draft.snap_aperture, 11.0);
        assert!(app.cursor_input);
        edit(&mut app, Edit::Save);
        assert_eq!(app.dialog, None);
        assert_eq!(app.draft.snap_aperture, 18.0);
        assert_eq!(app.draft.polar, Some(30.0));
        assert!(!app.cursor_input);
        assert!(
            matches!(app.history.last(), Some(kentos_ui::widget::command_line::Entry::Output(t)) if t == "Uygulama ayarları kaydedildi.")
        );

        // Vazgeç (or Esc) leaves everything as it was.
        let _ = app.run("tools.options");
        edit(
            &mut app,
            Edit::Value("drafting.snapAperture", Value::from(25)),
        );
        let _ = app.update(Message::DialogClosed);
        assert_eq!(app.draft.snap_aperture, 18.0);
        assert_eq!(app.settings.requested("drafting.snapAperture"), 18);
    }

    #[test]
    fn a_preset_only_fills_its_values_and_each_stays_changeable() {
        let (mut app, _) = App::boot(None);
        let _ = app.run("tools.options");
        edit(&mut app, Edit::Preset("fast"));
        let draft = app.settings_draft.clone().expect("open");
        assert_eq!(draft.values["graphics.msaa"], 1);
        assert_eq!(draft.values["graphics.hiDpi"], false);
        assert_eq!(
            draft.values["drafting.snapAperture"], 11,
            "a preset touches only graphics"
        );
        edit(&mut app, Edit::Value("graphics.msaa", Value::from(8)));
        edit(&mut app, Edit::Save);
        assert_eq!(app.graphics().samples, 8);
        assert!(!app.graphics().hi_dpi);
    }

    #[test]
    fn f8_and_f10_turn_the_session_aids_over_and_reach_the_draft() {
        let (mut app, _) = App::boot(None);
        assert_eq!(app.shortcut("F8"), Some("draft.ortho"));
        assert_eq!(app.shortcut("F10"), Some("draft.polar"));
        assert_eq!(app.shortcut("Ctrl+,"), Some("tools.options"));
        let _ = app.run("draft.ortho");
        assert!(app.draft.ortho);
        let _ = app.run("draft.polar");
        assert_eq!(app.draft.polar, Some(45.0));
        assert!(
            matches!(app.history.last(), Some(kentos_ui::widget::command_line::Entry::Output(t)) if t == "Kutupsal izleme açık")
        );
        // Session values: not in the stored document.
        assert!(!app.settings.export_text().contains("drafting.ortho"));
    }

    #[test]
    fn the_theme_is_a_preference_now() {
        let (mut app, _) = App::boot(None);
        let _ = app.run("view.theme.light");
        assert_eq!(app.mode, kentos_ui::theme::Mode::Light);
        assert!(
            app.settings
                .export_text()
                .contains("\"appearance.theme\": \"light\"")
        );
    }

    #[test]
    fn the_device_keeps_the_request_and_draws_what_it_can() {
        let (mut app, _) = App::boot(None);
        let _ = app.settings.choose(&[("graphics.msaa", Value::from(8))]);
        // What the first frame reports: WebGPU's guaranteed counts.
        app.viewport.set_status_for_tests(vec![1, 4], None);
        let _ = app.update(Message::CommandHistoryToggled);
        assert_eq!(app.graphics().samples, 4);
        let r = app
            .settings
            .resolved("graphics.msaa")
            .expect("resolved")
            .clone();
        assert_eq!(
            (r.requested, r.reason),
            (
                Value::from(8),
                Some(kentos_contracts::ResolveReason::DeviceUnsupported)
            )
        );
        // 4× could not be made: back to the last working count, said once.
        app.viewport.set_status_for_tests(
            vec![1, 4],
            Some(SampleFailure {
                requested: 4,
                working: 1,
                error: "bellek yetmedi".into(),
            }),
        );
        let _ = app.update(Message::CommandHistoryToggled);
        let _ = app.update(Message::CommandHistoryToggled);
        assert_eq!(app.graphics().samples, 1);
        let warnings = app
            .history
            .iter()
            .filter(|e| matches!(e, kentos_ui::widget::command_line::Entry::Warning(t) if t.contains("kurulamadı")))
            .count();
        assert_eq!(warnings, 1);
    }
}
