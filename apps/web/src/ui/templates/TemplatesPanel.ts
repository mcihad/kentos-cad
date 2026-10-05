import type { AppContext } from '../../app/context';
import { listen, type Disposable } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { TEMPLATE_TOOL_LABEL } from '../../model/objectTemplate';
import type { LibraryTemplate, Sourced } from '../../model/style';
import { listTemplates } from '../../style/templateList';
import { Panel } from '../dock/Panel';
import { h } from '../dom';
import { icon } from '../icons';
import { symbolOfItem, Thumbs } from '../style/thumbs';
import { commandButton } from '../widgets/CommandButton';
import { askRemove } from '../widgets/confirm';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { TreeView } from '../widgets/TreeView';

/** A row: a category with its templates under it, or a template (in the recent ones, again). */
type Node =
  | { readonly kind: 'group'; readonly id: string; readonly label: string; readonly children: readonly Node[] }
  | { readonly kind: 'template'; readonly id: string; readonly item: Sourced<LibraryTemplate> };

/** The picture's side, CSS pixels at scale 1 (its row is 40 px high, panels.css). */
const THUMB = 30;
/** The recent group's key: no category path joins to it. */
const RECENT = '\u0001son';

/** What a template draws where: “Kapalı alan · Kadastro / Parsel”. */
export function templateWhere(t: LibraryTemplate): string {
  const layer = [...t.template.layer.path, t.template.layer.name].join(' / ');
  return `${TEMPLATE_TOOL_LABEL[t.template.tool] ?? t.template.tool} · ${layer}`;
}

/**
 * Şablonlar (docs/adr/0176 §4): the style library's object templates in the dock's upper slot, a tab beside
 * Katmanlar, İşlemler and Bloklar. The templates are grouped by their categories (style/templateList.ts), each row
 * its picture, its name and what it draws where; the last used ones come first while nothing is searched, and the
 * one being drawn with is marked. A click draws with the template (app/objectTemplates.ts), as Enter does on the
 * row the keys chose; the drawing then has the keyboard. The row's menu draws, shows the template in the Stil
 * yöneticisi, copies it to Kitaplığım or the project and deletes it from an editable source. The desktop's panel
 * (`apps/desktop/src/templates_panel.rs`) is the same.
 */
