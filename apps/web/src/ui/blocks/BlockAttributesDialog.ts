import type { AppContext } from '../../app/context';
import { attributesInsert } from '../../app/blocks';
import type { BlocksEdit } from '../../contracts/generated/BlocksEdit';
import { blocksEdit } from '../../product/blocksEdit';
import { PickPointTool } from '../../tools/pickPointTool';
import { Grid, type GridModel } from '../calc/common';
import { h } from '../dom';
import { summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';
import { ATTRIBUTE_COLUMNS, definitionOf, newRow, outlineExtent, placed, rowOf, type AttributeRow } from './attributeTable';

/** The window's title, which a trace names it by. */
export const ATTRIBUTES_TITLE = 'Blok öznitelikleri';

/**
 * Blok öznitelikleri (docs/adr/0144 §7): the Bloklar panel's window for a
 * block's attribute definitions, a table as the Hesap windows' (`Grid`):
 * tag, prompt, default, text height, turn and place (east Y and north X of
 * the base point, in the block's own units), one row each. A row's place is
 * typed, or shown on an insert of the block (the selected one, or its only
 * one) and taken back into the definition, as the base point is: the window
 * closes for the point and opens again as it was left. Every change asks
 * `cad.blocks.edit` (`attributes`) and shows its refusal under the table;
 * Kaydet waits until it passes and writes the whole list, one undo step
 * “Blok değiştir”. The desktop's window (apps/desktop/src/blocks_attributes.rs)
 * is the same.
 */
export function openBlockAttributesDialog(ctx: AppContext, id: string, draft?: AttributeRow[]): void {
  const { doc, log } = ctx;
  const block = doc.block(id);
  if (!block) return;
  const { base, name } = block;
  const rows: AttributeRow[] = draft ?? (block.attributes ?? []).map((a) => rowOf(a, base));
  const extent = outlineExtent(ctx.view.blockOutlines(id, { x: 0, y: 0 }));
  // A new block's first text is as high as a text Yazı writes: 2.5 mm on paper.
  const height = (2.5 / 1000) * doc.settings.plotScale.value;
  const via = attributesInsert(ctx, id);
  const status = h('div', { role: 'status', hidden: true });
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');

  const input = (): BlocksEdit => ({ operation: 'attributes', block: id, attributes: rows.map((r) => definitionOf(r, base)) });
  const refresh = () => {
    const checked = blocksEdit.validate({ doc }, input());
    save.disabled = checked.status !== 'completed';
    const why = 'error' in checked ? checked.error.message : '';
    status.hidden = !why;
    status.replaceChildren(...(why ? [summaryLine('error', why)] : []));
  };
  /** The row's place shown on the insert: the window goes for the point and comes back with it. */
  const pick = (r: number) => {
    if ('why' in via) return;
    dialog.close();
    const done = (p: { x: number; y: number } | null) => {
      const local = p && ctx.view.insertLocal(via.insert, p);
      if (local) rows[r] = placed(rows[r], local, base);
      queueMicrotask(() => openBlockAttributesDialog(ctx, id, rows));
    };
    ctx.tools.run(new PickPointTool(ctx, `${r + 1}. özniteliğin yeri`, done), `Öznitelik yeri: ${name}`);
  };
  const model: GridModel = {
    // East and north as the project's type names them (docs/adr/0165 §4).
    columns: ATTRIBUTE_COLUMNS.map((c) => ({ ...c, label: c.key === 'y' ? ctx.format.eastLabel : c.key === 'x' ? ctx.format.northLabel : c.label })),
    rows: () => rows.map((r) => r.cells),
    readonly: () => false,
    canInsertAfter: () => true,
    insertAfter: (r) => rows.splice(r + 1, 0, newRow(rows.slice(0, r + 1), base, extent, height)),
    canRemove: () => true,
    remove: (r) => rows.splice(r, 1),
    addLabel: 'Öznitelik ekle',
    actions: (r) => [
      {
        icon: 'target',
        label: `${r + 1}. satırın yerini seç`,
        tip: 'why' in via ? via.why : `Yerini “${name}” bloğunun yerleştirmesinde gösterin.`,
        disabled: 'why' in via,
        run: () => pick(r),
      },
    ],
  };
  const grid = new Grid(model, refresh);
  const dialog = new Dialog({
    title: ATTRIBUTES_TITLE,
    width: 860,
    className: 'dialog--io',
    content: [
      h(
        'div',
        { class: 'io-summary' },
        summaryLine('info', `“${name}” bloğu: yerleştirmeler özniteliklerin kendi değerlerini, değeri olmayanlar varsayılanı yazar.`),
        summaryLine('info', 'Yer taban noktasına göredir: Y doğuya, X kuzeye, bloğun kendi ölçüsünde.'),
      ),
      grid.el,
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
  });

  save.addEventListener('click', () => {
    if (save.disabled) return;
    const given = input();
    const result = blocksEdit.execute({ doc }, given);
    if (result.status !== 'completed') {
      if ('error' in result) status.replaceChildren(summaryLine('error', result.error.message));
      status.hidden = false;
      return;
    }
    dialog.close();
    for (const w of result.warnings) log.warn(w.message);
    if (!result.output.changed.length) return;
    const n = given.attributes?.length ?? 0;
    log.success(n ? `“${name}” bloğunun öznitelikleri kaydedildi: ${n} öznitelik.` : `“${name}” bloğunun öznitelikleri kaldırıldı.`);
  });
  cancel.addEventListener('click', () => dialog.close());
  refresh();
}
