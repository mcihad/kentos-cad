//! Dosya › Kurtar (docs/adr/1013): a `.kcad` the reader refuses, recovered by `file.recover` into a separate copy
//! (`<name>-kurtarilan.kcad` beside it) which is then opened; the source is never written. Komut geçmişi says what came
//! back, what the container lacked and each part left out with the reader's words.

use std::path::PathBuf;

use iced::Task;
use kentos_commands::{CallOp, CommandResult, KcadRecovered, RecoverKcad};
use kentos_interaction::Level;

use crate::app::{App, Message, Picker};

/// How many lost parts Komut geçmişi lists.
const SHOWN: usize = 12;

impl App {
    /// Kurtar: the file asked for, then recovered.
    pub(crate) fn recover_file(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return self.recover_path(path);
        }
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Bozuk çizimi kurtar")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            Message::Recover,
        )
    }

    pub(crate) fn recover_path(&mut self, path: PathBuf) -> Task<Message> {
        let input = RecoverKcad { path: path.display().to_string(), target: None };
        let done = match kentos_headless::recover::run(&input, CallOp::Execute) {
            CommandResult::Completed { output, .. } => serde_json::from_value::<KcadRecovered>(output).ok(),
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                self.say(Level::Error, format!("Kurtarılamadı: {}", error.message));
                return Task::none();
            }
            _ => None,
        };
        let Some(done) = done else {
            return Task::none();
        };
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if done.whole {
            self.say(Level::Success, format!("“{name}” sağlam açılıyor; kurtarılacak bir şey yok. Dosyayı Aç ile açın."));
            return Task::none();
        }
        for line in &done.container {
            self.output(format!("Kurtar: {line}."));
        }
        for lost in done.lost.iter().take(SHOWN) {
            let id = lost.uid.as_deref().map(|u| format!(" ({u})")).unwrap_or_default();
            self.output(format!("Kurtar: {}{id} alınmadı: {}", lost.part, lost.reason));
        }
        if done.lost.len() > SHOWN {
            self.output(format!("Kurtar: … ve {} parça daha.", done.lost.len() - SHOWN));
        }
        self.say(
            Level::Success,
            format!(
                "“{name}” kurtarıldı: {} nesneden {}'i geldi, {} parça alınmadı. Kopya {} olarak yazıldı ve açılıyor; kaynak dosya değiştirilmedi.",
                done.objects_read,
                done.objects_kept,
                done.lost.len(),
                done.target
            ),
        );
        let target = PathBuf::from(&done.target);
        self.update(Message::Opening(crate::opening::Event::Picked(Some(target))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kcad/recover/v1").join(name)
    }

    #[test]
    fn kurtar_writes_a_copy_opens_it_and_leaves_the_source() {
        let dir = crate::files_testing::scratch("kurtar");
        let source = dir.join("pafta.kcad");
        std::fs::copy(fixture("kimlik-7.kcad"), &source).expect("copied");
        let before = std::fs::read(&source).expect("read");
        let mut app = crate::files_testing::app_with_drawing();
        app.picker = Picker::File(source.clone());
        let task = app.run("file.recover");
        crate::files_testing::drive(&mut app, task);
        assert_eq!(std::fs::read(&source).expect("read"), before, "the source is untouched");
        let copy = dir.join("pafta-kurtarilan.kcad");
        assert!(copy.exists(), "the copy is written");
        let doc = app.document.as_ref().expect("the copy is open");
        assert_eq!(doc.path.as_deref(), Some(copy.as_path()));
        assert_eq!(doc.entity_count(), 14, "15 objects, one lost");
        let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
        assert!(said.iter().any(|l| l.starts_with("Kurtar: entities[7]")), "{said:?}");
        assert!(said.iter().any(|l| l.contains("kurtarıldı: 15 nesneden 14'i geldi")), "{said:?}");
    }

    #[test]
    fn a_whole_file_is_not_copied() {
        let dir = crate::files_testing::scratch("kurtar-butun");
        let source = dir.join("pafta.kcad");
        std::fs::copy(fixture("butun.kcad"), &source).expect("copied");
        let mut app = crate::files_testing::app_with_drawing();
        app.picker = Picker::File(source);
        let task = app.run("file.recover");
        crate::files_testing::drive(&mut app, task);
        assert!(!dir.join("pafta-kurtarilan.kcad").exists());
        assert!(app.log.lines().any(|l| l.text.contains("sağlam açılıyor")));
    }
}

/// The picture of docs/adr/1013: a drawing cut inside its ninth object recovered with Kurtar, the copy open and Komut
/// geçmişi saying what came back. `KENTOS_SHOTS_ONLY=kurtar cargo test -p kentos-desktop --lib tools_screens::screens
/// -- --ignored --nocapture`.
#[cfg(test)]
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![("kurtar", |app| {
        let dir = crate::files_testing::scratch("kurtar-resim");
        let source = dir.join("kadastro.kcad");
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kcad/recover/v1/kesik-8.kcad"),
            &source,
        )
        .expect("copied");
        app.picker = Picker::File(source);
        let task = app.run("file.recover");
        crate::files_testing::drive(app, task);
    })]
}
