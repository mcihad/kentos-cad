//! The tool window's status line and footer as the web plans them
//! (`ui/processing/dialogPlan.ts`: `statusLine`, `footerOf`), played from
//! `fixtures/processing/v1/dialog.json` (`status`): what the line beside the
//! buttons says and offers, and what the buttons say, from the run's status,
//! whether Çalıştır was pressed and the values' problems.
//!
//! Nerede çalışır is planned the same way (`targetsView`, `effectiveChoice`;
//! the file's `targets`): Otomatik and what it picks now, then each place
//! the tool names; places this program does not have are coming.
//!
//! The words are the desktop's where the web speaks of its page: a big job
//! runs on another thread here, not in a worker, and the line says how to
//! stop it; the places are named for this computer.

use kentos_domain::Slot;
use kentos_processing::{Issue, Target, WORKER_THRESHOLD};

use super::RunStatus;

/// A run in the window's own thread (the web's `running.here`).
pub const RUNNING_HERE: &str = "Çalışıyor…";
/// A run on another thread (the web's `running.worker`, in the desktop's words).
pub const RUNNING_BACKGROUND: &str = "Arka planda çalışıyor; Durdur ile durdurabilirsiniz.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Idle,
    Running,
    Ok,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineIcon {
    Success,
    Warning,
    Error,
}

/// What the line offers after a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Seçime yakınlaştır: the run chose the selection.
    Zoom,
    /// Sonuçları seç: what the run made or changed.
    Select,
    /// Geri al: the run changed the drawing.
    Undo,
}

/// The line beside the buttons.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusLine {
    pub kind: LineKind,
    pub icon: Option<LineIcon>,
    pub text: String,
    /// While running: the bar, 0–100.
    pub progress: Option<u8>,
    pub actions: Vec<Action>,
    /// What Sonuçları seç selects, when it is offered.
    pub pick: Option<Vec<Slot>>,
}

/// What the buttons say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footer {
    pub run: &'static str,
    pub run_disabled: bool,
    /// Kapat, or Durdur while running (it stops the run).
    pub close: &'static str,
    /// Varsayılanlar waits while a run is going.
    pub reset_disabled: bool,
}

/// The line for `status`; `issues`: the values' problems, the run's refusal's first.
pub fn status_line(status: &RunStatus, attempted: bool, issues: &[Issue]) -> StatusLine {
    let line = |kind, icon, text: String| StatusLine {
        kind,
        icon,
        text,
        progress: None,
        actions: Vec::new(),
        pick: None,
    };
    match status {
        RunStatus::Running {
            fraction,
            label,
            background,
        } => {
            let text = if !label.is_empty() {
                label.clone()
            } else if *background {
                RUNNING_BACKGROUND.to_owned()
            } else {
                RUNNING_HERE.to_owned()
            };
            // Math.round, as the web rounds the share (fractions are 0..1).
            let progress = (fraction.clamp(0.0, 1.0) * 100.0 + 0.5).floor() as u8;
            return StatusLine {
                progress: Some(progress),
                ..line(LineKind::Running, None, text)
            };
        }
        RunStatus::Ok {
            text,
            pick,
            selected,
            undo,
        } => {
            let mut actions = if *selected {
                vec![Action::Zoom]
            } else if !pick.is_empty() {
                vec![Action::Select]
            } else {
                Vec::new()
            };
            if *undo {
                actions.push(Action::Undo);
            }
            let pick = actions.contains(&Action::Select).then(|| pick.clone());
            return StatusLine {
                actions,
                pick,
                ..line(LineKind::Ok, Some(LineIcon::Success), text.clone())
            };
        }
        _ => {}
    }
    let own = attempted
        .then(|| issues.iter().find(|i| i.param.is_none()))
        .flatten();
    let fields = if attempted {
        issues.iter().filter(|i| i.param.is_some()).count()
    } else {
        0
    };
    let invalid = match status {
        RunStatus::Invalid(text) => Some(text),
        _ => None,
    };
    if own.is_some() || fields > 0 || invalid.is_some() {
        let text = match (own, fields, invalid) {
            (Some(issue), _, _) => issue.message.clone(),
            (None, n, _) if n > 0 => format!("Çalıştırmadan önce {n} alanı düzeltin."),
            (None, _, Some(text)) => text.clone(),
            _ => String::new(),
        };
        return line(LineKind::Warn, Some(LineIcon::Warning), text);
    }
    if let RunStatus::Error(text) = status {
        return line(LineKind::Error, Some(LineIcon::Error), text.clone());
    }
    line(LineKind::Idle, None, String::new())
}

/// The buttons for `status`.
pub fn footer_of(status: &RunStatus) -> Footer {
    let running = matches!(status, RunStatus::Running { .. });
    Footer {
        run: if running {
            "Çalışıyor…"
        } else {
            "Çalıştır"
        },
        run_disabled: running,
        close: if running { "Durdur" } else { "Kapat" },
        reset_disabled: running,
    }
}

