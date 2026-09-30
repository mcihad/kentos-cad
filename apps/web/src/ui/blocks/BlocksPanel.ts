import type { AppContext } from '../../app/context';
import { insertBlock, insertsOf, rebaseBlock, rebaseInsert, redefineBlock, removeBlock, removeRefusal, renameBlock, selectInserts } from '../../app/blocks';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { placements, type BlockDefinition, type Placements } from '../../model/blocks';
import { BlockInsertTool } from '../../tools/blockTools';
import { Panel } from '../dock/Panel';
import { h } from '../dom';
import { icon } from '../icons';
import { commandButton } from '../widgets/CommandButton';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { TreeView } from '../widgets/TreeView';

/** A listed block: its definition and how often it is placed. */
interface Row {
  id: string;
  block: BlockDefinition;
  placed: Placements;
}

/** The picture's side, CSS pixels at scale 1 (its row is `--blk-row-h` high). */
const THUMB = 34;

/**
 * Bloklar (docs/adr/0144 §6): the drawing's blocks in the dock's upper slot,
 * a tab beside Katmanlar and İşlemler. Each row shows the block as it is
 * drawn (its outline from the geometry store at the origin, fitted), its
 * name and description, and how many inserts place it in the drawing. The
 * row Blok ekle places next is marked. A click chooses it, a double click or
 * Enter places it (Blok ekle), F2 renames it in the row, Delete deletes an
 * unused one; the row's menu also changes the base point, redefines it from
 * the selection and selects its inserts. Beside the search, buttons make a
 * block (Blok oluştur), place the chosen one and purge the unused ones. The desktop's
 * panel (`apps/desktop/src/blocks_panel.rs`) is the same.
 */
