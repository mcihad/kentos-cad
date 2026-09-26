//! The desktop's typed settings (docs/adr/0023; TODOS.md SET-02..05): the
//! user and device layers in a file of its own, the session layer in memory,
//! the organisation's policy and the device's constraints as inputs, all
//! resolved by the shared rules (`kentos_contracts::settings`) the web runs
//! too.
//!
//! The file is `$XDG_CONFIG_HOME/kentos-cad/ayarlar.json` (else
//! `~/.config/kentos-cad/ayarlar.json`): the `kentos.settings` v1 document
//! the web exports, so a file moves between them. It is not the KentOS UI
//! showcase's `ayarlar` beside it: that text file is the showcase's own and
//! is never written here. When there is no `ayarlar.json` yet, the one
//! setting both share, the theme (`tema`), is taken from it once.
//!
//! A file that cannot be read, or holds values the rules refuse, is copied
//! whole to `ayarlar-yedek-<time>.json` before it is written again: no value
//! the user had is lost. Writes go to a temporary file first and replace the
//! old one in one step, so a crash never leaves half a file.
//!
//! `App::boot` works in memory: tests, snapshots and the trace player never
//! touch the user's files; `main` opens the real one.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use kentos_contracts::{
    Resolution, ResolvedSetting, SettingConstraint, SettingDiagnostic, SettingErrorCode,
    SettingScope, SettingsFile, SettingsLayer, SettingsLayers, SettingsMigration, SettingsPolicy,
    SettingsSchema, check_write, resolve, settings_schema,
};
use serde_json::Value;

/// The settings schema of this version, built once.
pub fn schema() -> &'static SettingsSchema {
    static SCHEMA: OnceLock<SettingsSchema> = OnceLock::new();
    SCHEMA.get_or_init(settings_schema)
}

/// The desktop's settings file in the configuration folder.
pub const FILE_NAME: &str = "ayarlar.json";
/// The KentOS UI showcase's own settings file in the same folder (read once, never written).
pub const SHOWCASE_FILE: &str = "ayarlar";

/// What opening the settings found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OpenReport {
    /// The migration made now, when the file did not exist yet.
    pub migrated: Option<SettingsMigration>,
    /// The file could not be used as it was: why (a code for the whole, else
    /// the values dropped), and the copy kept of it.
    pub recovered: Option<Recovered>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recovered {
    pub code: Option<SettingErrorCode>,
    pub diagnostics: Vec<SettingDiagnostic>,
    pub backup: PathBuf,
}

pub struct Settings {
    schema: &'static SettingsSchema,
    file: SettingsFile,
    session: SettingsLayer,
    policy: SettingsPolicy,
    constraints: BTreeMap<String, SettingConstraint>,
    resolution: Resolution,
    /// Where the file is written; none: memory only.
    path: Option<PathBuf>,
    pub report: OpenReport,
    /// The last write that failed, said in the settings window.
    pub write_error: Option<String>,
}

impl Settings {
    /// Settings in memory only (tests, snapshots, the trace player).
    pub fn memory() -> Self {
        Self::with(SettingsFile::default(), None, OpenReport::default())
    }

    fn with(file: SettingsFile, path: Option<PathBuf>, report: OpenReport) -> Self {
        let mut settings = Self {
            schema: schema(),
            file,
            session: SettingsLayer::new(),
            policy: SettingsPolicy::default(),
            constraints: BTreeMap::new(),
            resolution: Resolution::default(),
            path,
            report,
            write_error: None,
        };
        settings.resolve();
        settings
    }

