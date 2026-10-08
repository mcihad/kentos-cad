//! Uygulama ayarları on the desktop (`tools.options`, Ctrl+,; docs/adr/0023):
//! the drafting aids the tool session uses, the drawing area's graphics and
//! the theme, from the typed schema, with the settings file's actions. Like
//! the web's window it works on a draft: nothing changes until Kaydet; Esc,
//! × and Vazgeç leave everything as it was (DESIGN.md §7.9, §7.10).

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use iced::widget::{button, row, text};
use iced::{Center, Element, Task};
use serde_json::Value;

use kentos_contracts::{ResolvedSetting, SettingErrorCode, SettingHost, SettingScope, same_value};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::number::Unit;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Segmented};

use crate::app::{App, Message};
use crate::settings::schema;
use crate::settings_sections::Section;

/// The settings the window shows, in its order.
pub const KEYS: [&str; 52] = [
    "drafting.ortho",
    "drafting.polar",
    "drafting.polarIncrement",
    "drafting.snapAperture",
    "drafting.pickAperture",
    "drafting.cursorInput",
    "drafting.commandBar",
    "drafting.hoverInfo",
    "drafting.snap",
    "snap.endpoint",
    "snap.midpoint",
    "snap.center",
    "snap.node",
    "snap.intersection",
    "snap.perpendicular",
    "snap.tangent",
    "snap.nearest",
    "snap.centroid",
    "snap.extension",
    "snap.parallel",
    "snap.grid",
    "snap.gridEast",
    "snap.gridNorth",
    "snap.self",
    "snap.scaleMin",
    "snap.scaleMax",
    "graphics.msaa",
    "graphics.hiDpi",
    "graphics.symbolSize",
    "graphics.lineWeights",
    "graphics.colorMode",
    "graphics.annotationSize",
    "graphics.fills",
    "graphics.areaEdges",
    "graphics.transparency",
    "graphics.highlightColor",
    "graphics.highlightWidth",
    "appearance.theme",
    "appearance.accent",
    "appearance.drawingBackground",
    "appearance.crosshair",
    "appearance.uiFont",
    "appearance.monoFont",
    "appearance.textSize",
    "appearance.corners",
    "appearance.shadows",
    "appearance.startScreen",
    "display.geographic",
    "newProjects.srid",
    "newProjects.workspace",
    "newProjects.drawingUnit",
    "newProjects.drawingFont",
];

/// The snap kinds, in the web's order (docs/adr/0029, 0163).
pub(crate) const SNAP_KINDS: [&str; 12] = [
    "snap.endpoint",
    "snap.midpoint",
    "snap.center",
    "snap.node",
    "snap.intersection",
    "snap.perpendicular",
    "snap.tangent",
    "snap.nearest",
    "snap.centroid",
    "snap.extension",
    "snap.parallel",
    "snap.grid",
];

pub(crate) const PX: &[Unit] = &[Unit::new("px", 1.0)];
pub(crate) const METRES: &[Unit] = &[Unit::new("m", 1.0)];
/// A screen scale's denominator, the N of 1:N.
pub(crate) const SCALE: &[Unit] = &[Unit::new("1:N", 1.0)];

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
    /// The section shown (settings_sections.rs).
    pub section: Section,
}

impl SettingsDraft {
    pub(crate) fn changed(&self) -> bool {
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
    /// Another section; its own values back to their defaults (the web's
    /// “Bu bölümü varsayılana döndür”).
    Section(Section),
    ResetSection,
    /// Proje ayarları over this window, which comes back when it closes.
    OpenProject,
}

impl App {
    /// A window that waited under the one that just closed comes back, as
    /// it was (the web's stacked dialogs); one whose state is gone does not.
    pub(crate) fn dialog_back(&mut self) {
        if self.dialog.is_some() {
            return;
        }
        let Some(under) = self.dialog_under.take() else {
            return;
        };
        let alive = match under {
            crate::app::Dialog::Settings => self.settings_draft.is_some(),
            crate::app::Dialog::Project => self.project.is_some(),
            _ => false,
        };
        if alive {
            self.dialog = Some(under);
        }
    }

    /// Opens the window on the values asked for now.
    pub(crate) fn open_settings(&mut self) {
        self.open_settings_at(Section::default());
    }

