import type { AppContext } from '../../app/context';
import { DisposableStore } from '../../core/disposable';
import type { LibrarySource, Sourced, Symbol } from '../../model/style';
import { exportStyles, parseStyleFile } from '../../style/file';
import type { CategoryNode } from '../../style/library';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { PopupMenu } from '../widgets/PopupMenu';
import { segmented } from '../widgets/controls';
import { TreeView } from '../widgets/TreeView';
import { hideTooltip, tooltip } from '../widgets/tooltip';
import { renderDetails, renderImport, type DetailsHost } from './managerDetails';
import { downloadStyles, pickStyleFile } from './styleFiles';
import { symbolOfItem, Thumbs } from './thumbs';

/**
 * Stil yöneticisi (docs/STYLE.md §7): the style library as a tree of
 * sources and categories on the left, the symbols of the chosen category
 * (or of a search) as pictures in the middle, the chosen one's details and
 * actions on the right. System items are read-only and copyable; the
 * user's and the project's can be edited, moved and deleted. The same
 * window picks a symbol for a layer style or for objects (`pick`).
 */

export type KindFilter = 'all' | Symbol['type'] | 'asset';

export interface PickOptions {
  /** Kind shown first (the geometry the symbol is for, or 'asset' to pick an SVG drawing); the user may widen it. */
  kind?: Symbol['type'] | 'asset';
  title?: string;
  current?: string;
  onPick(id: string): void;
}

interface Node {
  key: string;
  label: string;
  source: LibrarySource;
  path: readonly string[];
  children: Node[];
  count: number;
  description?: string;
  cat?: CategoryNode;
}

const SOURCES: { source: LibrarySource; label: string }[] = [
  { source: 'system', label: 'Sistem' },
  { source: 'user', label: 'Kitaplığım' },
  { source: 'project', label: 'Proje' },
];

const KIND_OPTIONS: { value: KindFilter; label: string }[] = [
  { value: 'all', label: 'Tümü' },
  { value: 'fill', label: 'Alan' },
  { value: 'line', label: 'Çizgi' },
  { value: 'marker', label: 'İşaret' },
  { value: 'asset', label: 'Çizim' },
];

export const kindOf = (i: Sourced): KindFilter => (i.kind === 'asset' ? 'asset' : i.symbol.type);

export function openStyleManager(ctx: AppContext, opts: { pick?: PickOptions; select?: string; stack?: boolean } = {}): void {
  new StyleManager(ctx, opts);
}

class StyleManager implements DetailsHost {
  readonly ctx: AppContext;
  readonly pick?: PickOptions;
  private readonly d = new DisposableStore();
  private readonly dialog: Dialog;
  readonly thumbs: Thumbs;
  private readonly tree: TreeView<Node>;
  private readonly grid: HTMLElement;
  private readonly details: HTMLElement;
  private readonly status: HTMLElement;
  private readonly count: HTMLElement;
  private readonly kindHost: HTMLElement;
  private readonly pickBtn: HTMLButtonElement | null;
  private readonly expanded = new Set<string>(['s:system', 's:user', 's:project']);
  private roots: Node[] = [];
  private at: { source: LibrarySource; path: readonly string[] } = { source: 'system', path: [] };
  private query = '';
  private kind: KindFilter = 'all';
  selected: string | null = null;
  private listed: Sourced[] = [];