    /// The configuration folder: `$XDG_CONFIG_HOME/kentos-cad`, else
    /// `~/.config/kentos-cad`; none when neither is known.
    pub fn config_dir() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(base.join("kentos-cad"))
    }

    /// Opens the settings kept in `folder` (see the module comment); `now`
    /// dates a migration and names a backup.
    pub fn open(folder: &Path, now: SystemTime) -> Self {
        let path = folder.join(FILE_NAME);
        let schema = schema();
        let mut report = OpenReport::default();
        let mut file = SettingsFile::default();
        let mut write = false;
        let text = match fs::read(&path) {
            Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            // Unreadable (permissions): work in memory rather than write over it.
            Err(_) => return Self::with(file, None, report),
        };
        let mut start_over = text.is_none();
        if let Some(text) = &text {
            let (code, diagnostics) = match SettingsFile::from_json(text, schema) {
                Ok((read, diagnostics)) => {
                    file = read;
                    let lost: Vec<SettingDiagnostic> = diagnostics
                        .into_iter()
                        .filter(|d| d.code != SettingErrorCode::UnknownKey)
                        .collect();
                    (None, lost)
                }
                Err(code) => (Some(code), Vec::new()),
            };
            if code.is_some() || !diagnostics.is_empty() {
                let backup = folder.join(format!("ayarlar-yedek-{}.json", file_stamp(now)));
                if fs::write(&backup, text).is_err() {
                    // Kept nowhere else: work in memory rather than write over the only copy.
                    return Self::with(file, None, report);
                }
                start_over = code.is_some();
                write = true;
                report.recovered = Some(Recovered {
                    code,
                    diagnostics,
                    backup,
                });
            }
        }
        if start_over {
            let (layer, record) = migrate_showcase(&folder.join(SHOWCASE_FILE), now);
            if let Some(record) = record {
                file = SettingsFile {
                    user: layer,
                    migrations: vec![record.clone()],
                    ..SettingsFile::default()
                };
                report.migrated = Some(record);
            } else {
                file = SettingsFile::default();
            }
            write = true;
        }
        let mut settings = Self::with(file, Some(path), report);
        if write {
            settings.save();
        }
        settings
    }

    // ── Reading ──────────────────────────────────────────────────────────

    pub fn resolved(&self, key: &str) -> Option<&ResolvedSetting> {
        self.resolution.get(key)
    }

    /// The value in use (a key the schema lacks is a programming error: the schema's default of nothing).
    pub fn effective(&self, key: &str) -> Value {
        self.resolved(key)
            .map_or(Value::Null, |r| r.effective.clone())
    }

    /// The value asked for.
    pub fn requested(&self, key: &str) -> Value {
        self.resolved(key)
            .map_or(Value::Null, |r| r.requested.clone())
    }

    pub fn bool(&self, key: &str) -> bool {
        self.effective(key).as_bool().unwrap_or(false)
    }

    pub fn number(&self, key: &str) -> f64 {
        self.effective(key).as_f64().unwrap_or(0.0)
    }

    /// A text setting's value in use (`cloud.server`).
    pub fn text(&self, key: &str) -> String {
        self.effective(key).as_str().unwrap_or_default().to_owned()
    }

    /// What `key` would resolve to if `value` were chosen (the settings window shows it before Kaydet).
    pub fn preview(&self, key: &str, value: &Value) -> Option<ResolvedSetting> {
        let mut layers = self.layers();
        match self.home(key)? {
            SettingScope::Session => {
                layers.session.insert(key.into(), value.clone());
            }
            home => {
                layers.session.remove(key);
                let layer = if home == SettingScope::Device {
                    &mut layers.device
                } else {
                    &mut layers.user
                };
                layer.insert(key.into(), value.clone());
            }
        }
        resolve(self.schema, &layers, &self.policy, &self.constraints)
            .settings
            .remove(key)
    }

    /// The path of the settings file, when there is one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn migrations(&self) -> &[SettingsMigration] {
        &self.file.migrations
    }

    // ── Writing ──────────────────────────────────────────────────────────

    /// The user chose these values (the settings window's Kaydet, a
    /// command): each goes to its setting's own layer (user, device or the
    /// session) and replaces a session override of it. The refused ones are
    /// not taken and come back with their codes.
    pub fn choose(&mut self, values: &[(&str, Value)]) -> Vec<(String, SettingErrorCode)> {
        let mut refused = Vec::new();
        let mut stored = false;
        for (key, value) in values {
            let Some(home) = self.home(key) else {
                let code = if self.schema.get(key).is_some() {
                    SettingErrorCode::WrongScope
                } else {
                    SettingErrorCode::UnknownKey
                };
                refused.push(((*key).to_owned(), code));
                continue;
            };
            match check_write(self.schema, home, key, value) {
                Ok(value) => {
                    if home != SettingScope::Session {
                        self.session.remove(*key);
                        stored = true;
                    }
                    self.layer_mut(home).insert((*key).to_owned(), value);
                }
                Err(code) => refused.push(((*key).to_owned(), code)),
            }
        }
        self.resolve();
        if stored {
            self.save();
        }
        refused
    }

    /// Forgets every stored value: the settings go back to their defaults.
    pub fn reset(&mut self) {
        self.file.user.clear();
        self.file.device.clear();
        self.resolve();
        self.save();
    }

    /// The organisation's policy: the explicit input a server will fill (docs/adr/0023).
    pub fn set_policy(&mut self, policy: SettingsPolicy) {
        self.policy = policy;
        self.resolve();
    }

    /// A policy file named by `KENTOS_SETTINGS_POLICY`: a stand-in for the
    /// organisation's server, for trying a lock or a bound locally. None when
    /// the variable is not set; an error names the file and why.
    pub fn policy_from_env() -> Option<Result<SettingsPolicy, String>> {
        let path = std::env::var_os("KENTOS_SETTINGS_POLICY").filter(|v| !v.is_empty())?;
        let path = PathBuf::from(path);
        Some(
            fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
                .map_err(|e| format!("{}: {e}", path.display())),
        )
    }

    /// What the device can use for one setting; none removes it. True when it changed.
    pub fn set_constraint(&mut self, key: &str, constraint: Option<SettingConstraint>) -> bool {
        if self.constraints.get(key) == constraint.as_ref() {
            return false;
        }
        match constraint {
            Some(c) => self.constraints.insert(key.into(), c),
            None => self.constraints.remove(key),
        };
        self.resolve();
        true
    }

    /// The stored settings as a `kentos.settings` document (the web reads it too).
    pub fn export_text(&self) -> String {
        self.file.to_json(self.schema)
    }

    /// Takes a document's user and device values in place of the stored
    /// ones. A document that is refused changes nothing; values the rules
    /// refuse are left out and named.
    pub fn import_text(&mut self, text: &str) -> Result<Vec<SettingDiagnostic>, SettingErrorCode> {
        let (read, diagnostics) = SettingsFile::from_json(text, self.schema)?;
        self.file.user = read.user;
        self.file.device = read.device;
        self.resolve();
        self.save();
        Ok(diagnostics)
    }

    // ── Inside ───────────────────────────────────────────────────────────

    fn home(&self, key: &str) -> Option<SettingScope> {
        self.schema.get(key).map(|d| d.scope).filter(|s| {
            matches!(
                s,
                SettingScope::User | SettingScope::Device | SettingScope::Session
            )
        })
    }

    fn layer_mut(&mut self, scope: SettingScope) -> &mut SettingsLayer {
        match scope {
            SettingScope::Device => &mut self.file.device,
            SettingScope::Session => &mut self.session,
            _ => &mut self.file.user,
        }
    }

    fn layers(&self) -> SettingsLayers {
        SettingsLayers {
            user: self.file.user.clone(),
            device: self.file.device.clone(),
            session: self.session.clone(),
            project: SettingsLayer::new(),
        }
    }

    fn resolve(&mut self) {
        self.resolution = resolve(self.schema, &self.layers(), &self.policy, &self.constraints);
    }

    /// Writes the file: a temporary file first, then one rename over the old.
    fn save(&mut self) {
        let Some(path) = &self.path else {
            return;
        };
        let text = self.file.to_json(self.schema);
        let written = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| {
                let temporary = path.with_extension("json.yeni");
                fs::write(&temporary, text)?;
                fs::rename(&temporary, path)
            });
        self.write_error = written.err().map(|e| format!("{}: {e}", path.display()));
    }
}

