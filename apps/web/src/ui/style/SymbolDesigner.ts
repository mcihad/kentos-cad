import type { AppContext } from '../../app/context';
import type { LibrarySymbol, MarkerLayer, Symbol } from '../../model/style';
import type { PreviewGeometry } from '../../render/symbolPreview';
import { sanitizeSvg, svgAsset, validateSymbol } from '../../style/file';
import { newItemId } from '../../style/library';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { PopupMenu } from '../widgets/PopupMenu';
import { segmented } from '../widgets/controls';
import { applyPatch, hasMarker, LAYER_LABEL, LAYER_TYPES, layerForm, newLayer, summary, type AnyLayer, type FormEnv, type LayerType } from './layerForms';
import { rasterAsset } from './styleFiles';
import { drawNow } from './thumbs';

/**
 * Sembol tasarımcısı (docs/STYLE.md §7): a symbol is a stack of layers.
 * The stack is on the left (a layer that places markers shows the marker's
 * own layers under it), the live preview on sample geometry in the middle,
 * the chosen layer's properties on the right. The designer edits a draft
 * with its own undo (Ctrl+Z / Ctrl+Y); Kaydet writes it to the library.
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

/** Where a layer is: [i] a symbol layer, [i, j] layer j of the marker of layer i. */
type LayerPath = readonly [number] | readonly [number, number];

interface Draft {
  name: string;
  path: string[];
  symbol: Symbol;
}

const KIND_TITLE: Record<Symbol['type'], string> = { fill: 'Alan sembolü', line: 'Çizgi sembolü', marker: 'İşaret sembolü' };
const GEOMETRIES: Record<Symbol['type'], { value: PreviewGeometry; label: string }[]> = {
  fill: [
    { value: 'area', label: 'Alan' },
    { value: 'hole', label: 'Adalı alan' },
  ],
  line: [
    { value: 'line', label: 'Düz' },
    { value: 'bent', label: 'Kırık' },
    { value: 'area', label: 'Alan kenarı' },
  ],
  marker: [{ value: 'point', label: 'Nokta' }],
};
const HISTORY = 100;

