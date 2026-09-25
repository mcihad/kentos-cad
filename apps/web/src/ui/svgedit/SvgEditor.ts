import type { AppContext } from '../../app/context';
import type { LibraryAsset } from '../../model/style';
import { resolveColor } from '../../render/color';
import { sanitizeSvg, svgAsset } from '../../style/file';
import { initSvgCore } from '../../style/svg/core';
import { svgText } from '../../style/svg/exportSvg';
import { importSummary } from '../../style/svg/importSvg';
import { newDoc, shapeId, transformShape, translate, type SvgDoc } from '../../style/svg/svgModel';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { askUnsaved } from '../widgets/confirm';
import { Dialog } from '../widgets/Dialog';
import { readSvg } from './readSvg';
import { EditActions, type ActionsHost, type Tab } from './svgActions';
import { SvgCanvas, type CanvasHost, type CanvasOptions, type ToolId } from './svgCanvas';
import { SvgFiles, type FileHost } from './svgFile';
import { editMenus, viewSwitches, type MenuHost } from './svgMenus';
import { objectRows, type ListHost } from './svgObjects';
import { actionMatrix, renderProps, type ActionName, type PropsHost } from './svgProps';
import { defaultOptions } from './svgView';

/**
 * KentOS's own SVG editor (docs/STYLE.md §7) for the drawings symbols use:
 * pictograms of markers and motifs of pattern fills. Tools on the left with
 * the shape list, the drawing in the middle, the chosen shape's properties
 * on the right. Its own undo; Kaydet writes the drawing to the library as
 * an SVG asset (a system drawing is saved as the user's copy).
 */

export interface SvgEditorOptions {
  /** An SVG asset of the library to open; a new drawing when absent. */
  id?: string;
  path?: readonly string[];
  onSaved?(id: string): void;
}

const TOOLS: { id: ToolId; label: string; key: string; icon: string; hint: string }[] = [
  { id: 'select', label: 'Seç', key: 'V', icon: 'select', hint: 'Tıkla, sürükle, köşelerden boyutlandır, üstteki düğmeden döndür' },
  { id: 'node', label: 'Düğüm', key: 'A', icon: 'vertex', hint: 'Yolun düğümlerini ve kollarını düzenler (yola çift tık da açar)' },
  { id: 'rect', label: 'Dikdörtgen', key: 'R', icon: 'rectangle', hint: 'Sürükleyin; Shift kare, Alt merkezden' },
  { id: 'ellipse', label: 'Elips', key: 'E', icon: 'ellipse', hint: 'Sürükleyin; Shift daire, Alt merkezden' },
  { id: 'polygon', label: 'Çokgen', key: 'P', icon: 'regularPolygon', hint: 'Merkezden sürükleyin; kenar sayısı ve yıldız sağda' },
  { id: 'line', label: 'Kırık çizgi', key: 'L', icon: 'polyline', hint: 'Tıklayarak noktalar; çift tık ya da Enter bitirir, ilk noktaya tık kapatır' },
  { id: 'pen', label: 'Kalem', key: 'B', icon: 'spline', hint: 'Tık köşe, sürükle eğri düğümü; ilk düğüme tık kapatır, Enter bitirir' },
  { id: 'text', label: 'Yazı', key: 'T', icon: 'text', hint: 'Tıklayın, metni sağdan yazın' },
  { id: 'measure', label: 'Ölç', key: 'M', icon: 'measure', hint: 'İki noktayı kenetleyerek ölçer (sürükleyin ya da iki tık); yolun üstünde parça boyları' },
];

/**
 * Opens the editor once its geometry core (a WASM package of its own,
 * CLAUDE.md §20) is running; a core that cannot load leaves the app as it
 * was and says so (the next try loads it again).
 */
