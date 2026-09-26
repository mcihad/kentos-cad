import type { AppContext } from '../../app/context';
import type { DisposableStore } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { LINE_TYPE_LABEL, type LayerNode, type LineType } from '../../model/layers';
import { h } from '../dom';
import { colorSwatch, layerSwatch } from '../layers/swatch';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * Current-property fields: the active layer, colour, line type and weight
 * for new objects, and the project's plot scale. The classic toolbar and
 * the ribbon show the same fields; each keeps itself current.
 */

/** Drawing colours. `ink` is CAD colour 7: black, drawn white on the dark theme. */
export const DRAW_COLORS: { name: string; value: string }[] = [
  { name: 'Siyah', value: 'ink' },
  { name: 'Kırmızı', value: '#E5484D' },
  { name: 'Sarı', value: '#F2C94C' },
  { name: 'Yeşil', value: '#5FBF77' },
  { name: 'Camgöbeği', value: '#4CC3D9' },
  { name: 'Mavi', value: '#4F8EF7' },
  { name: 'Eflatun', value: '#C86DD7' },
  { name: 'Gri', value: '#8C9AAA' },
];

export const LINE_WEIGHTS = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7];
export const PLOT_SCALES = [500, 1000, 2000, 5000, 25000];

export interface FieldOptions {
  /** Width in CSS px at the standard type scale. */
  width?: number;
  /** Leading label inside the field ("Renk"); false leaves it out. */
  label?: string | false;
}

export function layerField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const { doc } = ctx;
  const layers = doc.layers;
  const dd = new Dropdown({
    ariaLabel: 'Etkin katman',
    className: 'dropdown--layer',
    width: opts.width ?? 196,
    items: () => {
      const counts = doc.countByLayer();
      const out: MenuItem[] = [];
      const walk = (nodes: readonly LayerNode[]) => {
        for (const n of nodes) {
          if (n.type === 'group') {
            out.push({ kind: 'header', label: layers.path(n.id) });
            walk(n.children);
          } else {
            out.push({
              label: n.name,
              swatch: layerSwatch(n, ctx.view.palette),
              radio: true,
              checked: layers.active.value === n.id,
              hint: String(counts.get(n.id) ?? 0),
              disabled: layers.isLocked(n.id),
              run: () => layers.setActive(n.id),
            });
          }
        }
      };
      walk(layers.tree);
      return out;
    },
  });
  const sync = () => {
    const n = layers.get(layers.active.value);
    if (n) dd.set(h('span', { class: 'swatch', style: `--swatch:${layerSwatch(n, ctx.view.palette)}` }), h('span', { class: 'dropdown__text' }, n.name));
  };
  d.add(watchAll([layers.active, layers.version, ctx.ui.theme], sync));
  sync();
  return dd.el;
}

/** The current colour for new objects: its name and swatch, or “Katmana göre”. */
export function currentColor(ctx: AppContext): { text: string; swatch?: string } {
  const c = DRAW_COLORS.find((x) => x.value === ctx.settings.color.value);
  return c ? { text: c.name, swatch: colorSwatch(c.value, ctx.view.palette) } : { text: 'Katmana göre' };
}

