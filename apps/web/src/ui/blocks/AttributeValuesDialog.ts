import type { AppContext } from '../../app/context';
import { h } from '../dom';
import { field, summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const VALUES_TITLE = 'Öznitelik değerleri';

/**
 * Öznitelik değerleri (docs/adr/0144 §7): Blok ekle's question before it
 * places a block with attribute definitions. One field per attribute, named
 * by its prompt (else its tag), its default already in it. Yerleştir gives
 * the tool the values that are not empty and not the default (an attribute
 * left at its default follows it, as an insert without a value shows it);
 * Vazgeç, Esc, × or a click beside it drops the point and the tool waits for
 * the next. The desktop's window (apps/desktop/src/attribute_values.rs) is
 * the same.
 */
export function openAttributeValuesDialog(ctx: AppContext, id: string, done: (values: Record<string, string> | null) => void): void {
  const block = ctx.doc.block(id);
  if (!block?.attributes?.length) return done({});
  const attributes = block.attributes;
  const inputs = attributes.map((a) =>
    h('input', { class: 'field', value: a.value ?? '', placeholder: a.value ?? '', 'aria-label': a.prompt?.trim() || a.tag, spellcheck: 'false' }),
  );
  const place = h('button', { class: 'btn btn--primary', type: 'button' }, 'Yerleştir');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  let answered = false;
  const dialog = new Dialog({
    title: VALUES_TITLE,
    width: 420,
    className: 'dialog--io',
    content: [
      h('div', { class: 'io-summary' }, summaryLine('info', `“${block.name}” bloğunun yerleştirmesi: boş bırakılan öznitelik varsayılanını yazar.`)),
      ...attributes.map((a, i) => field(a.prompt?.trim() || a.tag, inputs[i])),
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, place],
    // Esc, × and a click beside it: the point is dropped.
    onClose: () => {
      if (!answered) done(null);
    },
  });
  const values = (): Record<string, string> => {
    const out: Record<string, string> = {};
    attributes.forEach((a, i) => {
      const v = inputs[i].value.trim();
      if (v && v !== (a.value ?? '').trim()) out[a.tag] = v;
    });
    return out;
  };
  const run = () => {
    answered = true;
    const given = values();
    dialog.close();
    done(given);
  };
  for (const input of inputs)
    input.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter') return;
      e.preventDefault();
      run();
    });
  place.addEventListener('click', run);
  cancel.addEventListener('click', () => dialog.close());
  inputs[0].focus();
  inputs[0].select();
}
