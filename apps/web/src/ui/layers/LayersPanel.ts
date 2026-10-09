import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { LINE_TYPE_LABEL, type LayerNode, type LineType } from '../../model/layers';
import { serviceIcon } from '../../model/servicePresets';
import { serviceHub } from '../../render/serviceHub';
import { layerSnapItems, SNAP_TIP, snapState, toggledSnap, type SnapState } from './layerSnap';
import { h } from '../dom';
import { icon } from '../icons';
import { Panel } from '../dock/Panel';
import { DRAW_COLORS, LINE_WEIGHTS } from '../ribbon/fields';
import { commandButton } from '../widgets/CommandButton';
import { askRemove } from '../widgets/confirm';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { TreeView } from '../widgets/TreeView';
import { objectsOfNode, zoomItem } from './layerZoom';
import { chosenLayer } from './chosenLayer';
import { feedItem, serviceFailure, serviceItems } from './serviceMenu';
import { colorSwatch, layerSwatch } from './swatch';
import { treeLocked } from './treeRights';
import { fixed } from '../../core/displayNumber';
import { applyLayerState, partsOf, partsText, quickSaveLayerState, stateMatches, statesLocked } from '../../app/layerStates';
import { tooltip } from '../widgets/tooltip';
import { filterText } from '../../app/layerFilterCommands';
import type { LayerFilter } from '../../contracts/generated/LayerFilter';

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
  /** The layer's own snapping (docs/adr/0163 §4); a group's writes to all its layers. */
  readonly magnet: HTMLElement;
  /** A layer's colour swatch; a group shows a folder, a map service layer its service's icon. */
  readonly swatch: HTMLElement | null;
  /** A map service layer's badge: why its service shows nothing, when it failed (docs/adr/0208 §14). */
  readonly fail: HTMLElement | null;
  /** A temporal layer's clock (docs/adr/0210 §10); a group's lead, which a scenario's own icon takes. */
  readonly clock: HTMLElement;
  /** A filtered layer's funnel (docs/adr/0211 §4), in the warning colour when its condition does not compile. */
  readonly funnel: HTMLElement;
  readonly lead: HTMLElement;
  readonly shown: {
    count?: string;
    visible?: boolean;
    locked?: boolean;
    hidden?: boolean;
    active?: boolean;
    color?: string;
    snap?: SnapState;
    fail?: string | null;
    time?: string;
    scenario?: boolean;
    filter?: string;
  };
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
 *
 * The tree follows the drawing's selection (the owner's request, 26
 * September; the desktop's `follow_selection_layers`, docs/adr/0058): the
 * rows of the selected objects' layers show selected, the groups around
 * them open, and a single such layer is scrolled into view. The active
 * layer, where new objects go, does not change. A row clicked in the tree is
 * shown selected until the selection changes again; an empty selection
 * shows the row last clicked again (or none).
 */
