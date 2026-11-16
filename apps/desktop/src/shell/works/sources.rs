//! Where each work is read (mod.rs): from its own state, as it is now.

use std::sync::atomic::Ordering;

use kentos_commands::JobPhase;

use super::{By, Key, Work};
use crate::app::App;
use crate::extensions::environment::EnvBusy;
use crate::python_personal::Busy;
use crate::python_setup::progress_words;

impl App {
    /// İşlemler' runs on their threads, a tool's or a model's: the user's (an agent's run is a job of its own).
    pub(super) fn processing_works(&self) -> Vec<Work> {
        self.gis
            .processing
            .running
            .iter()
            .map(|run| {
                let what = match run.work {
                    crate::processing::background::Work::Tool(_) => "İşlemler aracı",
                    crate::processing::background::Work::Model(_) => "İşlemler modeli",
                };
                let mut work = Work::new(Key::Processing(run.id), what, run.work.label());
                work.since = Some(run.started);
                if let Some((share, step)) = &run.progress {
                    work.share = Some(*share);
                    work.state = (!step.trim().is_empty()).then(|| step.clone());
                }
                work
            })
            .collect()
    }

    /// The extensions' commands in their processes, the audit rule and the trial running, and the environments being
    /// made.
    pub(super) fn extension_works(&self) -> Vec<Work> {
        let mut out = Vec::new();
        for (ext, runner) in &self.extensions.runners {
            let Some(run) = &runner.run else {
                continue;
            };
            let title = format!("{}: {}", self.extension_name(ext), run.title);
            let mut work = Work::new(Key::Command(ext.clone()), "Eklenti komutu", title);
            work.since = Some(run.started);
            if let Some((done, total, words)) = &run.progress {
                work.share = total
                    .filter(|t| *t > 0.0)
                    .map(|t| (done / t).clamp(0.0, 1.0));
                work.state.clone_from(words);
            }
            work.stopping = run.stopping.is_some();
            work.can_show = true;
            out.push(work);
        }
        if let Some(audit) = &self.extensions.audit
            && let Some(stop) = &audit.running
        {
            let title = format!("{}: {}", self.extension_name(&audit.ext), audit.title);
            let mut work = Work::new(Key::Audit, "Eklenti denetimi", title);
            work.by = audit.by;
            work.since = Some(audit.started);
            work.stopping = stop.load(Ordering::Relaxed);
            work.can_show = true;
            out.push(work);
        }
        if let Some(trial) = &self.extensions.trial
            && let Some(stop) = &trial.running
        {
            let title = format!(
                "{}: örnek çizimde deneniyor",
                self.extension_name(&trial.ext)
            );
            let mut work = Work::new(Key::Trial, "Deneme koşusu", title);
            work.by = trial.by;
            work.since = Some(trial.started);
            work.stopping = stop.load(Ordering::Relaxed);
            out.push(work);
        }
        for (ext, env) in &self.extensions.envs {
            let Some(busy) = env.busy else {
                continue;
            };
            let doing = match busy {
                EnvBusy::Previewing => "kurulacak paketler çözülüyor",
                EnvBusy::Preparing => "Python ortamı hazırlanıyor",
                EnvBusy::Removing => "Python ortamı kaldırılıyor",
            };
            let title = format!("{}: {doing}", self.extension_name(ext));
            let mut work = Work::new(Key::Environment(ext.clone()), "Eklenti ortamı", title);
            work.can_stop = busy == EnvBusy::Preparing && env.stop.is_some();
            work.stopping = env.stop.as_ref().is_some_and(|s| s.load(Ordering::Relaxed));
            if let Some(progress) = env.progress {
                work.share = progress.share().map(f64::from);
                work.state = Some(progress_words(progress));
            }
            work.can_show = true;
            out.push(work);
        }
        out
    }

    /// The console's code while it runs: the file, or its first line.
    pub(super) fn console_work(&self) -> Option<Work> {
        let run = self.python.running.as_ref()?;
        let mut work = Work::new(Key::Console, "Python konsolu", run.title.clone());
        work.since = Some(run.started);
        work.can_show = true;
        Some(work)
    }