export async function openSvgEditor(ctx: AppContext, opts: SvgEditorOptions = {}): Promise<void> {
  try {
    await initSvgCore();
  } catch (e) {
    ctx.log.error(`SVG düzenleyicisi açılamadı: çekirdeği yüklenemedi (${e instanceof Error ? e.message : String(e)}). Ağ bağlantısını denetleyip yeniden açın.`);
    return;
  }
  const lib = ctx.styles.library;
  const asset = opts.id ? lib.get(opts.id) : undefined;
  let doc = newDoc(100, 100);
  let skipped: string[] = [];
  if (asset) {
    if (asset.kind !== 'asset' || asset.format !== 'svg') {
      ctx.log.warn('Yalnızca SVG çizimleri düzenlenebilir; görüntüler (PNG, JPEG) değişmez.');
      return;
    }
    const read = readSvg(asset.data);
    if ('error' in read) {
      ctx.log.error(`“${asset.name}” açılamadı: ${read.error}`);
      return;
    }
    doc = read.doc;
    skipped = importSummary(read.report).lost;
  }
  new SvgEditor(ctx, doc, asset?.kind === 'asset' ? asset : null, opts, skipped);
}

class SvgEditor implements CanvasHost, PropsHost, FileHost, ActionsHost, MenuHost, ListHost {
  readonly ctx: AppContext;
  doc: SvgDoc;
  selection = new Set<string>();
  tool: ToolId = 'select';
  nodeEdit: string | null = null;
  options: CanvasOptions;
  original: LibraryAsset | null;
  private editable: boolean;
  private readonly opts: SvgEditorOptions;
  private readonly dialog: Dialog;
  readonly canvas: SvgCanvas;
  private readonly files: SvgFiles;
  readonly edit: EditActions;
  private readonly switches: { els: HTMLElement[]; update: () => void };
  private readonly toolsEl: HTMLElement;
  private readonly listEl: HTMLElement;
  private readonly propsEl: HTMLElement;
  private readonly statusEl: HTMLElement;
  private readonly zoomEl: HTMLElement;
  private readonly nameInput: HTMLInputElement;
  private readonly pathInput: HTMLInputElement;
  private readonly past: string[] = [];
  private readonly future: string[] = [];
  private pending: string | null = null;
  private lastKey = { key: '', at: 0 };
  /** The drawing, name and category as saved, or as opened: what `dirty` compares with. */
  private savedJson: string;
  private savedMeta = '';
  /** The unsaved-changes question is open. */
  private asking = false;

