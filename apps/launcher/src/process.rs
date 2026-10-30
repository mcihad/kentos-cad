//! Starting apps (docs/adr/0800 §4): each a process of its own, never
//! waited on. The launcher keeps the child and looks once a second whether it
//! ended (`try_wait`, which also reaps it); the card says “Çalışıyor” while it
//! runs. A program that cannot start, or ends with an error, is said with why
//! and what to do: its output goes to a log file whose last line the sentence
//! quotes. A web address and a folder are handed to the system's opener.
//! Closing the launcher leaves the programs running.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};

use crate::files;
use crate::resolve::Spec;

/// A process the launcher started.
#[derive(Debug)]
pub struct Running {
    /// The app's id.
    pub app: String,
    /// The app's name, for a sentence.
    pub name: String,
    child: Child,
    log: Option<PathBuf>,
    /// The system's opener (a web address, a folder): not a running app.
    pub opener: bool,
}

/// How a process ended, when it is worth saying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    pub app: String,
    /// The sentence: what ended how, its last line, its log.
    pub sentence: String,
}

/// The processes started and where their logs go.
#[derive(Debug, Default)]
pub struct Processes {
    running: Vec<Running>,
    /// `…/kentos-launcher/gunluk`; none: output is dropped.
    logs: Option<PathBuf>,
}

impl Processes {
    /// Logs in `$XDG_STATE_HOME/kentos-launcher/gunluk`.
    pub fn system() -> Self {
        Self {
            running: Vec::new(),
            logs: files::state_home().map(|s| s.join("kentos-launcher").join("gunluk")),
        }
    }

    /// No logs (the snapshot, tests).
    pub fn quiet() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub fn with_logs(logs: PathBuf) -> Self {
        Self {
            running: Vec::new(),
            logs: Some(logs),
        }
    }

    /// How many of an app's processes run.
    pub fn count(&self, app: &str) -> usize {
        self.running
            .iter()
            .filter(|r| !r.opener && r.app == app)
            .count()
    }

    /// How many apps run (not the openers).
    pub fn apps_running(&self) -> usize {
        let mut apps: Vec<&str> = self
            .running
            .iter()
            .filter(|r| !r.opener)
            .map(|r| r.app.as_str())
            .collect();
        apps.sort_unstable();
        apps.dedup();
        apps.len()
    }

    /// Whether anything is to be looked at.
    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }

    /// Starts an app. A refusal is the sentence said.
    pub fn start(&mut self, app: &str, name: &str, spec: &Spec) -> Result<(), String> {
        let log = self.log_for(app);
        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .envs(spec.env.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null());
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        let file = log.as_ref().and_then(|p| File::create(p).ok());
        match file
            .as_ref()
            .and_then(|f| Some((f.try_clone().ok()?, f.try_clone().ok()?)))
        {
            Some((out, err)) => {
                command.stdout(out).stderr(err);
            }
            None => {
                command.stdout(Stdio::null()).stderr(Stdio::null());
            }
        }
        detach(&mut command);
        let child = command
            .spawn()
            .map_err(|e| start_failed(name, &spec.program, &e))?;
        self.running.push(Running {
            app: app.to_owned(),
            name: name.to_owned(),
            child,
            log: file.and(log),
            opener: false,
        });
        Ok(())
    }

    /// Hands a web address or a folder to the system's opener.
    pub fn open(&mut self, app: &str, name: &str, target: &OsStr) -> Result<(), String> {
        let (program, args) = opener(target);
        let mut command = Command::new(&program);
        command
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        detach(&mut command);
        let child = command.spawn().map_err(|e| {
            format!(
                "“{}” açılamadı: sistemin açıcısı {} başlamadı ({e}). Varsayılan tarayıcıyı ve dosya yöneticisini denetleyin.",
                target.to_string_lossy(),
                program.to_string_lossy()
            )
        })?;
        self.running.push(Running {
            app: app.to_owned(),
            name: name.to_owned(),
            child,
            log: None,
            opener: true,
        });
        Ok(())
    }

    /// Looks at every process; those that ended leave the list. Ended with
    /// an error, they are said.
    pub fn poll(&mut self) -> Vec<Ended> {
        let mut said = Vec::new();
        self.running.retain_mut(|run| match run.child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                if let Some(sentence) = ended(run, status) {
                    said.push(Ended {
                        app: run.app.clone(),
                        sentence,
                    });
                }
                false
            }
            Err(e) => {
                said.push(Ended {
                    app: run.app.clone(),
                    sentence: format!(
                        "{} izlenemiyor ({e}); çalışıyor olabilir, launcher artık izlemiyor.",
                        run.name
                    ),
                });
                false
            }
        });
        said
    }

    /// A log file for the app's next process: `<id>.log`, `<id>-2.log` while
    /// an earlier one of it still writes.
    fn log_for(&self, app: &str) -> Option<PathBuf> {
        let folder = self.logs.as_ref()?;
        std::fs::create_dir_all(folder).ok()?;
        let busy = |path: &Path| self.running.iter().any(|r| r.log.as_deref() == Some(path));
        (1..)
            .map(|n| {
                folder.join(if n == 1 {
                    format!("{app}.log")
                } else {
                    format!("{app}-{n}.log")
                })
            })
            .find(|p| !busy(p))
    }
}