    /// Opens the window on a section (Proje ayarları's “Uygulama ayarlarını aç”: Yeni projeler).
    pub(crate) fn open_settings_at(&mut self, section: Section) {
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
            section,
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
            Edit::Section(section) => draft.section = section,
            Edit::ResetSection => {
                for key in draft.section.keys() {
                    if let Some(d) = schema().get(key) {
                        draft.values.insert(key, d.default.clone());
                    }
                }
            }
            Edit::OpenProject => {
                // This window waits under Proje ayarları, its draft as it is (`dialog_back`).
                let task = self.project_command("file.settings");
                if self.dialog == Some(crate::app::Dialog::Project) {
                    self.dialog_under = Some(crate::app::Dialog::Settings);
                }
                return task;
            }
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
                // The web's own values (its classic interface, its drawing engine) have
                // nothing to go to here: said by name, never dropped in silence
                // (docs/inventory/parity-audit.md A1). The look is shared (docs/adr/0126).
                let web_only: Vec<String> = file
                    .user
                    .keys()
                    .chain(file.device.keys())
                    .filter_map(|key| schema().get(key))
                    .filter(|d| !d.hosts.contains(&SettingHost::Desktop))
                    .map(|d| d.title.clone())
                    .collect();
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
                let web = if web_only.is_empty() {
                    String::new()
                } else {
                    format!(
                        " Web uygulamasına özgü {} değerin masaüstünde karşılığı yok, alınmadı: {}.",
                        web_only.len(),
                        web_only.join(", ")
                    )
                };
                draft.note = Some(if left.is_empty() {
                    (
                        false,
                        format!("“{name}” okundu. Değerleri pencerede; Kaydet ile uygulanır.{web}"),
                    )
                } else {
                    (
                        true,
                        format!(
                            "“{name}” okundu; geçersiz {} değer alınmadı: {}.{web}",
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
        let mut changes: Vec<(&str, Value)> = draft
            .values
            .iter()
            .filter(|(key, value)| !same_value(&self.settings.requested(key), value))
            .map(|(key, value)| (*key, value.clone()))
            .collect();
        // A colour of one's own is taken only when it reads (as its id or
        // #rrggbb, the shared keys, docs/adr/0126); the one in use stays otherwise.
        let before = changes.len();
        changes = changes
            .into_iter()
            .filter_map(|(key, value)| {
                if key != "appearance.accent" {
                    return Some((key, value));
                }
                let accent = value.as_str().and_then(kentos_ui::theme::Accent::parse)?;
                Some((key, Value::from(accent.key())))
            })
            .collect();
        if changes.len() != before {
            self.warn("Vurgu rengi okunamadı; #RRGGBB biçiminde yazın (ör. #2f80ed). Önceki renk duruyor.");
        }
        let refused = self.settings.choose(&changes);
        self.apply_settings();
        if let Some(error) = &self.settings.write_error {
            self.warn(format!("Ayarlar dosyaya yazılamadı ({error}); bu oturumda geçerli. Klasörün yazma iznini denetleyin."));
        }
        // New projects' defaults leave the open project as it is: said (the web's, CLAUDE.md §4.4).
        let taken = changes
            .iter()
            .filter(|(key, _)| !refused.iter().any(|(r, _)| r == key));
        let defaults: Vec<String> = taken
            .filter_map(|(key, value)| match *key {
                "newProjects.srid" => value
                    .as_u64()
                    .and_then(|srid| u32::try_from(srid).ok())
                    .and_then(crate::crs::system)
                    .map(|c| {
                        format!(
                            "Yeni projeler {} ile oluşturulacak. Açık projenin sistemi değişmedi.",
                            crate::crs::title(c)
                        )
                    }),
                "newProjects.workspace" => serde_json::from_value::<kentos_contracts::Workspace>(
                    value.clone(),
                )
                .ok()
                .map(|w| {
                    format!(
                        "Yeni projeler “{}” türüyle önerilecek. Açık projenin türü değişmedi.",
                        crate::catalog::mode_of(Some(w))
                    )
                }),
                "newProjects.drawingUnit" => serde_json::from_value::<kentos_contracts::DrawingUnit>(
                    value.clone(),
                )
                .ok()
                .map(|u| {
                    let name = match u {
                        kentos_contracts::DrawingUnit::Mm => "milimetre",
                        kentos_contracts::DrawingUnit::Cm => "santimetre",
                        kentos_contracts::DrawingUnit::M => "metre",
                    };
                    format!(
                        "Yeni yerel CAD projeleri {name} biriminde başlayacak. Açık projenin birimi değişmedi."
                    )
                }),
                _ => None,
            })
            .collect();
        for line in defaults {
            self.output(line);
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

    /// The value in use against the one asked for, with the device's reason when they differ (SET-03).
    pub(crate) fn effective_note(&self, key: &str, draft: &Value) -> Element<'_, Message> {
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
    pub(crate) fn where_kept(&self) -> String {
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

pub(crate) fn range(key: &str) -> std::ops::RangeInclusive<f64> {
    let d = schema().get(key);
    d.and_then(|d| d.min).unwrap_or(0.0)..=d.and_then(|d| d.max).unwrap_or(100.0)
}

pub(crate) fn action(label_text: &'static str, edit: Edit) -> Element<'static, Message> {
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
pub(crate) fn listed(key: &'static str, current: &Value) -> Element<'static, Message> {
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
pub(crate) fn crs_choice(srid: u32) -> Element<'static, Message> {
    let systems = crate::crs::systems();
    let labels = systems
        .iter()
        .map(|s| Choice::new(crate::crs::title(s)).detail(crate::crs::datum_label(&s.datum)));
    let selected = systems.iter().position(|s| s.srid == srid);
    Select::new(labels, selected, move |i| {
        Message::Settings(Edit::Value(
            "newProjects.srid",
            Value::from(systems.get(i).map_or(5256, |s| s.srid)),
        ))
    })
    .into()
}

pub(crate) fn choices(key: &'static str, current: &Value) -> Element<'static, Message> {
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
pub(crate) fn presets(draft: &SettingsDraft) -> Element<'static, Message> {
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
        assert_eq!(
            app.log.last().map(|l| l.text.as_str()),
            Some("Uygulama ayarları kaydedildi.")
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

    /// Uygulama ayarları's sections for the owner, dark and light, at both
    /// sizes; `.run/shots/uygulama-ayarlari-*`.
    /// `cargo test -p kentos-desktop settings_view::tests::settings_screens -- --ignored --nocapture`
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn settings_screens() {
        use crate::settings_sections::Section;
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let names = [
            (Section::Appearance, "gorunum"),
            (Section::Snap, "kenetleme"),
            (Section::NewProjects, "yeni-projeler"),
            (Section::Engine, "cizim-motoru"),
            (Section::File, "ayar-dosyasi"),
        ];
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                for (section, name) in names {
                    let mut app = crate::files_testing::app_with_drawing();
                    let _ = app
                        .settings
                        .choose(&[("appearance.theme", Value::from(mode))]);
                    app.apply_settings();
                    let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.settle(&mut app, App::view, &mut update);
                    let _ = app.run("tools.options");
                    edit(&mut app, Edit::Section(section));
                    snapshot.settle(&mut app, App::view, &mut update);
                    let file = out.join(format!(
                        "uygulama-ayarlari-{name}-{width}x{height}{suffix}.png"
                    ));
                    snapshot
                        .render(app.view(), &app.theme())
                        .save(&file)
                        .expect("writes the picture");
                    println!("{}", file.display());
                }
            }
        }
    }

    /// The web's sections: one's own values go back to their defaults, the
    /// others' stay (parity-audit A2, A5).
    #[test]
    fn a_section_resets_only_its_own_values() {
        use crate::settings_sections::Section;
        let (mut app, _) = App::boot(None);
        let _ = app.run("tools.options");
        let draft = app.settings_draft.as_ref().expect("open");
        assert_eq!(draft.section, Section::Appearance, "the web's first");
        edit(&mut app, Edit::Value("snap.endpoint", Value::Bool(false)));
        edit(
            &mut app,
            Edit::Value("appearance.textSize", Value::from(16)),
        );
        edit(&mut app, Edit::Section(Section::Snap));
        edit(&mut app, Edit::ResetSection);
        let draft = app.settings_draft.as_ref().expect("open");
        assert_eq!(draft.values.get("snap.endpoint"), Some(&Value::Bool(true)));
        assert_eq!(
            draft.values.get("appearance.textSize"),
            Some(&Value::from(16)),
            "another section's value stays"
        );
        // Every value the window shows is in exactly one section.
        let mut all: Vec<&str> = Section::ALL
            .iter()
            .flat_map(|s| s.keys())
            .copied()
            .collect();
        all.sort_unstable();
        let mut keys = KEYS.to_vec();
        keys.sort_unstable();
        assert_eq!(all, keys);
    }

    /// Proje ayarları and Uygulama ayarları open over each other, the one
    /// under comes back as it was (the web's stacked dialogs; A3, P2).
    #[test]
    fn the_settings_windows_open_over_each_other_and_come_back() {
        use crate::app::Dialog;
        use crate::project::Event as ProjectEvent;
        use crate::settings_sections::Section;
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.run("tools.options");
        edit(
            &mut app,
            Edit::Value("drafting.snapAperture", Value::from(19)),
        );
        edit(&mut app, Edit::Section(Section::NewProjects));
        edit(&mut app, Edit::OpenProject);
        assert_eq!(app.dialog, Some(Dialog::Project));
        let _ = app.update(Message::Project(Box::new(ProjectEvent::Close)));
        assert_eq!(app.dialog, Some(Dialog::Settings), "back as it was");
        let draft = app.settings_draft.as_ref().expect("the draft kept");
        assert_eq!(
            draft.values.get("drafting.snapAperture"),
            Some(&Value::from(19))
        );
        let _ = app.update(Message::DialogClosed);
        assert_eq!(app.dialog, None);

        // The other way: Uygulama ayarları on Yeni projeler over Proje ayarları.
        let _ = app.run("file.settings");
        let _ = app.update(Message::Project(Box::new(ProjectEvent::Settings(
            crate::project::settings::Event::OpenApp,
        ))));
        assert_eq!(app.dialog, Some(Dialog::Settings));
        assert_eq!(
            app.settings_draft.as_ref().map(|d| d.section),
            Some(Section::NewProjects)
        );
        let _ = app.update(Message::DialogClosed);
        assert_eq!(app.dialog, Some(Dialog::Project), "Proje ayarları again");
    }

    /// New projects' defaults: saving says the open project stays as it is
    /// (the web's AppSettingsDialog, parity-audit A4).
    #[test]
    fn saving_new_projects_defaults_says_the_open_project_stays() {
        let (mut app, _) = App::boot(None);
        let _ = app.run("tools.options");
        edit(&mut app, Edit::Value("newProjects.srid", Value::from(5254)));
        edit(
            &mut app,
            Edit::Value("newProjects.workspace", Value::from("cad")),
        );
        edit(&mut app, Edit::Save);
        let said: Vec<&str> = app.log.lines().map(|l| l.text.as_str()).collect();
        assert!(
            said.iter().any(|t| t.starts_with("Yeni projeler ")
                && t.ends_with("(EPSG:5254) ile oluşturulacak. Açık projenin sistemi değişmedi.")),
            "{said:?}"
        );
        assert!(
            said.contains(&"Yeni projeler “CAD” türüyle önerilecek. Açık projenin türü değişmedi."),
            "{said:?}"
        );
        assert_eq!(said.last(), Some(&"Uygulama ayarları kaydedildi."));
    }

    /// A web settings file: its values the desktop has are taken, the look
    /// too (docs/adr/0126; an older one's five text sizes as pixels); its own
    /// (the drawing engine) are said by name, not dropped in silence
    /// (docs/inventory/parity-audit.md A1); a retired one (the classic
    /// shell's, docs/adr/0155) is dropped without a word.
    #[test]
    fn a_web_settings_file_says_what_has_no_place_here() {
        let (mut app, _) = App::boot(None);
        let _ = app.run("tools.options");
        let body = r#"{"format":"kentos.settings","version":1,"user":{"drafting.snapAperture":16,"appearance.accent":"teal","appearance.uiFont":"inter","appearance.uiScale":"large","appearance.shell":"ribbon"},"device":{"graphics.backend":"webgpu"}}"#;
        edit(
            &mut app,
            Edit::Imported(Some(Ok((
                "kentos-ayarlar.json".to_owned(),
                body.to_owned(),
            )))),
        );
        let draft = app.settings_draft.clone().expect("open");
        assert_eq!(
            draft.values.get("drafting.snapAperture"),
            Some(&Value::from(16))
        );
        for (key, value) in [
            ("appearance.accent", Value::from("teal")),
            ("appearance.uiFont", Value::from("inter")),
            ("appearance.textSize", Value::from(14)),
        ] {
            assert_eq!(draft.values.get(key), Some(&value), "{key}");
        }
        let (warn, note) = draft.note.expect("said");
        assert!(!warn, "nothing invalid");
        assert_eq!(
            note,
            "“kentos-ayarlar.json” okundu. Değerleri pencerede; Kaydet ile uygulanır. Web uygulamasına özgü 1 değerin masaüstünde karşılığı yok, alınmadı: Çizim arka ucu."
        );
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
        assert_eq!(
            app.log.last().map(|l| l.text.as_str()),
            Some("Kutupsal izleme açık")
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
            .log
            .lines()
            .filter(|l| l.level == kentos_interaction::Level::Warn && l.text.contains("kurulamadı"))
            .count();
        assert_eq!(warnings, 1);
    }
}
