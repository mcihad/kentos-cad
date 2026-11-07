//! Closing an editor that has its own save (DESIGN.md §7.9, docs/adr/3006): a click beside it, its × or Esc asks when
//! it holds unsaved changes, and closes at once when it does not. Its Vazgeç button still throws the changes away: it
//! says so. The question is kentos_ui's (`widget::unsaved`).

/// The answer to “Kaydedilmemiş değişiklikler”.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Kaydet ve kapat.
    Save,
    /// Kaydetmeden kapat.
    Discard,
    /// Vazgeç: the editor stays as it was.
    Stay,
}
