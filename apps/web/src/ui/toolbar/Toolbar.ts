import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import { LINE_TYPE_LABEL, type LayerNode, type LineType } from '../../model/layers';
import { Component } from '../Component';
import { h } from '../dom';
import { commandButton } from '../widgets/CommandButton';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { colorSwatch, layerSwatch } from '../layers/swatch';

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

/** Toolbar = groups of command buttons + current-property fields. */
export class Toolbar extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const btn = (id: string) => commandButton(ctx, id, this.d);
    const group = (label: string, ...children: HTMLElement[]) => h('div', { class: 'toolbar__group', role: 'group', 'aria-label': label }, ...children);

    this.el = h(
      'div',
      { class: 'toolbar', role: 'toolbar', 'aria-label': 'Araç çubuğu' },
      group('Dosya', btn('file.new'), btn('file.open'), btn('file.save')),
      group('Geçmiş', btn('edit.undo'), btn('edit.redo')),
      group('Görünüm', btn('view.zoomExtents'), btn('tool.zoomWindow'), btn('view.zoomSelection'), btn('view.zoomIn'), btn('view.zoomOut'), btn('tool.pan')),
      group('Geçerli özellikler', this.layerField(), this.colorField(), this.lineTypeField(), this.weightField()),
      h('div', { class: 'toolbar__spacer' }),
      group('Çizim ölçeği', this.scaleField()),
      group('Paneller', btn('view.toolbox'), btn('view.bottomPanel'), btn('view.rightPanel')),
    );
  }

  private layerField(): HTMLElement {
    const { doc } = this.ctx;
    const layers = doc.layers;
    const dd = new Dropdown({
      ariaLabel: 'Etkin katman',
      className: 'dropdown--layer',
      width: 196,
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
                swatch: layerSwatch(n, this.ctx.view.palette),
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
      if (n) dd.set(h('span', { class: 'swatch', style: `--swatch:${layerSwatch(n, this.ctx.view.palette)}` }), h('span', { class: 'dropdown__text' }, n.name));
    };
    this.d.add(watchAll([layers.active, layers.version, this.ctx.ui.theme], sync));
    sync();
    return dd.el;
  }

  private colorField(): HTMLElement {
    const s = this.ctx.settings.color;
    const dd = new Dropdown({
      ariaLabel: 'Renk',
      label: 'Renk',
      width: 150,
      items: () => [
        { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
        { kind: 'separator' },
        ...DRAW_COLORS.map((c): MenuItem => ({ label: c.name, swatch: colorSwatch(c.value, this.ctx.view.palette), radio: true, checked: s.value === c.value, run: () => s.set(c.value) })),
      ],
    });
    const sync = () => {
      const c = DRAW_COLORS.find((x) => x.value === s.value);
      dd.set(c ? h('span', { class: 'swatch', style: `--swatch:${colorSwatch(c.value, this.ctx.view.palette)}` }) : null, h('span', { class: 'dropdown__text' }, c?.name ?? 'Katmana göre'));
    };
    this.d.add(s.subscribe(sync, true));
    return dd.el;
  }

  private lineTypeField(): HTMLElement {
    const s = this.ctx.settings.lineType;
    const types = Object.keys(LINE_TYPE_LABEL) as LineType[];
    const dd = new Dropdown({
      ariaLabel: 'Çizgi tipi',
      label: 'Tip',
      width: 150,
      items: () => [
        { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
        { kind: 'separator' },
        ...types.map((t): MenuItem => ({ label: LINE_TYPE_LABEL[t], radio: true, checked: s.value === t, run: () => s.set(t) })),
      ],
    });
    this.d.add(s.subscribe((v) => dd.set(h('span', { class: 'dropdown__text' }, v ? LINE_TYPE_LABEL[v] : 'Katmana göre')), true));
    return dd.el;
  }

  private weightField(): HTMLElement {
    const s = this.ctx.settings.lineWeight;
    const fmt = (w: number) => `${w.toFixed(2)} mm`;
    const dd = new Dropdown({
      ariaLabel: 'Çizgi kalınlığı',
      label: 'Kalınlık',
      width: 160,
      items: () => [
        { label: 'Katmana göre', radio: true, checked: s.value === null, run: () => s.set(null) },
        { kind: 'separator' },
        ...LINE_WEIGHTS.map((w): MenuItem => ({ label: fmt(w), radio: true, checked: s.value === w, run: () => s.set(w) })),
      ],
    });
    this.d.add(s.subscribe((v) => dd.set(h('span', { class: 'dropdown__text' }, v ? fmt(v) : 'Katmana göre')), true));
    return dd.el;
  }

  private scaleField(): HTMLElement {
    const s = this.ctx.doc.settings.plotScale;
    const dd = new Dropdown({
      ariaLabel: 'Çizim ölçeği',
      label: 'Ölçek',
      width: 128,
      items: () => PLOT_SCALES.map((v): MenuItem => ({ label: `1:${v}`, radio: true, checked: s.value === v, run: () => s.set(v) })),
    });
    this.d.add(s.subscribe((v) => dd.set(h('span', { class: 'dropdown__text num' }, `1:${v}`)), true));
    return dd.el;
  }
}