export class TemplatesPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly tree: TreeView<Node>;
  private readonly thumbs: Thumbs;
  private query = '';
  /** Groups closed by hand, by key. */
  private readonly closed = new Set<string>();
  /** Built template rows by template id, so the drawn-with mark moves without building them again. */
  private readonly built = new Map<string, HTMLElement[]>();
  private queued = false;

  constructor(ctx: AppContext) {
    super({ title: 'Şablonlar', className: 'panel--templates', actions: [] });
    this.ctx = ctx;
    // The buttons sit beside the search, not in the head: the head holds the dock's tabs.
    const buttons = ['template.new', 'template.fromSelection', 'style.manager'].map((id) => commandButton(ctx, id, this.d, { className: 'ibtn', size: 16 }));
    const search = h('input', { class: 'field field--search', type: 'search', placeholder: 'Şablon ara', 'aria-label': 'Şablon ara', spellcheck: 'false' });
    this.tree = new TreeView<Node>(
      {
        id: (n) => n.id,
        children: (n) => (n.kind === 'group' ? n.children : []),
        isExpanded: (n) => n.kind === 'group' && !this.closed.has(n.id),
        setExpanded: (n, open) => {
          if (open) this.closed.delete(n.id);
          else this.closed.add(n.id);
        },
        renderRow: (n, content) => this.renderRow(n, content),
        releaseRow: (n) => {
          if (n.kind === 'template') this.built.set(n.item.id, (this.built.get(n.item.id) ?? []).filter((r) => r.isConnected));
        },
        onActivate: (n) => {
          if (n.kind === 'template') this.draw(n.item.id);
        },
        onDelete: (n) => {
          if (n.kind === 'template') this.remove(n.item);
        },
        onContextMenu: (n, e) => {
          if (n.kind === 'template') PopupMenu.open(this.menuFor(n.item), { x: e.clientX, y: e.clientY });
        },
        clickToggles: true,
        tip: (n) => (n.kind === 'template' ? { title: n.item.name, description: [n.item.description, templateWhere(n.item), sourceText(n.item)].filter(Boolean).join('\n') } : null),
        empty: (filtered) =>
          filtered || this.query.trim()
            ? 'Aramayla eşleşen şablon yok. Başka bir ad deneyin.'
            : 'Kitaplıkta nesne şablonu yok. Şablonlar Stil yöneticisinde ya da bir .kstil dosyasıyla gelir.',
      },
      'Şablonlar',
    );
    this.thumbs = new Thumbs(ctx, this.tree.el);
    this.d.add(() => this.thumbs.dispose());
    this.body.append(h('div', { class: 'panel__toolbar tpl__toolbar' }, h('span', { class: 'field-icon' }, icon('search', 14)), search, ...buttons), this.tree.el);
    this.d.add(() => this.tree.dispose());
    this.d.add(
      listen(search, 'input', () => {
        this.query = search.value;
        this.rebuild();
      }),
    );
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
            this.query = '';
            this.rebuild();
          } else ctx.view.focus();
        }
      }),
    );
    // The library (the project's part follows the drawing), the recent ones and the theme (the pictures) change the rows.
    this.d.add(watchAll([ctx.styles.library.version, ctx.tools.recentTemplates, ctx.prefs.theme], () => this.schedule()));
    this.d.add(ctx.settings.template.subscribe(() => this.writeDrawing()));
    this.rebuild();
  }

  /** Lists the rows again once, at the end of the task: an import adds its items one by one. */
  private schedule(): void {
    if (this.queued) return;
    this.queued = true;
    queueMicrotask(() => this.rebuild());
  }

  private rebuild(): void {
    this.queued = false;
    const lib = this.ctx.styles.library;
    const groups = listTemplates(lib, this.query);
    const count = groups.reduce((n, g) => n + g.items.length, 0);
    const leaf = (key: string, item: Sourced<LibraryTemplate>): Node => ({ kind: 'template', id: `${key}\u0000${item.id}`, item });
    const nodes: Node[] = [];
    // The last used ones first, while nothing is searched (the tool manager keeps them, newest first).
    const recent = this.query.trim() ? [] : this.ctx.tools.recentTemplates.value.map((id) => lib.template(id)).filter((t): t is Sourced<LibraryTemplate> => !!t);
    if (recent.length) nodes.push({ kind: 'group', id: RECENT, label: 'Son kullanılanlar', children: recent.map((t) => leaf(RECENT, t)) });
    for (const g of groups) {
      const key = g.path.join('\u0000');
      nodes.push({ kind: 'group', id: key, label: g.path.length ? g.path.join(' / ') : 'Kategorisiz', children: g.items.map((t) => leaf(key, t)) });
    }
    this.setMeta(count ? `${count} şablon` : '');
    this.built.clear();
    this.tree.render(nodes);
  }

  private renderRow(n: Node, content: HTMLElement): Disposable | void {
    if (n.kind === 'group') {
      content.append(h('span', { class: 'tree__name tpl__group' }, n.label), h('span', { class: 'tree__count num' }, String(n.children.length)));
      return;
    }
    const row = content.parentElement ?? content;
    const t = n.item;
    const picture = this.thumbs.canvas(symbolOfItem(t, this.ctx.styles.library), THUMB, THUMB);
    picture.classList.add('tpl__thumb');
    content.append(picture, h('span', { class: 'blk__text' }, h('span', { class: 'tree__name' }, t.name), h('span', { class: 'blk__about' }, templateWhere(t))));
    this.built.set(t.id, [...(this.built.get(t.id) ?? []), row]);
    row.toggleAttribute('data-active', t.id === this.ctx.settings.template.value?.id);
    // A click draws with it at once (the keys only choose; Enter draws).
    return listen<MouseEvent>(row, 'click', (e) => {
      if (e.button === 0) this.draw(t.id);
    });
  }

  /** The template being drawn with is marked, wherever its rows are. */
  private writeDrawing(): void {
    const drawing = this.ctx.settings.template.value?.id;
    for (const [id, rows] of this.built) for (const row of rows) row.toggleAttribute('data-active', id === drawing);
  }

  /** Draws with the template; the drawing takes the keyboard, as after a ribbon button. */
  private draw(id: string): void {
    this.ctx.commands.execute('template.draw', id);
    this.ctx.view.focus();
  }

  private menuFor(t: Sourced<LibraryTemplate>): MenuItem[] {
    const { ctx } = this;
    const lib = ctx.styles.library;
    const copyTo = (to: 'user' | 'project') => {
      lib.copy(t.id, to);
      ctx.log.success(`“${t.name}” ${to === 'user' ? 'Kitaplığım' : 'Proje'} kitaplığına kopyalandı.`);
    };
    const editable = t.source !== 'system';
    const edit = () => void import('./TemplateEditor').then((m) => m.openTemplateEditor(ctx, { id: t.id }));
    return [
      { label: 'Şablonla çiz', icon: 'templateDraw', shortcut: 'Enter', run: () => this.draw(t.id) },
      // Şablonu uygula (docs/adr/0176 §6): the selected objects of its kind take its layer and look.
      {
        label: 'Seçili nesnelere uygula',
        icon: 'templateApply',
        disabled: ctx.selection.size === 0,
        detail: ctx.selection.size === 0 ? 'Önce çizimde nesne seçin.' : undefined,
        run: () => ctx.commands.execute('template.apply', t.id),
      },
      { label: editable ? 'Düzenle…' : 'Kopyasını düzenle…', icon: 'edit', detail: editable ? undefined : 'Sistem şablonu değişmez; kopyası Kitaplığım’a kaydedilir.', run: edit },
      { label: 'Stil yöneticisinde göster', icon: 'styles', run: () => void import('../style/StyleManager').then((m) => m.openStyleManager(ctx, { select: t.id })) },
      { kind: 'separator' },
      { label: 'Kitaplığıma kopyala', icon: 'copy', hint: 'bu tarayıcıda', run: () => copyTo('user') },
      { label: 'Projeye kopyala', icon: 'save', hint: 'proje dosyasında', run: () => copyTo('project') },
      { kind: 'separator' },
      { label: 'Sil', icon: 'trash', shortcut: 'Delete', disabled: !editable, detail: editable ? undefined : 'Sistem şablonu silinmez; kopyası düzenlenir.', run: () => this.remove(t) },
    ];
  }

  /** Sil (and Delete on a row): asked in a window, as the library keeps no undo. */
  private remove(t: Sourced<LibraryTemplate>): void {
    const { ctx } = this;
    if (t.source === 'system') return void ctx.log.warn('Sistem şablonu silinmez; kopyasını Kitaplığım’a alıp onu düzenleyin.');
    void askRemove({
      title: 'Kitaplıktan sil',
      message: `“${t.name}” ${t.source === 'project' ? 'projenin kitaplığından' : 'Kitaplığım’dan'} silinsin mi? Bu geri alınamaz.`,
      action: 'Sil',
    }).then((yes) => {
      if (!yes || !ctx.styles.library.get(t.id)) return;
      ctx.styles.library.remove(t.id);
      ctx.log.success(`“${t.name}” silindi.`);
    });
  }
}

/** Where a template lives, as the Stil yöneticisi says it. */
function sourceText(t: Sourced<LibraryTemplate>): string {
  return t.source === 'system' ? 'Sistem' : t.source === 'user' ? 'Kitaplığım' : 'Proje';
}