  constructor(ctx: AppContext, doc: SvgDoc, original: LibraryAsset | null, opts: SvgEditorOptions, skipped: string[]) {
    this.ctx = ctx;
    this.doc = doc;
    this.original = original;
    this.opts = opts;
    this.editable = !!original && ctx.styles.library.canEdit(original.id);
    const pal = ctx.view.palette;
    this.options = defaultOptions(doc, resolveColor('ink', pal), resolveColor('paper', pal));
    this.savedJson = JSON.stringify(doc);
    this.canvas = new SvgCanvas(this);
    this.canvas.el.dataset.escape = 'local';
    this.files = new SvgFiles(this);
    this.edit = new EditActions(this);
    this.switches = viewSwitches(this);
    this.toolsEl = h('div', { class: 'svge__tools' });
    this.listEl = h('div', { class: 'svge__list' });
    this.propsEl = h('div', { class: 'svge__props' });
    this.statusEl = h('div', { class: 'sdes__status', role: 'status' });
    this.zoomEl = h('span', { class: 'sdes__zoom num' });
    const name = original ? (this.editable ? original.name : `${original.name} (kopya)`) : 'Yeni çizim';
    this.nameInput = h('input', { class: 'field', value: name, 'aria-label': 'Çizim adı', spellcheck: 'false' });
    this.pathInput = h('input', { class: 'field', value: (this.editable && original ? original.path : (opts.path ?? ['Çizimlerim'])).join(' / '), 'aria-label': 'Kategori', spellcheck: 'false' });
    this.savedMeta = this.meta();
    for (const f of [this.nameInput, this.pathInput]) f.addEventListener('input', () => this.renderTitle());
    const bar = (label: string, text: string, run: () => void) => {
      const b = h('button', { class: 'ibtn', type: 'button', title: label, 'aria-label': label }, text);
      b.addEventListener('click', run);
      return b;
    };
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    cancel.addEventListener('click', () => this.dialog.request());
    const save = h('button', { class: 'btn btn--primary', type: 'button' }, icon('check', 16), 'Kaydet');
    save.addEventListener('click', () => this.save(false));
    const root = h(
      'div',
      { class: 'svge' },
      h('aside', { class: 'svge__left' }, this.toolsEl, h('div', { class: 'sdes__title svge__ltitle' }, 'Şekiller (öndeki üstte)'), this.listEl),
      h(
        'section',
        { class: 'svge__center' },
        h('div', { class: 'sdes__pbar svge__pbar' }, ...this.files.buttons, h('span', { class: 'svge__barsep' }), ...editMenus(this), h('div', { class: 'dialog__foot-spacer' }), ...this.switches.els, bar('Uzaklaş', '−', () => this.canvas.zoomBy(1 / 1.25)), this.zoomEl, bar('Yakınlaş', '+', () => this.canvas.zoomBy(1.25)), bar('Tuvale sığdır (0)', '⤢', () => this.canvas.fit())),
        this.files.reference.bar,
        this.canvas.el,
        this.files.source.el,
      ),
      h('aside', { class: 'svge__right' }, this.propsEl),
    );
    root.addEventListener('keydown', (e) => this.key(e));
    this.files.attach(root);
    this.dialog = new Dialog({
      title: 'SVG çizim düzenleyicisi',
      width: 1320,
      className: 'dialog--sdesign dialog--svge',
      content: [root],
      footer: [h('label', { class: 'sdes__flabel' }, 'Ad', this.nameInput), h('label', { class: 'sdes__flabel sdes__flabel--path' }, 'Kategori', this.pathInput), this.statusEl, h('div', { class: 'dialog__foot-spacer' }), cancel, this.files.saveAsButton, save],
      beforeClose: () => this.confirmClose(),
      // The preview colours follow a theme switch while the window is open.
      onClose: () => themeSub(),
      stack: true,
    });
    const themeSub = ctx.ui.theme.subscribe(() => this.refresh());
    if (skipped.length) this.status(`Açılırken: ${skipped.join(', ')}.`, 'warn');
    else if (original && !this.editable) this.status('Sistem çizimi: kaydedince Kitaplığım\'a kopyası yazılır.');
    this.refresh();
    queueMicrotask(() => this.canvas.el.focus());
  }

  // ── Host interfaces ──────────────────────────────────────────────────

  begin(): void {
    this.pending ??= JSON.stringify(this.doc);
  }

  commit(label: string): void {
    const before = this.pending;
    this.pending = null;
    if (before && label && before !== JSON.stringify(this.doc)) {
      this.past.push(before);
      if (this.past.length > 100) this.past.shift();
      this.future.length = 0;
      this.lastKey = { key: '', at: 0 };
    }
    this.refresh();
  }

  changed(): void {
    this.canvas.render();
  }

  change(key: string, fn: () => void): void {
    const now = performance.now();
    if (key !== this.lastKey.key || now - this.lastKey.at > 1000) {
      this.past.push(JSON.stringify(this.doc));
      if (this.past.length > 100) this.past.shift();
      this.future.length = 0;
    }
    this.lastKey = { key, at: now };
    fn();
    this.canvas.invalidate();
    this.canvas.render();
    this.renderList();
    this.renderTitle();
    this.files.refresh();
  }

  setOption(patch: Partial<CanvasOptions>): void {
    this.options = { ...this.options, ...patch };
    if ('tile' in patch) this.canvas.fit();
    this.refresh();
  }

  select(ids: string[]): void {
    this.selection = new Set(ids);
    if (this.nodeEdit && !this.selection.has(this.nodeEdit)) this.nodeEdit = null;
    this.refresh();
    this.edit.previewArray();
  }

