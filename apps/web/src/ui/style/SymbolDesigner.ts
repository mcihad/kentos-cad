import type { AppContext } from '../../app/context';
import type { LibrarySymbol, Symbol } from '../../model/style';
import type { PreviewGeometry } from '../../render/symbolPreview';
import { sanitizeSvg, svgAsset, validateSymbol } from '../../style/file';
import { newItemId } from '../../style/library';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { askUnsaved } from '../widgets/confirm';
import { Dialog } from '../widgets/Dialog';
import { PopupMenu } from '../widgets/PopupMenu';
import { segmented } from '../widgets/controls';
import {
  addLayer,
  addParent,
  applyPatch,
  canMove,
  canRemove,
  DESIGNER_TEXTS as T,
  designerTitle,
  duplicateLayer,
  GEOMETRIES,
  hasMarker,
  LAYER_LABEL,
  LAYER_TYPES,
  layerAt,
  moveLayer,
  newDraft,
  pathOf,
  putLayer,
  removeLayer,
  savedAs,
  setEnabled,
  summary,
  ZOOM,
  zoomed,
  zoomText,
  type AnyLayer,
  type Draft,
  type Edited,
  type LayerPath,
  type LayerType,
} from './designerModel';
import { layerForm, type FormEnv } from './layerForms';
import { rasterAsset } from './styleFiles';
import { drawNow } from './thumbs';

/**
 * Sembol tasarımcısı (docs/STYLE.md §7, docs/adr/0094): a symbol is a stack
 * of layers. The stack is on the left (a layer that places markers shows the
 * marker's own layers under it), the live preview on sample geometry in the
 * middle, the chosen layer's properties on the right. The designer edits a
 * draft with its own undo (Ctrl+Z / Ctrl+Y and the list's buttons); Kaydet
 * writes it to the library. The model (layer types, new layers, summaries,
 * patches, the list's edits) is designerModel.ts, held with the desktop's to
 * fixtures/style/v1/designer.json.
 */

export interface DesignerOptions {
  /** A user or project symbol to edit. */
  id?: string;
  /** Or a new symbol of this kind, in `path` of `source`. */
  newKind?: Symbol['type'];
  path?: readonly string[];
  source?: 'user' | 'project';
  onSaved?(id: string): void;
  /**
   * Or a symbol that lives in a layer style, not in the library: "Uygula"
   * hands the edited symbol back and closes.
   */
  inline?: { symbol: Symbol; title: string; onDone(symbol: Symbol): void };
}

const HISTORY = 100;

/** A layer type's icon in Katman ekle's menu (the desktop's list draws the same). */
const LAYER_ICON: Record<LayerType, string> = {
  simpleFill: 'fillSolid',
  hatchFill: 'hatch',
  patternFill: 'fillPattern',
  imageFill: 'fillImage',
  centroidMarker: 'snapCentroid',
  simpleLine: 'symbolStroke',
  markerLine: 'symbolLine',
  shape: 'sheetShape',
  svg: 'penTool',
  text: 'text',
  raster: 'sheetPicture',
};

export function openSymbolDesigner(ctx: AppContext, opts: DesignerOptions): void {
  const lib = ctx.styles.library;
  const item = opts.id ? lib.get(opts.id) : undefined;
  if (opts.id && (!item || item.kind !== 'symbol' || !lib.canEdit(opts.id))) {
    ctx.log.error(T.cannotEdit);
    return;
  }
  const kind = item?.kind === 'symbol' ? item.symbol.type : (opts.inline?.symbol.type ?? opts.newKind ?? 'fill');
  const draft: Draft = opts.inline
    ? { name: opts.inline.title, path: [], symbol: structuredClone(opts.inline.symbol) }
    : item?.kind === 'symbol'
      ? { name: item.name, path: [...item.path], symbol: structuredClone(item.symbol) }
      : newDraft(kind, opts.path);
  new SymbolDesigner(ctx, draft, item?.kind === 'symbol' ? item : null, opts);
}

