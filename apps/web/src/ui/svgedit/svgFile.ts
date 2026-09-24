import type { AppContext } from '../../app/context';
import type { LibraryAsset } from '../../model/style';
import { resolveColor } from '../../render/color';
import type { EditableSource } from '../../style/library';
import { sanitizeSvg, svgAsset } from '../../style/file';
import { importSummary, type ReferenceSpec } from '../../style/svg/importSvg';
import { svgText } from '../../style/svg/exportSvg';
import { newDoc, shapeId, transformShape, type SvgDoc, type SvgShape } from '../../style/svg/svgModel';
import { h } from '../dom';
import { icon } from '../icons';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { openDocProps } from './svgDocProps';
import { openExportDialog } from './svgExport';
import { openImportDialog } from './svgImport';
import { ReferenceLayer } from './svgReference';
import { SourcePanel } from './svgSource';
import { openTraceDialog } from './svgTrace';
import { looksLikeSvg, readSvg } from './readSvg';
import type { CanvasOptions, SvgCanvas } from './svgCanvas';

/**
 * The SVG editor's files (docs/STYLE.md §7, Inkscape's File menu): open an
 * SVG as a new drawing or into this one (file, drag and drop, clipboard),
 * open a drawing of the library, save under a new name, export, document
 * properties, the tracing reference, trace bitmap and the XML source view.
 * The editor hosts it through `FileHost`; everything here goes through the
 * editor's own undo.
 */

export interface FileHost {
  readonly ctx: AppContext;
  doc: SvgDoc;
  readonly selection: ReadonlySet<string>;
  readonly options: CanvasOptions;
  readonly canvas: SvgCanvas;
  /** Unsaved changes. */
  readonly dirty: boolean;
  /** The library drawing being edited (null: a new drawing). */
  readonly original: LibraryAsset | null;
  /** The name and category in the footer. */
  readonly name: string;
  readonly path: string[];
  change(key: string, fn: () => void): void;
  select(ids: string[]): void;
  status(text: string, kind?: 'ok' | 'warn'): void;
  setOption(patch: Partial<CanvasOptions>): void;
  /** Another drawing in the window (history starts anew). */
  open(doc: SvgDoc, asset: LibraryAsset | null, name: string): void;
  /** The drawing was saved as this library item (Farklı kaydet). */
  adopt(asset: LibraryAsset): void;
}