  setTool(t: ToolId): void {
    this.canvas.toolChanged();
    this.tool = t;
    if (t !== 'node') this.nodeEdit = null;
    else if (!this.nodeEdit) {
      const path = this.doc.shapes.find((s) => this.selection.has(s.id) && s.kind === 'path');
      this.nodeEdit = path?.id ?? null;
      const shape = this.doc.shapes.find((s) => this.selection.has(s.id) && (s.kind === 'rect' || s.kind === 'ellipse'));
      if (!path) this.status(shape ? 'Dikdörtgen ve elips düğümle düzenlenmez: önce Yol → Nesneyi yola çevir (Ctrl+Shift+C).' : 'Düğüm düzenlemek için bir yol seçin ya da yola çift tıklayın.', shape ? 'warn' : 'ok');
    }
    if (t === 'measure') this.status('Ölç: iki noktayı tıklayın ya da sürükleyin (kenetlenir); bir yolun üstünde durunca parça boyları görünür.');
    this.refresh();
  }

  editNodes(id: string | null): void {
    if (this.canvas.nodes.mode && !id) this.canvas.nodes.setMode(null);
    this.nodeEdit = id;
    this.tool = id ? 'node' : 'select';
    if (id) this.selection = new Set([id]);
    this.refresh();
  }

  status(text: string, kind: 'ok' | 'warn' = 'ok'): void {
    this.statusEl.textContent = text;
    this.statusEl.dataset.kind = kind;
  }

  action(name: ActionName): void {
    const sel = this.doc.shapes.filter((s) => this.selection.has(s.id));
    if (!sel.length) return;
    const ids = this.selection;
    const shapes = this.doc.shapes;
    this.change(`act:${name}`, () => {
      switch (name) {
        case 'delete':
          // A locked shape stays (unlock it in the list first).
          this.doc.shapes = shapes.filter((s) => !ids.has(s.id) || s.locked);
          this.selection = new Set();
          this.nodeEdit = null;
          return;
        case 'duplicate': {
          const group = new Map<string, string>();
          const copies = sel.map((s) => {
            const g = s.group ? (group.get(s.group) ?? group.set(s.group, shapeId()).get(s.group)) : undefined;
            return { ...transformShape(structuredClone(s), translate(this.options.grid || 2, this.options.grid || 2)), id: shapeId(), group: g };
          });
          this.doc.shapes = [...shapes, ...copies];
          this.selection = new Set(copies.map((c) => c.id));
          return;
        }
        case 'front':
        case 'back': {
          const rest = shapes.filter((s) => !ids.has(s.id));
          this.doc.shapes = name === 'front' ? [...rest, ...sel] : [...sel, ...rest];
          return;
        }
        case 'raise':
        case 'lower': {
          const list = [...shapes];
          const order = name === 'raise' ? [...list.keys()].reverse() : [...list.keys()];
          for (const i of order) {
            const j = name === 'raise' ? i + 1 : i - 1;
            if (ids.has(list[i].id) && j >= 0 && j < list.length && !ids.has(list[j].id)) [list[i], list[j]] = [list[j], list[i]];
          }
          this.doc.shapes = list;
          return;
        }
        case 'group': {
          const g = shapeId();
          this.doc.shapes = shapes.map((s) => (ids.has(s.id) ? { ...s, group: g } : s));
          // Group members sit together in the stack (the file writes them in one <g>).
          const members = this.doc.shapes.filter((s) => ids.has(s.id));
          const at = this.doc.shapes.findIndex((s) => ids.has(s.id));
          const rest = this.doc.shapes.filter((s) => !ids.has(s.id));
          rest.splice(at, 0, ...members);
          this.doc.shapes = rest;
          return;
        }
        case 'ungroup':
          this.doc.shapes = shapes.map((s) => (ids.has(s.id) ? { ...s, group: undefined } : s));
          return;
        default: {
          const matrix = actionMatrix(name, sel, this.doc);
          this.doc.shapes = shapes.map((s) => {
            if (!ids.has(s.id) || s.locked) return s;
            const m = matrix(s);
            return m ? transformShape(s, m) : s;
          });
        }
      }
    });
    this.refresh();
  }

  // ── Rendering ────────────────────────────────────────────────────────

  refresh(): void {
    // The symbol colour follows the theme until one is picked (the paper follows it in files.refresh).
    if (this.options.inkAuto) {
      const ink = resolveColor('ink', this.ctx.view.palette);
      if (ink !== this.options.ink) this.options = { ...this.options, ink };
    }
    this.canvas.invalidate();
    this.switches?.update();
    this.canvas.render();
    this.renderTools();
    this.renderList();
    replaceChildren(this.propsEl, renderProps(this));
    this.zoomed(this.canvas.scale);
    this.renderTitle();
    this.files.refresh();
  }

