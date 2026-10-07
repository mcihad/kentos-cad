import type { AppContext } from '../../app/context';
import { takeLayerInto } from '../../app/drawingExchange';
import {
  addFileAsLayer,
  CLOUD_LIMIT,
  CLOUD_LISTS,
  cloudDrawing,
  cloudProjects,
  folderListing,
  foldersSupported,
  mayRead,
  pickFolder,
  SourceFolders,
  type FolderHandle,
  type SourceFolder,
} from '../../app/sources';
import { listen, type Disposable } from '../../core/disposable';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import type { Json } from '../../model/exchange';
import { SOURCE_ICONS, SOURCE_LABELS, type SourceFile } from '../../model/sources';
import { Panel } from '../dock/Panel';
import { h } from '../dom';
import { icon } from '../icons';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { TreeView } from '../widgets/TreeView';

type View = (typeof CLOUD_LISTS)[number]['view'];

/** A row of the panel. */
type Node =
  | { readonly kind: 'section'; readonly id: string; readonly label: string; readonly glyph: string; readonly children: readonly Node[] }
  | { readonly kind: 'folder'; readonly id: string; readonly label: string; readonly handle: FolderHandle; readonly root: SourceFolder | null; readonly children: readonly Node[] }
  | { readonly kind: 'file'; readonly id: string; readonly file: SourceFile; readonly folder: FolderHandle }
  | { readonly kind: 'list'; readonly id: string; readonly label: string; readonly view: View; readonly children: readonly Node[] }
  | { readonly kind: 'project'; readonly id: string; readonly project: ProjectSummary; readonly children: readonly Node[] }
  | { readonly kind: 'group'; readonly id: string; readonly label: string; readonly children: readonly Node[] }
  | { readonly kind: 'layer'; readonly id: string; readonly label: string; readonly path: string; readonly count: number; readonly project: ProjectSummary }
  | { readonly kind: 'note'; readonly id: string; readonly label: string; readonly tone: 'muted' | 'warn'; readonly glyph?: string; readonly act?: () => void };

/** What is known of something read when it is opened: a folder, a list, a project's drawing. */
type Read<T> = { state: 'reading' } | { state: 'ready'; value: T } | { state: 'failed'; why: string };

const FOLDERS = 'folders';
const KENTOS = 'kentos';
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** A drawing's layer tree as rows: groups and layers by path, each layer with its object count. */
function layerRows(project: ProjectSummary, drawing: Json): Node[] {
  const counts = new Map<string, number>();
  for (const e of drawing.entities ?? []) counts.set(e.layerId, (counts.get(e.layerId) ?? 0) + 1);
  const rows = (nodes: readonly Json[], above: readonly string[]): Node[] =>
    nodes.map((n): Node => {
      const here = [...above, n.name as string];
      const id = `${project.id}\u0000${here.join('\u0000')}`;
      return n.type === 'layer'
        ? { kind: 'layer', id, label: n.name, path: here.join(' / '), count: counts.get(n.id) ?? 0, project }
        : { kind: 'group', id, label: n.name, children: rows(n.children ?? [], here) };
    });
  return rows(drawing.layers ?? [], []);
}

/**
 * Kaynaklar (docs/adr/0199 §7): the dock's sources, a tab beside Katmanlar, İşlemler, Bloklar and Şablonlar. Klasörler
 * lists the folders the user added (Klasör ekle; the browser's folder access, its leave asked once the page reloads)
 * with their folders and the files the panel adds (model/sources.ts); a file's Katman olarak ekle (its button, a double
 * click, its menu) opens its format's import window with it. KentOS lists the projects the signed-in account reaches;
 * opening one downloads its drawing and lists its layers, and a layer's Katman olarak ekle takes it with its objects
 * into the open drawing as one undo step (app/drawingExchange.ts `takeLayerInto`). What is opened is read again when
 * it is opened again. The desktop's panel (`apps/desktop/src/sources/`) is the same.
 */