export class BlocksPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly tree: TreeView<Row>;
  private rows: Row[] = [];
  /** Built rows by block id, so the chosen mark moves without building them again. */
  private readonly built = new Map<string, HTMLElement>();
  private queued = false;

  constructor(ctx: AppContext) {
    super({ title: 'Bloklar', className: 'panel--blocks', actions: [] });
    this.ctx = ctx;
    // The buttons sit beside the search, not in the head: the head holds the dock's tabs.
    const buttons = ['tool.blockDefine', 'tool.blockInsert', 'block.purge'].map((id) => commandButton(ctx, id, this.d, { className: 'ibtn', size: 16 }));
    const search = h('input', { class: 'field field--search', type: 'search', placeholder: 'Blok ara', 'aria-label': 'Blok ara', spellcheck: 'false' });
    this.tree = new TreeView<Row>(
      {
        id: (r) => r.id,
        children: () => [],
        isExpanded: () => false,
        setExpanded: () => {},
        matches: (r, q) => r.block.name.toLocaleLowerCase('tr-TR').includes(q) || (r.block.description ?? '').toLocaleLowerCase('tr-TR').includes(q),
        renderRow: (r, content) => this.renderRow(r, content),
        releaseRow: (r) => this.built.delete(r.id),
        onSelect: (r) => (BlockInsertTool.block = r.id),
        onActivate: (r) => insertBlock(ctx, r.id),
        onRename: (r) => this.rename(r),
        onDelete: (r) => this.remove(r),
        onContextMenu: (r, e) => PopupMenu.open(this.menuFor(r), { x: e.clientX, y: e.clientY }),
        tip: (r) => ({ title: r.block.name, description: [r.block.description, placedText(r.placed)].filter(Boolean).join('\n') }),
        empty: (filtered) =>
          filtered ? 'Aramayla eşleşen blok yok. Başka bir ad deneyin.' : 'Çizimde blok yok. Seçili nesnelerden Blok oluştur ile bir blok tanımlayın.',
      },
      'Bloklar',
    );
    this.body.append(h('div', { class: 'panel__toolbar blk__toolbar' }, h('span', { class: 'field-icon' }, icon('search', 14)), search, ...buttons), this.tree.el);
    this.d.add(() => this.tree.dispose());
    this.d.add(listen(search, 'input', () => this.tree.setFilter(search.value)));
    this.d.add(
      listen<KeyboardEvent>(search, 'keydown', (e) => {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          this.tree.enterFirst();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          if (search.value) {
            search.value = '';
            this.tree.setFilter('');
          } else ctx.view.focus();
        }
      }),
    );
    // The definitions, the drawing's inserts and the theme (the pictures' ink) change the rows.
    this.d.add(watchAll([ctx.doc.blocks, ctx.prefs.theme], () => this.schedule()));
    this.d.add(ctx.doc.events.on('changed', () => this.schedule()));
    this.d.add(BlockInsertTool.chosen.subscribe(() => this.writeChosen()));
    this.rebuild();
  }

  /** Lists the rows again once, at the end of the task: an import adds its blocks and inserts one by one. */
  private schedule(): void {
    if (this.queued) return;
    this.queued = true;
    queueMicrotask(() => this.rebuild());
  }

  private rebuild(): void {
    this.queued = false;
    const blocks = this.ctx.doc.blocks.value;
    const placed = placements(blocks, this.ctx.doc.all());
    this.rows = blocks.map((block, i) => ({ id: block.id, block, placed: placed[i] }));
    this.setMeta(blocks.length ? `${blocks.length} blok` : '');
    this.built.clear();
    this.tree.render(this.rows);
  }

  private renderRow(r: Row, content: HTMLElement): void {
    const row = content.parentElement ?? content;
    const canvas = h('canvas', { class: 'blk__thumb', 'aria-hidden': 'true' });
    const text = h(
      'span',
      { class: 'blk__text' },
      h('span', { class: 'tree__name blk__name' }, r.block.name),
      r.block.description ? h('span', { class: 'blk__about' }, r.block.description) : null,
    );
    const count = h('span', { class: 'tree__count num blk__count' }, String(r.placed.drawing));
    content.append(canvas, text, count);
    this.built.set(r.id, row);
    row.toggleAttribute('data-active', r.id === BlockInsertTool.block);
    // The canvas is sized once it is on the page (its CSS size follows the UI scale).
    requestAnimationFrame(() => this.paintThumb(canvas, r.id));
  }

  /** The block as drawn, fitted into its square with a small margin, in the text's ink. */
  private paintThumb(canvas: HTMLCanvasElement, id: string): void {
    if (!canvas.isConnected) return;
    const dpr = window.devicePixelRatio || 1;
    const side = Math.max(1, Math.round((canvas.getBoundingClientRect().width || THUMB) * dpr));
    canvas.width = canvas.height = side;
    const g = canvas.getContext('2d');
    if (!g) return;
    const paths = this.ctx.view.blockOutlines(id, { x: 0, y: 0 });
    drawThumb(g, paths, side, getComputedStyle(canvas).color, dpr);
  }

  private writeChosen(): void {
    for (const [id, row] of this.built) row.toggleAttribute('data-active', id === BlockInsertTool.block);
  }

  private menuFor(r: Row): MenuItem[] {
    const { ctx } = this;
    const via = rebaseInsert(ctx, r.id);
    const refusal = removeRefusal(ctx, r.id);
    const selected = ctx.selection.size;
    const inserts = insertsOf(ctx, r.id).length;
    return [
      { label: 'Blok ekle', icon: 'blockInsert', shortcut: 'Enter', run: () => insertBlock(ctx, r.id) },
      { label: 'Yeniden adlandır', shortcut: 'F2', run: () => this.rename(r) },
      { label: 'Taban noktasını değiştir…', disabled: 'why' in via, detail: 'why' in via ? via.why : undefined, run: () => rebaseBlock(ctx, r.id) },
      {
        label: 'Seçili nesnelerle yeniden tanımla…',
        disabled: !selected,
        detail: selected ? undefined : 'Önce bloğun yeni nesnelerini seçin.',
        run: () => redefineBlock(ctx, r.id),
      },
      { label: 'Yerleştirmelerini seç', icon: 'select', hint: String(inserts), disabled: !inserts, run: () => selectInserts(ctx, r.id) },
      { kind: 'separator' },
      { label: 'Sil', icon: 'trash', shortcut: 'Delete', disabled: !!refusal, detail: refusal ?? undefined, run: () => removeBlock(ctx, r.id) },
    ];
  }

  private remove(r: Row): void {
    const refusal = removeRefusal(this.ctx, r.id);
    if (refusal) return this.ctx.log.warn(refusal);
    removeBlock(this.ctx, r.id);
  }

  /** The name becomes a text box in its row: Enter or leaving it renames, Esc keeps the name. */
  private rename(r: Row): void {
    this.tree.rowOf(r.id);
    const nameEl = this.built.get(r.id)?.querySelector<HTMLElement>('.blk__name');
    if (!nameEl?.isConnected) return;
    const input = h('input', { class: 'field field--inline', value: r.block.name, 'aria-label': 'Blok adı', spellcheck: 'false' });
    nameEl.replaceWith(input);
    input.focus();
    input.select();
    let done = false;
    const finish = (commit: boolean) => {
      if (done) return;
      done = true;
      const name = input.value.trim();
      // A new name rebuilds the rows; otherwise (or refused) the name goes back into the same row.
      if (!(commit && name && name !== r.block.name && renameBlock(this.ctx, r.id, name))) input.replaceWith(nameEl);
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

/** “Çizimde 3 yerleştirme; 1 bloğun içinde” (the row's tip). */
function placedText(p: Placements): string {
  const parts = [p.drawing ? `Çizimde ${p.drawing} yerleştirme` : 'Çizimde yerleştirmesi yok', p.nested ? `${p.nested} bloğun içinde` : ''];
  return parts.filter(Boolean).join('; ');
}

/**
 * Draws outline paths (`flags, n, x0, y0, …`; flags 0 open, 1 closed, 2 a
 * mark) fitted into a `side`² square, north up, 3 CSS pixels in from the
 * edges; marks are small squares. Nothing to draw leaves it empty.
 */
export function drawThumb(g: CanvasRenderingContext2D, paths: Float64Array, side: number, ink: string, dpr: number): void {
  g.clearRect(0, 0, side, side);
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (let i = 0; i + 1 < paths.length; ) {
    const n = paths[i + 1];
    for (let k = 0; k < n; k++) {
      const x = paths[i + 2 + 2 * k];
      const y = paths[i + 3 + 2 * k];
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
    }
    i += 2 + 2 * n;
  }
  if (!Number.isFinite(minX)) return;
  const pad = 3 * dpr;
  const span = Math.max(maxX - minX, maxY - minY) || 1;
  const s = (side - 2 * pad) / span;
  const cx = (minX + maxX) / 2;
  const cy = (minY + maxY) / 2;
  const X = (x: number) => side / 2 + (x - cx) * s;
  const Y = (y: number) => side / 2 - (y - cy) * s;
  g.strokeStyle = ink;
  g.fillStyle = ink;
  g.lineWidth = Math.max(1, dpr);
  g.lineJoin = 'round';
  g.lineCap = 'round';
  for (let i = 0; i + 1 < paths.length; ) {
    const flags = paths[i];
    const n = paths[i + 1];
    if (flags === 2 && n) {
      const m = 1.5 * dpr;
      g.fillRect(X(paths[i + 2]) - m, Y(paths[i + 3]) - m, 2 * m, 2 * m);
    } else if (n) {
      g.beginPath();
      for (let k = 0; k < n; k++) {
        const x = X(paths[i + 2 + 2 * k]);
        const y = Y(paths[i + 3 + 2 * k]);
        if (k) g.lineTo(x, y);
        else g.moveTo(x, y);
      }
      if (flags === 1) g.closePath();
      g.stroke();
    }
    i += 2 + 2 * n;
  }
}