/// The theme of the KentOS UI showcase's `ayarlar` file (`tema = koyu`), as
/// the desktop's `appearance.theme`, and the record of it. None when there is
/// no such file. Only the theme is shared; the showcase's other lines (accent,
/// typefaces, dock layout) stay its own.
fn migrate_showcase(path: &Path, now: SystemTime) -> (SettingsLayer, Option<SettingsMigration>) {
    let Ok(text) = fs::read_to_string(path) else {
        return (SettingsLayer::new(), None);
    };
    let schema = schema();
    let mut layer = SettingsLayer::new();
    let mut dropped = Vec::new();
    let mut moved = 0;
    for line in text.lines().map(str::trim) {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if line.starts_with('#') || key.trim() != "tema" {
            continue;
        }
        let theme = match value.trim() {
            "koyu" => "dark",
            "aydinlik" => "light",
            // Night and high contrast are the showcase's; the desktop offers dark and light.
            other => other,
        };
        match check_write(
            schema,
            SettingScope::User,
            "appearance.theme",
            &Value::from(theme),
        ) {
            Ok(value) => {
                layer.insert("appearance.theme".into(), value);
                moved += 1;
            }
            Err(code) => dropped.push(SettingDiagnostic {
                layer: SettingScope::User,
                key: "tema".into(),
                code,
            }),
        }
    }
    let record = SettingsMigration {
        from: path.display().to_string(),
        at: rfc3339(now),
        moved,
        dropped,
    };
    (layer, Some(record))
}