    /// KentOS's own Python being installed, and the personal environment's change.
    pub(super) fn python_works(&self) -> Vec<Work> {
        let mut out = Vec::new();
        let setup = &self.python_setup;
        if let Some(stop) = &setup.installing {
            let mut work = Work::new(
                Key::Install,
                "Python kurulumu",
                "KentOS'un Python'u kuruluyor",
            );
            if let Some(progress) = setup.progress {
                work.share = progress.share().map(f64::from);
                work.state = Some(progress_words(progress));
            }
            work.stopping = stop.load(Ordering::Relaxed);
            work.can_show = true;
            out.push(work);
        }
        if let Some(busy) = setup.personal.busy {
            let title = match busy {
                Busy::Creating => "Kişisel Python ortamı kuruluyor",
                Busy::Previewing => "Kişisel ortamın değişikliği çözülüyor",
                Busy::Applying => "Kişisel ortama paketler kuruluyor",
                Busy::Removing => "Kişisel Python ortamı kaldırılıyor",
                Busy::Cleaning => "Paket önbelleği temizleniyor",
                Busy::Checking => "Paketlerin yeni sürümleri aranıyor",
                Busy::Adding => "Eklentiye bağımlılık ekleniyor",
            };
            let mut work = Work::new(Key::Personal, "Kişisel Python ortamı", title);
            // Only installing packages has a Durdur of its own; the rest are short or uv's to finish.
            let stop = setup.personal.stop.as_ref();
            work.can_stop = busy == Busy::Applying && stop.is_some();
            work.stopping = stop.is_some_and(|s| s.load(Ordering::Relaxed));
            if busy == Busy::Applying
                && let Some(progress) = setup.personal.progress
            {
                work.share = progress.share().map(f64::from);
                work.state = Some(progress_words(progress));
            }
            work.can_show = true;
            out.push(work);
        }
        out
    }

    /// Asistan's turn under way: the user asked it; what it waits for.
    pub(super) fn assistant_work(&self) -> Option<Work> {
        let turn = self.assistant_turn()?;
        let model = self.assistant.shown.1.trim();
        let title = if model.is_empty() {
            "Asistan yanıtlıyor".to_owned()
        } else {
            format!("Asistan yanıtlıyor ({model})")
        };
        let mut work = Work::new(Key::Assistant, "Asistan", title);
        work.since = Some(turn.started);
        work.state = if turn.waiting > 0 {
            Some(format!("Onayınızı bekleyen araç çağrısı: {}", turn.waiting))
        } else if turn.later {
            Some("Başlattığı işin bitmesini bekliyor".to_owned())
        } else {
            None
        };
        work.can_show = true;
        Some(work)
    }

    /// The jobs the agents and Asistan started, while they go: from queued to written.
    pub(super) fn agent_job_works(&self) -> Vec<Work> {
        self.python
            .job_starts
            .iter()
            .filter_map(|(id, start)| {
                let status = self.python.jobs.peek(id)?;
                let state = match (status.phase, start.plan) {
                    (JobPhase::Queued, _) => "Sırada",
                    (JobPhase::Computing, false) => "Hesaplanıyor",
                    (JobPhase::Computing, true) => "Planı hesaplanıyor",
                    (JobPhase::Staged, false) => "Hesaplandı; yazmak için ajanı bekliyor",
                    (JobPhase::Staged, true) => "Plan hazır; ajanı bekliyor",
                    (JobPhase::Committing, _) => "Çizime yazılıyor",
                    _ => return None,
                };
                // Computed: its bar full, not running on; what it waits for is the agent's next question.
                let computed = matches!(status.phase, JobPhase::Staged | JobPhase::Committing);
                let mut work = Work::new(Key::Job(id.clone()), "Ajan işi", start.title());
                work.by = start.by;
                work.since = Some(start.at);
                work.share = if computed { Some(1.0) } else { status.progress };
                work.state = Some(state.to_owned());
                work.can_stop = status.phase != JobPhase::Committing;
                // An agent's on the link: the Python tab's link and its log; Asistan's: its panel.
                work.can_show = start.by == By::Agent;
                Some(work)
            })
            .collect()
    }
}