/// Where a run goes: Otomatik or one place (the web's `TargetChoice`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Auto,
    At(Target),
}

impl Choice {
    /// The stored id (`islemler.json`, the web's): "auto" or a place's.
    pub fn id(self) -> &'static str {
        match self {
            Choice::Auto => "auto",
            Choice::At(t) => t.id(),
        }
    }

    /// A stored id; anything else is Otomatik.
    pub fn read(id: &str) -> Choice {
        [
            Target::Client,
            Target::Worker,
            Target::Server,
            Target::Postgis,
        ]
        .into_iter()
        .find(|t| t.id() == id)
        .map_or(Choice::Auto, Choice::At)
    }
}

/// A place as the window names it.
pub fn target_label(t: Target) -> &'static str {
    match t {
        Target::Client => "Bu bilgisayarda",
        Target::Worker => "Arka planda",
        Target::Server => "KentOS sunucusunda",
        Target::Postgis => "PostGIS veritabanında",
    }
}

/// A place in a sentence (“şimdi: arka planda”).
pub fn target_short(t: Target) -> &'static str {
    match t {
        Target::Client => "bu bilgisayarda",
        Target::Worker => "arka planda",
        Target::Server => "sunucuda",
        Target::Postgis => "PostGIS’te",
    }
}

/// Under Otomatik: when it sends a job to the background (the web's rule).
pub const AUTO_HINT: &str =
    "2.000 ya da daha çok nesneli işler arka planda çalışır; program donmaz.";

/// What the window knows of the places (the web's `TargetsInput`).
pub struct Targets {
    /// The places the tool names, in its order.
    pub declared: Vec<Target>,
    /// The places this program can run it.
    pub available: Vec<Target>,
    /// Where Otomatik sends these inputs now; none for a model (each step decides).
    pub auto: Option<Target>,
    pub model: bool,
}

/// One row of Nerede çalışır.
#[derive(Clone, Debug, PartialEq)]
pub struct TargetOption {
    pub value: Choice,
    pub label: &'static str,
    pub note: Option<String>,
    pub disabled: bool,
    pub checked: bool,
}

/// Where Otomatik sends a job of `size` objects (the web's `executorFor`):
/// the background from [`WORKER_THRESHOLD`] objects when the tool may go
/// there, else the tool's first place here.
pub fn auto_target(available: &[Target], size: usize) -> Option<Target> {
    if size >= WORKER_THRESHOLD && available.contains(&Target::Worker) {
        return Some(Target::Worker);
    }
    available.first().copied()
}

/// The choice a run takes: with several places the stored one, Otomatik
/// when that place is not here; with one place that place; none with none.
pub fn effective_choice(choice: Choice, available: &[Target]) -> Option<Choice> {
    match available {
        [] => None,
        [only] => Some(Choice::At(*only)),
        _ => Some(match choice {
            Choice::At(t) if !available.contains(&t) => Choice::Auto,
            c => c,
        }),
    }
}