export function openSymbolDesigner(ctx: AppContext, opts: DesignerOptions): void {
  const lib = ctx.styles.library;
  const item = opts.id ? lib.get(opts.id) : undefined;
  if (opts.id && (!item || item.kind !== 'symbol' || !lib.canEdit(opts.id))) {
    ctx.log.error('Bu sembol düzenlenemez: sistem sembollerinin kopyası düzenlenir.');
    return;
  }
  const kind = item?.kind === 'symbol' ? item.symbol.type : (opts.inline?.symbol.type ?? opts.newKind ?? 'fill');
  const draft: Draft = opts.inline
    ? { name: opts.inline.title, path: [], symbol: structuredClone(opts.inline.symbol) }
    : item?.kind === 'symbol'
    ? { name: item.name, path: [...item.path], symbol: structuredClone(item.symbol) }
    : { name: `Yeni ${KIND_TITLE[kind].toLocaleLowerCase('tr')}`, path: [...(opts.path ?? ['Sembollerim'])], symbol: { type: kind, layers: [newLayer(LAYER_TYPES[kind][0], '0', kind)] } as Symbol };
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
  private selected: LayerPath = [0];
  private geometry: PreviewGeometry;
  private pxPerMm = 4;
  private savedJson: string;
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
    this.savedJson = original || opts.inline ? JSON.stringify(draft) : '';
    this.list = h('div', { class: 'sdes__layers', role: 'listbox', 'aria-label': 'Sembol katmanları' });
    this.props = h('div', { class: 'sdes__props' });
    this.canvas = h('canvas', { class: 'sdes__canvas' });
    this.previewBar = h('div', { class: 'sdes__pbar' });
    this.status = h('div', { class: 'sdes__status', role: 'status' });
    this.nameInput = h('input', { class: 'field', value: draft.name, 'aria-label': 'Sembol adı', spellcheck: 'false' });
    this.pathInput = h('input', { class: 'field', value: draft.path.join(' / '), 'aria-label': 'Kategori', placeholder: 'Ana / Alt', spellcheck: 'false' });
    this.nameInput.addEventListener('input', () => ((this.draft.name = this.nameInput.value), this.touched('name')));
    this.pathInput.addEventListener('input', () => ((this.draft.path = this.pathInput.value.split('/').map((s) => s.trim()).filter(Boolean)), this.touched('path')));

    const addBtn = h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), 'Katman ekle');
    addBtn.addEventListener('click', () => this.addMenu(addBtn));
    const tool = (name: string, label: string, run: () => void) => {
      const b = h('button', { class: 'ibtn', type: 'button', 'aria-label': label, title: label }, icon(name, 16));
      b.addEventListener('click', run);
      return b;
    };
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    cancel.addEventListener('click', () => this.dialog.request());
    const save = h('button', { class: 'btn btn--primary', type: 'button' }, icon('check', 16), opts.inline ? 'Uygula' : 'Kaydet');
    save.addEventListener('click', () => this.save(false));

    this.dialog = new Dialog({
      title: opts.inline ? `${opts.inline.title}: ${KIND_TITLE[draft.symbol.type].toLocaleLowerCase('tr')}` : `${KIND_TITLE[draft.symbol.type]} tasarımcısı`,
      width: 1320,
      className: 'dialog--sdesign',
      content: [
        h(
        'div',
        { class: 'sdes' },
        h(
          'aside',
          { class: 'sdes__left' },
          h('div', { class: 'sdes__lhead' }, h('span', { class: 'sdes__title' }, 'Katmanlar'), addBtn),
          this.list,
          h('div', { class: 'sdes__ltools' }, tool('chevronUp', 'Yukarı taşı (önce çizilir)', () => this.move(-1)), tool('chevronDown', 'Aşağı taşı (sonra çizilir)', () => this.move(1)), tool('copy', 'Çoğalt', () => this.duplicate()), tool('trash', 'Sil', () => this.remove())),
          h('p', { class: 'sdes__note' }, 'Listede üstteki katman önce, alttaki en son (en üstte) çizilir.'),
        ),
        h('section', { class: 'sdes__center' }, this.previewBar, h('div', { class: 'sdes__stage' }, this.canvas)),
        h('aside', { class: 'sdes__right' }, this.props),
        ),
      ],
      footer: [
        opts.inline ? h('span', { class: 'sdes__note' }, 'Bu sembol katman stilinin içindedir; kitaplığa yazılmaz.') : null,
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
    new ResizeObserver(() => this.redraw()).observe(this.canvas);
    this.renderAll();
  }

  // ── Layers ───────────────────────────────────────────────────────────

  private get layers(): AnyLayer[] {
    return this.draft.symbol.layers as AnyLayer[];
  }

  private layerAt(p: LayerPath): AnyLayer | undefined {
    const top = this.layers[p[0]];
    if (p.length === 1 || !top) return top;
    return hasMarker(top) ? (top.marker.layers[p[1]] as AnyLayer | undefined) : undefined;
  }

  /** Replaces the layer at `p` (the marker of its parent for child rows). */
  private putLayer(p: LayerPath, l: AnyLayer): void {
    if (p.length === 1) {
      this.draft.symbol = { ...this.draft.symbol, layers: this.layers.map((x, i) => (i === p[0] ? l : x)) } as Symbol;
      return;
    }
    const parent = this.layers[p[0]];
    if (!hasMarker(parent)) return;
    const marker = { ...parent.marker, layers: parent.marker.layers.map((x, j) => (j === p[1] ? (l as MarkerLayer) : x)) };
    this.putLayer([p[0]], { ...parent, marker } as AnyLayer);
  }

  /** The list a layer lives in, and the setter for it. */
  private siblings(p: LayerPath): { list: AnyLayer[]; set: (list: AnyLayer[]) => void; index: number } {
    if (p.length === 1) return { list: [...this.layers], set: (list) => (this.draft.symbol = { ...this.draft.symbol, layers: list } as Symbol), index: p[0] };
    const parent = this.layers[p[0]];
    const list = hasMarker(parent) ? [...(parent.marker.layers as AnyLayer[])] : [];
    return { list, set: (l) => hasMarker(parent) && this.putLayer([p[0]], { ...parent, marker: { type: 'marker', layers: l as MarkerLayer[] } } as AnyLayer), index: p[1] };
  }

  private uid(list: readonly AnyLayer[]): string {
    const used = new Set(list.map((l) => l.id));
    let i = list.length;
    while (used.has(String(i))) i++;
    return String(i);
  }

  private addMenu(anchor: HTMLElement): void {
    const r = anchor.getBoundingClientRect();
    const kind = this.draft.symbol.type;
    const sel = this.layerAt(this.selected);
    const parentIndex = this.selected.length === 2 ? this.selected[0] : sel && hasMarker(sel) ? this.selected[0] : null;
    const items = LAYER_TYPES[kind].map((t) => ({ label: LAYER_LABEL[t], run: () => this.add(t, null) }));
    const nested =
      parentIndex !== null
        ? [{ kind: 'header' as const, label: `“${LAYER_LABEL[this.layers[parentIndex].type]}” işaretine` }, ...LAYER_TYPES.marker.map((t) => ({ label: LAYER_LABEL[t], run: () => this.add(t, parentIndex) }))]
        : [];
    PopupMenu.open([{ kind: 'header', label: 'Sembole' }, ...items, ...(nested.length ? [{ kind: 'separator' as const }, ...nested] : [])], { x: r.left, y: r.bottom + 4 });
  }

  private add(type: LayerType, parent: number | null): void {
    this.snapshot('add');
    if (parent === null) {
      const layer = newLayer(type, this.uid(this.layers), this.draft.symbol.type);
      this.draft.symbol = { ...this.draft.symbol, layers: [...this.layers, layer] } as Symbol;
      this.selected = [this.layers.length - 1];
    } else {
      const { list, set } = this.siblings([parent, 0]);
      const layer = newLayer(type, this.uid(list), 'marker');
      set([...list, layer]);
      this.selected = [parent, list.length];
    }
    this.renderAll();
  }

  private move(delta: number): void {
    const { list, set, index } = this.siblings(this.selected);
    const to = index + delta;
    if (to < 0 || to >= list.length) return;
    this.snapshot('move');
    [list[index], list[to]] = [list[to], list[index]];
    set(list);
    this.selected = this.selected.length === 1 ? [to] : [this.selected[0], to];
    this.renderAll();
  }

  private duplicate(): void {
    const { list, set, index } = this.siblings(this.selected);
    if (!list[index]) return;
    this.snapshot('dup');
    const copy = { ...structuredClone(list[index]), id: this.uid(list) };
    list.splice(index + 1, 0, copy);
    set(list);
    this.selected = this.selected.length === 1 ? [index + 1] : [this.selected[0], index + 1];
    this.renderAll();
  }

  private remove(): void {
    const { list, set, index } = this.siblings(this.selected);
    if (!list[index]) return;
    if (this.selected.length === 1 && list.length === 1) return this.say('Sembolün en az bir katmanı olmalı.', 'warn');
    this.snapshot('remove');
    list.splice(index, 1);
    set(list);
    const next = Math.min(index, list.length - 1);
    this.selected = this.selected.length === 1 ? [Math.max(0, next)] : next < 0 ? [this.selected[0]] : [this.selected[0], next];
    this.renderAll();
  }

  // ── Rendering ────────────────────────────────────────────────────────

  private renderAll(): void {
    this.renderList();
    this.renderProps();
    this.renderPreviewBar();
    this.redraw();
    if (this.dialog) this.renderTitle();
  }

  private renderList(): void {
    const rows: HTMLElement[] = [];
    const rowOf = (l: AnyLayer, p: LayerPath) => {
      const on = h('input', { type: 'checkbox', checked: l.enabled !== false, 'aria-label': 'Çizilsin', title: typeof l.enabled === 'object' ? 'Koşula bağlı' : 'Çizilsin' });
      on.addEventListener('click', (e) => e.stopPropagation());
      on.addEventListener('change', () => {
        this.snapshot('enabled');
        this.putLayer(p, { ...l, enabled: on.checked ? undefined : false } as AnyLayer);
        this.renderAll();
      });
      const selected = p.length === this.selected.length && p.every((v, i) => v === this.selected[i]);
      const r = h(
        'div',
        { class: `sdes__row${p.length === 2 ? ' sdes__row--child' : ''}`, role: 'option', 'aria-selected': String(selected), tabindex: selected ? '0' : '-1' },
        on,
        h('span', { class: 'sdes__rlabel' }, LAYER_LABEL[l.type]),
        h('span', { class: 'sdes__rsum' }, summary(l)),
      );
      r.addEventListener('click', () => {
        this.selected = p;
        this.renderList();
        this.renderProps();
      });
      rows.push(r);
    };
    this.layers.forEach((l, i) => {
      rowOf(l, [i]);
      if (hasMarker(l)) l.marker.layers.forEach((m, j) => rowOf(m as AnyLayer, [i, j]));
    });
    replaceChildren(this.list, rows);
  }

  private renderProps(): void {
    const l = this.layerAt(this.selected);
    if (!l) return replaceChildren(this.props, h('p', { class: 'sdes__note' }, 'Bir katman seçin.'));
    const lib = this.ctx.styles.library;
    const env: FormEnv = {
      palette: this.ctx.view.palette,
      assets: lib.items().filter((i) => i.kind === 'asset').map((a) => ({ id: a.id, name: `${a.name}${a.source === 'system' ? '' : a.source === 'user' ? ' (Kitaplığım)' : ' (Proje)'}`, format: a.kind === 'asset' ? a.format : 'svg' })),
      importSvg: () => this.importSvg(),
      drawSvg: (id) =>
        new Promise((resolve) => {
          void import('../svgedit/SvgEditor').then((m) => m.openSvgEditor(this.ctx, { id, onSaved: (saved) => resolve(saved) }));
        }),
      context: this.selected.length === 2 ? 'marker' : this.draft.symbol.type,
    };
    const p = this.selected;
    const form = layerForm(
      l,
      (patch) => {
        const cur = this.layerAt(p);
        if (!cur) return;
        this.snapshot(`${p.join('.')}:${Object.keys(patch).join(',')}`);
        this.putLayer(p, applyPatch(cur, patch));
        // The form stays (focus is kept); list summaries and the preview follow.
        this.renderList();
        this.redraw();
        this.renderTitle();
        if ('type' in patch || 'placement' in patch || 'shape' in patch || 'dash' in patch || 'halo' in patch) queueMicrotask(() => this.keepFocus(() => this.renderProps()));
      },
      env,
    );
    const title = h('div', { class: 'sdes__ptitle' }, h('span', null, LAYER_LABEL[l.type]), p.length === 2 ? h('span', { class: 'sdes__pctx' }, `${LAYER_LABEL[this.layers[p[0]].type]} işaretinde`) : null);
    replaceChildren(this.props, title, form);
  }

  /** Rebuilds part of the form without losing the field being typed in. */
  private keepFocus(render: () => void): void {
    const label = (document.activeElement as HTMLElement | null)?.getAttribute('aria-label');
    render();
    if (label) (this.props.querySelector(`[aria-label="${CSS.escape(label)}"]`) as HTMLElement | null)?.focus();
  }

  private renderPreviewBar(): void {
    const zoom = (f: number) => {
      this.pxPerMm = Math.min(40, Math.max(1, this.pxPerMm * f));
      this.renderPreviewBar();
      this.redraw();
    };
    const b = (label: string, text: string, run: () => void) => {
      const el = h('button', { class: 'ibtn', type: 'button', 'aria-label': label, title: label }, text);
      el.addEventListener('click', run);
      return el;
    };
    const options = GEOMETRIES[this.draft.symbol.type];
    replaceChildren(
      this.previewBar,
      options.length > 1 ? segmented({ label: 'Örnek geometri', options, value: this.geometry, onChange: (v) => ((this.geometry = v), this.renderPreviewBar(), this.redraw()) }) : h('span', { class: 'sdes__note' }, 'Örnek: tek nokta'),
      h('div', { class: 'dialog__foot-spacer' }),
      b('Uzaklaş', '−', () => zoom(1 / 1.25)),
      h('span', { class: 'sdes__zoom num', title: 'Kâğıt milimetresinin ekrandaki boyu' }, `1 mm = ${this.pxPerMm.toFixed(1)} px`),
      b('Yakınlaş', '+', () => zoom(1.25)),
      b('Gerçek boyut (96 dpi)', '1:1', () => ((this.pxPerMm = 96 / 25.4), this.renderPreviewBar(), this.redraw())),
    );
  }

  private redraw(): void {
    cancelAnimationFrame(this.frame);
    this.frame = requestAnimationFrame(() => drawNow(this.ctx, this.canvas, this.draft.symbol, this.geometry, this.pxPerMm));
  }

  private renderTitle(): void {
    const base = this.opts.inline ? `${this.opts.inline.title}: ${KIND_TITLE[this.draft.symbol.type].toLocaleLowerCase('tr')}` : `${KIND_TITLE[this.draft.symbol.type]} tasarımcısı`;
    this.dialog.el.querySelector('.dialog__title')?.replaceChildren(`${base}${this.dirty ? ' •' : ''}`);
  }

  private say(text: string, kind: 'ok' | 'warn' = 'ok'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  // ── History ──────────────────────────────────────────────────────────

  private get dirty(): boolean {
    return JSON.stringify(this.draft) !== this.savedJson;
  }

  /** Records the draft before a change; typing into one field within a second is one step. */
  private snapshot(key: string): void {
    const now = performance.now();
    if (key === this.lastEdit.key && now - this.lastEdit.at < 1000) {
      this.lastEdit.at = now;
      return;
    }
    this.lastEdit = { key, at: now };
    this.past.push(JSON.stringify(this.draft));
    if (this.past.length > HISTORY) this.past.shift();
    this.future.length = 0;
  }

  private touched(key: string): void {
    this.snapshot(key);
    this.renderTitle();
  }

  private restore(json: string): void {
    const d = JSON.parse(json) as Draft;
    this.draft.name = d.name;
    this.draft.path = d.path;
    this.draft.symbol = d.symbol;
    this.nameInput.value = d.name;
    this.pathInput.value = d.path.join(' / ');
    if (!this.layerAt(this.selected)) this.selected = [0];
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
    const issues = validateSymbol(this.draft.symbol);
    if (issues.length) {
      this.say(`Kaydedilemedi: ${issues[0]}${issues.length > 1 ? ` (ve ${issues.length - 1} sorun daha)` : ''}`, 'warn');
      return false;
    }
    if (this.opts.inline) {
      this.savedJson = JSON.stringify(this.draft);
      this.opts.inline.onDone(structuredClone(this.draft.symbol));
      this.dialog.close();
      return true;
    }
    const lib = this.ctx.styles.library;
    const name = this.draft.name.trim() || 'Adsız sembol';
    const path = this.draft.path.length ? this.draft.path : ['Sembollerim'];
    let id: string;
    if (this.original) {
      lib.update(this.original.id, { name, path, symbol: this.draft.symbol });
      id = this.original.id;
    } else {
      id = newItemId(this.opts.source === 'project' ? 'p' : 'u');
      lib.add(this.opts.source ?? 'user', { kind: 'symbol', id, name, path, symbol: this.draft.symbol });
      this.original = lib.get(id) as LibrarySymbol;
    }
    this.savedJson = JSON.stringify(this.draft);
    this.renderTitle();
    this.say(`“${name}” kaydedildi.`);
    this.opts.onSaved?.(id);
    if (thenClose) this.dialog.close();
    return true;
  }

  /** Unsaved changes: the footer asks instead of closing. */
  private confirmClose(): boolean {
    if (!this.dirty) return true;
    const leave = h('button', { class: 'btn btn--small', type: 'button' }, 'Kaydetmeden kapat');
    const stay = h('button', { class: 'btn btn--small', type: 'button' }, 'Vazgeç');
    const both = h('button', { class: 'btn btn--small btn--primary', type: 'button' }, 'Kaydet ve kapat');
    leave.addEventListener('click', () => this.dialog.close());
    stay.addEventListener('click', () => this.say(''));
    both.addEventListener('click', () => this.save(true));
    this.status.dataset.kind = 'warn';
    replaceChildren(this.status, h('span', null, 'Kaydedilmemiş değişiklikler var.'), leave, stay, both);
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
