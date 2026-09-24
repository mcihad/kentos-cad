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

export function colorField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const s = ctx.settings.color;
  const dd = new Dropdown({
    ariaLabel: 'Renk',
    label: opts.label === false ? undefined : (opts.label ?? 'Renk'),
    width: opts.width ?? 150,
    items: () => [
      { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
      { kind: 'separator' },
      ...DRAW_COLORS.map((c): MenuItem => ({ label: c.name, swatch: colorSwatch(c.value, ctx.view.palette), radio: true, checked: s.value === c.value, run: () => s.set(c.value) })),
    ],
  });
  const sync = () => {
    const c = DRAW_COLORS.find((x) => x.value === s.value);
    dd.set(c ? h('span', { class: 'swatch', style: `--swatch:${colorSwatch(c.value, ctx.view.palette)}` }) : null, h('span', { class: 'dropdown__text' }, c?.name ?? 'Katmana göre'));
  };
  d.add(watchAll([s, ctx.ui.theme], sync));
  sync();
  return dd.el;
}

export function lineTypeField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const s = ctx.settings.lineType;
  const types = Object.keys(LINE_TYPE_LABEL) as LineType[];
  const dd = new Dropdown({
    ariaLabel: 'Çizgi tipi',
    label: opts.label === false ? undefined : (opts.label ?? 'Tip'),
    width: opts.width ?? 150,
    items: () => [
      { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
      { kind: 'separator' },
      ...types.map((t): MenuItem => ({ label: LINE_TYPE_LABEL[t], radio: true, checked: s.value === t, run: () => s.set(t) })),
    ],
  });
  d.add(s.subscribe((v) => dd.set(h('span', { class: 'dropdown__text' }, v ? LINE_TYPE_LABEL[v] : 'Katmana göre')), true));
  return dd.el;
}

export function weightField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const s = ctx.settings.lineWeight;
  const fmt = (w: number) => `${w.toFixed(2)} mm`;
  const dd = new Dropdown({
    ariaLabel: 'Çizgi kalınlığı',
    label: opts.label === false ? undefined : (opts.label ?? 'Kalınlık'),
    width: opts.width ?? 160,
    items: () => [
      { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
      { kind: 'separator' },
      ...LINE_WEIGHTS.map((w): MenuItem => ({ label: fmt(w), radio: true, checked: s.value === w, run: () => s.set(w) })),
    ],
  });
  d.add(s.subscribe((v) => dd.set(h('span', { class: 'dropdown__text' }, v ? fmt(v) : 'Katmana göre')), true));
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
