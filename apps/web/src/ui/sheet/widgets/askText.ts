import { h } from '../../dom';
import { Dialog } from '../../widgets/Dialog';

/**
 * A name asked in a small window (Paftaya ad ver, Öğeye ad ver): one field,
 * Tamam and Vazgeç. Enter takes it, Esc, × and the backdrop leave it. An
 * empty name is not taken (the field says so). Resolves with the trimmed
 * text, or null when it was left.
 */
export function askText(o: { title: string; label: string; value: string; stack?: boolean }): Promise<string | null> {
  return new Promise((resolve) => {
    let done = false;
    const input = h('input', { class: 'field field--setting', value: o.value, 'aria-label': o.label, spellcheck: 'false', autocomplete: 'off' });
    const error = h('p', { class: 'sheet-ask__error', role: 'alert', hidden: true }, 'Ad boş olamaz.');
    const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Tamam');
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    const finish = (v: string | null) => {
      if (done) return;
      done = true;
      dialog.close();
      resolve(v);
    };
    const take = () => {
      const v = input.value.trim();
      if (!v) {
        error.hidden = false;
        input.focus();
        return;
      }
      finish(v);
    };
    const dialog = new Dialog({
      title: o.title,
      width: 380,
      className: 'sheet-ask',
      stack: o.stack,
      content: [h('label', { class: 'sheet-ask__field' }, h('span', { class: 'sheet-field__label' }, o.label), input), error],
      footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, ok],
      onClose: () => {
        if (!done) {
          done = true;
          resolve(null);
        }
      },
    });
    ok.addEventListener('click', take);
    cancel.addEventListener('click', () => finish(null));
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        take();
      }
    });
    input.addEventListener('input', () => (error.hidden = true));
    queueMicrotask(() => {
      input.focus();
      input.select();
    });
  });
}