/// Its own process group on Unix: a terminal's Ctrl+C to the launcher does not reach it.
fn detach(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(not(unix))]
    let _ = command;
}

/// The system's opener for a web address or a folder.
pub fn opener(target: &OsStr) -> (OsString, Vec<OsString>) {
    if cfg!(target_os = "macos") {
        (OsString::from("open"), vec![target.to_owned()])
    } else if cfg!(windows) {
        (OsString::from("explorer"), vec![target.to_owned()])
    } else {
        (OsString::from("xdg-open"), vec![target.to_owned()])
    }
}

fn start_failed(name: &str, program: &Path, error: &std::io::Error) -> String {
    let what = match error.kind() {
        std::io::ErrorKind::NotFound => {
            "program bulunamadı. Konumu seç… ile yerini gösterin.".to_owned()
        }
        std::io::ErrorKind::PermissionDenied => {
            "çalıştırma izni yok. Dosyaya çalıştırma izni verin (chmod +x) ya da başka bir program seçin."
                .to_owned()
        }
        _ => format!("{error}. Programın yerini ve iznini denetleyin."),
    };
    format!("{name} başlatılamadı ({}): {what}", program.display())
}

/// The sentence for a process that ended, or none when it ended well.
fn ended(run: &Running, status: ExitStatus) -> Option<String> {
    if status.success() {
        return None;
    }
    if run.opener {
        return Some(format!(
            "{} açılamadı: sistemin açıcısı {} kapandı. Varsayılan tarayıcıyı ve dosya yöneticisini denetleyin.",
            run.name,
            how(status)
        ));
    }
    let mut sentence = format!("{} {} kapandı.", run.name, how(status));
    if let Some(log) = &run.log {
        if let Some(line) = last_line(log) {
            sentence.push_str(&format!(" Son satırı: “{line}”."));
        }
        sentence.push_str(&format!(" Çıktısı: {}", log.display()));
    }
    Some(sentence)
}

/// “1 çıkış koduyla”, “11 sinyaliyle”.
fn how(status: ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("{code} çıkış koduyla");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return format!("{signal} sinyaliyle");
        }
    }
    "bilinmeyen bir sonuçla".to_owned()
}

