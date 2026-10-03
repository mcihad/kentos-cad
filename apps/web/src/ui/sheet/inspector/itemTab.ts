import type { Bindable } from '../../../contracts/generated/sheet/Bindable';
import type { Op } from '../../../contracts/generated/sheet/Op';
import { kindName } from '../../../product/sheet/profile';
import { h, type Child } from '../../dom';
import { icon } from '../../icons';
import { note } from '../../widgets/controls';
import { tooltip } from '../../widgets/tooltip';
import { openExpressionDialog } from '../ExpressionDialog';
import { bindablesFor, bindingsOf, common, frameValues, notesOf, sameGroup, summaryOf } from '../inspectorPlan';
import { anchorsSentence, commonAnchors } from '../widgets/anchors';
import { constraintEditor } from '../widgets/ConstraintEditor';
import { numberField } from '../widgets/fields';
import { choice } from '../widgets/form';
import { dataSection } from './dataSections';
import { kindSection } from './kindSections';
import { mapSection, type MapHooks } from './mapSection';
import { color, flag, mmField, patchEach, type SectionCtx } from './parts';

/**
 * Denetçi's Öğe tab (docs/sheet/design.md §11): the chosen items' summary,
 * the engine's note for another mode's item (§11a), Konum ve boyut,
 * Kısıtlar, the kind's own section, Görünüş and Veri (ƒ). A field bound to
 * an expression is read-only and carries a pressed ƒ; ƒ opens the
 * expression's window. Every change is one undo step through the engine.
 */

export interface ItemTabHooks extends MapHooks {
  section(id: string, title: string, body: Child[]): HTMLElement;
  /** Asks for a picture and puts it in the chosen picture frame. */
  choosePicture(): void;
}