  private renderTitle(): void {
    this.dialog?.el.querySelector('.dialog__title')?.replaceChildren(`SVG çizim düzenleyicisi${this.dirty ? ' •' : ''}`);
  }

  private renderTools(): void {
    replaceChildren(
      this.toolsEl,
      TOOLS.map((t) => {
        const b = h('button', { class: 'svge__tool', type: 'button', 'aria-pressed': String(this.tool === t.id), title: `${t.label} (${t.key}): ${t.hint}` }, icon(t.icon, 18), h('span', null, t.label), h('kbd', null, t.key));
        b.addEventListener('click', () => this.setTool(t.id));
        return b;
      }),
    );
  }

  private renderList(): void {
    const rows = objectRows(this, this.listEl);
    replaceChildren(this.listEl, rows.length ? rows : h('p', { class: 'sdes__note' }, 'Henüz şekil yok: soldaki araçlarla çizin ya da bir SVG dosyası ekleyin.'));
  }

  /** The chosen nodes changed: the node box follows. */
  nodesChanged(): void {
    replaceChildren(this.propsEl, renderProps(this));
  }

  focusCanvas(): void {
    this.canvas.el.focus();
  }

  showTab(tab: Tab): void {
    this.edit.ui.tab = tab;
    this.edit.previewArray();
    this.refresh();
  }

  // ── Keys, history, files ─────────────────────────────────────────────

