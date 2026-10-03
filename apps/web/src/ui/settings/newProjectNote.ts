/**
 * What Yeni proje says of the drawing on screen before anything is done
 * (NewProjectWizard.ts; docs/inventory/parity-audit.md N1): a cloud project
 * that saves by itself is closed and what waits is sent; unsaved changes of
 * a database project that does not save (read-only, archived, gone) are not
 * sent, and what to do is asked at Oluştur; a local drawing's or a file
 * project's unsaved changes (Kaydet saves them) get the unsaved question at
 * Oluştur. Nothing is said over a clean drawing.
 */

export interface NewProjectNote {
  tone: 'info' | 'warn';
  text: string;
}

export function newProjectNote(t: {
  /** The drawing's name. */
  name: string;
  /** The open cloud project: its name, whether it saves by itself now, and whether it keeps objects in the database. */
  cloud: { name: string; autosaves: boolean; database: boolean } | null;
  dirty: boolean;
}): NewProjectNote | null {
  if (t.cloud?.autosaves)
    return { tone: 'info', text: `“${t.cloud.name}” bulut projesi kapanır. Bekleyen değişiklikleri buluta gönderilir; gönderilemeyenler bu cihazda kalır ve proje yeniden açılınca geri gelir.` };
  if (!t.dirty) return null;
  if (t.cloud?.database) return { tone: 'warn', text: `“${t.cloud.name}” projesindeki değişiklikleriniz buluta kaydedilmiyor; Oluştur’a basınca ne yapılacağı sorulur.` };
  return { tone: 'warn', text: `“${t.name}” içinde kaydedilmemiş değişiklikler var; Oluştur’a basınca önce sorulur.` };
}