export function itemTab(c: SectionCtx, hooks: ItemTabHooks): Child[] {
  const { host, items } = c;
  const profile = host.profile.value;
  const s = summaryOf(items, profile);
  const ids = items.map((i) => i.id);
  const bindables = host.engine()?.bindableProperties() ?? [];
  const out: Child[] = [h('div', { class: 'sheet-insp__summary' }, icon(s.icon, 20), h('div', null, h('span', { class: 'sheet-insp__kind' }, s.title), h('span', { class: 'sheet-insp__sub' }, s.sub)))];
  if (items.some((i) => i.locked)) out.push(h('div', { class: 'sheet-insp__note' }, note('info', h('strong', null, 'Kilitli.'), ' Değiştirmek için Öğeler listesinde kilidini açın.')));
  for (const n of notesOf(items)) out.push(h('div', { class: 'sheet-insp__note' }, note('info', n)));

  // ƒ on a field: one item's expression window; several items are bound one by one.
  const bound = (path: string) => (items.length === 1 ? (items[0].bindings[path] ?? null) : null);
  const fx = (path: string) => () => {
    const b = bindables.find((x) => x.property === path);
    if (items.length !== 1 || !b) return void c.ctx.log.warn('Veri bağı tek öğe seçiliyken kurulur: Öğeler listesinden bir öğe seçin.');
    openExpressionDialog(host, c.sheet.id, items[0].source, b);
  };
  const groups = items.some((i) => i.kind === 'group');
  const f = frameValues(items);
  const move = (axis: 'left' | 'top', mmValue: number) => {
    const ops: Op[] = items.map((i) => ({ op: 'moveItems', ids: [i.id], delta: axis === 'left' ? [Math.round(mmValue * 1000) - i.source.frame.left, 0] : [0, Math.round(mmValue * 1000) - i.source.frame.top] }));
    host.apply(ops, 'Konum');
  };
  const size = (axis: 'width' | 'height', mmValue: number) =>
    host.apply([{ op: 'setFrames', frames: items.map((i) => ({ id: i.id, frame: { ...i.source.frame, [axis]: Math.max(1000, Math.round(mmValue * 1000)) }, rotation: i.source.rotation })) }], 'Boyut');
  const turn = (deg: number) => host.apply([{ op: 'setFrames', frames: items.map((i) => ({ id: i.id, frame: i.source.frame, rotation: Math.round((((deg % 360) + 360) % 360) * 1000) })) }], 'Dönüş');
  const num = (label: string, key: string, value: number | null, unit: string, take: (v: number) => void, o: { min?: number; max?: number; why?: string | null } = {}) =>
    numberField({ label, key, value, unit, decimals: 1, readOnly: c.readOnly ?? o.why ?? null, bound: bound(key), onCommit: take, onBind: fx(key), min: o.min, max: o.max }, c.d);
  const groupWhy = groups ? 'Grubun boyu çocuklarından gelir: çocukları boyutlandırın.' : null;
  out.push(
    hooks.section('frame', 'Konum ve boyut', [
      h(
        'div',
        { class: 'sheet-fields' },
        num('Sol', 'frame.left', f.left, 'mm', (v) => move('left', v)),
        num('Üst', 'frame.top', f.top, 'mm', (v) => move('top', v)),
        num('Genişlik', 'frame.width', f.width, 'mm', (v) => size('width', v), { min: 1, why: groupWhy }),
        num('Yükseklik', 'frame.height', f.height, 'mm', (v) => size('height', v), { min: 1, why: groupWhy }),
        num('Dönüş', 'rotation', f.rotation, '°', turn, { min: -360, max: 360, why: groupWhy }),
      ),
      h('p', { class: 'sheet-insp__hint' }, 'Konum kâğıdın sol üst köşesinden, aşağı doğru ölçülür. ↑/↓ 1 mm, Shift 10 mm, Alt 0.1 mm.'),
    ]),
  );

  const anchors = items.map((i) => i.anchors);
  const ca = commonAnchors(anchors);
  out.push(
    hooks.section('anchors', 'Kısıtlar', [
      constraintEditor(
        {
          anchors,
          inGroup: sameGroup(items),
          readOnly: c.readOnly,
          onChange: (p) => patchEach(c, 'Kısıtlar', () => ({ constraints: { ...(p.h ? { h: p.h } : {}), ...(p.v ? { v: p.v } : {}), ...(p.box ? { relativeTo: p.box } : {}) } })),
        },
        c.d,
      ),
      h('p', { class: 'sheet-insp__hint' }, anchorsSentence(ca.h, ca.v)),
    ]),
  );

  const kind = common(items, (i) => i.kind);
  if (kind && kind !== 'group') {
    const body = kind === 'map' ? mapSection(c, hooks) : (kindSection(c, kind, hooks.choosePicture) ?? dataSection(c, kind, hooks.layers));
    if (body) out.push(hooks.section(`kind:${kind}`, kindName(profile, kind), body));
  }

  const open: SectionCtx = { ...c, readOnly: host.whyReadOnly() };
  const fill = common(items, (i) => i.source.fill ?? null);
  const border = common(items, (i) => i.source.border ?? null, (a, b) => JSON.stringify(a) === JSON.stringify(b));
  out.push(
    hooks.section('look', 'Görünüş', [
      h(
        'div',
        { class: 'sheet-fields' },
        numberField({ label: 'Saydamlık', key: 'opacity', value: common(items, (i) => i.opacity), unit: '%', decimals: 0, min: 0, max: 100, readOnly: c.readOnly, bound: bound('opacity'), onBind: fx('opacity'), onCommit: (v) => patchEach(c, 'Saydamlık', () => ({ opacity: Math.round(v) })) }, c.d),
        mmField(c, 'İç boşluk', 'padding', common(items, (i) => i.source.padding), (padding) => patchEach(c, 'İç boşluk', () => ({ padding })), { min: 0 }),
      ),
      flag(c, 'Zemin', common(items, (i) => !!i.source.fill), (on) => patchEach(c, 'Zemin', () => ({ fill: on ? (fill ?? '#ffffff') : null })), 'Çerçevenin içi bir renkle dolar.'),
      fill ? color(c, 'Zemin rengi', 'fill', fill, (v) => patchEach(c, 'Zemin rengi', () => ({ fill: v }))) : null,
      flag(c, 'Çerçeve çizgisi', common(items, (i) => !!i.source.border), (on) => patchEach(c, 'Çerçeve çizgisi', () => ({ border: on ? (border ?? { color: '#000000', width: 250, dash: [], cap: 'butt', join: 'miter' }) : null }))),
      border ? h('div', { class: 'sheet-fields' }, color(c, 'Çizgi rengi', 'border.color', border.color, (v) => patchEach(c, 'Çerçeve çizgisi', () => ({ border: { color: v } }))), mmField(c, 'Çizgi kalınlığı', 'border.width', border.width, (v) => patchEach(c, 'Çerçeve çizgisi', () => ({ border: { width: Math.max(1, v) } })), { decimals: 2, min: 0.01 })) : null,
      flag(c, 'Yazdırılır', common(items, (i) => i.printable), (v) => patchEach(c, v ? 'Yazdırılır' : 'Yazdırılmaz', () => ({ printable: v })), 'Dışa aktarılan paftada çizilir. Kapalıysa yalnız ekranda görünür.'),
      // Hiding and locking stay open on a locked item: that is how it is unlocked.
      flag(open, 'Gizli', common(items, (i) => i.hidden), (v) => host.apply([{ op: 'hide', ids, value: v }], v ? 'Gizle' : 'Göster'), 'Paftada çizilmez; Öğeler listesinde kalır.'),
      flag(open, 'Kilitli', common(items, (i) => i.locked), (v) => host.apply([{ op: 'lock', ids, value: v }], v ? 'Kilitle' : 'Kilidi aç'), 'Paftada seçilmez ve taşınmaz; denetçide salt okunur.'),
    ]),
  );
  out.push(hooks.section('data', 'Veri', [dataBody(c, bindables)]));
  return out;
}

