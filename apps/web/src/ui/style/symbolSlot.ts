import type { AppContext } from '../../app/context';
import type { Symbol, SymbolRef, SymbolSet } from '../../model/style';
import type { GeometryClass } from '../../style/geometry';
import { newItemId } from '../../style/library';
import { h } from '../dom';
import { PopupMenu } from '../widgets/PopupMenu';
import { defaultFor } from './designerModel';
import { drawNow } from './thumbs';

/**
 * A symbol in a layer style: a small picture that opens a menu to pick a
 * library symbol, edit the symbol here (it then lives in the style), keep
 * it in the user's library, or go back to the layer's plain look. A class
 * without a symbol shows the plain look it falls back to, when given.
 */

const CLASS_LABEL: Record<GeometryClass, string> = { fill: 'Alan', line: 'Çizgi', marker: 'Nokta' };

/** The symbol a reference stands for (a library symbol or one written into the style). */
export function symbolOfRef(ctx: AppContext, ref: SymbolRef | undefined): Symbol | undefined {
  if (!ref) return undefined;
  return 'ref' in ref ? ctx.styles.library.symbol(ref.ref) : ref;
}

export function symbolSlot(ctx: AppContext, ref: SymbolRef | undefined, cls: GeometryClass, onChange: (ref: SymbolRef | undefined) => void, title: string, fallback?: SymbolRef): HTMLElement {
  const lib = ctx.styles.library;
  const sym = symbolOfRef(ctx, ref);
  const shown = ref ? sym : symbolOfRef(ctx, fallback);
  const canvas = h('canvas', { class: 'slot__pic', width: '64', height: '36', style: 'width:64px;height:36px' });
  const name = ref && 'ref' in ref ? (lib.get(ref.ref)?.name ?? 'Kitaplıkta yok') : ref ? 'Bu stilde' : 'Basit görünüş';
  const btn = h('button', { class: `slot${ref && 'ref' in ref && !sym ? ' slot--missing' : ''}`, type: 'button', title: `${CLASS_LABEL[cls]}: ${name}` }, canvas, h('span', { class: 'slot__name' }, name));
  queueMicrotask(() => {
    if (shown) drawNow(ctx, canvas, shown);
    else {
      const g = canvas.getContext('2d');
      g?.clearRect(0, 0, canvas.width, canvas.height);
    }
  });
  btn.addEventListener('click', () => {
    const r = btn.getBoundingClientRect();
    const editHere = (s: Symbol) =>
      void import('./SymbolDesigner').then((m) => m.openSymbolDesigner(ctx, { inline: { symbol: s, title, onDone: (edited) => onChange(edited) } }));
    PopupMenu.open(
      [
        {
          label: 'Kitaplıktan seç…',
          icon: 'styles',
          run: () =>
            void import('./StyleManager').then((m) =>
              m.openStyleManager(ctx, { pick: { kind: cls, title: `${title}: sembol seçin`, current: ref && 'ref' in ref ? ref.ref : undefined, onPick: (id) => onChange({ ref: id }) } }),
            ),
        },
        {
          label: ref && 'ref' in ref ? 'Kopyasını burada düzenle…' : 'Düzenle…',
          icon: 'edit',
          disabled: !sym && !!ref,
          // What the slot shows: its symbol, or the plain look it falls back to (a default only without one).
          run: () => editHere(structuredClone(sym ?? shown ?? defaultFor(cls))),
        },
        {
          label: 'Kitaplığıma kaydet',
          icon: 'save',
          disabled: !ref || 'ref' in ref,
          run: () => {
            if (!ref || 'ref' in ref) return;
            const id = newItemId('u');
            lib.add('user', { kind: 'symbol', id, name: title, path: ['Sembollerim'], symbol: ref });
            onChange({ ref: id });
            ctx.log.info(`“${title}” Kitaplığım'a kaydedildi.`);
          },
        },
        { kind: 'separator' },
        { label: 'Basit görünüşe dön', disabled: !ref, run: () => onChange(undefined) },
      ],
      { x: r.left, y: r.bottom + 4 },
    );
  });
  return btn;
}

/** Slots for each geometry class a set covers (or the layer has); `simple`: the look a class without a symbol falls back to. */
export function symbolSetSlots(ctx: AppContext, set: SymbolSet, classes: readonly GeometryClass[], onChange: (set: SymbolSet) => void, title: string, simple?: SymbolSet): HTMLElement {
  return h(
    'div',
    { class: 'slots' },
    classes.map((c) => symbolSlot(ctx, set[c], c, (ref) => onChange({ ...set, [c]: ref }), `${title} (${CLASS_LABEL[c].toLocaleLowerCase('tr')})`, simple?.[c])),
  );
}