/// Seconds since 1970 as `2026-09-25T20:00:00Z` (UTC; no date crate needed).
pub fn rfc3339(time: SystemTime) -> String {
    let (date, [h, m, s]) = civil(time);
    format!("{date}T{h:02}:{m:02}:{s:02}Z")
}

/// A time for a file name: `2026-09-25T20-00-00Z`.
fn file_stamp(time: SystemTime) -> String {
    let (date, [h, m, s]) = civil(time);
    format!("{date}T{h:02}-{m:02}-{s:02}Z")
}

/// The UTC calendar date and time of day (Howard Hinnant's days-to-civil).
fn civil(time: SystemTime) -> (String, [u64; 3]) {
    let secs = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        format!("{year:04}-{month:02}-{day:02}"),
        [rest / 3600, rest % 3600 / 60, rest % 60],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 25 September 2026, 20:00:00 UTC.
    fn at() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_790_366_400)
    }

    /// A folder of its own under the system's temporary folder: never the user's configuration.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kentos-desktop-settings-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a temporary folder");
        dir
    }

    /// The showcase's `ayarlar` as it writes itself (apps/ui-showcase/src/settings.rs).
    const SHOWCASE: &str = "# KentOS CAD ayarları\ntema = aydinlik\nvurgu = turuncu\nharita-zemini = siyah\n\
        yazi-ailesi = inter\nes-aralikli = ibm-plex-mono\nyazi-boyutu = 14\n\
        yuva = sol 260: ; sag 360: katmanlar* @0.45 / ozellikler* @0.55; alt 252: tablo* gorevler @1.00\n";

    #[test]
    fn times_are_written_as_utc_rfc3339() {
        assert_eq!(rfc3339(at()), "2026-09-25T20:00:00Z");
        assert_eq!(rfc3339(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(
            rfc3339(UNIX_EPOCH + Duration::from_secs(951_782_400)),
            "2000-02-29T00:00:00Z"
        );
        assert_eq!(file_stamp(at()), "2026-09-25T20-00-00Z");
    }

    #[test]
    fn the_first_start_takes_the_showcase_theme_once_and_leaves_its_file_alone() {
        let dir = folder("showcase");
        fs::write(dir.join(SHOWCASE_FILE), SHOWCASE).expect("the showcase file");
        let settings = Settings::open(&dir, at());
        assert_eq!(settings.requested("appearance.theme"), "light");
        let record = settings.report.migrated.clone().expect("a migration");
        assert_eq!((record.moved, record.dropped.len()), (1, 0));
        assert_eq!(record.at, "2026-09-25T20:00:00Z");
        assert_eq!(
            fs::read_to_string(dir.join(SHOWCASE_FILE)).expect("still there"),
            SHOWCASE,
            "the showcase's own file is never written"
        );
        // The file exists now: a changed showcase file is not read again.
        fs::write(
            dir.join(SHOWCASE_FILE),
            SHOWCASE.replace("aydinlik", "koyu"),
        )
        .expect("rewritten");
        let again = Settings::open(&dir, at());
        assert_eq!(again.report.migrated, None);
        assert_eq!(again.requested("appearance.theme"), "light");
        assert_eq!(again.migrations().len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_showcase_theme_the_desktop_lacks_is_named_and_left_there() {
        let dir = folder("night");
        fs::write(
            dir.join(SHOWCASE_FILE),
            SHOWCASE.replace("aydinlik", "gece"),
        )
        .expect("the showcase file");
        let settings = Settings::open(&dir, at());
        let record = settings.report.migrated.clone().expect("a migration");
        assert_eq!(record.moved, 0);
        assert_eq!(record.dropped[0].code, SettingErrorCode::NotAllowed);
        assert_eq!(settings.requested("appearance.theme"), "dark");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn values_persist_and_graphics_go_to_the_device_layer() {
        let dir = folder("persist");
        let mut settings = Settings::open(&dir, at());
        let refused = settings.choose(&[
            ("drafting.snapAperture", Value::from(14)),
            ("drafting.polarIncrement", Value::from(30)),
            ("drafting.cursorInput", Value::from(false)),
            ("graphics.msaa", Value::from(8)),
            ("graphics.hiDpi", Value::from(false)),
            ("drafting.ortho", Value::from(true)),
            ("drafting.snapAperture", Value::from(99)),
        ]);
        assert_eq!(
            refused,
            vec![("drafting.snapAperture".into(), SettingErrorCode::OutOfRange)]
        );
        let text = fs::read_to_string(dir.join(FILE_NAME)).expect("written");
        let doc: Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(doc["device"]["graphics.msaa"], 8);
        assert_eq!(doc["user"]["drafting.snapAperture"], 14);
        assert!(
            doc["user"].get("drafting.ortho").is_none(),
            "session values are not saved"
        );
        let reopened = Settings::open(&dir, at());
        assert_eq!(reopened.number("drafting.snapAperture"), 14.0);
        assert_eq!(reopened.number("drafting.polarIncrement"), 30.0);
        assert!(!reopened.bool("drafting.cursorInput"));
        assert!(!reopened.bool("drafting.ortho"), "ortho is this session's");
        assert!(
            !dir.join("ayarlar.json.yeni").exists(),
            "no temporary file is left"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_corrupt_file_is_kept_as_a_backup_and_the_settings_start_over() {
        let dir = folder("corrupt");
        fs::write(dir.join(FILE_NAME), "{\"format\": \"kentos.settings\",").expect("written");
        let settings = Settings::open(&dir, at());
        let recovered = settings.report.recovered.clone().expect("recovered");
        assert_eq!(recovered.code, Some(SettingErrorCode::NotJson));
        assert_eq!(
            fs::read_to_string(&recovered.backup).expect("the backup"),
            "{\"format\": \"kentos.settings\","
        );
        assert!(
            SettingsFile::from_json(
                &fs::read_to_string(dir.join(FILE_NAME)).expect("rewritten"),
                schema()
            )
            .is_ok()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_values_fall_back_and_the_rest_are_kept() {
        let dir = folder("invalid");
        let text = r#"{"format":"kentos.settings","version":1,
            "user":{"drafting.snapAperture":99,"drafting.polarIncrement":30,"zz.future":1},
            "device":{"graphics.msaa":"x","graphics.hiDpi":false}}"#;
        fs::write(dir.join(FILE_NAME), text).expect("written");
        let settings = Settings::open(&dir, at());
        let recovered = settings.report.recovered.clone().expect("recovered");
        assert_eq!(recovered.code, None);
        assert_eq!(recovered.diagnostics.len(), 2);
        assert_eq!(fs::read_to_string(&recovered.backup).expect("backup"), text);
        assert_eq!(settings.number("drafting.snapAperture"), 11.0);
        assert_eq!(settings.number("drafting.polarIncrement"), 30.0);
        assert!(!settings.bool("graphics.hiDpi"));
        let doc: Value =
            serde_json::from_str(&fs::read_to_string(dir.join(FILE_NAME)).expect("rewritten"))
                .expect("JSON");
        assert_eq!(doc["user"]["zz.future"], 1, "a newer version's key is kept");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn export_import_and_reset() {
        let mut settings = Settings::memory();
        let _ = settings.choose(&[
            ("drafting.snapAperture", Value::from(17)),
            ("graphics.msaa", Value::from(2)),
        ]);
        let text = settings.export_text();
        settings.reset();
        assert_eq!(settings.number("drafting.snapAperture"), 11.0);
        assert_eq!(settings.import_text(&text), Ok(Vec::new()));
        assert_eq!(settings.number("graphics.msaa"), 2.0);
        assert_eq!(
            settings.import_text("[]"),
            Err(SettingErrorCode::NotSettings)
        );
        assert_eq!(
            settings.number("graphics.msaa"),
            2.0,
            "a refused file changes nothing"
        );
    }

    #[test]
    fn the_device_limits_the_effective_value_and_the_preference_stays() {
        let mut settings = Settings::memory();
        let _ = settings.choose(&[("graphics.msaa", Value::from(8))]);
        assert!(settings.set_constraint(
            "graphics.msaa",
            Some(SettingConstraint {
                allowed: vec![Value::from(1), Value::from(4)],
                reason: kentos_contracts::ResolveReason::DeviceUnsupported,
                detail: "Bu aygıt en çok 4× destekliyor.".into(),
            })
        ));
        let r = settings.resolved("graphics.msaa").expect("resolved");
        assert_eq!(
            (r.requested.clone(), r.effective.clone()),
            (Value::from(8), Value::from(4))
        );
        let preview = settings
            .preview("graphics.msaa", &Value::from(1))
            .expect("previewed");
        assert_eq!(preview.effective, Value::from(1));
    }
}