export const fileSlug = (s: string) =>
  s
    .toLocaleLowerCase('tr')
    .replace(/[çğıöşü]/g, (c) => ({ ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u' })[c] ?? c)
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'cizim';

export function download(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function pickFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;
    input.addEventListener('change', () => resolve(input.files?.[0] ?? null));
    input.addEventListener('cancel', () => resolve(null));
    input.click();
  });
}

export const readDataUrl = (f: Blob) =>
  new Promise<string>((ok, fail) => {
    const r = new FileReader();
    r.onload = () => ok(String(r.result));
    r.onerror = () => fail(r.error);
    r.readAsDataURL(f);
  });

const RASTER = /^image\/(png|jpeg)$/;
const baseName = (f: string) => f.replace(/\.[a-z0-9]+$/i, '') || 'Çizim';

/** A shape's stroke scaled with its geometry (transforms move points, not stroke widths). */
export function scaleStroke(s: SvgShape, k: number): SvgShape {
  return k === 1 ? s : { ...s, strokeWidth: s.strokeWidth * k, dash: s.dash?.map((d) => d * k) };
}

export class SvgFiles {
  readonly host: FileHost;
  readonly reference: ReferenceLayer;
  readonly source: SourcePanel;
  /** Buttons for the bar above the canvas. */
  readonly buttons: HTMLElement[];
  readonly saveAsButton: HTMLButtonElement;
  private readonly sourceBtn: HTMLButtonElement;

  constructor(host: FileHost) {
    this.host = host;
    this.reference = new ReferenceLayer(host, () => this.trace());
    this.source = new SourcePanel(host, () => this.toggleSource());
    const btn = (label: string, iconName: string, title: string, run: (b: HTMLButtonElement) => void) => {
      const b = h('button', { class: 'btn btn--small', type: 'button', title }, icon(iconName, 14), label);
      b.addEventListener('click', () => run(b));
      return b;
    };
    const menu = btn('Dosya', 'fileOpen', 'Aç, kitaplıktan aç, panodan al, dışa aktar, farklı kaydet, belge özellikleri', (b) => {
      const r = b.getBoundingClientRect();
      PopupMenu.open(this.menuItems(), { x: r.left, y: r.bottom + 4 }, { owner: b, minWidth: 260 });
    });
    menu.append(icon('chevronDown', 12));
    this.sourceBtn = btn('Kaynak', 'terminal', 'SVG kaynağı: seçilen şeklin öğesi vurgulanır; düzenleyip Uygula ile çizime aktarın (Ctrl+Shift+X)', () => this.toggleSource());
    this.buttons = [
      menu,
      btn('Altlık…', 'layers', 'İzleme altlığı: PNG, JPEG ya da SVG görüntüsünü kilitli, yarı saydam arka plan olarak koyar', () => void this.addReference()),
      btn('Bitmap izle…', 'spline', 'Bir görüntüyü (altlığı ya da dosyayı) delikli yollara çevirir', () => this.trace()),
      btn('Dışa aktar…', 'export', 'SVG (sembol ya da düz renkli) ya da PNG olarak indir veya panoya kopyala (Ctrl+Shift+E)', () => openExportDialog(this.host)),
      this.sourceBtn,
    ];
    this.saveAsButton = h('button', { class: 'btn', type: 'button', title: 'Yeni bir ad ve kategoriyle kitaplığa kaydeder (Ctrl+Shift+S)' }, 'Farklı kaydet…');
    this.saveAsButton.addEventListener('click', () => this.saveAs());
    const kept = host.original && host.original.data.includes('data-kentos="reference"') ? readSvg(host.original.data) : null;
    if (kept && 'reference' in kept && kept.reference) this.reference.restore(kept.reference);
  }

  /** Keys, clipboard paste and drops, on the editor's root and canvas. */
  attach(root: HTMLElement): void {
    root.addEventListener(
      'keydown',
      (e) => {
        if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
        const k = e.key.toLowerCase();
        const run = (fn: () => void) => {
          e.preventDefault();
          e.stopPropagation();
          fn();
        };
        if (k === 'o' && !e.shiftKey) run(() => void this.openFile());
        else if (k === 's' && e.shiftKey) run(() => this.saveAs());
        else if (k === 'e' && e.shiftKey) run(() => openExportDialog(this.host));
        else if (k === 'x' && e.shiftKey) run(() => this.toggleSource());
      },
      true,
    );
    root.addEventListener('paste', (e) => {
      if (e.defaultPrevented || (e.target as HTMLElement).closest('input, textarea, select')) return;
      const text = e.clipboardData?.getData('text/plain') ?? '';
      if (!looksLikeSvg(text)) return;
      e.preventDefault();
      this.addSvg(text, 'Pano');
    });
    const stage = this.host.canvas.el;
    stage.addEventListener('dragover', (e) => {
      if (!e.dataTransfer?.types.includes('Files') && !e.dataTransfer?.types.includes('text/plain')) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
      stage.classList.add('svge__stage--drop');
    });
    stage.addEventListener('dragleave', () => stage.classList.remove('svge__stage--drop'));
    stage.addEventListener('drop', (e) => {
      stage.classList.remove('svge__stage--drop');
      const f = e.dataTransfer?.files?.[0];
      const text = e.dataTransfer?.getData('text/plain') ?? '';
      if (!f && !looksLikeSvg(text)) return;
      e.preventDefault();
      if (!f) return this.importDialog(text, 'Bırakılan SVG');
      if (RASTER.test(f.type)) {
        // A picture: trace it, or keep it under the drawing.
        PopupMenu.open(
          [
            { label: 'İzleme altlığı yap', icon: 'layers', detail: 'Kilitli, yarı saydam arka plan; üzerinden çizin', run: () => void this.reference.loadFile(f) },
            { label: 'Bitmap izle…', icon: 'spline', detail: 'Görüntüyü delikli yollara çevirir', run: () => openTraceDialog(this.host, { file: f }) },
          ],
          { x: e.clientX, y: e.clientY },
          { placement: 'point', minWidth: 240 },
        );
        return;
      }
      if (/svg/i.test(f.type) || /\.svg$/i.test(f.name)) void f.text().then((t) => this.importDialog(t, baseName(f.name)));
      else this.host.status(`“${f.name}” alınamadı: SVG, PNG ya da JPEG dosyası bırakın.`, 'warn');
    });
  }

  private menuItems(): MenuItem[] {
    const has = this.host.doc.shapes.length > 0;
    return [
      { label: 'Yeni çizim', icon: 'fileNew', detail: 'Boş 100 × 100 tuval', run: () => this.newDrawing() },
      { label: 'SVG dosyası aç…', icon: 'fileOpen', shortcut: 'Ctrl+O', detail: 'Yeni çizim olarak ya da bu çizime ekleyerek; dosyayı tuvale bırakmak da olur', run: () => void this.openFile() },
      { label: 'Kitaplıktan aç…', icon: 'styles', detail: 'Stil kitaplığındaki bir SVG çizimi (sistem çiziminin kopyası açılır)', run: () => this.openLibrary() },
      { label: 'Panodan içe al', icon: 'paste', detail: 'Panodaki SVG metni (Ctrl+V de ekler)', run: () => void this.pasteDialog() },
      { kind: 'separator' },
      { label: 'İzleme altlığı ekle…', icon: 'layers', run: () => void this.addReference() },
      { label: 'Bitmap izle…', icon: 'spline', run: () => this.trace() },
      { kind: 'separator' },
      { label: 'Dışa aktar…', icon: 'export', shortcut: 'Ctrl+Shift+E', run: () => openExportDialog(this.host) },
      { label: 'SVG’yi panoya kopyala', icon: 'copy', disabled: !has, detail: 'Sembol SVG’si (currentColor ve param(stroke) ile)', run: () => void this.copySvg() },
      { kind: 'separator' },
      { label: 'Farklı kaydet…', icon: 'save', shortcut: 'Ctrl+Shift+S', run: () => this.saveAs() },
      { label: 'Belge özellikleri…', icon: 'settings', detail: 'Tuval ve görünüm kutusu, sembol boyu (mm), önizleme zemini', run: () => openDocProps(this) },
      { label: 'XML kaynağı', icon: 'terminal', shortcut: 'Ctrl+Shift+X', checked: this.source.open, run: () => this.toggleSource() },
    ];
  }

  // ── Hooks from the editor ────────────────────────────────────────────

  /** After every change: the source view follows, the paper follows the drawing's background. */
  refresh(): void {
    // The theme's paper is read each time: the theme may change while the editor is open.
    const paper = this.host.doc.background ?? resolveColor('paper', this.host.ctx.view.palette);
    if (this.host.options.paper !== paper) this.host.setOption({ paper });
    this.source.update();
  }

  underlay(world: SVGGElement): void {
    this.reference.underlay(world);
  }

  /** The reference as the saved file keeps it (only when asked to). */
  keptReference(): ReferenceSpec | null {
    return this.reference.kept();
  }

  // ── Opening ──────────────────────────────────────────────────────────

  private async openFile(): Promise<void> {
    const f = await pickFile('.svg,image/svg+xml');
    if (f) this.importDialog(await f.text(), baseName(f.name));
  }

  private async pasteDialog(): Promise<void> {
    let text = '';
    try {
      text = await navigator.clipboard.readText();
    } catch {
      return this.host.status('Tarayıcı panoyu okumaya izin vermedi: çizim alanını tıklayıp Ctrl+V basın.', 'warn');
    }
    if (!looksLikeSvg(text)) return this.host.status('Panoda SVG metni yok: bir çizimin kaynağını (<svg …>) kopyalayın.', 'warn');
    this.importDialog(text, 'Pano');
  }

  importDialog(text: string, name: string): void {
    openImportDialog(this, text, name);
  }

  /** SVG markup straight into this drawing (paste), fitted, with black as the symbol colour. */
  addSvg(text: string, name: string): void {
    const read = readSvg(text);
    if ('error' in read) return this.host.status(`${name}: ${read.error}`, 'warn');
    const ids = this.addShapes(read.doc);
    const { done, lost } = importSummary(read.report);
    this.host.status(`${done}${ids.length ? ' ve çizime eklendi' : ''}${lost.length ? `; ${lost.join(', ')}` : ''}.`, lost.length ? 'warn' : 'ok');
  }

  /** Another drawing's shapes fitted into this canvas (one undo step); the new ids. */
  addShapes(from: SvgDoc): string[] {
    const doc = this.host.doc;
    const k = Math.min(doc.width / from.width, doc.height / from.height);
    const dx = (doc.width - from.width * k) / 2;
    const dy = (doc.height - from.height * k) / 2;
    const groups = new Map<string, string>();
    const added = from.shapes.map((s) => {
      const group = s.group ? (groups.get(s.group) ?? groups.set(s.group, shapeId()).get(s.group)) : undefined;
      return { ...scaleStroke(transformShape(s, [k, 0, 0, k, dx, dy]), k), id: shapeId(), group };
    });
    if (!added.length) return [];
    this.host.change('import', () => (this.host.doc.shapes = [...this.host.doc.shapes, ...added]));
    this.host.select(added.map((s) => s.id));
    return added.map((s) => s.id);
  }

  /** Asks before a new drawing replaces unsaved work; runs `go` when it may. */
  confirmReplace(go: () => void): void {
    if (!this.host.dirty || !this.host.doc.shapes.length) return go();
    const at = this.buttons[0].getBoundingClientRect();
    PopupMenu.open(
      [
        { kind: 'header', label: 'Kaydedilmemiş değişiklikler var' },
        { label: 'Kaydetmeden aç', icon: 'fileOpen', run: go },
        { label: 'Vazgeç', icon: 'close', run: () => undefined },
      ],
      { x: at.left, y: at.bottom + 4 },
      { minWidth: 240 },
    );
  }

  private newDrawing(): void {
    this.confirmReplace(() => {
      this.host.open(newDoc(100, 100), null, 'Yeni çizim');
      this.host.status('Yeni çizim: soldaki araçlarla çizin ya da bir SVG dosyasını tuvale bırakın.');
    });
  }

  private openLibrary(): void {
    const lib = this.host.ctx.styles.library;
    void import('../style/StyleManager').then((m) =>
      m.openStyleManager(this.host.ctx, {
        pick: {
          kind: 'asset',
          title: 'Kitaplıktan SVG çizimi aç',
          current: this.host.original?.id,
          onPick: (id) =>
            this.confirmReplace(() => {
              let asset = lib.asset(id);
              if (!asset || asset.format !== 'svg') return this.host.status('Yalnızca SVG çizimleri açılır; görüntüler (PNG, JPEG) düzenlenmez.', 'warn');
              let note = '';
              if (!lib.canEdit(id)) {
                // System drawings are read-only: the copy is opened.
                asset = lib.copy(id, 'user', { path: ['Çizimlerim', ...asset.path.slice(-1)] }) as LibraryAsset;
                note = ' Sistem çizimi: kopyası Kitaplığım’a alındı.';
              }
              const read = readSvg(asset.data);
              if ('error' in read) return this.host.status(`“${asset.name}” açılamadı: ${read.error}`, 'warn');
              this.host.open(read.doc, asset, asset.name);
              this.reference.restore(read.reference ?? null);
              this.host.status(`“${asset.name}” açıldı.${note}`);
            }),
        },
      }),
    );
  }

  // ── Saving and the clipboard ─────────────────────────────────────────

  private saveAs(): void {
    const host = this.host;
    if (!host.doc.shapes.some((s) => !s.hidden)) return host.status('Boş çizim kaydedilmez: önce bir şekil çizin.', 'warn');
    const name = h('input', { class: 'field', value: host.original ? `${host.name} (kopya)` : host.name, 'aria-label': 'Ad', spellcheck: 'false' });
    const path = h('input', { class: 'field', value: host.path.join(' / '), 'aria-label': 'Kategori', spellcheck: 'false' });
    let to: EditableSource = 'user';
    const where = h('div');
    const renderWhere = () =>
      where.replaceChildren(
        segmented<EditableSource>({
          label: 'Nereye',
          options: [
            { value: 'user', label: 'Kitaplığım', hint: 'Bütün projelerde' },
            { value: 'project', label: 'Proje', hint: 'Yalnızca bu projede' },
          ],
          value: to,
          onChange: (v) => ((to = v), renderWhere()),
        }),
      );
    renderWhere();
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    const ok = h('button', { class: 'btn btn--primary', type: 'button' }, icon('check', 16), 'Kaydet');
    const dialog = new Dialog({
      title: 'Farklı kaydet',
      width: 440,
      className: 'dialog--svgfile',
      content: [
        h(
          'div',
          { class: 'svgf__form' },
          h('label', { class: 'svgf__field' }, h('span', { class: 'sdf__label' }, 'Ad'), name),
          h('label', { class: 'svgf__field' }, h('span', { class: 'sdf__label' }, 'Kategori (A / B)'), path),
          h('div', { class: 'svgf__field' }, h('span', { class: 'sdf__label' }, 'Nereye'), where),
          h('p', { class: 'sdf__hint' }, 'Kitaplığım bütün projelerde, Proje yalnızca bu projede görünür. Açık çizim artık yeni kayda bağlanır.'),
        ),
      ],
      footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, ok],
      stack: true,
    });
    cancel.addEventListener('click', () => dialog.close());
    const save = () => {
      const lib = host.ctx.styles.library;
      const parts = path.value.split('/').map((s) => s.trim()).filter(Boolean);
      const asset = svgAsset(name.value.trim() || 'Adsız çizim', parts.length ? parts : ['Çizimlerim'], sanitizeSvg(svgText(host.doc, { reference: this.keptReference() })));
      lib.add(to, asset);
      dialog.close();
      host.adopt(asset);
      host.status(`“${asset.name}” ${to === 'user' ? 'Kitaplığım' : 'Proje'} kitaplığına kaydedildi.`);
    };
    ok.addEventListener('click', save);
    name.addEventListener('keydown', (e) => e.key === 'Enter' && save());
    queueMicrotask(() => (name.focus(), name.select()));
  }

  private async copySvg(): Promise<void> {
    try {
      await navigator.clipboard.writeText(svgText(this.host.doc, { pretty: true }));
      this.host.status('Sembol SVG’si panoya kopyalandı.');
    } catch {
      this.host.status('Tarayıcı panoya yazmaya izin vermedi; Dışa aktar ile indirin.', 'warn');
    }
  }

  // ── Reference, trace, source ─────────────────────────────────────────

  private async addReference(): Promise<void> {
    const f = await pickFile('image/png,image/jpeg,.svg,image/svg+xml');
    if (f) await this.reference.loadFile(f);
  }

  private trace(): void {
    openTraceDialog(this.host, this.reference.ref ? { reference: this.reference } : {});
  }

  toggleSource(): void {
    this.source.toggle();
    this.sourceBtn.setAttribute('aria-pressed', String(this.source.open));
  }
}