/** The colours new objects can take (a menu: the field's list, the folded toolbar's submenu). */
export function colorItems(ctx: AppContext): MenuItem[] {
  const s = ctx.settings.color;
  return [
    { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
    { kind: 'separator' },
    ...DRAW_COLORS.map((c): MenuItem => ({ label: c.name, swatch: colorSwatch(c.value, ctx.view.palette), radio: true, checked: s.value === c.value, run: () => s.set(c.value) })),
  ];
}

/** The current line type for new objects, or “Katmana göre”. */
export function currentLineType(ctx: AppContext): string {
  const v = ctx.settings.lineType.value;
  return v ? LINE_TYPE_LABEL[v] : 'Katmana göre';
}

export function lineTypeItems(ctx: AppContext): MenuItem[] {
  const s = ctx.settings.lineType;
  const types = Object.keys(LINE_TYPE_LABEL) as LineType[];
  return [
    { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
    { kind: 'separator' },
    ...types.map((t): MenuItem => ({ label: LINE_TYPE_LABEL[t], radio: true, checked: s.value === t, run: () => s.set(t) })),
  ];
}

const weightText = (w: number) => `${w.toFixed(2)} mm`;

/** The current line weight for new objects, or “Katmana göre”. */
export function currentWeight(ctx: AppContext): string {
  const v = ctx.settings.lineWeight.value;
  return v ? weightText(v) : 'Katmana göre';
}

export function weightItems(ctx: AppContext): MenuItem[] {
  const s = ctx.settings.lineWeight;
  return [
    { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
    { kind: 'separator' },
    ...LINE_WEIGHTS.map((w): MenuItem => ({ label: weightText(w), radio: true, checked: s.value === w, run: () => s.set(w) })),
  ];
}

export function colorField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const dd = new Dropdown({
    ariaLabel: 'Renk',
    label: opts.label === false ? undefined : (opts.label ?? 'Renk'),
    width: opts.width ?? 150,
    items: () => colorItems(ctx),
  });
  const sync = () => {
    const c = currentColor(ctx);
    dd.set(c.swatch ? h('span', { class: 'swatch', style: `--swatch:${c.swatch}` }) : null, h('span', { class: 'dropdown__text' }, c.text));
  };
  d.add(watchAll([ctx.settings.color, ctx.ui.theme], sync));
  sync();
  return dd.el;
}

export function lineTypeField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const dd = new Dropdown({
    ariaLabel: 'Çizgi tipi',
    label: opts.label === false ? undefined : (opts.label ?? 'Tip'),
    width: opts.width ?? 150,
    items: () => lineTypeItems(ctx),
  });
  d.add(ctx.settings.lineType.subscribe(() => dd.set(h('span', { class: 'dropdown__text' }, currentLineType(ctx))), true));
  return dd.el;
}

export function weightField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const dd = new Dropdown({
    ariaLabel: 'Çizgi kalınlığı',
    label: opts.label === false ? undefined : (opts.label ?? 'Kalınlık'),
    width: opts.width ?? 160,
    items: () => weightItems(ctx),
  });
  d.add(ctx.settings.lineWeight.subscribe(() => dd.set(h('span', { class: 'dropdown__text' }, currentWeight(ctx))), true));
  return dd.el;
}

/**
 * The colour, line type and weight fields folded into one (the classic
 * toolbar at its narrowest, DESIGN.md §7.3): each is a submenu showing its
 * current value, the same lists as the fields.
 */
export function propertiesField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const dd = new Dropdown({
    ariaLabel: 'Geçerli özellikler: renk, çizgi tipi, kalınlık',
    width: opts.width ?? 128,
    items: () => {
      const c = currentColor(ctx);
      return [
        { kind: 'header', label: 'Yeni nesnelerin özellikleri' },
        { label: 'Renk', swatch: c.swatch, hint: c.text, items: () => colorItems(ctx) },
        { label: 'Çizgi tipi', hint: currentLineType(ctx), items: () => lineTypeItems(ctx) },
        { label: 'Çizgi kalınlığı', hint: currentWeight(ctx), items: () => weightItems(ctx) },
      ];
    },
  });
  const sync = () => {
    const c = currentColor(ctx);
    dd.set(c.swatch ? h('span', { class: 'swatch', style: `--swatch:${c.swatch}` }) : null, h('span', { class: 'dropdown__text' }, 'Özellikler'));
  };
  d.add(watchAll([ctx.settings.color, ctx.ui.theme], sync));
  sync();
  return dd.el;
}

export function scaleField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const s = ctx.doc.settings.plotScale;
  const dd = new Dropdown({
    ariaLabel: 'Çizim ölçeği',
    label: opts.label === false ? undefined : (opts.label ?? 'Ölçek'),
    width: opts.width ?? 128,
    items: () => PLOT_SCALES.map((v): MenuItem => ({ label: `1:${v}`, radio: true, checked: s.value === v, run: () => s.set(v) })),
  });
  d.add(s.subscribe((v) => dd.set(h('span', { class: 'dropdown__text num' }, `1:${v}`)), true));
  return dd.el;
}
