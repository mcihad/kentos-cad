import type { AppContext } from '../../app/context';
import { modelCommandId, processingCommandId } from '../../app/processing';
import { foldTurkish } from '../../core/text';
import type { ProcessingModel } from '../../processing/model';
import { DisposableStore, listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import type { ProcessingCategory } from '../../processing/categories';
import type { CategoryNode } from '../../processing/registry';
import type { RunRecord } from '../../processing/runner';
import type { ProcessingTool } from '../../processing/types';
import { Panel } from '../dock/Panel';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { segmented } from '../widgets/controls';
import { tooltip } from '../widgets/tooltip';
import { TreeView } from '../widgets/TreeView';
import { openToolDialog, TARGET_SHORT } from './ToolDialog';

type Node =
  | { kind: 'category'; id: string; category: ProcessingCategory; children: Node[]; count: number }
  | { kind: 'tool'; id: string; tool: ProcessingTool; children: Node[] }
  | { kind: 'model'; id: string; model: ProcessingModel; children: Node[] }
  | { kind: 'newModel'; id: string; children: Node[] };

/** The models branch sits first; it is not a registry category. */
const MODELS: ProcessingCategory = { id: '__models', label: 'Modeller', icon: 'processing', description: 'Araçları birbirine bağlayan akışlar; her biri tek adımda geri alınır' };

/**
 * İşlem araç kutusu (QGIS Processing Toolbox): the tools by category with
 * a search box, and this session's runs. A click opens the tool's dialog;
 * a history row opens it again with the values of that run.
 */
export class ProcessingPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly tree: TreeView<Node>;
  private readonly search: HTMLInputElement;
  private readonly toolsView: HTMLElement;
  private readonly historyView = h('div', { class: 'phist', role: 'list', 'aria-label': 'İşlem geçmişi' });
  private readonly switcher = h('div', { class: 'panel__toolbar pproc__switch' });
  /** Tooltips of the current rows, released on every rebuild (the store is reusable). */
  private readonly rowSubs = new DisposableStore();
  private query = '';

  constructor(ctx: AppContext) {
    super({ title: 'İşlemler', className: 'panel--processing' });
    this.ctx = ctx;
    const { registry, runner } = ctx.processing;
    const folded = ctx.ui.processingFolded;

    this.search = h('input', { class: 'field field--search', type: 'search', placeholder: 'İşlem ara: numara, kenar, parsel…', 'aria-label': 'İşlem ara', spellcheck: 'false' });
    this.tree = new TreeView<Node>(
      {
        id: (n) => n.id,
        children: (n) => n.children,
        isExpanded: (n) => !!this.query || !folded.value.includes(n.id),
        setExpanded: (n, open) => folded.set(open ? folded.value.filter((id) => id !== n.id) : [...folded.value, n.id]),
        renderRow: (n, row) => this.renderRow(n, row),
        onActivate: (n) => {
          if (n.kind === 'tool') this.open(n.tool);
          else if (n.kind === 'model') ctx.commands.execute(modelCommandId(n.model.id));
          else if (n.kind === 'newModel') ctx.commands.execute('processing.newModel');
          else folded.set(folded.value.includes(n.id) ? folded.value.filter((id) => id !== n.id) : [...folded.value, n.id]);
        },
        empty: (filtered) => (filtered || this.query ? 'Aramayla eşleşen işlem yok. Başka bir kelime deneyin.' : 'Kayıtlı işlem aracı yok.'),
      },
      'İşlem araçları',
    );
    this.toolsView = h('div', { class: 'pproc__tools' }, h('div', { class: 'panel__toolbar' }, h('span', { class: 'field-icon' }, icon('search', 14)), this.search), this.tree.el);
    this.body.append(this.switcher, this.toolsView, this.historyView);

    this.d.add(
      listen(this.search, 'input', () => {
        this.query = this.search.value.trim();
        this.renderTools();
      }),
    );
    // Esc in the search clears it before anything else.
    this.d.add(
      listen<KeyboardEvent>(this.search, 'keydown', (e) => {
        if (e.key === 'Escape' && this.search.value) {
          e.stopPropagation();
          this.search.value = '';
          this.query = '';
          this.renderTools();
        } else if (e.key === 'ArrowDown') {
          e.preventDefault();
          this.tree.el.focus();
        }
      }),
    );
    this.d.add(watchAll([registry.version, folded, ctx.processing.models], () => this.renderTools()));
    this.d.add(watchAll([runner.history, ctx.ui.processingTab], () => this.renderView()));
    // Undo and erase change what "n nesneyi seç" can still select.
    this.d.add(ctx.doc.events.on('changed', () => ctx.ui.processingTab.value === 'history' && this.renderHistory(runner.history.value)));
    this.renderTools();
    this.renderView();
  }

  /** Puts the cursor in the search box (İşlemler → İşlem araç kutusu). */
  focusSearch(): void {
    this.search.focus();
    this.search.select();
  }

  private open(tool: ProcessingTool): void {
    this.ctx.commands.execute(processingCommandId(tool.id));
  }

  private renderView(): void {
    const tab = this.ctx.ui.processingTab.value;
    const runs = this.ctx.processing.runner.history.value;
    replaceChildren(
      this.switcher,
      segmented({
        label: 'İşlem paneli görünümü',
        options: [
          { value: 'tools', label: 'Araçlar' },
          { value: 'history', label: runs.length ? `Geçmiş (${runs.length})` : 'Geçmiş' },
        ],
        value: tab,
        onChange: (v) => this.ctx.ui.processingTab.set(v),
      }),
    );
    this.toolsView.hidden = tab !== 'tools';
    this.historyView.hidden = tab !== 'history';
    if (tab === 'history') this.renderHistory(runs);
    this.updateMeta();
  }

  private updateMeta(): void {
    const tab = this.ctx.ui.processingTab.value;
    this.setMeta(tab === 'history' ? `${this.ctx.processing.runner.history.value.length} kayıt` : `${this.ctx.processing.registry.list().length} araç`);
  }

  // ── Tools ────────────────────────────────────────────────────────────

  private renderTools(): void {
    const { registry } = this.ctx.processing;
    const hits = this.query ? new Set(registry.search(this.query).map((t) => t.id)) : null;
    const toNode = (c: CategoryNode): Node => {
      const children = [...c.children.map(toNode), ...c.tools.map((tool): Node => ({ kind: 'tool', id: `tool:${tool.id}`, tool, children: [] }))];
      return { kind: 'category', id: c.category.id, category: c.category, children, count: children.reduce((s, n) => s + (n.kind === 'category' ? n.count : n.kind === 'tool' ? 1 : 0), 0) };
    };
    this.rowSubs.dispose();
    const q = this.query ? foldTurkish(this.query) : '';
    const models = this.ctx.processing.models.value.filter((m) => !q || foldTurkish(`${m.label} ${m.description}`).includes(q));
    const modelNodes: Node[] = models.map((m) => ({ kind: 'model', id: `model:${m.id}`, model: m, children: [] }));
    const branch: Node[] = modelNodes.length || !q ? [{ kind: 'category', id: MODELS.id, category: MODELS, children: [...modelNodes, ...(q ? [] : [{ kind: 'newModel', id: '__newModel', children: [] } as Node])], count: modelNodes.length }] : [];
    this.tree.render([...branch, ...registry.tree(hits ? (t) => hits.has(t.id) : undefined).map(toNode)]);
    this.updateMeta();
  }

  private renderRow(n: Node, row: HTMLElement): void {
    row.parentElement?.toggleAttribute('data-group', n.kind === 'category');
    if (n.kind === 'category') {
      row.append(h('span', { class: 'tree__folder' }, icon(n.category.icon ?? 'folder', 15)), h('span', { class: 'tree__name' }, n.category.label), h('span', { class: 'tree__count num' }, String(n.count)));
      if (n.category.description) this.rowSubs.add(tooltip(row, () => ({ title: n.category.label, description: n.category.description }), 'bottom'));
      return;
    }
    if (n.kind === 'newModel') {
      row.classList.add('pproc__tool', 'pproc__new');
      row.append(h('span', { class: 'pproc__icon' }, icon('modelNew', 15)), h('span', { class: 'tree__name' }, 'Yeni model…'));
      row.addEventListener('click', () => this.ctx.commands.execute('processing.newModel'));
      return;
    }
    if (n.kind === 'model') {
      const m = n.model;
      row.classList.add('pproc__tool');
      const builtin = this.ctx.processing.isBuiltinModel(m.id);
      const edit = h('button', { class: 'ibtn ibtn--row', type: 'button', 'aria-label': builtin ? 'Kopyasını düzenle' : 'Modeli düzenle', title: builtin ? 'Kopyasını düzenle' : 'Modeli düzenle' }, icon('edit', 14));
      edit.addEventListener('click', (e) => {
        e.stopPropagation();
        this.ctx.commands.execute('processing.newModel', m.id);
      });
      row.append(h('span', { class: 'pproc__icon' }, icon('processing', 15)), h('span', { class: 'tree__name' }, m.label), edit);
      row.addEventListener('click', () => this.ctx.commands.execute(modelCommandId(m.id)));
      this.rowSubs.add(tooltip(row, () => ({ title: m.label, description: m.description || undefined, note: `${m.steps.length} adım${builtin ? '; hazır model' : ''}` }), 'bottom'));
      return;
    }
    const t = n.tool;
    row.classList.add('pproc__tool');
    row.append(h('span', { class: 'pproc__icon' }, icon(t.icon ?? 'processing', 15)), h('span', { class: 'tree__name' }, t.label));
    // A single click opens the tool: the toolbox is a launcher, not a selection list.
    row.addEventListener('click', () => this.open(t));
    this.rowSubs.add(
      tooltip(
        row,
        () => ({
          title: t.label,
          description: t.description,
          note: t.aliases?.length ? `Komut satırı: ${t.aliases.join(', ')}` : undefined,
        }),
        'bottom',
      ),
    );
  }

  // ── History ──────────────────────────────────────────────────────────

  private renderHistory(runs: readonly RunRecord[]): void {
    if (!runs.length) {
      replaceChildren(
        this.historyView,
        h('div', { class: 'empty empty--inline' }, 'Bu oturumda henüz işlem çalıştırılmadı. Araçlar sekmesinden bir işlem açıp çalıştırın; burada yeniden açabilirsiniz.'),
      );
      return;
    }
    replaceChildren(
      this.historyView,
      runs.map((r) => this.historyRow(r)),
    );
  }

  private historyRow(r: RunRecord): HTMLElement {
    const { doc } = this.ctx;
    const alive = (r.added.length ? r.added : r.touched).filter((id) => doc.get(id));
    const known = !!this.ctx.processing.registry.get(r.toolId);
    const reopen = h('button', { class: 'btn btn--small', type: 'button', disabled: !known, title: 'Aynı değerlerle pencereyi açar' }, 'Yeniden aç');
    reopen.addEventListener('click', () => openToolDialog(this.ctx, r.toolId, r.values));
    const select = alive.length ? h('button', { class: 'btn btn--small btn--ghost', type: 'button', title: 'Bu çalıştırmanın eklediği nesneleri seçer' }, `${alive.length} nesneyi seç`) : null;
    select?.addEventListener('click', () => {
      this.ctx.selection.set(alive);
      this.ctx.view.zoomToSelection();
    });
    const time = new Date(r.started).toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    const took = `${r.ms < 1000 ? `${r.ms} ms` : `${(r.ms / 1000).toFixed(1)} sn`}${r.target && r.target !== 'client' ? `, ${TARGET_SHORT[r.target]}` : ''}`;
    const state = r.status === 'ok' ? 'success' : r.status === 'canceled' ? 'warning' : 'error';
    return h(
      'div',
      { class: 'phist__row', role: 'listitem', 'data-status': r.status },
      h('span', { class: 'phist__icon' }, icon(state, 16)),
      h(
        'div',
        { class: 'phist__main' },
        h('div', { class: 'phist__head' }, h('span', { class: 'phist__label' }, r.label), h('span', { class: 'phist__time num' }, time)),
        h('div', { class: 'phist__summary' }, r.summary),
        h('div', { class: 'phist__foot' }, h('span', { class: 'phist__took num' }, took), h('span', { class: 'phist__actions' }, select, reopen)),
      ),
    );
  }

  override dispose(): void {
    this.rowSubs.dispose();
    super.dispose();
  }
}