/** The bound properties of one item, each with its ƒ; a property is bound from the list under them. */
function dataBody(c: SectionCtx, bindables: readonly Bindable[]): HTMLElement {
  const { items } = c;
  if (items.length !== 1) return h('p', { class: 'sheet-insp__hint' }, 'Veri bağları tek öğe seçiliyken listelenir.');
  const item = items[0];
  const list = bindingsOf(item, bindables);
  const open = (b: Bindable) => openExpressionDialog(c.host, c.sheet.id, item.source, b);
  const free = bindablesFor(items, bindables).filter((b) => !item.bindings[b.property]);
  const add = choice<string>(
    {
      label: 'Bağ ekle',
      key: 'bind.add',
      value: null,
      readOnly: c.readOnly,
      options: free.map((b) => ({ value: b.property, label: b.label, detail: b.unit || undefined })),
      onChange: (p) => {
        const b = bindables.find((x) => x.property === p);
        if (b) open(b);
      },
    },
    c.d,
  );
  return h(
    'div',
    { class: 'sheet-binds' },
    list.length
      ? list.map((b) => {
          const edit = h('button', { class: 'sheet-fx', type: 'button', 'aria-pressed': 'true', 'aria-label': `${b.label}: ifadeyi düzenle`, disabled: c.readOnly !== null }, 'ƒ');
          const bindable = bindables.find((x) => x.property === b.path);
          edit.addEventListener('click', () => bindable && open(bindable));
          c.d.add(tooltip(edit, () => ({ title: `${b.label}: ifade`, description: b.expression })));
          return h('div', { class: 'sheet-bind' }, h('span', { class: 'sheet-bind__prop' }, b.label), edit, h('span', { class: 'sheet-bind__expr', title: b.expression }, b.expression));
        })
      : h('p', { class: 'sheet-insp__hint' }, 'Bu öğenin veriye bağlı özelliği yok. Bir alanın yanındaki ƒ ile değerini bir ifadeden alın: atlas nesnesinin alanları, @pafta_adi, @olcek, @tarih …'),
    free.length ? add : null,
  );
}