/// Otomatik (and what it picks now) when there is a choice, then each
/// declared place; places not here are "yakında". Also the hint under
/// Otomatik and the choice in effect.
pub fn targets_view(
    t: &Targets,
    choice: Choice,
) -> (Vec<TargetOption>, Option<&'static str>, Option<Choice>) {
    let several = t.available.len() > 1;
    let current = effective_choice(choice, &t.available);
    let option = |value: Choice, label, note: Option<String>, disabled: bool| TargetOption {
        value,
        label,
        note,
        disabled,
        checked: !disabled && current == Some(value),
    };
    let mut options = Vec::new();
    if several {
        let note = match (t.auto, t.model) {
            (Some(now), _) => Some(format!("şimdi: {}", target_short(now))),
            (None, true) => Some("adım adım".to_owned()),
            (None, false) => None,
        };
        options.push(option(Choice::Auto, "Otomatik", note, false));
    }
    for d in &t.declared {
        options.push(if t.available.contains(d) {
            let note = (!several).then(|| "bu çalıştırmada".to_owned());
            option(Choice::At(*d), target_label(*d), note, false)
        } else {
            option(
                Choice::At(*d),
                target_label(*d),
                Some("yakında".to_owned()),
                true,
            )
        });
    }
    let hint = (several && current == Some(Choice::Auto)).then_some(AUTO_HINT);
    (options, hint, current)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../fixtures/processing/v1/dialog.json"
        ))
        .expect("dialog.json")
    }

    fn status_of(v: &Value) -> RunStatus {
        let text = || v["text"].as_str().unwrap_or_default().to_owned();
        match v["kind"].as_str().unwrap_or_default() {
            "running" => RunStatus::Running {
                fraction: v["fraction"].as_f64().unwrap_or(0.0),
                label: v["label"].as_str().unwrap_or_default().to_owned(),
                background: v["where"] == "worker",
            },
            "ok" => RunStatus::Ok {
                text: text(),
                pick: v["pick"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_u64)
                    .map(|id| Slot(id as u32))
                    .collect(),
                selected: v["selected"].as_bool().unwrap_or(false),
                undo: v["undo"].as_bool().unwrap_or(false),
            },
            "error" => RunStatus::Error(text()),
            "invalid" => RunStatus::Invalid(text()),
            _ => RunStatus::Idle,
        }
    }

    fn as_json(line: &StatusLine, footer: &Footer) -> Value {
        let kind = |k: LineKind| match k {
            LineKind::Idle => "idle",
            LineKind::Running => "running",
            LineKind::Ok => "ok",
            LineKind::Warn => "warn",
            LineKind::Error => "error",
        };
        let icon = line.icon.map(|i| match i {
            LineIcon::Success => "success",
            LineIcon::Warning => "warning",
            LineIcon::Error => "error",
        });
        let mut status = json!({
            "kind": kind(line.kind),
            "icon": icon,
            "text": line.text,
            "actions": line.actions.iter().map(|a| match a {
                Action::Zoom => "zoom",
                Action::Select => "select",
                Action::Undo => "undo",
            }).collect::<Vec<_>>(),
        });
        if let Some(p) = line.progress {
            status["progress"] = json!(p);
        }
        if let Some(pick) = &line.pick {
            status["pick"] = json!(pick.iter().map(|s| s.0).collect::<Vec<_>>());
        }
        json!({
            "status": status,
            "footer": {
                "run": { "label": footer.run, "disabled": footer.run_disabled },
                "close": footer.close,
                "reset": { "disabled": footer.reset_disabled },
            },
        })
    }

    /// Every status case of the shared file gives the web's line and
    /// buttons; a background run's line is in the desktop's words.
    #[test]
    fn the_status_line_and_the_buttons_are_the_webs() {
        let f = fixture();
        let web_background = f["texts"]["running"]["worker"].as_str().unwrap_or_default();
        let cases = f["status"].as_array().expect("status cases");
        assert!(cases.len() >= 16);
        for c in cases {
            let issues: Vec<Issue> = c["issues"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|i| Issue {
                    param: i["param"].as_str().map(str::to_owned),
                    message: i["message"].as_str().unwrap_or_default().to_owned(),
                })
                .collect();
            let status = status_of(&c["status"]);
            let got = as_json(
                &status_line(&status, c["attempted"] == true, &issues),
                &footer_of(&status),
            );
            let mut want = c["expect"].clone();
            if want["status"]["text"] == web_background {
                want["status"]["text"] = json!(RUNNING_BACKGROUND);
            }
            assert_eq!(got, want, "{}", c["title"]);
        }
    }

    fn target_of(id: &Value) -> Target {
        match Choice::read(id.as_str().unwrap_or_default()) {
            Choice::At(t) => t,
            Choice::Auto => panic!("a place: {id}"),
        }
    }

    /// Every Nerede çalışır case of the shared file gives the web's rows,
    /// hint and choice, in the desktop's words for the places.
    #[test]
    fn where_it_runs_is_the_webs() {
        let f = fixture();
        let places = |v: &Value| -> Vec<Target> {
            v.as_array().into_iter().flatten().map(target_of).collect()
        };
        // The web's words for its page, and the desktop's for this computer.
        let mut words: Vec<(String, String)> = Vec::new();
        for t in [
            Target::Client,
            Target::Worker,
            Target::Server,
            Target::Postgis,
        ] {
            let web = |key: &str| f[key][t.id()].as_str().unwrap_or_default().to_owned();
            words.push((web("targetLabels"), target_label(t).to_owned()));
            words.push((
                format!("şimdi: {}", web("targetShort")),
                format!("şimdi: {}", target_short(t)),
            ));
        }
        let web_hint = f["texts"]["targets"]["hint"].as_str().unwrap_or_default();
        let cases = f["targets"].as_array().expect("targets cases");
        assert!(cases.len() >= 9);
        for c in cases {
            let t = Targets {
                declared: places(&c["declared"]),
                available: places(&c["available"]),
                auto: (!c["auto"].is_null()).then(|| target_of(&c["auto"])),
                model: c["model"] == true,
            };
            let choice = Choice::read(c["choice"].as_str().unwrap_or_default());
            let (options, hint, current) = targets_view(&t, choice);
            let got = json!({
                "options": options.iter().map(|o| json!({
                    "value": o.value.id(),
                    "label": o.label,
                    "note": o.note,
                    "disabled": o.disabled,
                    "checked": o.checked,
                })).collect::<Vec<_>>(),
                "hint": hint,
                "choice": current.map(Choice::id),
            });
            let mut want = c["expect"].clone();
            if let Some(options) = want["options"].as_array_mut() {
                for o in options {
                    for key in ["label", "note"] {
                        if let Some((_, ours)) =
                            words.iter().find(|(web, _)| o[key] == web.as_str())
                        {
                            o[key] = json!(ours);
                        }
                    }
                }
            }
            if want["hint"] == web_hint {
                want["hint"] = json!(AUTO_HINT);
            }
            assert_eq!(got, want, "{}", c["title"]);
        }
    }
}
