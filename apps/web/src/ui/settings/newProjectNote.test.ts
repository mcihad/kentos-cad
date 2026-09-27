import { describe, expect, it } from 'vitest';
import { newProjectNote } from './newProjectNote';

/** Yeni proje's note about the drawing on screen (newProjectNote.ts). */
describe('Yeni proje says what becomes of the drawing on screen', () => {
  const cloud = (database: boolean, autosaves: boolean) => ({ name: 'Ada 101', autosaves, database });

  it('a database project that saves by itself is closed, what waits sent', () => {
    expect(newProjectNote({ name: 'Ada 101', cloud: cloud(true, true), dirty: true })).toEqual({
      tone: 'info',
      text: '“Ada 101” bulut projesi kapanır. Bekleyen değişiklikleri buluta gönderilir; gönderilemeyenler bu cihazda kalır ve proje yeniden açılınca geri gelir.',
    });
  });

  it('a database project that does not save: its edits are not sent, what to do is asked', () => {
    expect(newProjectNote({ name: 'Ada 101', cloud: cloud(true, false), dirty: true })).toEqual({
      tone: 'warn',
      text: '“Ada 101” projesindeki değişiklikleriniz buluta kaydedilmiyor; Oluştur’a basınca ne yapılacağı sorulur.',
    });
  });

  it('a file project’s edits are saved by Kaydet: the unsaved question, as a local drawing’s (not “read-only”)', () => {
    const local = newProjectNote({ name: 'Ada 101', cloud: null, dirty: true });
    expect(newProjectNote({ name: 'Ada 101', cloud: cloud(false, false), dirty: true })).toEqual(local);
    expect(local).toEqual({ tone: 'warn', text: '“Ada 101” içinde kaydedilmemiş değişiklikler var; Oluştur’a basınca önce sorulur.' });
  });

  it('nothing over a clean drawing that does not save by itself', () => {
    expect([newProjectNote({ name: 'Ada 101', cloud: null, dirty: false }), newProjectNote({ name: 'Ada 101', cloud: cloud(false, false), dirty: false }), newProjectNote({ name: 'Ada 101', cloud: cloud(true, false), dirty: false })]).toEqual([null, null, null]);
  });
});