export class SourcesPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly tree: TreeView<Node>;
  private readonly folders = new SourceFolders();
  /** Rows opened by hand (the sections start open). */
  private readonly open = new Set<string>([FOLDERS, KENTOS]);
  private readonly listings = new Map<string, Read<Awaited<ReturnType<typeof folderListing>>>>();
  /** Folders inside the added ones, their handles asked of their parents when opened. */
  private readonly subHandles = new Map<string, Read<FolderHandle>>();
  private readonly lists = new Map<View, Read<{ projects: ProjectSummary[]; total: number }>>();
  private readonly drawings = new Map<string, Read<Json>>();
  /** Downloads under way by project id, stopped when the project is closed. */
  private readonly downloads = new Map<string, AbortController>();
  private queued = false;

  constructor(ctx: AppContext) {
    super({ title: 'Kaynaklar', className: 'panel--sources', actions: [] });
    this.ctx = ctx;
    const add = h('button', { class: 'btn btn--ghost src__add-folder', type: 'button' }, icon('folderAdd', 16), h('span', null, 'Klasör ekle'));
    this.d.add(listen(add, 'click', () => void this.addFolder()));
    this.d.add(tooltip(add, () => ({ title: 'Klasör ekle', description: 'Bilgisayarınızdaki bir klasörü listeye ekler; içindeki GeoJSON, Shapefile, DXF, NCZ, GPX, NMEA ve koordinat listesi dosyaları katman olarak eklenebilir.' })));
    this.tree = new TreeView<Node>(
      {
        id: (n) => n.id,
        children: (n) => ('children' in n ? n.children : []),
        isExpanded: (n) => this.open.has(n.id),
        setExpanded: (n, open) => this.expand(n, open),
        renderRow: (n, content) => this.renderRow(n, content),
        onActivate: (n) => this.activate(n),
        onContextMenu: (n, e) => {
          const items = this.menuFor(n);
          if (items.length) PopupMenu.open(items, { x: e.clientX, y: e.clientY });
        },
        clickToggles: true,
        tip: (n) => this.tipFor(n),
        empty: () => 'Kaynak yok.',
      },
      'Kaynaklar',
    );
    this.body.append(h('div', { class: 'panel__toolbar src__toolbar' }, add), this.tree.el);
    this.d.add(() => this.tree.dispose());
    this.d.add(() => {
      for (const c of this.downloads.values()) c.abort();
    });
    this.d.add(this.folders.list.subscribe(() => this.schedule()));
    this.d.add(ctx.cloud.auth.subscribe(() => this.schedule()));
    this.rebuild();
  }

  private schedule(): void {
    if (this.queued) return;
    this.queued = true;
    queueMicrotask(() => this.rebuild());
  }

  private rebuild(): void {
    this.queued = false;
    const roots: Node[] = [
      { kind: 'section', id: FOLDERS, label: 'Klasörler', glyph: 'folder', children: this.folderRows() },
      { kind: 'section', id: KENTOS, label: 'KentOS', glyph: 'cloud', children: this.kentosRows() },
    ];
    this.tree.render(roots);
  }

  // ── Rows ────────────────────────────────────────────────────────────

  private folderRows(): Node[] {
    const list = this.folders.list.value;
    if (!list.length)
      return [
        foldersSupported()
          ? { kind: 'note', id: 'folders:none', label: 'Klasör yok. Klasör ekle ile bir klasör ekleyin.', tone: 'muted', glyph: 'folderAdd', act: () => void this.addFolder() }
          : { kind: 'note', id: 'folders:none', label: 'Bu tarayıcı klasör seçtirmiyor; klasörler Chrome ya da Edge’de eklenir.', tone: 'warn' },
      ];
    return list.map((f) => this.folderNode(f.id, f.name, f.handle, f));
  }

  private folderNode(id: string, label: string, handle: FolderHandle, root: SourceFolder | null): Node {
    return { kind: 'folder', id, label, handle, root, children: this.open.has(id) ? this.folderChildren(id, handle) : [this.waiting(id)] };
  }

  /** Under a closed row: a stand-in so it can be opened. */
  private waiting(id: string): Node {
    return { kind: 'note', id: `${id}\u0000…`, label: 'Açılınca okunur', tone: 'muted' };
  }

  private folderChildren(id: string, handle: FolderHandle): Node[] {
    const read = this.listings.get(id);
    if (!read || read.state === 'reading') return [{ kind: 'note', id: `${id}\u0000okunuyor`, label: 'Okunuyor…', tone: 'muted' }];
    if (read.state === 'failed') return [{ kind: 'note', id: `${id}\u0000hata`, label: read.why, tone: 'warn', act: () => this.reread(id) }];
    const { folders, files } = read.value;
    const out: Node[] = [];
    for (const name of folders) {
      const sub = `${id}/${name}`;
      out.push({
        kind: 'folder',
        id: sub,
        label: name,
        handle,
        root: null,
        children: this.open.has(sub) ? this.subfolderChildren(sub, handle, name) : [this.waiting(sub)],
      });
    }
    for (const file of files) out.push({ kind: 'file', id: `${id}/${file.name}`, file, folder: handle });
    if (!out.length) out.push({ kind: 'note', id: `${id}\u0000boş`, label: 'Eklenebilecek dosya yok.', tone: 'muted' });
    return out;
  }

  /** A folder inside another: its handle is asked of its parent when it is opened. */
  private subfolderChildren(id: string, parent: FolderHandle, name: string): Node[] {
    const read = this.subHandles.get(id);
    if (read?.state === 'ready') return this.folderChildren(id, read.value);
    if (read?.state === 'failed') return [{ kind: 'note', id: `${id}\u0000hata`, label: read.why, tone: 'warn' }];
    if (!read) void this.openSub(id, parent, name);
    return [{ kind: 'note', id: `${id}\u0000okunuyor`, label: 'Okunuyor…', tone: 'muted' }];
  }

  private async openSub(id: string, parent: FolderHandle, name: string): Promise<void> {
    this.subHandles.set(id, { state: 'reading' });
    try {
      const handle = await parent.getDirectoryHandle(name);
      this.subHandles.set(id, { state: 'ready', value: handle });
      void this.list(id, handle, false);
    } catch (e) {
      this.subHandles.set(id, { state: 'failed', why: `Klasör açılamadı: ${message(e)}.` });
    }
    this.schedule();
  }

  private kentosRows(): Node[] {
    const auth = this.ctx.cloud.auth.value;
    if (auth === 'signedOut')
      return [{ kind: 'note', id: 'kentos:giris', label: 'Projeleri görmek için giriş yapın.', tone: 'muted', glyph: 'signIn', act: () => void this.ctx.commands.execute('cloud.signIn') }];
    if (auth === 'unknown')
      return [{ kind: 'note', id: 'kentos:sunucu', label: 'Sunucuya ulaşılamıyor. Yeniden denemek için tıklayın.', tone: 'warn', glyph: 'server', act: () => void this.ctx.cloud.refresh() }];
    return CLOUD_LISTS.map((l): Node => ({ kind: 'list', id: `kentos:${l.view}`, label: l.label, view: l.view, children: this.open.has(`kentos:${l.view}`) ? this.listChildren(l.view) : [this.waiting(`kentos:${l.view}`)] }));
  }

  private listChildren(view: View): Node[] {
    const id = `kentos:${view}`;
    const read = this.lists.get(view);
    if (!read || read.state === 'reading') return [{ kind: 'note', id: `${id}\u0000okunuyor`, label: 'Okunuyor…', tone: 'muted' }];
    if (read.state === 'failed') return [{ kind: 'note', id: `${id}\u0000hata`, label: read.why, tone: 'warn' }];
    const { projects, total } = read.value;
    const out: Node[] = projects.map((p) => ({ kind: 'project', id: `proje:${p.id}`, project: p, children: this.open.has(`proje:${p.id}`) ? this.projectChildren(p) : [this.waiting(`proje:${p.id}`)] }));
    if (!out.length) out.push({ kind: 'note', id: `${id}\u0000boş`, label: 'Proje yok.', tone: 'muted' });
    if (total > projects.length) out.push({ kind: 'note', id: `${id}\u0000fazla`, label: `İlk ${CLOUD_LIMIT} proje gösteriliyor (toplam ${total}).`, tone: 'muted' });
    return out;
  }

  private projectChildren(p: ProjectSummary): Node[] {
    const id = `proje:${p.id}`;
    const read = this.drawings.get(p.id);
    if (!read || read.state === 'reading') return [{ kind: 'note', id: `${id}\u0000iniyor`, label: 'Çizimi indiriliyor…', tone: 'muted' }];
    if (read.state === 'failed') return [{ kind: 'note', id: `${id}\u0000hata`, label: read.why, tone: 'warn' }];
    const rows = layerRows(p, read.value);
    return rows.length ? rows : [{ kind: 'note', id: `${id}\u0000boş`, label: 'Projede katman yok.', tone: 'muted' }];
  }

  private renderRow(n: Node, content: HTMLElement): Disposable | void {
    const row = content.parentElement ?? content;
    const name = (text: string) => h('span', { class: 'tree__name' }, text);
    const meta = (text: string) => h('span', { class: 'src__meta' }, text);
    switch (n.kind) {
      case 'section':
        row.toggleAttribute('data-group', true);
        content.append(h('span', { class: 'tree__folder' }, icon(n.glyph, 15)), h('span', { class: 'tree__name src__section' }, n.label));
        return;
      case 'folder':
        content.append(h('span', { class: 'tree__folder' }, icon('folder', 15)), name(n.label));
        return;
      case 'list':
        content.append(h('span', { class: 'tree__folder' }, icon(n.view === 'mine' ? 'folder' : 'share', 15)), name(n.label));
        return;
      case 'group':
        row.toggleAttribute('data-group', true);
        content.append(h('span', { class: 'tree__folder' }, icon('folder', 15)), name(n.label));
        return;
      case 'project':
        content.append(h('span', { class: 'tree__folder' }, icon(n.project.storage === 'file' ? 'fileOpen' : 'server', 15)), name(n.project.name), meta(n.project.storage === 'file' ? 'dosya' : 'veritabanı'));
        return;
      case 'note': {
        content.append(...(n.glyph ? [h('span', { class: 'tree__folder' }, icon(n.glyph, 15))] : []), h('span', { class: `tree__name src__note src__note--${n.tone}` }, n.label));
        if (!n.act) return;
        const act = n.act;
        return listen<MouseEvent>(row, 'click', (e) => {
          if (e.button === 0) act();
        });
      }
      case 'file':
        content.append(h('span', { class: 'src__icon' }, icon(SOURCE_ICONS[n.file.kind], 15)), name(n.file.name), meta(SOURCE_LABELS[n.file.kind]), this.addButton(() => this.activate(n)));
        return;
      case 'layer':
        content.append(h('span', { class: 'src__icon' }, icon('layers', 15)), name(n.label), meta(`${n.count} nesne`), this.addButton(() => this.activate(n)));
        return;
    }
  }

  /** The row's Katman olarak ekle: always there, as the mouse should not have to look for it. */
  private addButton(run: () => void): HTMLElement {
    const b = h('button', { class: 'ibtn ibtn--row src__add', type: 'button', 'aria-label': 'Katman olarak ekle' }, icon('layerAdd', 15));
    b.addEventListener('click', (e) => {
      e.stopPropagation();
      run();
    });
    b.addEventListener('dblclick', (e) => e.stopPropagation());
    tooltip(b, () => ({ title: 'Katman olarak ekle' }));
    return b;
  }

  private tipFor(n: Node): { title: string; description?: string } | null {
    switch (n.kind) {
      case 'file':
        return { title: n.file.name, description: `${SOURCE_LABELS[n.file.kind]}${n.file.parts.length > 1 ? ` (${n.file.parts.join(', ')})` : ''}\nÇift tık ya da Katman olarak ekle: içe aktarma penceresi açılır.` };
      case 'layer':
        return { title: n.path, description: `${n.project.name} · ${n.count} nesne\nÇift tık ya da Katman olarak ekle: katman nesneleriyle bu çizime alınır.` };
      case 'project':
        return { title: n.project.name, description: [n.project.description, `${n.project.tenantName} · ${n.project.storage === 'file' ? 'dosya projesi' : 'veritabanı projesi'}`, 'Açılınca çizimi indirilir ve katmanları listelenir.'].filter(Boolean).join('\n') };
      case 'folder':
        return n.root ? { title: n.label, description: 'Kapatıp açınca yeniden okunur. Listeden kaldırmak için sağ tıklayın.' } : null;
      default:
        return null;
    }
  }

  // ── Opening and reading ─────────────────────────────────────────────

  private expand(n: Node, open: boolean): void {
    if (!open) {
      this.open.delete(n.id);
      // What was read is read again when it is opened again.
      if (n.kind === 'folder') {
        this.listings.delete(n.id);
        this.subHandles.delete(n.id);
      } else if (n.kind === 'list') this.lists.delete(n.view);
      else if (n.kind === 'project') {
        this.downloads.get(n.project.id)?.abort();
        this.downloads.delete(n.project.id);
        this.drawings.delete(n.project.id);
      }
      this.schedule();
      return;
    }
    this.open.add(n.id);
    if (n.kind === 'folder' && n.root) void this.list(n.id, n.handle, true);
    else if (n.kind === 'list') void this.readList(n.view);
    else if (n.kind === 'project') void this.download(n.project);
    this.schedule();
  }

  private async list(id: string, handle: FolderHandle, ask: boolean): Promise<void> {
    this.listings.set(id, { state: 'reading' });
    this.schedule();
    if (!(await mayRead(handle, ask))) {
      this.listings.set(id, { state: 'failed', why: 'Tarayıcı bu klasörü okuma izni vermedi. İzin vermek için tıklayın.' });
      return this.schedule();
    }
    try {
      this.listings.set(id, { state: 'ready', value: await folderListing(handle) });
    } catch (e) {
      this.listings.set(id, { state: 'failed', why: `Klasör okunamadı: ${message(e)}. Klasör taşınmış ya da silinmiş olabilir.` });
    }
    this.schedule();
  }

  /** A failed folder's note clicked: asked again (the click lets the browser ask for leave). */
  private reread(id: string): void {
    const root = this.folders.list.value.find((f) => f.id === id);
    if (root) void this.list(id, root.handle, true);
  }

  private async readList(view: View): Promise<void> {
    this.lists.set(view, { state: 'reading' });
    this.schedule();
    try {
      this.lists.set(view, { state: 'ready', value: await cloudProjects(this.ctx, view) });
    } catch (e) {
      this.lists.set(view, { state: 'failed', why: `Projeler alınamadı: ${message(e)}` });
    }
    this.schedule();
  }

  private async download(p: ProjectSummary): Promise<void> {
    this.downloads.get(p.id)?.abort();
    const stop = new AbortController();
    this.downloads.set(p.id, stop);
    this.drawings.set(p.id, { state: 'reading' });
    this.schedule();
    try {
      const drawing = await cloudDrawing(this.ctx, p, stop.signal);
      if (stop.signal.aborted) return;
      this.drawings.set(p.id, { state: 'ready', value: drawing });
    } catch (e) {
      if (stop.signal.aborted) return;
      this.drawings.set(p.id, { state: 'failed', why: `Çizim indirilemedi: ${message(e)}` });
    } finally {
      if (this.downloads.get(p.id) === stop) this.downloads.delete(p.id);
    }
    this.schedule();
  }

  // ── Doing ───────────────────────────────────────────────────────────

  private async addFolder(): Promise<void> {
    const handle = await pickFolder(this.ctx);
    if (!handle) return;
    const f = await this.folders.add(handle);
    this.open.add(f.id);
    void this.list(f.id, f.handle, false);
    this.ctx.log.info(`Kaynaklar: “${f.name}” klasörü eklendi.`);
  }

  private activate(n: Node): void {
    if (n.kind === 'file') void addFileAsLayer(this.ctx, n.folder, n.file);
    else if (n.kind === 'layer') {
      const read = this.drawings.get(n.project.id);
      if (read?.state !== 'ready') return;
      takeLayerInto(this.ctx, read.value, n.path, n.project.name);
    } else if (n.kind === 'note') n.act?.();
  }

  private menuFor(n: Node): MenuItem[] {
    switch (n.kind) {
      case 'file':
        return [{ label: 'Katman olarak ekle…', icon: 'layerAdd', shortcut: 'Enter', run: () => this.activate(n) }];
      case 'layer':
        return [{ label: 'Katman olarak ekle', icon: 'layerAdd', shortcut: 'Enter', run: () => this.activate(n) }];
      case 'folder': {
        const root = n.root;
        if (!root) return [];
        return [
          {
            label: 'Listeden kaldır',
            icon: 'trash',
            detail: 'Klasör ve dosyaları silinmez; yalnız bu listeden çıkar.',
            run: () => {
              this.open.delete(root.id);
              this.listings.delete(root.id);
              void this.folders.remove(root.id);
              this.ctx.log.info(`Kaynaklar: “${root.name}” listeden kaldırıldı.`);
            },
          },
        ];
      }
      default:
        return [];
    }
  }
}