class SymbolDesigner {
  private readonly ctx: AppContext;
  private readonly draft: Draft;
  private original: LibrarySymbol | null;
  private readonly opts: DesignerOptions;
  private readonly dialog: Dialog;
  private readonly list: HTMLElement;
  private readonly props: HTMLElement;
  private readonly canvas: HTMLCanvasElement;
  private readonly previewBar: HTMLElement;
  private readonly status: HTMLElement;
  private readonly nameInput: HTMLInputElement;
  private readonly pathInput: HTMLInputElement;
  /** The list's tools, enabled as the chosen layer and the history allow. */
  private readonly tools: Record<'up' | 'down' | 'dup' | 'remove' | 'undo' | 'redo', HTMLButtonElement>;
  private selected: LayerPath = [0];
  private geometry: PreviewGeometry;
  private pxPerMm: number = ZOOM.start;
  /** The draft as saved, or as opened (a new symbol: the starting one): what `dirty` compares with. */
  private savedJson: string;
  /** The unsaved-changes question is open. */
  private asking = false;
  private readonly past: string[] = [];
  private readonly future: string[] = [];
  private lastEdit = { key: '', at: 0 };
  private frame = 0;

  constructor(ctx: AppContext, draft: Draft, original: LibrarySymbol | null, opts: DesignerOptions) {
    this.ctx = ctx;
    this.draft = draft;
    this.original = original;
    this.opts = opts;
    this.geometry = GEOMETRIES[draft.symbol.type][0].value;
    this.savedJson = JSON.stringify(draft);
    this.list = h('div', { class: 'sdes__layers', role: 'listbox', 'aria-label': 'Sembol katmanları' });
    this.props = h('div', { class: 'sdes__props' });
    this.canvas = h('canvas', { class: 'sdes__canvas', title: 'Tekerlekle yakınlaşıp uzaklaşır' });
    this.previewBar = h('div', { class: 'sdes__pbar' });
    this.status = h('div', { class: 'sdes__status', role: 'status' });
    this.nameInput = h('input', { class: 'field', value: draft.name, 'aria-label': 'Sembol adı', spellcheck: 'false' });
    this.pathInput = h('input', { class: 'field', value: draft.path.join(' / '), 'aria-label': 'Kategori', placeholder: 'Ana / Alt', spellcheck: 'false' });
    this.nameInput.addEventListener('input', () => (this.snapshot('name'), (this.draft.name = this.nameInput.value), this.renderTitle(), this.renderTools()));
    this.pathInput.addEventListener('input', () => (this.snapshot('path'), (this.draft.path = pathOf(this.pathInput.value)), this.renderTitle(), this.renderTools()));

    const addBtn = h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), T.add);
    addBtn.addEventListener('click', () => this.addMenu(addBtn));
    const tool = (name: string, label: string, run: () => void) => {
      const b = h('button', { class: 'ibtn', type: 'button', 'aria-label': label, title: label }, icon(name, 16));
      b.addEventListener('click', run);
      return b;
    };
    this.tools = {
      up: tool('chevronUp', 'Yukarı taşı (önce çizilir)', () => this.move(-1)),
      down: tool('chevronDown', 'Aşağı taşı (sonra çizilir)', () => this.move(1)),
      dup: tool('copy', 'Çoğalt', () => this.duplicate()),
      remove: tool('trash', 'Sil', () => this.remove()),
      undo: tool('undo', 'Geri al (Ctrl+Z)', () => this.undo()),
      redo: tool('redo', 'Yinele (Ctrl+Y)', () => this.redo()),
    };
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    cancel.addEventListener('click', () => this.dialog.request());
    const save = h('button', { class: 'btn btn--primary', type: 'button' }, icon('check', 16), opts.inline ? 'Uygula' : 'Kaydet');
    save.addEventListener('click', () => this.save(false));

    this.dialog = new Dialog({
      title: designerTitle(draft.symbol.type, opts.inline?.title ?? null, false),
      width: 1320,
      className: 'dialog--sdesign',
      content: [
        h(
          'div',
          { class: 'sdes' },
          h(
            'aside',
            { class: 'sdes__left' },
            h('div', { class: 'sdes__lhead' }, h('span', { class: 'sdes__title' }, T.layers), addBtn),
            this.list,
            h('div', { class: 'sdes__ltools' }, this.tools.up, this.tools.down, this.tools.dup, this.tools.remove, h('span', { class: 'sdes__lgap' }), this.tools.undo, this.tools.redo),
            h('p', { class: 'sdes__note' }, T.order),
          ),
          h('section', { class: 'sdes__center' }, this.previewBar, h('div', { class: 'sdes__stage' }, this.canvas)),
          h('aside', { class: 'sdes__right' }, this.props),
        ),
      ],
      footer: [
        opts.inline ? h('span', { class: 'sdes__note' }, T.inline) : null,
        opts.inline ? null : h('label', { class: 'sdes__flabel' }, 'Ad', this.nameInput),
        opts.inline ? null : h('label', { class: 'sdes__flabel sdes__flabel--path' }, 'Kategori', this.pathInput),
        this.status,
        h('div', { class: 'dialog__foot-spacer' }),
        cancel,
        save,
      ],
      beforeClose: () => this.confirmClose(),
      stack: true,
    });
    this.dialog.el.addEventListener('keydown', (e) => {
      if (!(e.ctrlKey || e.metaKey) || (e.target as HTMLElement).closest('input, textarea, select')) return;
      if (e.key.toLowerCase() === 'z' && !e.shiftKey) (e.preventDefault(), this.undo());
      else if (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey)) (e.preventDefault(), this.redo());
    });
    // ↑ ↓ in the list choose the row above or below.
    this.list.addEventListener('keydown', (e) => {
      if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
      e.preventDefault();
      const rows = this.rows();
      const i = rows.findIndex((p) => same(p, this.selected));
      const next = rows[Math.min(rows.length - 1, Math.max(0, i + (e.key === 'ArrowUp' ? -1 : 1)))];
      if (!next || same(next, this.selected)) return;
      this.selected = next;
      this.renderList();
      this.renderProps();
      this.renderTools();
      (this.list.querySelector('[aria-selected="true"]') as HTMLElement | null)?.focus();
    });
    // The wheel over the preview changes its scale, a step a notch.
    this.canvas.addEventListener(
      'wheel',
      (e) => {
        if (!e.deltaY) return;
        e.preventDefault();
        this.zoom(e.deltaY < 0 ? ZOOM.step : 1 / ZOOM.step);
      },
      { passive: false },
    );
    new ResizeObserver(() => this.redraw()).observe(this.canvas);
    this.renderAll();
  }

  // ── Layers ───────────────────────────────────────────────────────────

  private get symbol(): Symbol {
    return this.draft.symbol;
  }

  /** The list's rows in order: each layer, and under a layer that places markers its marker's layers. */
  private rows(): LayerPath[] {
    const out: LayerPath[] = [];
    (this.symbol.layers as readonly AnyLayer[]).forEach((l, i) => {
      out.push([i]);
      if (hasMarker(l)) l.marker.layers.forEach((_, j) => out.push([i, j]));
    });
    return out;
  }

  private addMenu(anchor: HTMLElement): void {
    const r = anchor.getBoundingClientRect();
    const kind = this.symbol.type;
    const parent = addParent(this.symbol, this.selected);
    const items = LAYER_TYPES[kind].map((t) => ({ label: LAYER_LABEL[t], icon: LAYER_ICON[t], run: () => this.add(t, null) }));
    const nested =
      parent !== null
        ? [{ kind: 'header' as const, label: T.intoMarker(LAYER_LABEL[(this.symbol.layers as readonly AnyLayer[])[parent].type]) }, ...LAYER_TYPES.marker.map((t) => ({ label: LAYER_LABEL[t], icon: LAYER_ICON[t], run: () => this.add(t, parent) }))]
        : [];
    PopupMenu.open([{ kind: 'header', label: T.intoSymbol }, ...items, ...(nested.length ? [{ kind: 'separator' as const }, ...nested] : [])], { x: r.left, y: r.bottom + 4 });
  }

  /** A list edit: recorded as its own undo step, done, the chosen row moved. */
  private listEdit(key: string, edit: (s: Symbol) => Edited | null): void {
    const done = edit(this.symbol);
    if (!done) return;
    this.snapshot(key, false);
    this.draft.symbol = done.symbol;
    this.selected = done.selected;
    this.renderAll();
  }

  private add(type: LayerType, parent: number | null): void {
    this.listEdit('add', (s) => addLayer(s, type, parent));
  }

  private move(delta: number): void {
    this.listEdit('move', (s) => moveLayer(s, this.selected, delta));
  }

  private duplicate(): void {
    this.listEdit('dup', (s) => duplicateLayer(s, this.selected));
  }

  private remove(): void {
    if (!canRemove(this.symbol, this.selected)) return this.say(T.lastLayer, 'warn');
    this.listEdit('remove', (s) => removeLayer(s, this.selected));
  }

  // ── Rendering ────────────────────────────────────────────────────────

  private renderAll(): void {
    this.renderList();
    this.renderProps();
    this.renderPreviewBar();
    this.renderTools();
    this.redraw();
    if (this.dialog) this.renderTitle();
  }

  private renderList(): void {
    const rows = this.rows().map((p) => {
      const l = layerAt(this.symbol, p);
      if (!l) return null;
      const conditional = typeof l.enabled === 'object' && l.enabled !== null;
      const on = h('input', { type: 'checkbox', checked: l.enabled !== false, 'aria-label': 'Çizilsin', title: conditional ? 'Koşula bağlı: Görünür alanındaki ifade her nesnede karar verir' : 'Çizilsin' });
      // A condition shows as neither on nor off; a click chooses the row, where Görünür edits it.
      on.indeterminate = conditional;
      on.addEventListener('click', (e) => {
        e.stopPropagation();
        if (!conditional) return;
        e.preventDefault();
        this.choose(p);
      });
      on.addEventListener('change', () => {
        if (conditional) return;
        this.listEdit('enabled', (s) => ({ symbol: setEnabled(s, p, on.checked), selected: this.selected }));
      });
      const selected = same(p, this.selected);
      const r = h(
        'div',
        { class: `sdes__row${p.length === 2 ? ' sdes__row--child' : ''}`, role: 'option', 'aria-selected': String(selected), tabindex: selected ? '0' : '-1' },
        on,
        h('span', { class: 'sdes__rlabel' }, LAYER_LABEL[l.type]),
        h('span', { class: 'sdes__rsum' }, summary(l)),
      );
      r.addEventListener('click', () => this.choose(p));
      return r;
    });
    replaceChildren(this.list, rows);
  }

  private choose(p: LayerPath): void {
    this.selected = p;
    this.renderList();
    this.renderProps();
    this.renderTools();
  }

  /** The list's tools: each enabled when it can act, and saying why not. */
  private renderTools(): void {
    const s = this.symbol;
    const exists = !!layerAt(s, this.selected);
    const set = (b: HTMLButtonElement, on: boolean, why?: string) => {
      b.disabled = !on;
      if (why !== undefined) b.title = why;
    };
    set(this.tools.up, canMove(s, this.selected, -1));
    set(this.tools.down, canMove(s, this.selected, 1));
    set(this.tools.dup, exists);
    set(this.tools.remove, canRemove(s, this.selected), canRemove(s, this.selected) || !exists ? 'Sil' : `Sil: ${T.lastLayer.toLocaleLowerCase('tr')}`);
    set(this.tools.undo, this.past.length > 0);
    set(this.tools.redo, this.future.length > 0);
  }

  private renderProps(): void {
    const l = layerAt(this.symbol, this.selected);
    if (!l) return replaceChildren(this.props, h('p', { class: 'sdes__note' }, T.pickLayer));
    const lib = this.ctx.styles.library;
    const env: FormEnv = {
      palette: this.ctx.view.palette,
      assets: lib.items().filter((i) => i.kind === 'asset').map((a) => ({ id: a.id, name: `${a.name}${a.source === 'system' ? '' : a.source === 'user' ? ' (Kitaplığım)' : ' (Proje)'}`, format: a.kind === 'asset' ? a.format : 'svg' })),
      importSvg: () => this.importSvg(),
      drawSvg: (id) =>
        new Promise((resolve) => {
          void import('../svgedit/SvgEditor').then((m) => m.openSvgEditor(this.ctx, { id, onSaved: (saved) => resolve(saved) }));
        }),
      context: this.selected.length === 2 ? 'marker' : this.symbol.type,
    };
    const p = this.selected;
    const form = layerForm(
      l,
      (patch) => {
        const cur = layerAt(this.symbol, p);
        if (!cur) return;
        this.snapshot(`${p.join('.')}:${Object.keys(patch).join(',')}`);
        this.draft.symbol = putLayer(this.symbol, p, applyPatch(cur, patch));
        // The form stays (focus is kept); list summaries and the preview follow.
        this.renderList();
        this.renderTools();
        this.redraw();
        this.renderTitle();
        if ('type' in patch || 'placement' in patch || 'shape' in patch || 'dash' in patch || 'halo' in patch) queueMicrotask(() => this.keepFocus(() => this.renderProps()));
      },
      env,
    );
    const parent = p.length === 2 ? (this.symbol.layers as readonly AnyLayer[])[p[0]] : null;
    const title = h('div', { class: 'sdes__ptitle' }, h('span', null, LAYER_LABEL[l.type]), parent ? h('span', { class: 'sdes__pctx' }, T.inMarker(LAYER_LABEL[parent.type])) : null);
    replaceChildren(this.props, title, form);
  }

  /** Rebuilds part of the form without losing the field being typed in. */
  private keepFocus(render: () => void): void {
    const label = (document.activeElement as HTMLElement | null)?.getAttribute('aria-label');
    render();
    if (label) (this.props.querySelector(`[aria-label="${CSS.escape(label)}"]`) as HTMLElement | null)?.focus();
  }

  private zoom(factor: number): void {
    this.pxPerMm = zoomed(this.pxPerMm, factor);
    this.renderPreviewBar();
    this.redraw();
  }

  private renderPreviewBar(): void {
    const b = (label: string, text: string, run: () => void, enabled = true) => {
      const el = h('button', { class: 'ibtn', type: 'button', 'aria-label': label, title: label, disabled: !enabled }, text);
      el.addEventListener('click', run);
      return el;
    };
    const options = GEOMETRIES[this.symbol.type];
    replaceChildren(
      this.previewBar,
      options.length > 1 ? segmented({ label: 'Örnek geometri', options, value: this.geometry, onChange: (v) => ((this.geometry = v), this.renderPreviewBar(), this.redraw()) }) : h('span', { class: 'sdes__note' }, T.onePoint),
      h(
        'div',
        { class: 'sdes__zoombar' },
        b('Uzaklaş (tekerlek aşağı)', '−', () => this.zoom(1 / ZOOM.step), this.pxPerMm > ZOOM.min),
        h('span', { class: 'sdes__zoom num', title: 'Kâğıt milimetresinin ekrandaki boyu' }, zoomText(this.pxPerMm)),
        b('Yakınlaş (tekerlek yukarı)', '+', () => this.zoom(ZOOM.step), this.pxPerMm < ZOOM.max),
        b('Gerçek boyut (96 dpi)', '1:1', () => ((this.pxPerMm = ZOOM.real), this.renderPreviewBar(), this.redraw())),
      ),
    );
  }

  private redraw(): void {
    cancelAnimationFrame(this.frame);
    this.frame = requestAnimationFrame(() => drawNow(this.ctx, this.canvas, this.symbol, this.geometry, this.pxPerMm));
  }

  private renderTitle(): void {
    this.dialog.el.querySelector('.dialog__title')?.replaceChildren(designerTitle(this.symbol.type, this.opts.inline?.title ?? null, this.dirty));
  }

  private say(text: string, kind: 'ok' | 'warn' = 'ok'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  // ── History ──────────────────────────────────────────────────────────

  private get dirty(): boolean {
    return JSON.stringify(this.draft) !== this.savedJson;
  }

  /**
   * Records the draft before a change. Typing into one field within a second
   * is one step (`merge`); the list's edits are a step each.
   */
  private snapshot(key: string, merge = true): void {
    const now = performance.now();
    if (merge && key === this.lastEdit.key && now - this.lastEdit.at < 1000) {
      this.lastEdit.at = now;
      return;
    }
    this.lastEdit = { key, at: now };
    this.past.push(JSON.stringify(this.draft));
    if (this.past.length > HISTORY) this.past.shift();
    this.future.length = 0;
  }

  private restore(json: string): void {
    const d = JSON.parse(json) as Draft;
    this.draft.name = d.name;
    this.draft.path = d.path;
    this.draft.symbol = d.symbol;
    this.nameInput.value = d.name;
    this.pathInput.value = d.path.join(' / ');
    if (!layerAt(this.symbol, this.selected)) this.selected = [0];
    this.lastEdit = { key: '', at: 0 };
    this.renderAll();
  }

  private undo(): void {
    const prev = this.past.pop();
    if (!prev) return;
    this.future.push(JSON.stringify(this.draft));
    this.restore(prev);
  }

  private redo(): void {
    const next = this.future.pop();
    if (!next) return;
    this.past.push(JSON.stringify(this.draft));
    this.restore(next);
  }

  // ── Saving ───────────────────────────────────────────────────────────

  private save(thenClose: boolean): boolean {
    const issues = validateSymbol(this.symbol);
    if (issues.length) {
      this.say(T.notSaved(issues[0], issues.length - 1), 'warn');
      return false;
    }
    if (this.opts.inline) {
      this.savedJson = JSON.stringify(this.draft);
      this.opts.inline.onDone(structuredClone(this.symbol));
      this.dialog.close();
      return true;
    }
    const lib = this.ctx.styles.library;
    const { name, path } = savedAs(this.draft);
    let id: string;
    if (this.original) {
      lib.update(this.original.id, { name, path, symbol: this.symbol });
      id = this.original.id;
    } else {
      id = newItemId(this.opts.source === 'project' ? 'p' : 'u');
      lib.add(this.opts.source ?? 'user', { kind: 'symbol', id, name, path, symbol: this.symbol });
      this.original = lib.get(id) as LibrarySymbol;
    }
    this.savedJson = JSON.stringify(this.draft);
    this.renderTitle();
    this.say(T.saved(name));
    this.opts.onSaved?.(id);
    if (thenClose) this.dialog.close();
    return true;
  }

  /** Unsaved changes are asked about in a window over the designer (DESIGN.md §7.9.1); the answer closes or stays. */
  private confirmClose(): boolean {
    if (!this.dirty) return true;
    if (!this.asking) {
      this.asking = true;
      const name = this.opts.inline ? this.opts.inline.title : savedAs(this.draft).name;
      void askUnsaved({ name, after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat', apply: !!this.opts.inline }).then((a) => {
        this.asking = false;
        if (a === 'discard') this.dialog.close();
        else if (a === 'save') this.save(true);
      });
    }
    return false;
  }

  private importSvg(): Promise<string | null> {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.svg,image/svg+xml,image/png,image/jpeg';
      input.addEventListener('change', () => {
        const f = input.files?.[0];
        if (!f) return resolve(null);
        if (/^image\/(png|jpeg)$/.test(f.type)) {
          void rasterAsset(f).then(({ image }) => {
            this.ctx.styles.library.add('user', image);
            this.say(`“${image.name}” Kitaplığım'a eklendi.`);
            resolve(image.id);
          });
          return;
        }
        void f.text().then((text) => {
          const clean = sanitizeSvg(text);
          if (!/<svg\b/i.test(clean)) {
            this.say(`“${f.name}” bir SVG çizimi değil.`, 'warn');
            return resolve(null);
          }
          const asset = svgAsset(f.name.replace(/\.svg$/i, ''), ['Çizimlerim'], clean);
          this.ctx.styles.library.add('user', asset);
          this.say(`“${asset.name}” Kitaplığım'a eklendi.`);
          resolve(asset.id);
        });
      });
      input.click();
    });
  }
}

const same = (a: LayerPath, b: LayerPath) => a.length === b.length && a.every((v, i) => v === b[i]);
