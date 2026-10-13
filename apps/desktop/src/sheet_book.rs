//! The open project's sheet book (docs/adr/0251): the book is the project's,
//! in its file (`.kcad` schema 44), not this device's.
//!
//! - A drawing opened, or its book changed by something else (a sync, a
//!   script): the designer shows the drawing's book. One the sheet core does
//!   not read is said once and kept as it is: nothing done in the sheet mode
//!   is written over it.
//! - A change in the sheet mode is written into the drawing as it is made:
//!   an edit, never an undo step of the drawing (the designer keeps its own).
//! - A project without a book whose book this device kept before (the
//!   project's key, `kentos_sheet_ui::store`) takes it, and the drawing
//!   is changed. Once the drawing is saved with it, the device's copy is set
//!   aside (`kitaplar/tasindi/`), never deleted. A project that has a book
//!   has the device's copy set aside as well: the file's is the project's.

use std::sync::Arc;

use kentos_contracts::ProjectSheets;
use kentos_sheet::model::SheetBook;

use crate::app::App;

/// What the designer shows of the open drawing's book.
#[derive(Default)]
pub(crate) struct BookLink {
    /// The drawing's book the designer was last given or wrote, by pointer.
    seen: Option<Arc<ProjectSheets>>,
    /// The designer's book revision written into the drawing.
    written: u64,
    /// The drawing's book could not be read: nothing is written over it.
    unreadable: bool,
    /// The device's key whose book was moved into the drawing: set aside once the drawing is saved.
    moved: Option<String>,
}

fn same(a: Option<&Arc<ProjectSheets>>, b: Option<&Arc<ProjectSheets>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

impl App {
    /// After every message: the designer shows the drawing's book, the
    /// drawing keeps the designer's (see the module's comment).
    pub(crate) fn follow_sheet_book(&mut self) {
        let session = self.document.as_ref().map(|d| d.session);
        let opened = session != self.sheet_session;
        self.sheet_session = session;
        if opened {
            self.sheet_book = BookLink::default();
            self.open_sheet_book();
            return;
        }
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let now = doc.model.sheets_shared().cloned();
        if !same(now.as_ref(), self.sheet_book.seen.as_ref()) {
            // Changed by something else: the designer shows it.
            self.open_sheet_book();
            return;
        }
        if self.sheets.book_revision() != self.sheet_book.written && !self.sheet_book.unreadable {
            self.write_sheet_book();
        }
        self.retire_moved_book();
    }

    /// The designer given the drawing's book, or the book this device kept for the project.
    fn open_sheet_book(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            self.sheets.open_project(SheetBook::default(), Vec::new());
            self.sheet_book.written = self.sheets.book_revision();
            return;
        };
        let kept = doc.model.sheets_shared().cloned();
        let read = kept
            .as_deref()
            .map(kentos_sheet::project::read_project_sheets);
        self.sheet_book.seen = kept;
        self.sheet_book.unreadable = false;
        match read {
            Some(Ok((book, pictures))) => {
                self.sheets.open_project(book, pictures);
                self.sheet_book.written = self.sheets.book_revision();
                self.set_aside_device_book();
            }
            Some(Err(e)) => {
                self.sheets.open_project(SheetBook::default(), Vec::new());
                self.sheet_book.written = self.sheets.book_revision();
                self.sheet_book.unreadable = true;
                self.warn(format!(
                    "Bu çizimin pafta kitabı okunamadı: {} Kitap çizimde olduğu gibi duruyor; \
                     paftalarda yapılan değişiklik ona yazılmaz.",
                    e.message
                ));
            }
            None => {
                self.sheets.open_project(SheetBook::default(), Vec::new());
                self.sheet_book.written = self.sheets.book_revision();
                self.move_device_book();
            }
        }
    }

    /// The designer's book into the drawing: an edit, not an undo step.
    fn write_sheet_book(&mut self) {
        self.sheet_book.written = self.sheets.book_revision();
        let sheets = match self.sheets.project_sheets() {
            Ok(s) => s,
            Err(e) => {
                self.error(format!("Pafta kitabı çizime yazılamadı: {e}"));
                return;
            }
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        if let Err(e) = doc.model.set_sheets(sheets) {
            self.error(format!("Pafta kitabı çizime yazılamadı: {e}"));
            return;
        }
        self.sheet_book.seen = doc.model.sheets_shared().cloned();
    }

    /// A project without a book takes the one this device kept for it (docs/adr/0251 §3).
    fn move_device_book(&mut self) {
        let Some(store) = self.sheet_store.clone() else {
            return;
        };
        let key = self.sheet_key();
        if !key.lasting {
            return;
        }
        let Some(book) = store
            .load_book(&key.id)
            .filter(|b| !kentos_sheet::project::is_empty(b))
        else {
            return;
        };
        let pictures: Vec<(String, Vec<u8>)> = book
            .assets
            .iter()
            .filter_map(|a| store.asset(&a.sha256).map(|b| (a.sha256.clone(), b)))
            .collect();
        let missing = book.assets.len() - pictures.len();
        let sheets = book.sheets.len();
        self.sheets.open_project(book, pictures);
        self.write_sheet_book();
        if self.sheet_book.seen.is_none() {
            return;
        }
        self.sheet_book.moved = Some(key.id);
        let mut said = format!(
            "Bu projenin pafta kitabı ({sheets} pafta) bu bilgisayardan çizime alındı; \
             çizimi kaydedince dosyasında olur."
        );
        if missing > 0 {
            said.push_str(&format!(
                " {missing} resmin baytları bu bilgisayarda yok; paftanın ön denetimi söyler."
            ));
        }
        self.output(said);
    }

    /// Once the drawing is saved with the book it took from this device, the device's copy is set aside.
    fn retire_moved_book(&mut self) {
        let saved = self
            .document
            .as_ref()
            .is_some_and(|d| !d.model.is_dirty() && d.model.sheets().is_some());
        if !saved {
            return;
        }
        let (Some(key), Some(store)) = (self.sheet_book.moved.take(), self.sheet_store.clone())
        else {
            return;
        };
        let _ = store.retire_book(&key);
    }

    /// A project whose file has its book: a copy this device kept from before is set aside (the file's is the project's).
    fn set_aside_device_book(&mut self) {
        let Some(store) = self.sheet_store.clone() else {
            return;
        };
        let key = self.sheet_key();
        if !key.lasting {
            return;
        }
        let Some(device) = store.load_book(&key.id) else {
            return;
        };
        let same = Some(&device) == Some(self.sheets.book());
        if let Ok(Some(aside)) = store.retire_book(&key.id)
            && !same
        {
            self.output(format!(
                "Bu bilgisayarda bu projenin dosyasındakinden farklı bir pafta kitabı vardı; \
                 dosyadaki kullanılıyor, cihazdaki silinmedi: {}",
                aside.display()
            ));
        }
    }
}

#[cfg(test)]
#[path = "sheet_book_tests.rs"]
mod tests;