export class LayersPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly tree: TreeView<LayerNode>;
  /** The rows on the page by node id (the tree builds those in its scroll window). */
  private readonly rows = new Map<string, Row>();
  /** Object count of every node; a group's is the sum over all its layers. */
  private totals = new Map<string, number>();
  /** Where a filter leaves objects out (docs/adr/0211 §4): how many show; a group's sums its layers'. */
  private passing = new Map<string, number>();
  private rebuildQueued = false;
  private countsQueued = false;

  constructor(ctx: AppContext) {
    super({ title: 'Katmanlar', className: 'panel--layers', actions: [] });
    this.ctx = ctx;
    const layers = ctx.doc.layers;
    const actions = this.el.querySelector('.panel__actions')!;
    actions.append(
      commandButton(ctx, 'layer.new', this.d, { className: 'ibtn', size: 16 }),
      commandButton(ctx, 'layer.newGroup', this.d, { className: 'ibtn', size: 16 }),
      this.menuButton('layerStates', 'Katman durumları', 'Kayıtlı durumlar (işaretli olan şimdiki hâl), yeni durum kaydetme ve Katman durumları penceresi.', () => this.statesMenu()),
      this.menuButton('more', 'Katman işlemleri', 'Katmanları birleştir, Kullanılmayanları temizle ve Katman listesi.', () => this.moreMenu()),
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
        // A map service layer takes no objects: its settings instead (docs/adr/0208 §14).
        onActivate: (n) =>
          n.service
            ? void import('../services/ServiceDialog').then((m) => m.openServiceDialog(ctx, n.id))
            : n.type === 'layer'
              ? layers.setActive(n.id)
              : layers.setExpanded(n.id, !n.expanded),
        // The row chosen: Öznitelikler shows its service or source with nothing selected (docs/adr/0208 §14).
        onSelect: (n) => chosenLayer.set(n.id),
        onToggle: (n) => layers.toggleVisible(n.id),
        onRename: (n) => this.rename(n),
        onDelete: (n) => void this.remove(n),
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
    // ↓ takes the keys into the tree, at its first listed row. Esc clears the search; on an empty box it
    // gives the keys back to the drawing.
    this.d.add(
      listen<KeyboardEvent>(filter, 'keydown', (e) => {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          this.tree.enterFirst();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          if (filter.value) {
            filter.value = '';
            this.rows.clear();
            this.tree.setFilter('');
          } else ctx.view.focus();
        }
      }),
    );

    this.d.add(layers.events.on('structure', () => this.scheduleRebuild()));
    this.d.add(layers.events.on('expanded', () => this.scheduleRebuild()));
    this.d.add(
      layers.events.on('state', () => {
        this.writeStates();
        this.scheduleCounts();
      }),
    );
    this.d.add(watchAll([layers.active, ctx.prefs.theme], () => this.writeStates()));
    // Counted once the change is all in: a filtered layer's count asks the geometry store, which takes the change's
    // objects after this event (docs/adr/0211 §4); an attribute edit or a filter can change it too.
    this.d.add(ctx.doc.events.on('changed', () => this.scheduleCounts()));
    this.d.add(ctx.doc.events.on('touched', () => this.scheduleCounts()));
    this.d.add(ctx.selection.ids.subscribe(() => this.followSelection()));
    this.d.add(ctx.doc.events.on('reset', () => chosenLayer.set(null)));
    // A service that failed or came back: its row's badge, once a frame at most.
    let badges = 0;
    this.d.add(
      serviceHub().listen(() => {
        if (!badges) badges = requestAnimationFrame(() => ((badges = 0), this.writeFailures()));
      }),
    );
    this.d.add(() => cancelAnimationFrame(badges));
    this.rebuild();
    this.followSelection();
  }

  /** A header button that opens `items` under it (docs/adr/0177 §7). */
  private menuButton(glyph: string, title: string, description: string, items: () => MenuItem[]): HTMLButtonElement {
    const btn = h('button', { class: 'ibtn', type: 'button', 'aria-label': title, 'aria-haspopup': 'menu' }, icon(glyph, 16)) as HTMLButtonElement;
    btn.addEventListener('click', () => PopupMenu.open(items(), btn.getBoundingClientRect(), { placement: 'below', owner: btn }));
    this.d.add(tooltip(btn, () => ({ title, description }), 'bottom'));
    return btn;
  }

  /**
   * Katman durumları ▾ (docs/adr/0177 §4): the project's states, the one the layers are in now marked, each applied
   * by a click; Yeni durum kaydet saves the layers at once as “Durum n”; Katman durumları… opens the window.
   */
  private statesMenu(): MenuItem[] {
    const ctx = this.ctx;
    const states = ctx.doc.settings.layerStates.value;
    const locked = statesLocked(ctx);
    return [
      ...(states.length
        ? states.map((s): MenuItem => ({ label: s.name, radio: true, checked: stateMatches(ctx, s), hint: partsText(partsOf(s)), run: () => applyLayerState(ctx, s.id) }))
        : [{ label: 'Kayıtlı durum yok', disabled: true }]),
      { kind: 'separator' },
      { label: 'Yeni durum kaydet', icon: 'layerStateSave', disabled: !!locked, run: () => void quickSaveLayerState(ctx) },
      { label: 'Katman durumları…', icon: 'layerStates', run: () => void ctx.commands.execute('layer.states') },
    ];
  }

  /** The panel's ⋯ (docs/adr/0177 §7): the actions on the whole tree. */
  private moreMenu(): MenuItem[] {
    const ctx = this.ctx;
    const locked = !!treeLocked(ctx);
    return [
      { label: 'Katmanları birleştir…', icon: 'layerMerge', disabled: locked, run: () => void ctx.commands.execute('layer.merge') },
      { label: 'Kullanılmayanları temizle…', icon: 'layerPurge', disabled: locked, run: () => void ctx.commands.execute('layer.purge') },
      { label: 'Katman listesi…', icon: 'layerList', run: () => void ctx.commands.execute('layer.list') },
    ];
  }

  /**
   * The drawing's selection changed: its objects' layers show selected in the
   * tree, all of them, and the groups around them open so their rows can be
   * seen; one such layer is scrolled into view. Opening a group is not an
   * edit (nothing unsaved, no undo step), and the active layer stays. An
   * empty selection gives the tree back the row last clicked.
   */
  private followSelection(): void {
    const { doc, selection } = this.ctx;
    const layers = doc.layers;
    const ids = new Set<string>();
    for (const id of selection.ids.value) {
      const e = doc.get(id);
      if (e) ids.add(e.layerId);
    }
    if (!ids.size) return this.tree.show(null);
    let opened = false;
    for (const id of ids)
      for (let g = layers.parentOf(id); g; g = layers.parentOf(g.id))
        if (!g.expanded) {
          layers.setExpanded(g.id, true);
          opened = true;
        }
    // The opened groups' rows are listed now, not at the end of the task, so the layer can be scrolled to.
    if (opened) this.rebuild();
    this.tree.show(ids, ids.size === 1);
  }

  /**
   * Rebuilds once, at the end of the current task: a DXF import adds its
   * layers one by one (300 for a large file), and each would otherwise
   * render the whole growing tree again. Until then counts and layer state
   * go to the old rows, which the rebuild replaces. A rebuild done meanwhile
   * (followSelection) makes this one unneeded.
   */
  private scheduleRebuild(): void {
    if (this.rebuildQueued) return;
    this.rebuildQueued = true;
    queueMicrotask(() => {
      if (this.rebuildQueued) this.rebuild();
    });
  }

  /** Counts again once, at the end of the current task. */
  private scheduleCounts(): void {
    if (this.countsQueued) return;
    this.countsQueued = true;
    queueMicrotask(() => {
      if (this.countsQueued) this.writeCounts();
    });
  }

  /** Renders the whole tree again: its shape changed. */
  private rebuild(): void {
    this.rebuildQueued = false;
    this.countTotals();
    this.rows.clear();
    this.tree.render(this.ctx.doc.layers.tree);
    this.setMeta(`${this.ctx.doc.layers.leaves().length} katman`);
  }

  /**
   * Object counts of every node, from the document's per-layer index (it
   * does not walk the drawing). A group sums all its layers. A filtered
   * layer's objects that pass come from the geometry store, which keeps them
   * (docs/adr/0211 §4); a group with one under it sums what shows of each.
   */
  private countTotals(): void {
    const counts = this.ctx.doc.countByLayer();
    const geometry = this.ctx.view.geometry;
    const totals = new Map<string, number>();
    const passing = new Map<string, number>();
    const walk = (n: LayerNode): [total: number, shown: number] => {
      let sum = 0;
      let shown = 0;
      if (n.type === 'layer') {
        sum = counts.get(n.id) ?? 0;
        const f = n.filter && !n.service ? geometry.filterCounts(n.id) : null;
        shown = f ? f.passed : sum;
        if (f) passing.set(n.id, shown);
      } else {
        for (const child of n.children) {
          const [t, p] = walk(child);
          sum += t;
          shown += p;
        }
        if (shown !== sum || n.children.some((c) => passing.has(c.id))) passing.set(n.id, shown);
      }
      totals.set(n.id, sum);
      return [sum, shown];
    };
    for (const n of this.ctx.doc.layers.tree) walk(n);
    this.totals = totals;
    this.passing = passing;
  }

  /** Objects were added, removed, moved between layers or edited, or a filter changed: only the counts can differ. */
  private writeCounts(): void {
    this.countsQueued = false;
    this.countTotals();
    for (const r of this.rows.values()) this.writeCount(r);
  }

  private writeCount(r: Row): void {
    const total = this.totals.get(r.node.id) ?? 0;
    const passed = this.passing.get(r.node.id);
    const count = passed === undefined ? String(total) : `${passed} / ${total}`;
    if (r.shown.count === count) return;
    r.shown.count = count;
    r.count.textContent = count;
    r.count.title = passed === undefined ? '' : `Süzgeçten ${passed} nesne geçiyor; katmanda ${total} nesne var.`;
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
    const snap = snapState(layers, n);
    if (shown.snap !== snap) {
      shown.snap = snap;
      r.magnet.replaceChildren(icon(snap === 'off' ? 'magnetOff' : snap === 'kinds' ? 'magnetKinds' : 'magnet', 15));
      r.magnet.setAttribute('aria-label', SNAP_TIP[snap]);
      r.magnet.setAttribute('aria-pressed', String(snap !== 'none'));
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
    this.writeFailure(r);
    // A temporal layer's clock and a scenario group's icon (docs/adr/0210 §10).
    const time = n.time ? `Zamansal katman: ${n.time.start}${n.time.end ? ` – ${n.time.end}` : n.time.cumulative ? '' : ' (anlık)'}${n.time.cumulative ? ' (birikimli)' : ''}` : '';
    if (shown.time !== time) {
      shown.time = time;
      r.clock.hidden = !time;
      r.clock.title = time;
    }
    // A filtered layer's funnel (docs/adr/0211 §4): its condition and list in its tip, warning when it does not compile.
    const filter = n.filter && !n.service ? filterTip(n.filter, this.ctx.view.geometry.filterError(n.id)) : '';
    if (shown.filter !== filter) {
      shown.filter = filter;
      r.funnel.hidden = !filter;
      r.funnel.title = filter;
      r.funnel.toggleAttribute('data-error', !!n.filter && !!this.ctx.view.geometry.filterError(n.id));
    }
    const scenario = !!n.scenario;
    if (n.type === 'group' && shown.scenario !== scenario) {
      shown.scenario = scenario;
      r.lead.replaceChildren(icon(scenario ? 'scenario' : 'folder', 15));
      r.lead.title = scenario ? `Senaryo${n.scenario?.note ? `: ${n.scenario.note}` : ''}` : '';
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
    // Off and on again: off goes back to the general kinds; a group's writes to all its layers.
    const magnet = rowButton('ibtn ibtn--row', () => layers.setSnap(n.id, toggledSnap(layers, n)));
    let swatch: HTMLElement | null = null;
    if (isLayer && !n.service) {
      const s = rowButton('swatch swatch--btn', () => PopupMenu.open(this.colorItems(n), s.getBoundingClientRect(), { minWidth: 180 }));
      s.setAttribute('aria-label', 'Katman rengi');
      swatch = s;
    }
    const name = h('span', { class: 'tree__name', title: layers.path(n.id) }, n.name);
    const count = h('span', { class: 'tree__count num' });
    const fail = n.service ? h('span', { class: 'tree__fail', hidden: true }, icon('warning', 12)) : null;
    const lead = swatch ?? h('span', { class: 'tree__folder' }, icon(n.service ? serviceIcon(n.service) : 'folder', 15));
    const clock = h('span', { class: 'tree__clock', hidden: true }, icon('clock', 12));
    const funnel = h('span', { class: 'tree__filter', hidden: true }, icon('funnel', 12));
    content.append(lead, name, funnel, clock, ...(fail ? [fail] : []), count, eye, lock, magnet);

    const row: Row = { node: n, item, name, count, eye, lock, magnet, swatch, fail, clock, funnel, lead, shown: {} };
    this.rows.set(n.id, row);
    this.writeCount(row);
    this.writeState(row);
  }

  /** The map service rows' badges. */
  private writeFailures(): void {
    for (const r of this.rows.values()) if (r.fail) this.writeFailure(r);
  }

  /** Why a service layer shows nothing, as its row's badge and in its name's tip. */
  private writeFailure(r: Row): void {
    if (!r.fail) return;
    const why = serviceFailure(this.ctx, r.node);
    if (r.shown.fail === why) return;
    r.shown.fail = why;
    r.fail.hidden = !why;
    const path = this.ctx.doc.layers.path(r.node.id);
    r.name.title = why ? `${path}\n${why}` : path;
    r.fail.title = why ?? '';
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
    // A map service layer takes no objects (docs/adr/0208 §2).
    if (isLayer) items.push({ label: 'Etkin katman yap', icon: 'check', disabled: layers.active.value === n.id || !!n.service, run: () => layers.setActive(n.id) });
    items.push(
      { label: n.visible ? 'Gizle' : 'Göster', icon: n.visible ? 'eyeOff' : 'eye', shortcut: 'Space', run: () => layers.toggleVisible(n.id) },
      { label: n.locked ? 'Kilidi aç' : 'Kilitle', icon: n.locked ? 'unlock' : 'lock', run: () => layers.toggleLocked(n.id) },
    );
    // A layer drawn from a map service: its service's items, none of the objects' (docs/adr/0208 §14).
    if (n.service) {
      items.push(
        { kind: 'separator' },
        ...serviceItems(this.ctx, n),
        { kind: 'separator' },
        { label: 'Yeniden adlandır', icon: 'edit', shortcut: 'F2', run: () => this.rename(n) },
        { kind: 'separator' },
        { label: 'Sil', icon: 'trash', shortcut: 'Delete', run: () => void this.remove(n) },
      );
      return items;
    }
    // A layer whose objects came from a service: taken again (docs/adr/0208 §10).
    if (n.feed) items.push(feedItem(this.ctx, n));
    items.push(
      { label: 'Kenet', icon: 'magnet', items: () => layerSnapItems(this.ctx, n) },
      { label: 'Yalnızca bunu göster', icon: 'layerIsolate', run: () => layers.isolate(n.id) },
      { label: 'Tüm katmanları göster', icon: 'layersShowAll', run: () => layers.showAll() },
      { kind: 'separator' },
      {
        label: 'Nesnelerini seç',
        icon: 'select',
        run: () => {
          const ids = objectsOfNode(this.ctx, n);
          this.ctx.selection.set(ids);
          this.ctx.log.info(`${n.name}: ${ids.length} nesne seçildi.`);
        },
      },
      zoomItem(this.ctx, n),
      { kind: 'separator' },
    );
    if (isLayer) {
      items.push(
        { label: 'Renk', icon: 'color', items: () => this.colorItems(n) },
        {
          label: 'Çizgi tipi',
          icon: 'lineType',
          items: () =>
            (Object.keys(LINE_TYPE_LABEL) as LineType[]).map((t) => ({ label: LINE_TYPE_LABEL[t], radio: true, checked: n.style.lineType === t, run: () => this.ctx.doc.setLayerStyle(n.id, { lineType: t }, 'Çizgi tipi') })),
        },
        {
          label: 'Kalınlık',
          icon: 'lineWeight',
          items: () => LINE_WEIGHTS.map((w) => ({ label: `${fixed(w, 2)} mm`, radio: true, checked: n.style.lineWeight === w, run: () => this.ctx.doc.setLayerStyle(n.id, { lineWeight: w }, 'Çizgi kalınlığı') })),
        },
        {
          label: n.style.renderer ? 'Katman stili… (özel)' : 'Katman stili…',
          icon: 'layerStyle',
          run: () => void import('../style/LayerStyleDialog').then((m) => m.openLayerStyle(this.ctx, n.id)),
        },
        // Alanlar (docs/adr/0199 §3): the schema of its objects' attributes.
        { label: n.fields?.length ? `Alanlar… (${n.fields.length})` : 'Alanlar…', icon: 'layerFields', run: () => void this.ctx.commands.execute('layer.fields', n.id) },
        // Zaman ayarları (docs/adr/0210 §10): its objects' start, end and key fields.
        { label: n.time ? 'Zaman ayarları… (zamansal)' : 'Zaman ayarları…', icon: 'timeLayer', run: () => void this.ctx.commands.execute('time.layer', n.id) },
        // Katman süzgeci (docs/adr/0211 §4): the window, the selection's objects, taken away.
        { label: n.filter ? 'Süzgeç… (süzgeçli)' : 'Süzgeç…', icon: 'layerFilter', run: () => void this.ctx.commands.execute('layer.filter', n.id) },
        { label: 'Seçimden süzgeç', icon: 'layerFilterSelection', disabled: !this.selectedOn(n.id), run: () => void this.ctx.commands.execute('layer.filterFromSelection', n.id) },
        { label: 'Süzgeci kaldır', icon: 'layerFilterClear', disabled: !n.filter, run: () => void this.ctx.commands.execute('layer.filterClear', n.id) },
        { kind: 'separator' },
      );
    }
    // A scenario group (docs/adr/0210 §10): shown, compared, applied.
    if (n.scenario)
      items.push(
        { label: 'Senaryoyu göster', icon: 'scenarioShow', run: () => void this.ctx.commands.execute('scenario.show', n.id) },
        { label: 'Mevcut durum', icon: 'scenarioBase', run: () => void this.ctx.commands.execute('scenario.base') },
        { label: 'Senaryoyu karşılaştır…', icon: 'scenarioCompare', run: () => void this.ctx.commands.execute('scenario.compare', n.id) },
        { label: 'Senaryoyu uygula…', icon: 'scenarioApply', run: () => void this.ctx.commands.execute('scenario.apply', n.id) },
        { kind: 'separator' },
      );
    items.push(
      { label: 'Yeniden adlandır', icon: 'edit', shortcut: 'F2', run: () => this.rename(n) },
      {
        label: isLayer ? 'Yanına yeni katman' : 'İçine yeni katman',
        icon: 'layerAdd',
        // As Yeni katman: one undo step, “Katman ekle”, the new layer made active.
        run: () => {
          const locked = treeLocked(this.ctx);
          if (locked) return this.ctx.log.warn(locked);
          const node = this.ctx.doc.addLayer({ name: layers.uniqueName('Yeni katman') }, n.id, { activate: true });
          this.ctx.log.success(`“${node.name}” katmanı eklendi ve etkin yapıldı.`);
        },
      },
      // Kopyasını oluştur and Başka katmanlarla birleştir… (docs/adr/0177 §3): this layer the source, or the target.
      ...(isLayer
        ? [
            { label: 'Kopyasını oluştur', icon: 'layerDuplicate', run: () => void this.ctx.commands.execute('layer.duplicate', n.id) },
            { label: 'Başka katmanlarla birleştir…', icon: 'layerMerge', run: () => void this.ctx.commands.execute('layer.merge', n.id) },
          ]
        : []),
      { kind: 'separator' },
      { label: 'Sil', icon: 'trash', shortcut: 'Delete', run: () => void this.remove(n) },
    );
    return items;
  }

  /** Whether a selected object is on the layer (Seçimden süzgeç's item). */
  private selectedOn(layerId: string): boolean {
    for (const id of this.ctx.selection.ids.value) if (this.ctx.doc.get(id)?.layerId === layerId) return true;
    return false;
  }

  /**
   * Sil: the layer, or the group with everything under it, and their objects,
   * in one undo step (CadDocument.removeLayer). What it refuses (the last or
   * the active layer, a lock) is said first, in its words; with objects on it
   * the user is asked.
   */
  private async remove(n: LayerNode): Promise<void> {
    const { doc, log } = this.ctx;
    const locked = treeLocked(this.ctx);
    if (locked) return log.warn(locked);
    const refused = doc.layerRemovalRefused(n.id);
    if (refused) return log.warn(refused);
    const group = n.type === 'group';
    const leaves = doc.layers.leavesOf(n.id);
    const count = leaves.reduce((sum, l) => sum + doc.byLayer(l.id).length, 0);
    if (count) {
      const yes = await askRemove({
        title: group ? 'Grubu sil' : 'Katmanı sil',
        message: group ? `“${n.name}” grubu, içindeki ${leaves.length} katman ve ${count} nesneyle birlikte silinsin mi?` : `“${n.name}” katmanı üzerindeki ${count} nesneyle birlikte silinsin mi?`,
        details: ['Geri al (Ctrl+Z) katmanı nesneleriyle geri getirir.'],
        action: 'Sil',
      });
      // Answered after a while: the drawing may have changed meanwhile, so its refusal is asked again.
      if (!yes || !doc.layers.get(n.id)) return;
      const now = doc.layerRemovalRefused(n.id);
      if (now) return log.warn(now);
    }
    const gone = doc.removeLayer(n.id);
    if (!group) log.success(gone ? `“${n.name}” katmanı ve üzerindeki ${gone} nesne silindi.` : `“${n.name}” katmanı silindi.`);
    else if (gone) log.success(`“${n.name}” grubu, içindeki ${leaves.length} katman ve ${gone} nesne silindi.`);
    else log.success(leaves.length ? `“${n.name}” grubu ve içindeki ${leaves.length} katman silindi.` : `“${n.name}” grubu silindi.`);
  }

  private rename(n: LayerNode): void {
    const locked = treeLocked(this.ctx);
    if (locked) return this.ctx.log.warn(locked);
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
/** A filtered layer's funnel's tip: what it keeps, or why it lets nothing through. */
function filterTip(f: LayerFilter, error: string | null): string {
  const what = filterText(f);
  return error ? `Süzgeç çalışmıyor, hiçbir nesne geçmiyor: ${error}\nSüzgeç: ${what}` : `Süzgeç: ${what}`;
}

function rowButton(className: string, run: () => void): HTMLButtonElement {
  const b = h('button', { class: className, type: 'button' });
  b.addEventListener('pointerdown', (e) => e.button === 0 && e.preventDefault());
  b.addEventListener('click', (e) => {
    e.stopPropagation();
    run();
  });
  return b;
}
