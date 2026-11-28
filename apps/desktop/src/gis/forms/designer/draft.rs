//! The Formlar window's draft kept on this device as it is edited (docs/adr/0441 §5), written off the interface thread
//! (docs/adr/9401 §6): one write out at a time, the newest draft after it, a failure said once.

use super::*;
use crate::forms::library::KeptDraft;

impl App {
    /// The window's draft handed to be written; the write's task when none is out.
    pub(super) fn keep_form_draft(&mut self, draft: KeptDraft) -> Task<Message> {
        let project = self.form_project_key();
        self.write_form_draft(project, draft)
    }

    fn write_form_draft(&mut self, project: String, draft: KeptDraft) -> Task<Message> {
        let Some((project, draft)) = self.gis.form_drafts.take(project, draft) else {
            return Task::none();
        };
        let files = self.gis.form_files.clone();
        crate::db_connections::off_thread(
            move || files.keep_draft(&project, &draft),
            |written| Message::Gis(gis::Event::Forms(Event::DraftKept(written))),
        )
    }

    /// A draft's write ended: a failure that is news is said, the draft that waited is written.
    pub(super) fn form_draft_kept(&mut self, written: Result<(), String>) -> Task<Message> {
        let ended = self.gis.form_drafts.ended(&written);
        if let Some((project, form)) = &ended.let_go {
            self.gis.form_files.drop_draft(project, form);
        }
        if let (true, Err(why)) = (ended.news, &written) {
            self.warn(format!(
                "Formun taslağı bu aygıta yazılamadı ({why}): program kapanırsa penceredeki değişiklikler geri gelmez. \
                 Yitirmemek için Taslağı kaydet’e basın."
            ));
        }
        match ended.next {
            Some((project, draft)) => self.write_form_draft(project, draft),
            None => Task::none(),
        }
    }

    /// The draft of `form` let go (saved, or Vazgeç), a write still out included.
    pub(super) fn drop_form_draft(&mut self, form: &str) {
        let project = self.form_project_key();
        self.gis.form_drafts.let_go(&project, form);
        self.gis.form_files.drop_draft(&project, form);
    }
}