  constructor(ctx: AppContext, opts: { pick?: PickOptions; select?: string; stack?: boolean }) {
    this.ctx = ctx;
    this.pick = opts.pick;
    this.kind = opts.pick?.kind ?? 'all';
    const lib = ctx.styles.library;
    const start = opts.select ?? opts.pick?.current;
    const startItem = start ? lib.get(start) : undefined;
    if (startItem) {
      this.selected = startItem.id;
      this.at = { source: startItem.source, path: startItem.path };
      startItem.path.forEach((_, i) => this.expanded.add(keyOf(startItem.source, startItem.path.slice(0, i + 1))));
    } else if (lib.items('user').length) this.at = { source: 'user', path: [] };

    const search = h('input', { class: 'field smgr__search', type: 'search', placeholder: 'Sembol ara: konut, sınır, tarama…', 'aria-label': 'Sembol ara', spellcheck: 'false' });
    search.addEventListener('input', () => {
      this.query = search.value.trim();
      this.refreshGrid();
    });
    this.kindHost = h('div', { class: 'smgr__kinds' });
    const newBtn = h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), 'Yeni sembol');
    newBtn.addEventListener('click', () => {
      const r = newBtn.getBoundingClientRect();
      PopupMenu.open(
        [
          { label: 'Alan sembolü', run: () => this.design({ newKind: 'fill' }) },
          { label: 'Çizgi sembolü', run: () => this.design({ newKind: 'line' }) },
          { label: 'İşaret sembolü', run: () => this.design({ newKind: 'marker' }) },
          { kind: 'separator' },
          { label: 'SVG çizimi (düzenleyicide)…', icon: 'edit', run: () => this.drawSvg() },
        ],
        { x: r.left, y: r.bottom + 4 },
      );
    });
    const menuBtn = (label: string, iconName: string, title: string, items: () => { label: string; hint?: string; run: () => void }[]) => {
      const b = h('button', { class: 'btn btn--small', type: 'button', title }, icon(iconName, 14), label, icon('chevronDown', 12));
      b.addEventListener('click', () => {
        const r = b.getBoundingClientRect();
        PopupMenu.open(items(), { x: r.left, y: r.bottom + 4 });
      });
      return b;
    };
    // Until the server exists, styles travel as .kstil files or as text on the clipboard (a chat message is enough).
    const importBtn = menuBtn('İçe aktar', 'import', 'Sembolleri kitaplığa ya da projeye alır', () => [
      { label: 'Dosyadan…', hint: '.kstil, PNG, JPEG', run: () => void this.importFile() },
      { label: 'Panodan yapıştır', hint: 'paylaşılan metin', run: () => void this.importClipboard() },
    ]);
    const exportBtn = menuBtn('Dışa aktar', 'export', 'Listedeki sembolleri kullandıkları çizimlerle birlikte verir', () => [
      { label: 'Dosyaya (.kstil)', run: () => this.exportListed() },
      { label: 'Panoya kopyala', hint: 'paylaşmak için', run: () => void this.exportListed(true) },
    ]);

    this.tree = new TreeView<Node>(
      {
        id: (n) => n.key,
        children: (n) => n.children,
        isExpanded: (n) => this.expanded.has(n.key),
        setExpanded: (n, on) => {
          if (on) this.expanded.add(n.key);
          else this.expanded.delete(n.key);
          this.renderTree();
        },
        renderRow: (n, row) =>
          row.append(
            icon(n.path.length ? 'folder' : n.source === 'system' ? 'lock' : n.source === 'user' ? 'styles' : 'save', 15),
            h('span', { class: 'tree__name' }, n.label),
            h('span', { class: 'smgr__count' }, String(n.count)),
          ),
        onSelect: (n) => {
          this.at = { source: n.source, path: n.path };
          this.query = '';
          search.value = '';
          this.refreshGrid();
        },
        onContextMenu: (n, e) => this.categoryMenu(n, e),
        onRename: (n) => {
          if (n.source !== 'system' && n.path.length) this.renameInline(n.key, n.label, (to) => this.ctx.styles.library.renameCategory(n.source as 'user' | 'project', n.path, to));
        },
        empty: () => 'Kitaplık boş.',
        clickToggles: true,
        // The full name when it is cut short, and the category's description.
        tip: (n, row) => {
          const name = row.querySelector<HTMLElement>('.tree__name');
          const cut = !!name && name.scrollWidth > name.clientWidth + 1;
          return cut || n.description ? { title: n.label, description: n.description } : null;
        },
      },
      'Sembol kategorileri',
    );
    this.grid = h('div', { class: 'smgr__grid', role: 'listbox', 'aria-label': 'Semboller' });
    this.thumbs = new Thumbs(ctx, this.grid);
    this.details = h('aside', { class: 'smgr__details' });
    this.count = h('span', { class: 'smgr__listcount' });
    this.status = h('div', { class: 'smgr__status', role: 'status' });
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    close.addEventListener('click', () => this.dialog.close());
    this.pickBtn = opts.pick ? h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, icon('check', 16), 'Seç') : null;
    this.pickBtn?.addEventListener('click', () => this.choose());

    this.dialog = new Dialog({
      title: opts.pick?.title ?? 'Stil yöneticisi',
      width: 1240,
      className: 'dialog--styles',
      content: [
        h('div', { class: 'smgr__bar' }, h('div', { class: 'smgr__searchbox' }, icon('search', 15), search), this.kindHost, h('div', { class: 'smgr__spacer' }), newBtn, importBtn, exportBtn),
        h(
          'div',
          { class: 'smgr__cols' },
          h('nav', { class: 'smgr__tree' }, this.tree.el),
          h('section', { class: 'smgr__list' }, h('div', { class: 'smgr__listhead' }, this.count), this.grid),
          this.details,
        ),
      ],
      footer: [this.status, h('div', { class: 'dialog__foot-spacer' }), close, this.pickBtn],
      onClose: () => this.d.dispose(),
      stack: !!opts.pick || !!opts.stack,
    });
    this.d.add(() => this.thumbs.dispose());
    this.d.add(lib.version.subscribe(() => this.refreshAll()));
    this.d.add(ctx.selection.ids.subscribe(() => this.refreshDetails()));
    this.refreshAll();
    queueMicrotask(() => search.focus());
  }

  // ── Building ─────────────────────────────────────────────────────────

  private refreshAll(): void {
    const lib = this.ctx.styles.library;
    const toNode = (source: LibrarySource, c: CategoryNode): Node => {
      const children = c.children.map((k) => toNode(source, k));
      return { key: keyOf(source, c.path), label: c.name, source, path: c.path, children, count: c.items.length + children.reduce((s, k) => s + k.count, 0), description: c.description, cat: c };
    };
    this.roots = SOURCES.map(({ source, label }) => {
      const children = lib.tree({ source }).map((c) => toNode(source, c));
      return { key: `s:${source}`, label, source, path: [], children, count: children.reduce((s, k) => s + k.count, 0) };
    });
    this.renderTree();
    this.refreshGrid();
  }

  private renderTree(): void {
    this.tree.render(this.roots);
    this.tree.mark(this.at.path.length ? keyOf(this.at.source, this.at.path) : `s:${this.at.source}`);
  }

  private findNode(source: LibrarySource, path: readonly string[]): Node | undefined {
    let node = this.roots.find((r) => r.source === source);
    for (const part of path) node = node?.children.find((c) => c.label === part);
    return node;
  }

  /** The items the grid shows: a search across the library, or everything under the chosen category. */
  private listItems(): Sourced[] {
    const lib = this.ctx.styles.library;
    const out: Sourced[] = [];
    const collect = (c: CategoryNode) => {
      out.push(...c.items);
      c.children.forEach(collect);
    };
    if (this.query) lib.tree({ query: this.query }).forEach(collect);
    else {
      const node = this.findNode(this.at.source, this.at.path);
      if (node?.cat) collect(node.cat);
      else if (node) node.children.forEach((c) => c.cat && collect(c.cat));
    }
    return this.kind === 'all' ? out : out.filter((i) => kindOf(i) === this.kind);
  }

  private refreshGrid(): void {
    replaceChildren(this.kindHost, segmented({ label: 'Tür', options: KIND_OPTIONS, value: this.kind, onChange: (v) => ((this.kind = v), this.refreshGrid()) }));
    this.listed = this.listItems();
    const where = this.query ? `“${this.query}” araması` : [SOURCES.find((s) => s.source === this.at.source)!.label, ...this.at.path].join(' › ');
    this.count.textContent = `${where}: ${this.listed.length} öğe`;
    if (this.selected && !this.listed.some((i) => i.id === this.selected) && !this.ctx.styles.library.get(this.selected)) this.selected = null;
    const cards = this.listed.map((item) => {
      const card = h(
        'button',
        { class: 'scard', type: 'button', role: 'option', 'aria-selected': String(item.id === this.selected), 'aria-label': item.name, dataset: { id: item.id } },
        this.thumbs.canvas(symbolOfItem(item), 116, 66),
        h('span', { class: 'scard__name' }, item.name),
        this.query ? h('span', { class: `scard__src scard__src--${item.source}` }, SOURCES.find((s) => s.source === item.source)!.label) : null,
      );
      // The full name (card names are cut to two lines) and where the item sits.
      tooltip(card, () => ({ title: item.name, description: item.path.join(' › ') }), 'bottom');
      card.addEventListener('click', () => this.select(item.id));
      card.addEventListener('dblclick', () => (this.pick ? this.choose() : this.edit(item.id)));
      return card;
    });
    hideTooltip(this.grid);
    replaceChildren(this.grid, cards.length ? cards : h('div', { class: 'smgr__empty' }, this.query ? 'Aramayla eşleşen sembol yok.' : 'Bu kategoride sembol yok.'));
    this.refreshDetails();
  }

  refreshDetails(): void {
    const item = this.selected ? this.ctx.styles.library.get(this.selected) : undefined;
    if (this.pickBtn) this.pickBtn.disabled = !item || !this.pickable(item);
    replaceChildren(this.details, renderDetails(this, item));
  }

  // ── Actions (DetailsHost) ───────────────────────────────────────────

  select(id: string): void {
    this.selected = id;
    for (const c of this.grid.querySelectorAll<HTMLElement>('.scard')) c.setAttribute('aria-selected', String(c.dataset.id === id));
    this.refreshDetails();
  }

  say(text: string, kind: 'ok' | 'warn' = 'ok'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  /** Opens the symbol designer on an item (a system one is copied first) or on a new symbol. */
  design(what: { id?: string; newKind?: Symbol['type'] }): void {
    const lib = this.ctx.styles.library;
    const editableAt = this.at.source !== 'system' && !this.query ? this.at : null;
    void import('./SymbolDesigner').then((m) =>
      m.openSymbolDesigner(this.ctx, {
        id: what.id,
        newKind: what.newKind,
        path: what.newKind ? (editableAt?.path.length ? editableAt.path : ['Sembollerim']) : undefined,
        source: editableAt ? (editableAt.source as 'user' | 'project') : 'user',
        onSaved: (id) => {
          this.at = { source: lib.get(id)?.source ?? 'user', path: lib.get(id)?.path ?? [] };
          this.select(id);
          this.refreshAll();
        },
      }),
    );
  }

  /** Opens the SVG editor on a drawing of the library (a system one is saved as a copy) or a new one. */
  drawSvg(id?: string): void {
    const lib = this.ctx.styles.library;
    void import('../svgedit/SvgEditor').then((m) =>
      m.openSvgEditor(this.ctx, {
        id,
        onSaved: (saved) => {
          this.at = { source: lib.get(saved)?.source ?? 'user', path: lib.get(saved)?.path ?? [] };
          this.select(saved);
          this.refreshAll();
        },
      }),
    );
  }

  edit(id: string): void {
    const lib = this.ctx.styles.library;
    const item = lib.get(id);
    if (item?.kind === 'asset') return item.format === 'svg' ? this.drawSvg(id) : undefined;
    if (!item || item.kind !== 'symbol') return;
    if (lib.canEdit(id)) return this.design({ id });
    const copy = lib.copy(id, 'user', { path: ['Sembollerim', ...item.path.slice(-1)] });
    this.say(`“${item.name}” sistem sembolü; kopyası Kitaplığım'a alındı ve açıldı.`);
    this.design({ id: copy.id });
  }

  /** What the pick mode may choose: symbols, or SVG drawings when it picks a drawing. */
  private pickable(item: Sourced): boolean {
    return this.pick?.kind === 'asset' ? item.kind === 'asset' && item.format === 'svg' : item.kind === 'symbol';
  }

  choose(): void {
    const id = this.selected;
    const item = id ? this.ctx.styles.library.get(id) : undefined;
    if (!this.pick || !id || !item || !this.pickable(item)) return;
    this.pick.onPick(id);
    this.dialog.close();
  }

  private async exportListed(toClipboard = false): Promise<void> {
    const ids = this.selected && !this.listed.length ? [this.selected] : this.listed.map((i) => i.id);
    if (!ids.length) return this.say('Dışa aktarılacak öğe yok: bir kategori seçin ya da arayın.', 'warn');
    if (toClipboard) {
      const file = exportStyles(this.ctx.styles.library, ids);
      try {
        await navigator.clipboard.writeText(JSON.stringify(file));
        this.say(`${file.items.length} öğe panoya kopyalandı: bir iletiye yapıştırıp paylaşabilirsiniz; alan kişi “İçe aktar → Panodan yapıştır” der.`);
      } catch {
        this.say('Tarayıcı panoya yazmaya izin vermedi; dosyaya aktarmayı deneyin.', 'warn');
      }
      return;
    }
    const name = this.query || this.at.path.at(-1) || SOURCES.find((s) => s.source === this.at.source)!.label;
    const n = downloadStyles(this.ctx, ids, name);
    this.say(`${n} öğe (kullandıkları çizimlerle) .kstil dosyasına yazıldı.`);
  }

  private async importClipboard(): Promise<void> {
    let text = '';
    try {
      text = await navigator.clipboard.readText();
    } catch {
      return this.say('Tarayıcı panoyu okumaya izin vermedi; dosyadan içe aktarmayı deneyin.', 'warn');
    }
    const { file, issues } = parseStyleFile(text);
    if (!file) return this.say(`Panodaki metin bir KentOS stili değil: ${issues[0] ?? 'biçim tanınmadı'}.`, 'warn');
    replaceChildren(this.details, renderImport(this, 'Pano', file, issues));
  }

  private async importFile(): Promise<void> {
    const picked = await pickStyleFile();
    if (!picked) return;
    if ('image' in picked) {
      const lib = this.ctx.styles.library;
      lib.add('user', picked.image);
      this.at = { source: 'user', path: picked.image.path };
      this.select(picked.image.id);
      this.refreshAll();
      return this.say(`“${picked.image.name}” Kitaplığım'a alındı: görüntü dolgusunda ya da görüntü işaretinde kullanılabilir.`);
    }
    const { file, issues } = parseStyleFile(picked.text);
    if (!file) return this.say(`“${picked.name}” okunamadı: ${issues.slice(0, 2).join('; ')}`, 'warn');
    replaceChildren(this.details, renderImport(this, picked.name, file, issues));
  }

  private categoryMenu(n: Node, e: MouseEvent): void {
    const lib = this.ctx.styles.library;
    const editable = n.source !== 'system';
    const ids = (n.cat ? flatten(n.cat) : n.children.flatMap((c) => (c.cat ? flatten(c.cat) : []))).map((i) => i.id);
    PopupMenu.open(
      [
        {
          label: 'Yeni alt kategori',
          icon: 'folderAdd',
          disabled: !editable,
          run: () => {
            const src = n.source as 'user' | 'project';
            const siblings = new Set(n.children.map((c) => c.label));
            let name = 'Yeni kategori';
            for (let i = 2; siblings.has(name); i++) name = `Yeni kategori ${i}`;
            lib.addCategory(src, { path: [...n.path, name] });
            this.expanded.add(n.key);
            this.renderTree();
            this.renameInline(keyOf(n.source, [...n.path, name]), name, (to) => lib.renameCategory(src, [...n.path, name], to));
          },
        },
        {
          label: 'Yeniden adlandır',
          shortcut: 'F2',
          disabled: !editable || !n.path.length,
          run: () => this.renameInline(n.key, n.label, (to) => lib.renameCategory(n.source as 'user' | 'project', n.path, to)),
        },
        { kind: 'separator' },
        { label: `Dışa aktar (${ids.length})`, icon: 'export', disabled: !ids.length, run: () => this.say(`${downloadStyles(this.ctx, ids, n.label)} öğe dışa aktarıldı.`) },
      ],
      { x: e.clientX, y: e.clientY },
    );
  }

  /** Swaps a tree row's label for a text field; Enter or leaving the field renames, Esc keeps the name. */
  private renameInline(key: string, current: string, done: (name: string) => void): void {
    const label = this.tree.rowOf(key)?.querySelector('.tree__name');
    if (!label) return;
    const input = h('input', { class: 'field field--inline', value: current, 'aria-label': 'Kategori adı', dataset: { escape: 'local' }, spellcheck: 'false' });
    label.replaceWith(input);
    input.focus();
    input.select();
    let settled = false;
    const finish = (ok: boolean) => {
      if (settled) return;
      settled = true;
      const name = input.value.trim();
      if (ok && name && name !== current) done(name);
      else this.renderTree();
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

const keyOf = (source: LibrarySource, path: readonly string[]) => `c:${source}:${path.join('\u0001')}`;

function flatten(c: CategoryNode): Sourced[] {
  return [...c.items, ...c.children.flatMap(flatten)];
}