  private key(e: KeyboardEvent): void {
    const inField = !!(e.target as HTMLElement).closest('input, textarea, select');
    const k = e.key;
    const ctrl = e.ctrlKey || e.metaKey;
    const run = (fn: () => void) => {
      e.preventDefault();
      fn();
    };
    if (k === 'Escape' && !inField) {
      e.preventDefault();
      if (this.canvas.cancel()) return;
      if (this.nodeEdit) return this.editNodes(null);
      if (this.selection.size) return this.select([]);
      this.dialog.request();
      return;
    }
    if (inField) return;
    const nodes = this.nodeEdit && this.tool === 'node' ? this.canvas.nodes : null;
    if (ctrl && k.toLowerCase() === 'z') return run(() => (e.shiftKey ? this.redo() : this.undo()));
    if (ctrl && k.toLowerCase() === 'y') return run(() => this.redo());
    if (ctrl && k.toLowerCase() === 'd') return run(() => this.action('duplicate'));
    if (ctrl && k.toLowerCase() === 'g') return run(() => this.action(e.shiftKey ? 'ungroup' : 'group'));
    if (ctrl && !e.shiftKey && k.toLowerCase() === 'a') return run(() => (nodes ? this.edit.selectAllNodes() : this.edit.selectAll()));
    if (ctrl) {
      // Path operations and panels (Inkscape's keys).
      const low = k.toLowerCase();
      if ((k === '+' || k === '=') && !e.altKey) return run(() => this.edit.path('union'));
      if (k === '-' && !e.altKey) return run(() => this.edit.path('difference'));
      if (k === '*') return run(() => this.edit.path('intersection'));
      if (k === '^') return run(() => this.edit.path('exclusion'));
      if (k === '/') return run(() => this.edit.path(e.altKey ? 'cut' : 'division'));
      if (k === '(') return run(() => this.edit.path('inset'));
      if (k === ')') return run(() => this.edit.path('outset'));
      if (low === 'k') return run(() => this.edit.path(e.shiftKey ? 'breakApart' : 'combine'));
      if (low === 'c' && e.shiftKey) return run(() => this.edit.path('toPath'));
      if (low === 'c' && e.altKey) return run(() => this.edit.path('strokeToPath'));
      if (low === 'l' && !e.shiftKey) return run(() => this.edit.path('simplify'));
      if (low === 'a' && e.shiftKey) return run(() => this.showTab('align'));
      if (low === 'm' && e.shiftKey) return run(() => this.showTab('transform'));
      if (nodes && (k === 'Delete' || k === 'Backspace')) return run(() => this.edit.nodeDelete(false));
      return;
    }
    if (nodes) {
      // The node tool's keys come before the tools' letters.
      const up = k.toUpperCase();
      if (e.shiftKey && !e.altKey) {
        const typed = { C: 'cusp', S: 'smooth', Y: 'symmetric', A: 'auto' } as const;
        if (up in typed) return run(() => this.edit.nodeType(typed[up as keyof typeof typed]));
        if (up === 'J') return run(() => this.edit.nodeJoin(true));
        if (up === 'K') return run(() => this.edit.nodeJoin(false));
        if (up === 'B') return run(() => this.edit.nodeBreak());
        if (up === 'L') return run(() => this.edit.nodeSegments('line'));
        if (up === 'U') return run(() => this.edit.nodeSegments('curve'));
      }
      if (k === 'Insert') return run(() => this.edit.nodeInsert());
      if ((k === 'Delete' || k === 'Backspace') && nodes.selected.length) return run(() => (e.altKey ? this.edit.nodeDeleteSegment() : this.edit.nodeDelete(true)));
      if (k.startsWith('Arrow') && nodes.selected.length) {
        const step = (this.options.grid || 1) * (e.shiftKey ? 5 : 1);
        const d: [number, number] = k === 'ArrowLeft' ? [-step, 0] : k === 'ArrowRight' ? [step, 0] : k === 'ArrowUp' ? [0, -step] : [0, step];
        return run(() => this.edit.nodeNudge(d[0], d[1]));
      }
    }
    if (k === 'Delete' || k === 'Backspace') return run(() => this.action('delete'));
    if (k === 'Enter') return run(() => this.canvas.finishDraft(false));
    if (k === 'PageUp') return run(() => this.edit.restack('raise'));
    if (k === 'PageDown') return run(() => this.edit.restack('lower'));
    if (k === 'Home') return run(() => this.edit.restack('top'));
    if (k === 'End') return run(() => this.edit.restack('bottom'));
    if (k === '!') return run(() => this.edit.invertSelection());
    if (k === '%') return run(() => this.setOption({ snapObjects: !this.options.snapObjects }));
    if (k.startsWith('Arrow') && this.selection.size) {
      e.preventDefault();
      const step = (this.options.grid || 1) * (e.shiftKey ? 5 : 1);
      const d: [number, number] = k === 'ArrowLeft' ? [-step, 0] : k === 'ArrowRight' ? [step, 0] : k === 'ArrowUp' ? [0, -step] : [0, step];
      return this.change('nudge', () => (this.doc.shapes = this.doc.shapes.map((s) => (this.selection.has(s.id) && !s.locked ? transformShape(s, translate(d[0], d[1])) : s))));
    }
    if (k === '0') return this.canvas.fit();
    if (k === '+' || k === '=') return this.canvas.zoomBy(1.25);
    if (k === '-') return this.canvas.zoomBy(1 / 1.25);
    if (!e.altKey && (k === 'h' || k === 'H')) return run(() => this.action(e.shiftKey ? 'flipV' : 'flipH'));
    if (e.shiftKey || e.altKey) return;
    const tool = TOOLS.find((t) => t.key === k.toLocaleUpperCase('tr').replace('İ', 'I'));
    if (tool) this.setTool(tool.id);
  }

  /** What Kaydet would write differs from what was saved or opened (a new drawing: from the blank one). */
  get dirty(): boolean {
    return JSON.stringify(this.doc) !== this.savedJson || this.meta() !== this.savedMeta;
  }

  private meta(): string {
    return `${this.nameInput.value}\n${this.pathInput.value}`;
  }

  /** Kaydet writes nothing without a visible shape (it says so); closing such a drawing loses nothing. */
  private get savable(): boolean {
    return this.doc.shapes.some((s) => !s.hidden);
  }

  private restore(json: string): void {
    this.doc = JSON.parse(json) as SvgDoc;
    this.selection = new Set([...this.selection].filter((id) => this.doc.shapes.some((s) => s.id === id)));
    if (this.nodeEdit && !this.doc.shapes.some((s) => s.id === this.nodeEdit)) this.nodeEdit = null;
    this.lastKey = { key: '', at: 0 };
    this.refresh();
  }