/// A log's last line that says something, at most 160 characters.
fn last_line(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(4096)))
        .ok()?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail).ok()?;
    let text = String::from_utf8_lossy(&tail);
    let line = text.lines().rev().map(str::trim).find(|l| !l.is_empty())?;
    let mut line: String = line.chars().take(160).collect();
    if line.chars().count() == 160 {
        line.push('…');
    }
    Some(line)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn sh(script: &str) -> Spec {
        let mut spec = Spec {
            program: PathBuf::from("/bin/sh"),
            ..Spec::default()
        };
        spec.args = vec![OsString::from("-c"), OsString::from(script)];
        spec
    }

    /// Polls until nothing runs, at most five seconds.
    fn settle(processes: &mut Processes) -> Vec<Ended> {
        let until = Instant::now() + Duration::from_secs(5);
        let mut said = Vec::new();
        while !processes.is_empty() && Instant::now() < until {
            said.extend(processes.poll());
            std::thread::sleep(Duration::from_millis(20));
        }
        said
    }

    #[test]
    fn a_process_runs_ends_well_or_is_said_with_its_last_line() {
        let logs = files::scratch("process");
        let mut processes = Processes::with_logs(logs.clone());
        processes
            .start("iyi", "İyi", &sh("sleep 0.2; echo tamam"))
            .expect("starts");
        processes
            .start(
                "kotu",
                "Kötü",
                &sh("echo 'yükleniyor'; echo 'veritabanına bağlanılamadı' >&2; exit 3"),
            )
            .expect("starts");
        assert_eq!(processes.count("iyi"), 1);
        assert_eq!(processes.apps_running(), 2);
        let said = settle(&mut processes);
        assert_eq!(said.len(), 1, "{said:?}");
        assert_eq!(said[0].app, "kotu");
        assert!(
            said[0].sentence.starts_with(
                "Kötü 3 çıkış koduyla kapandı. Son satırı: “veritabanına bağlanılamadı”."
            ),
            "{}",
            said[0].sentence
        );
        assert!(said[0].sentence.contains("kotu.log"));
        assert_eq!(processes.count("iyi"), 0, "it left the list");
        let _ = std::fs::remove_dir_all(&logs);
    }

    #[test]
    fn a_program_that_cannot_start_says_why() {
        let mut processes = Processes::quiet();
        let missing = Spec {
            program: PathBuf::from("/yok/kentos-gis"),
            ..Spec::default()
        };
        let why = processes
            .start("g", "KentOS GIS", &missing)
            .expect_err("refused");
        assert!(
            why.starts_with("KentOS GIS başlatılamadı (/yok/kentos-gis): program bulunamadı."),
            "{why}"
        );
        let dir = files::scratch("process-perm");
        let file = dir.join("betik");
        std::fs::write(&file, "#!/bin/sh\n").expect("written");
        let why = processes
            .start(
                "b",
                "Betik",
                &Spec {
                    program: file,
                    ..Spec::default()
                },
            )
            .expect_err("refused");
        assert!(why.contains("çalıştırma izni yok"), "{why}");
        assert!(processes.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_process_of_the_same_app_logs_apart_and_signals_are_said() {
        let logs = files::scratch("process-two");
        let mut processes = Processes::with_logs(logs.clone());
        processes.start("a", "A", &sh("sleep 0.3")).expect("starts");
        processes
            .start("a", "A", &sh("kill -9 $$"))
            .expect("starts");
        assert_eq!(processes.count("a"), 2);
        assert_eq!(processes.apps_running(), 1);
        assert!(logs.join("a.log").exists() && logs.join("a-2.log").exists());
        let said = settle(&mut processes);
        assert_eq!(said.len(), 1);
        assert!(
            said[0].sentence.starts_with("A 9 sinyaliyle kapandı."),
            "{}",
            said[0].sentence
        );
        let _ = std::fs::remove_dir_all(&logs);
    }

    #[test]
    fn the_opener_is_the_platforms() {
        let (program, args) = opener(OsStr::new("https://kentos.app"));
        assert_eq!(
            program,
            OsString::from(if cfg!(target_os = "macos") {
                "open"
            } else {
                "xdg-open"
            })
        );
        assert_eq!(args, [OsString::from("https://kentos.app")]);
    }
}
