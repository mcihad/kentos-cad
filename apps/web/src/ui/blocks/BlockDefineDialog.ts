import type { AppContext } from '../../app/context';
import type { BlocksDefine } from '../../contracts/generated/BlocksDefine';
import { freeBlockName, trimName } from '../../model/blocks';
import type { Vec2 } from '../../model/geometry';
import { blocksDefine } from '../../product/blocksDefine';
import { BlockInsertTool } from '../../tools/blockTools';
import { h } from '../dom';
import { field, summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/**
 * Blok oluştur's window (docs/adr/0144 §6): the new block's name (the first
 * free “Blok n” offered), what it is, and whether the objects give way to an
 * insert of it on the active layer; the tool has picked the objects and the
 * base point. Every change asks `cad.blocks.define` and shows its refusal (or
 * its warning) under the fields; Oluştur waits until it passes and writes
 * through it, one undo step “Blok tanımla”. The new block is the one Blok
 * ekle places next; the insert that took the objects' place is selected. The
 * desktop's window (`apps/desktop/src/blocks.rs`) is the same.
 */
export function openBlockDefineDialog(ctx: AppContext, base: Vec2, uids: readonly string[]): void {
  const { doc, log } = ctx;
  const name = h('input', { class: 'field', value: freeBlockName(doc.blocks.value), 'aria-label': 'Ad', spellcheck: 'false' });
  const description = h('input', { class: 'field', 'aria-label': 'Açıklama', placeholder: 'İsteğe bağlı: bloğun ne olduğu', spellcheck: 'false' });
  const replace = h('input', { type: 'checkbox', checked: ctx.blocks.replace });
  const layer = doc.layers.get(doc.layers.active.value)?.name ?? '';
  const hint = h('p', { class: 'io-field__hint' }, replaceHint(replace.checked, layer));
  const status = h('div', { role: 'status', hidden: true });
  const make = h('button', { class: 'btn btn--primary', type: 'button' }, 'Oluştur');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Blok oluştur',
    width: 460,
    className: 'dialog--io',
    content: [
      h('div', { class: 'io-summary' }, summaryLine('info', `${uids.length} nesne bloğa alınır.`), summaryLine('info', `Taban noktası: ${ctx.format.point(base)}`)),
      field('Ad', name),
      field('Açıklama', description),
      h(
        'div',
        { class: 'io-field' },
        h('label', { class: 'io-check' }, replace, 'Seçilenleri blokla değiştir'),
        hint,
      ),
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, make],
  });

  const input = (): BlocksDefine => {
    const about = trimName(description.value);
    return {
      name: trimName(name.value),
      base: { x: base.x, y: base.y },
      uids: [...uids],
      ...(about && { description: about }),
      ...(replace.checked && { replace: true, layerId: doc.layers.active.value }),
    };
  };
  /** The command's refusal (red) or warnings (amber) under the fields; nothing when it passes. */
  const show = (kind: 'error' | 'warn', text: string) => {
    status.hidden = !text;
    status.replaceChildren(...(text ? [summaryLine(kind, text)] : []));
  };
  const refresh = () => {
    const checked = blocksDefine.validate({ doc }, input());
    make.disabled = checked.status !== 'completed';
    if ('error' in checked) show('error', checked.error.message);
    else show('warn', checked.status === 'completed' ? checked.warnings.map((w) => w.message).join(' ') : '');
  };
  const run = () => {
    if (make.disabled) return;
    const given = input();
    const result = blocksDefine.execute({ doc }, given);
    if (result.status !== 'completed') {
      if ('error' in result) show('error', result.error.message);
      return;
    }
    ctx.blocks.replace = replace.checked;
    BlockInsertTool.block = result.output.block;
    dialog.close();
    if (result.output.id !== undefined) ctx.selection.set([result.output.id]);
    for (const w of result.warnings) log.warn(w.message);
    log.success(`“${given.name}” bloğu tanımlandı: ${uids.length} nesne.${result.output.insert ? ' Nesneler yerleştirmeyle değiştirildi.' : ''}`);
  };

  for (const f of [name, description]) {
    f.addEventListener('input', refresh);
    f.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter') return;
      e.preventDefault();
      run();
    });
  }
  replace.addEventListener('change', () => {
    hint.textContent = replaceHint(replace.checked, layer);
    refresh();
  });
  make.addEventListener('click', run);
  cancel.addEventListener('click', () => dialog.close());
  refresh();
  name.focus();
  name.select();
}

/** What “Seçilenleri blokla değiştir” does, as it is set (the desktop's `replace_hint`). */
function replaceHint(replace: boolean, layer: string): string {
  return replace ? `Nesneler silinir; yerlerine bloğun bir yerleştirmesi “${layer}” katmanına konur.` : 'Nesneler yerlerinde kalır; blok yalnız tanımlanır.';
}
