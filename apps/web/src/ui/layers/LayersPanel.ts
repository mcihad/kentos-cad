import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { LINE_TYPE_LABEL, type LayerNode, type LineType } from '../../model/layers';
import { h } from '../dom';
import { icon } from '../icons';
import { Panel } from '../dock/Panel';
import { DRAW_COLORS, LINE_WEIGHTS } from '../toolbar/fields';
import { commandButton } from '../widgets/CommandButton';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { TreeView } from '../widgets/TreeView';
import { colorSwatch, layerSwatch } from './swatch';

/** Layer tree: visibility, lock, colour, active layer, groups, filter. */
export class LayersPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly tree: TreeView<LayerNode>;
  private counts = new Map<string, number>();

  constructor(ctx: AppContext) {
    super({ title: 'Katmanlar', className: 'panel--layers', actions: [] });
    this.ctx = ctx;
    const layers = ctx.doc.layers;
    const actions = this.el.querySelector('.panel__actions')!;
    actions.append(
      commandButton(ctx, 'layer.new', this.d, { className: 'ibtn', size: 16 }),
      commandButton(ctx, 'layer.newGroup', this.d, { className: 'ibtn', size: 16 }),
    );

    const filter = h('input', { class: 'field field--search', type: 'search', placeholder: 'Katman ara', 'aria-label': 'Katman ara', spellcheck: 'false' });
    this.tree = new TreeView<LayerNode>(
      {
        id: (n) => n.id,
        children: (n) => n.children,
        isExpanded: (n) => n.expanded,
        setExpanded: (n, v) => layers.setExpanded(n.id, v),
        matches: (n, q) => n.name.toLocaleLowerCase('tr-TR').includes(q),
        renderRow: (n, row) => this.renderRow(n, row),
        onActivate: (n) => (n.type === 'layer' ? layers.setActive(n.id) : layers.setExpanded(n.id, !n.expanded)),
        onToggle: (n) => layers.toggleVisible(n.id),
        onRename: (n) => this.rename(n),
        onContextMenu: (n, e) => PopupMenu.open(this.menuFor(n), { x: e.clientX, y: e.clientY }),
      },
      'Katman ağacı',
    );

    this.body.append(h('div', { class: 'panel__toolbar' }, h('span', { class: 'field-icon' }, icon('search', 14)), filter), this.tree.el);
    this.d.add(listen(filter, 'input', () => this.tree.setFilter(filter.value)));

    const refresh = () => {
      this.counts = ctx.doc.countByLayer();
      this.tree.render(layers.tree);
      this.setMeta(`${layers.leaves().length} katman`);
    };
    this.d.add(watchAll([layers.version, layers.active, ctx.ui.theme], refresh));
    this.d.add(ctx.doc.events.on('changed', refresh));
    refresh();
  }

  private renderRow(n: LayerNode, row: HTMLElement): void {
    const layers = this.ctx.doc.layers;
    const isLayer = n.type === 'layer';
    const active = layers.active.value === n.id;
    const hidden = !layers.isVisible(n.id);
    const count = isLayer ? (this.counts.get(n.id) ?? 0) : layers.leavesOf(n.id).reduce((s, l) => s + (this.counts.get(l.id) ?? 0), 0);
    row.parentElement?.toggleAttribute('data-active', active);
    row.parentElement?.toggleAttribute('data-hidden', hidden);
    row.parentElement?.toggleAttribute('data-group', !isLayer);

    const eye = h('button', { class: 'ibtn ibtn--row', type: 'button', 'aria-label': n.visible ? 'Gizle' : 'Göster', 'aria-pressed': String(!n.visible) }, icon(n.visible ? 'eye' : 'eyeOff', 15));
    const lock = h(
      'button',
      { class: 'ibtn ibtn--row', type: 'button', 'aria-label': n.locked ? 'Kilidi aç' : 'Kilitle', 'aria-pressed': String(n.locked), 'data-on': n.locked ? '' : null },
      icon(n.locked ? 'lock' : 'unlock', 15),
    );
    eye.addEventListener('click', (e) => {
      e.stopPropagation();
      layers.toggleVisible(n.id);
    });
    lock.addEventListener('click', (e) => {
      e.stopPropagation();
      layers.toggleLocked(n.id);
    });

    let lead: HTMLElement;
    if (isLayer) {
      lead = h('button', { class: 'swatch swatch--btn', type: 'button', style: `--swatch:${layerSwatch(n, this.ctx.view.palette)}`, 'aria-label': 'Katman rengi' });
      lead.addEventListener('click', (e) => {
        e.stopPropagation();
        PopupMenu.open(this.colorItems(n), lead.getBoundingClientRect(), { minWidth: 180 });
      });
    } else lead = h('span', { class: 'tree__folder' }, icon('folder', 15));

    row.append(
      lead,
      h('span', { class: 'tree__name', title: layers.path(n.id) }, n.name),
      h('span', { class: 'tree__count num' }, String(count)),
      eye,
      lock,
    );
  }

  private colorItems(n: LayerNode): MenuItem[] {
    return [
      { label: 'Ana mürekkep', swatch: this.ctx.view.palette.fg, radio: true, checked: n.style.color === 'fg', run: () => this.ctx.doc.setLayerStyle(n.id, { color: 'fg' }, 'Katman rengi') },
      { label: 'İkincil mürekkep', swatch: this.ctx.view.palette.fgDim, radio: true, checked: n.style.color === 'fg-dim', run: () => this.ctx.doc.setLayerStyle(n.id, { color: 'fg-dim' }, 'Katman rengi') },
      { kind: 'separator' },
      ...DRAW_COLORS.map((c): MenuItem => ({ label: c.name, swatch: colorSwatch(c.value, this.ctx.view.palette), radio: true, checked: n.style.color === c.value, run: () => this.ctx.doc.setLayerStyle(n.id, { color: c.value }, 'Katman rengi') })),
    ];
  }

  private menuFor(n: LayerNode): MenuItem[] {
    const layers = this.ctx.doc.layers;
    const isLayer = n.type === 'layer';
    const items: MenuItem[] = [];
    if (isLayer) items.push({ label: 'Etkin katman yap', icon: 'check', disabled: layers.active.value === n.id, run: () => layers.setActive(n.id) });
    items.push(
      { label: n.visible ? 'Gizle' : 'Göster', icon: n.visible ? 'eyeOff' : 'eye', shortcut: 'Space', run: () => layers.toggleVisible(n.id) },
      { label: n.locked ? 'Kilidi aç' : 'Kilitle', icon: n.locked ? 'unlock' : 'lock', run: () => layers.toggleLocked(n.id) },
      { label: 'Yalnızca bunu göster', run: () => layers.isolate(n.id) },
      { label: 'Tüm katmanları göster', run: () => layers.showAll() },
      { kind: 'separator' },
      {
        label: 'Nesnelerini seç',
        run: () => {
          const ids = layers.leavesOf(n.id).flatMap((l) => this.ctx.doc.byLayer(l.id).map((e) => e.id));
          this.ctx.selection.set(ids);
          this.ctx.log.info(`${n.name}: ${ids.length} nesne seçildi.`);
        },
      },
      { kind: 'separator' },
    );
    if (isLayer) {
      items.push(
        { label: 'Renk', items: () => this.colorItems(n) },
        {
          label: 'Çizgi tipi',
          items: () =>
            (Object.keys(LINE_TYPE_LABEL) as LineType[]).map((t) => ({ label: LINE_TYPE_LABEL[t], radio: true, checked: n.style.lineType === t, run: () => this.ctx.doc.setLayerStyle(n.id, { lineType: t }, 'Çizgi tipi') })),
        },
        {
          label: 'Kalınlık',
          items: () => LINE_WEIGHTS.map((w) => ({ label: `${w.toFixed(2)} mm`, radio: true, checked: n.style.lineWeight === w, run: () => this.ctx.doc.setLayerStyle(n.id, { lineWeight: w }, 'Çizgi kalınlığı') })),
        },
        {
          label: n.style.renderer ? 'Katman stili… (özel)' : 'Katman stili…',
          icon: 'layerStyle',
          run: () => void import('../style/LayerStyleDialog').then((m) => m.openLayerStyle(this.ctx, n.id)),
        },
        { kind: 'separator' },
      );
    }
    items.push(
      { label: 'Yeniden adlandır', shortcut: 'F2', run: () => this.rename(n) },
      { label: isLayer ? 'Yanına yeni katman' : 'İçine yeni katman', icon: 'layerAdd', run: () => layers.setActive(layers.add({ name: layers.uniqueName('Yeni katman') }, n.id).id) },
    );
    return items;
  }

  private rename(n: LayerNode): void {
    const row = this.tree.rowOf(n.id);
    const nameEl = row?.querySelector<HTMLElement>('.tree__name');
    if (!nameEl) return;
    const input = h('input', { class: 'field field--inline', value: n.name, 'aria-label': 'Katman adı', spellcheck: 'false' });
    nameEl.replaceWith(input);
    input.focus();
    input.select();
    let done = false;
    const finish = (commit: boolean) => {
      if (done) return;
      done = true;
      if (commit && input.value.trim() && input.value !== n.name) this.ctx.doc.layers.rename(n.id, input.value);
      else this.tree.render(this.ctx.doc.layers.tree);
      this.tree.el.focus();
    };
    input.addEventListener('keydown', (e) => {
      e.stopPropagation();
      if (e.key === 'Enter') finish(true);
      if (e.key === 'Escape') finish(false);
    });
    input.addEventListener('blur', () => finish(true));
    input.addEventListener('click', (e) => e.stopPropagation());
  }
}