  private undo(): void {
    const prev = this.past.pop();
    if (!prev) return;
    this.future.push(JSON.stringify(this.doc));
    this.restore(prev);
  }

  private redo(): void {
    const next = this.future.pop();
    if (!next) return;
    this.past.push(JSON.stringify(this.doc));
    this.restore(next);
  }

  // ── Files (svgFile.ts) ───────────────────────────────────────────────

  get name(): string {
    return this.nameInput.value.trim() || 'Adsız çizim';
  }

  get path(): string[] {
    return this.pathInput.value.split('/').map((s) => s.trim()).filter(Boolean);
  }

  zoomed(scale: number): void {
    this.zoomEl.textContent = `%${Math.round(scale * 100)}`;
  }

  underlay(world: SVGGElement): void {
    // The canvas may draw before the files are set up.
    this.files?.underlay(world);
  }

  /** Another drawing in the window: a file opened as new, or a library drawing; history starts anew. */
  open(doc: SvgDoc, asset: LibraryAsset | null, name: string): void {
    this.doc = doc;
    this.past.length = 0;
    this.future.length = 0;
    this.pending = null;
    this.lastKey = { key: '', at: 0 };
    this.original = asset;
    this.editable = !!asset && this.ctx.styles.library.canEdit(asset.id);
    this.nameInput.value = name;
    if (asset) this.pathInput.value = asset.path.join(' / ');
    this.savedJson = JSON.stringify(doc);
    this.savedMeta = this.meta();
    this.selection = new Set();
    this.nodeEdit = null;
    this.options = { ...this.options, grid: Math.max(1, Math.round(doc.width / 20)) };
    this.refresh();
    this.canvas.fit();
  }

  /** The drawing was saved as this library item (Farklı kaydet). */
  adopt(asset: LibraryAsset): void {
    this.original = asset;
    this.editable = true;
    this.nameInput.value = asset.name;
    this.pathInput.value = asset.path.join(' / ');
    this.savedJson = JSON.stringify(this.doc);
    this.savedMeta = this.meta();
    this.renderTitle();
    this.opts.onSaved?.(asset.id);
  }

  private save(thenClose: boolean): boolean {
    const lib = this.ctx.styles.library;
    if (!this.savable) {
      this.status('Boş çizim kaydedilmez: önce bir şekil çizin.', 'warn');
      return false;
    }
    const svg = sanitizeSvg(svgText(this.doc, { reference: this.files.keptReference() }));
    const name = this.nameInput.value.trim() || 'Adsız çizim';
    const path = this.pathInput.value.split('/').map((s) => s.trim()).filter(Boolean);
    let id: string;
    if (this.original && this.editable) {
      lib.update(this.original.id, { name, path: path.length ? path : ['Çizimlerim'], data: svg, width: this.doc.width, height: this.doc.height });
      id = this.original.id;
    } else {
      const asset = svgAsset(name, path.length ? path : ['Çizimlerim'], svg);
      lib.add('user', asset);
      id = asset.id;
      this.original = asset;
      this.editable = true;
    }
    this.savedJson = JSON.stringify(this.doc);
    this.savedMeta = this.meta();
    this.renderTitle();
    this.status(`“${name}” kaydedildi.`);
    this.opts.onSaved?.(id);
    if (thenClose) this.dialog.close();
    return true;
  }

  /** Saves for the file actions (Yeni, Aç) before they replace the drawing; false when Kaydet refused. */
  saveDrawing(): boolean {
    return this.save(false);
  }

  /** Unsaved changes are asked about in a window over the editor (DESIGN.md §7.9.1); the answer closes or stays. */
  private confirmClose(): boolean {
    if (!this.dirty || !this.savable) return true;
    if (!this.asking) {
      this.asking = true;
      void askUnsaved({ name: this.name, after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat' }).then((a) => {
        this.asking = false;
        if (a === 'discard') this.dialog.close();
        else if (a === 'save') this.save(true);
      });
    }
    return false;
  }
}
