import type { AppContext } from '../../app/context';
import { scaleText, setPlotScale } from '../../app/annotationScale';
import { projectScales } from '../../model/newProjectWizard';
import { openPlotScaleDialog } from '../settings/PlotScaleDialog';
import type { DisposableStore } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { LINE_TYPE_LABEL, type LayerNode, type LineType } from '../../model/layers';
import { TEMPLATE_TOOL_LABEL } from '../../model/objectTemplate';
import type { LibraryTemplate, Sourced } from '../../model/style';
import { listTemplates } from '../../style/templateList';
import { h } from '../dom';
import { icon } from '../icons';
import { colorSwatch, layerSwatch } from '../layers/swatch';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { fixed } from '../../core/displayNumber';

/**
 * Current-property fields: the active layer, colour, line type and weight
 * for new objects, the project's plot scale and the object template drawn
 * with. The ribbon's Katmanlar, Şablonlar and Özellikler panels show them;
 * each keeps itself current. The lists are the panels' too (Katmanlar,
 * Şablonlar, Öznitelikler, the project settings).
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

/** The colours a dimension's lines take (docs/adr/0205 §6): the drawing colours but ink, which is the object's own. */
export const LINE_COLORS = DRAW_COLORS.filter((c) => c.value !== 'ink');

export const LINE_WEIGHTS = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7];

export interface FieldOptions {
  /** Width in CSS px at the standard type scale. */
  width?: number;
  /** Leading label inside the field ("Renk"); false leaves it out. */
  label?: string | false;
}

/** The current-property fields' icons, shown in their labels' place in a narrow ribbon (`dropdown--glyph`). */
const FIELD_GLYPHS = { color: 'color', lineType: 'lineType', weight: 'lineWeight', scale: 'plotScale' } as const;

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
  d.add(watchAll([layers.active, layers.version, ctx.prefs.theme], sync));
  sync();
  return dd.el;
}

/** The current colour for new objects: its name and swatch, or “Katmana göre”. */
export function currentColor(ctx: AppContext): { text: string; swatch?: string } {
  const value = ctx.settings.color.value;
  if (value === null) return { text: 'Katmana göre' };
  // An object template's colour that is none of the drawing colours (docs/adr/0176 §3): by itself.
  const c = DRAW_COLORS.find((x) => x.value === value);
  return { text: c ? c.name : value.toUpperCase(), swatch: colorSwatch(value, ctx.view.palette) };
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

/** A line weight as the lists write it: “0.25 mm”. */
export const weightText = (w: number) => `${fixed(w, 2)} mm`;

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
    glyph: FIELD_GLYPHS.color,
    label: opts.label === false ? undefined : (opts.label ?? 'Renk'),
    width: opts.width ?? 150,
    items: () => colorItems(ctx),
  });
  const sync = () => {
    const c = currentColor(ctx);
    dd.set(c.swatch ? h('span', { class: 'swatch', style: `--swatch:${c.swatch}` }) : null, h('span', { class: 'dropdown__text' }, c.text));
  };
  d.add(watchAll([ctx.settings.color, ctx.prefs.theme], sync));
  sync();
  return dd.el;
}

export function lineTypeField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const dd = new Dropdown({
    ariaLabel: 'Çizgi tipi',
    glyph: FIELD_GLYPHS.lineType,
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
    glyph: FIELD_GLYPHS.weight,
    label: opts.label === false ? undefined : (opts.label ?? 'Kalınlık'),
    width: opts.width ?? 160,
    items: () => weightItems(ctx),
  });
  d.add(ctx.settings.lineWeight.subscribe(() => dd.set(h('span', { class: 'dropdown__text' }, currentWeight(ctx))), true));
  return dd.el;
}

/**
 * Ölçek (docs/adr/0205 §4): the project's type's scales and the current one when it is none of them, then Ölçek yaz…;
 * a choice sets the plot scale and the annotations at the old general height follow it in one step (`setPlotScale`).
 */
export function scaleField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const s = ctx.doc.settings.plotScale;
  const dd = new Dropdown({
    ariaLabel: 'Çizim ölçeği',
    glyph: FIELD_GLYPHS.scale,
    label: opts.label === false ? undefined : (opts.label ?? 'Ölçek'),
    width: opts.width ?? 128,
    items: (): MenuItem[] => [
      ...projectScales(ctx.doc.settings.workspace.value === 'cad', s.value).map((v): MenuItem => ({ label: scaleText(v), radio: true, checked: s.value === v, run: () => void setPlotScale(ctx, v) })),
      { kind: 'separator' },
      { label: 'Ölçek yaz…', run: () => openPlotScaleDialog(ctx) },
    ],
  });
  d.add(s.subscribe((v) => dd.set(h('span', { class: 'dropdown__text num' }, scaleText(v))), true));
  return dd.el;
}

/**
 * Şablonlar's field (docs/adr/0176 §4): the template being drawn with, or “Şablonla çiz”; its menu lists the templates
 * drawn with last, then every template under its category (style/templateList.ts), each with its tool's icon; a
 * choice draws with it. The desktop's is `ribbon_panels.rs`'s `templates_group`.
 */
export function templateField(ctx: AppContext, d: DisposableStore, opts: FieldOptions = {}): HTMLElement {
  const lib = ctx.styles.library;
  const item = (t: Sourced<LibraryTemplate>): MenuItem => ({
    label: t.name,
    icon: ctx.tools.get(t.template.tool)?.icon ?? 'templates',
    hint: TEMPLATE_TOOL_LABEL[t.template.tool],
    radio: true,
    checked: ctx.settings.template.value?.id === t.id,
    run: () => ctx.commands.execute('template.draw', t.id),
  });
  const dd = new Dropdown({
    ariaLabel: 'Şablonla çiz',
    className: 'dropdown--template',
    width: opts.width ?? 196,
    items: () => {
      const out: MenuItem[] = [];
      const recent = ctx.tools.recentTemplates.value.map((id) => lib.template(id)).filter((t): t is Sourced<LibraryTemplate> => !!t);
      if (recent.length) out.push({ kind: 'header', label: 'Son kullanılanlar' }, ...recent.map(item));
      for (const g of listTemplates(lib)) out.push({ kind: 'header', label: g.path.length ? g.path.join(' / ') : 'Kategorisiz' }, ...g.items.map(item));
      if (!out.length) out.push({ label: 'Kitaplıkta nesne şablonu yok', disabled: true, run: () => {} });
      return out;
    },
  });
  const sync = () => {
    const run = ctx.settings.template.value;
    dd.set(icon(run ? 'templateDraw' : 'templates', 14), h('span', { class: 'dropdown__text' }, run ? run.name : 'Şablonla çiz'));
  };
  d.add(ctx.settings.template.subscribe(sync, true));
  return dd.el;
}
