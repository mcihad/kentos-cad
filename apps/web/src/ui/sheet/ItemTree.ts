import type { AppContext } from '../../app/context';
import { DisposableStore, listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { KIND_ICON, kindName } from '../../product/sheet/profile';
import type { SheetView } from '../../product/sheet/view';
import { Panel } from '../dock/Panel';
import { append, h } from '../dom';
import { icon } from '../icons';
import { PopupMenu } from '../widgets/PopupMenu';
import { TreeView } from '../widgets/TreeView';
import { tooltip } from '../widgets/tooltip';
import type { SheetHost } from './host';
import { itemItems } from './menus';
import { dropOf, itemNodes, MASTER_NODE, orderAfterDrop, type ItemNode } from './treePlan';

/**
 * Öğeler (docs/sheet/design.md §11): the open sheet's items, the one in
 * front at the top, groups with their items under them, the master page's
 * items at the bottom (locked, dimmed, never chosen). A row's eye and lock
 * hide and lock it; a click chooses it (Shift adds, Ctrl flips), and the
 * paper's choice shows here; a double click or F2 names it; Space hides it;
 * Delete removes the choice; a right click gives the items' menu. Rows are
 * dragged to change the drawing order among their siblings. Each change is
 * the engine's operation, one undo step.
 */
export class ItemTree extends Panel {
  private readonly ctx: AppContext;
  private readonly host: SheetHost;
  private readonly tree: TreeView<ItemNode>;
  private readonly folded = new Set<string>([MASTER_NODE]);
  private readonly rows = new DisposableStore();
  /** The modifiers of the click a row is being chosen by (TreeView tells the choice, not the keys). */
  private mods: { shift: boolean; ctrl: boolean } | null = null;
  private dragged: string[] = [];

  constructor(ctx: AppContext, host: SheetHost) {
    super({ title: 'Öğeler', className: 'sheet-items' });
    this.ctx = ctx;
    this.host = host;
    const { state } = host;
    this.tree = new TreeView<ItemNode>(
      {
        id: (n) => n.id,
        children: (n) => n.children,
        isExpanded: (n) => !this.folded.has(n.id),
        setExpanded: (n, on) => {
          if (on) this.folded.delete(n.id);
          else this.folded.add(n.id);
          this.render();
        },
        renderRow: (n, content) => this.renderRow(n, content),
        onSelect: (n) => this.choose(n),
        onActivate: (n) => n.item && !n.item.master && this.ctx.commands.execute('sheet.renameItem'),
        onToggle: (n) => n.item && !n.item.master && this.host.apply([{ op: 'hide', ids: [n.item.id], value: !n.item.hidden }], n.item.hidden ? 'Göster' : 'Gizle'),
        onRename: (n) => n.item && !n.item.master && this.ctx.commands.execute('sheet.renameItem'),
        onDelete: (n) => n.item && !n.item.master && this.ctx.commands.execute('sheet.deleteItems'),
        onContextMenu: (n, e) => {
          if (!n.item || n.item.master) return;
          if (!state.selection.value.has(n.item.id)) state.select([n.item.id]);
          PopupMenu.open(itemItems(ctx, host), { x: e.clientX, y: e.clientY }, { placement: 'point', minWidth: 220 });
        },
        empty: () => 'Bu paftada öğe yok. Şeridin Pafta sekmesindeki Ekle panelinden harita, metin, antet … ekleyin.',
        tip: (n, row) => {
          if (n.id === MASTER_NODE) return { title: 'Ana sayfa', description: 'Ana sayfanın öğeleri her paftanın altında, kilitli çizilir; ana sayfada düzenlenir.' };
          const name = row.querySelector<HTMLElement>('.tree__name');
          const detail = row.querySelector<HTMLElement>('.tree__detail');
          const hidden = !!name && (name.scrollWidth > name.clientWidth + 1 || (!!detail && detail.offsetParent === null));
          if (!hidden || !n.item) return null;
          const kind = kindName(host.profile.value, n.item.kind);
          return { title: n.item.name, description: n.item.detail ? `${kind} · ${n.item.detail}` : kind };
        },
      },
      'Paftanın öğeleri',
    );
    this.body.append(this.tree.el);
    this.d.add(() => this.tree.dispose());
    this.d.add(() => this.rows.dispose());
    this.d.add(watchAll([state.book, state.open, host.profile, state.engine], () => this.render()));
    this.d.add(state.selection.subscribe(() => this.follow()));
    // The click's keys, read before the tree turns the click into a choice.
    this.d.add(
      listen<MouseEvent>(
        this.tree.el,
        'click',
        (e) => {
          this.mods = { shift: e.shiftKey, ctrl: e.ctrlKey || e.metaKey };
          queueMicrotask(() => (this.mods = null));
        },
        true,
      ),
    );
    this.bindDrag();
    this.render();
  }

  private sheet(): SheetView | null {
    return this.host.state.sheet;
  }

  private render(): void {
    const sheet = this.sheet();
    this.rows.dispose();
    const nodes = sheet ? itemNodes(sheet) : [];
    this.tree.render(nodes);
    const own = sheet ? sheet.items.filter((i) => !i.master && i.kind !== 'group').length : 0;
    this.setMeta(sheet ? `${own} öğe` : '');
    this.follow();
  }

  /** The paper's choice shown in the tree (the groups round a chosen item open). */
  private follow(): void {
    const ids = this.host.state.selection.value;
    this.tree.show(ids.size ? ids : null, ids.size === 1);
  }

  private choose(n: ItemNode): void {
    const item = n.item;
    if (!item || item.master) return;
    const mods = this.mods;
    this.host.state.select([item.id], mods?.ctrl ? 'toggle' : mods?.shift ? 'add' : 'replace');
  }

  private renderRow(n: ItemNode, content: HTMLElement): void {
    const row = content.parentElement ?? content;
    if (n.id === MASTER_NODE) {
      row.toggleAttribute('data-master', true);
      content.append(icon('lock', 15), h('span', { class: 'tree__name' }, n.label), h('span', { class: 'tree__count num' }, String(n.children.length)));
      return;
    }
    const item = n.item!;
    const profile = this.host.profile.value;
    row.toggleAttribute('data-hidden', item.hidden);
    row.toggleAttribute('data-master', !!item.master);
    row.toggleAttribute('data-group', item.kind === 'group');
    const editable = !item.master && this.host.whyReadOnly() === null;
    row.setAttribute('draggable', String(editable));
    row.dataset.item = item.id;
    const button = (on: boolean, iconOn: string, iconOff: string, labelOn: string, labelOff: string, take: () => void) => {
      const b = h('button', { class: 'ibtn ibtn--row', type: 'button', 'aria-label': on ? labelOn : labelOff, 'aria-pressed': String(on), tabindex: '-1' }, icon(on ? iconOn : iconOff, 15));
      // A click does not choose the row or take the keyboard.
      b.addEventListener('pointerdown', (e) => e.preventDefault());
      b.addEventListener('click', (e) => {
        e.stopPropagation();
        take();
      });
      this.rows.add(tooltip(b, () => ({ title: on ? labelOn : labelOff, note: item.master ? 'Ana sayfanın öğesi: ana sayfada değişir.' : (this.host.whyReadOnly() ?? undefined) })));
      return b;
    };
    const eye = button(item.hidden, 'eyeOff', 'eye', 'Göster', 'Gizle', () => !item.master && this.host.apply([{ op: 'hide', ids: [item.id], value: !item.hidden }], item.hidden ? 'Göster' : 'Gizle'));
    const lock = button(item.locked || !!item.master, 'lock', 'unlock', 'Kilidi aç', 'Kilitle', () => !item.master && this.host.apply([{ op: 'lock', ids: [item.id], value: !item.locked }], item.locked ? 'Kilidi aç' : 'Kilitle'));
    if (item.locked) lock.toggleAttribute('data-on', true);
    append(content, [
      icon(KIND_ICON[item.kind], 15),
      h('span', { class: 'tree__name' }, item.name),
      item.detail ? h('span', { class: 'tree__detail num' }, item.detail) : item.kind === 'group' ? h('span', { class: 'tree__detail' }, kindName(profile, 'group')) : null,
      eye,
      lock,
    ]);
  }

  // ── Dragging: drawing order and groups ──────────────────────────

  private bindDrag(): void {
    const el = this.tree.el;
    const rowOf = (t: EventTarget | null) => (t as HTMLElement | null)?.closest?.<HTMLElement>('.tree__row[data-item]') ?? null;
    const clear = () => el.querySelectorAll('[data-drop]').forEach((r) => r.removeAttribute('data-drop'));
    this.d.add(
      listen<DragEvent>(el, 'dragstart', (e) => {
        const row = rowOf(e.target);
        const id = row?.dataset.item;
        if (!row || !id || row.getAttribute('draggable') !== 'true') return;
        const chosen = this.host.state.selection.value;
        this.dragged = chosen.has(id) ? [...chosen] : [id];
        e.dataTransfer?.setData('text/plain', this.dragged.join(','));
        if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
      }),
    );
    this.d.add(
      listen<DragEvent>(el, 'dragover', (e) => {
        const row = rowOf(e.target);
        const sheet = this.sheet();
        if (!row || !sheet || !this.dragged.length) return;
        const r = row.getBoundingClientRect();
        const drop = dropOf(sheet, this.dragged, row.dataset.item!, (e.clientY - r.top) / r.height);
        clear();
        if (!drop) return;
        e.preventDefault();
        row.dataset.drop = drop.zone;
      }),
    );
    this.d.add(listen(el, 'dragleave', (e) => !el.contains((e as DragEvent).relatedTarget as Node) && clear()));
    this.d.add(
      listen<DragEvent>(el, 'drop', (e) => {
        const row = rowOf(e.target);
        const sheet = this.sheet();
        clear();
        if (!row || !sheet) return;
        e.preventDefault();
        const r = row.getBoundingClientRect();
        const drop = dropOf(sheet, this.dragged, row.dataset.item!, (e.clientY - r.top) / r.height);
        if (drop) this.host.apply([{ op: 'setOrder', owner: { kind: 'sheet', id: sheet.id }, order: orderAfterDrop(sheet, this.dragged, drop) }], 'Sıra');
        this.dragged = [];
      }),
    );
    this.d.add(listen(el, 'dragend', () => ((this.dragged = []), clear())));
  }
}
