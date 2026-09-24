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

/**
 * A row on screen: the cells that edits and layer state change, and what
 * they show. A value is written only when it differs, so an update leaves
 * the rows it does not change untouched.
 */
interface Row {
  readonly node: LayerNode;
  /** The tree item: carries data-active and data-hidden. */
  readonly item: HTMLElement;
  readonly name: HTMLElement;
  readonly count: HTMLElement;
  readonly eye: HTMLElement;
  readonly lock: HTMLElement;
  /** A layer's colour swatch; a group shows a folder. */
  readonly swatch: HTMLElement | null;
  readonly shown: { count?: number; visible?: boolean; locked?: boolean; hidden?: boolean; active?: boolean; color?: string };
}

/**
 * Layer tree: visibility, lock, colour, active layer, groups, filter.
 *
 * The tree is rendered again only when its shape changes (layers added,
 * renamed or replaced, a group opened or closed), once per task. An edit
 * changes object counts and layer state a few cells (visibility, lock,
 * colour, the active layer): both are written into the rows in place, so an
 * edit costs the same beside 3 layers or 300 and a row stays the same
 * element across edits, undo and redo (hover, focus and an open rename
 * field survive them).
 */
export class LayersPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly tree: TreeView<LayerNode>;
  /** The rows on the page by node id (the tree builds those in its scroll window). */
  private readonly rows = new Map<string, Row>();
  /** Object count of every node; a group's is the sum over all its layers. */
  private totals = new Map<string, number>();
  private rebuildQueued = false;

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
        // Only the rows in view are built; one scrolled away takes no more writes.
        releaseRow: (n) => this.rows.delete(n.id),
        onActivate: (n) => (n.type === 'layer' ? layers.setActive(n.id) : layers.setExpanded(n.id, !n.expanded)),
        onToggle: (n) => layers.toggleVisible(n.id),
        onRename: (n) => this.rename(n),
        onContextMenu: (n, e) => PopupMenu.open(this.menuFor(n), { x: e.clientX, y: e.clientY }),
      },
      'Katman ağacı',
    );

    this.body.append(h('div', { class: 'panel__toolbar' }, h('span', { class: 'field-icon' }, icon('search', 14)), filter), this.tree.el);
    this.d.add(() => this.tree.dispose());
    this.d.add(
      listen(filter, 'input', () => {
        this.rows.clear();
        this.tree.setFilter(filter.value);
      }),
    );

    this.d.add(layers.events.on('structure', () => this.scheduleRebuild()));
    this.d.add(layers.events.on('expanded', () => this.scheduleRebuild()));
    this.d.add(layers.events.on('state', () => this.writeStates()));
    this.d.add(watchAll([layers.active, ctx.ui.theme], () => this.writeStates()));
    this.d.add(ctx.doc.events.on('changed', () => this.writeCounts()));
    this.rebuild();
  }

  /**
   * Rebuilds once, at the end of the current task: a DXF import adds its
   * layers one by one (300 for a large file), and each would otherwise
   * render the whole growing tree again. Until then counts and layer state
   * go to the old rows, which the rebuild replaces.
   */
  private scheduleRebuild(): void {
    if (this.rebuildQueued) return;
    this.rebuildQueued = true;
    queueMicrotask(() => {
      this.rebuildQueued = false;
      this.rebuild();
    });
  }

  /** Renders the whole tree again: its shape changed. */
  private rebuild(): void {
    this.totals = this.countTotals();
    this.rows.clear();
    this.tree.render(this.ctx.doc.layers.tree);
    this.setMeta(`${this.ctx.doc.layers.leaves().length} katman`);
  }

  /**
   * Object counts of every node, from the document's per-layer index (it
   * does not walk the drawing). A group sums all its layers, also those a
   * filter leaves out.
   */
  private countTotals(): Map<string, number> {
    const counts = this.ctx.doc.countByLayer();
    const totals = new Map<string, number>();
    const walk = (n: LayerNode): number => {
      let sum = 0;
      if (n.type === 'layer') sum = counts.get(n.id) ?? 0;
      else for (const child of n.children) sum += walk(child);
      totals.set(n.id, sum);
      return sum;
    };
    for (const n of this.ctx.doc.layers.tree) walk(n);
    return totals;
  }

  /** Objects were added, removed or moved between layers: only the counts can differ. */
  private writeCounts(): void {
    this.totals = this.countTotals();
    for (const r of this.rows.values()) this.writeCount(r);
  }

  private writeCount(r: Row): void {
    const count = this.totals.get(r.node.id) ?? 0;
    if (r.shown.count === count) return;
    r.shown.count = count;
    r.count.textContent = String(count);
  }

  /** Layer state, the active layer or the theme changed. */
  private writeStates(): void {
    for (const r of this.rows.values()) this.writeState(r);
  }

  /** Visibility, lock, hidden by a group, active layer and colour of one row. */
  private writeState(r: Row): void {
    const layers = this.ctx.doc.layers;
    const { node: n, shown } = r;
    if (shown.visible !== n.visible) {
      shown.visible = n.visible;
      r.eye.replaceChildren(icon(n.visible ? 'eye' : 'eyeOff', 15));
      r.eye.setAttribute('aria-label', n.visible ? 'Gizle' : 'Göster');
      r.eye.setAttribute('aria-pressed', String(!n.visible));
    }
    if (shown.locked !== n.locked) {
      shown.locked = n.locked;
      r.lock.replaceChildren(icon(n.locked ? 'lock' : 'unlock', 15));
      r.lock.setAttribute('aria-label', n.locked ? 'Kilidi aç' : 'Kilitle');
      r.lock.setAttribute('aria-pressed', String(n.locked));
      r.lock.toggleAttribute('data-on', n.locked);
    }
    const hidden = !layers.isVisible(n.id);
    if (shown.hidden !== hidden) {
      shown.hidden = hidden;
      r.item.toggleAttribute('data-hidden', hidden);
    }
    const active = layers.active.value === n.id;
    if (shown.active !== active) {
      shown.active = active;
      r.item.toggleAttribute('data-active', active);
    }
    if (r.swatch) {
      const color = layerSwatch(n, this.ctx.view.palette);
      if (shown.color !== color) {
        shown.color = color;
        r.swatch.style.setProperty('--swatch', color);
      }
    }
  }

  private renderRow(n: LayerNode, content: HTMLElement): void {
    const layers = this.ctx.doc.layers;
    const isLayer = n.type === 'layer';
    const item = content.parentElement ?? content;
    item.toggleAttribute('data-group', !isLayer);

    const eye = rowButton('ibtn ibtn--row', () => layers.toggleVisible(n.id));
    const lock = rowButton('ibtn ibtn--row', () => layers.toggleLocked(n.id));
    let swatch: HTMLElement | null = null;
    if (isLayer) {
      const s = rowButton('swatch swatch--btn', () => PopupMenu.open(this.colorItems(n), s.getBoundingClientRect(), { minWidth: 180 }));
      s.setAttribute('aria-label', 'Katman rengi');
      swatch = s;
    }
    const name = h('span', { class: 'tree__name', title: layers.path(n.id) }, n.name);
    const count = h('span', { class: 'tree__count num' });
    content.append(swatch ?? h('span', { class: 'tree__folder' }, icon('folder', 15)), name, count, eye, lock);

    const row: Row = { node: n, item, name, count, eye, lock, swatch, shown: {} };
    this.rows.set(n.id, row);
    this.writeCount(row);
    this.writeState(row);
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
    // Scrolled into view first: a row out of view is not built.
    this.tree.rowOf(n.id);
    const nameEl = this.rows.get(n.id)?.name;
    if (!nameEl?.isConnected) return;
    const input = h('input', { class: 'field field--inline', value: n.name, 'aria-label': 'Katman adı', spellcheck: 'false' });
    nameEl.replaceWith(input);
    input.focus();
    input.select();
    let done = false;
    const finish = (commit: boolean) => {
      if (done) return;
      done = true;
      // A new name rebuilds the tree (structure); otherwise the name goes back into the same row.
      if (commit && input.value.trim() && input.value !== n.name) this.ctx.doc.layers.rename(n.id, input.value);
      else input.replaceWith(nameEl);
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

/**
 * An icon button in a row. A click leaves the keyboard focus where it was
 * (the tree or the drawing): the row outlives the click now, and a focused
 * button in it would take the next Enter or Space away from the drawing.
 */
function rowButton(className: string, run: () => void): HTMLButtonElement {
  const b = h('button', { class: className, type: 'button' });
  b.addEventListener('pointerdown', (e) => e.button === 0 && e.preventDefault());
  b.addEventListener('click', (e) => {
    e.stopPropagation();
    run();
  });
  return b;
}
