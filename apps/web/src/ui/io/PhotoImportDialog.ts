import { PHOTO_PATH } from '../../app/attachments';
import type { AppContext } from '../../app/context';
import type { PickedFile } from '../../app/fileIO';
import type { PhotoExif } from '../../contracts/generated/PhotoExif';
import { applyImport, layerNamed, type LayerTarget } from '../../io/apply';
import { formats } from '../../io/client';
import { POINT_LAYER_STYLE } from '../../io/coords';
import { formatDd } from '../../model/geom/crsTransform';
import { photoEntities, placePhotos, type Photo, type PhotoPlan, type PlacedPhoto } from '../../model/photoImport';
import { crsSettings, datumChoices, ownSystem, type NamedSystem } from '../../model/projectCrs';
import type { LibraryAsset } from '../../model/style';
import { hasPicture } from '../../product/entitiesEdit';
import { pictureItem } from '../../tools/pictureFile';
import { h, replaceChildren } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { extentLine, field, fileLine, select, summaryLine } from './common';
import { zoomToImported } from './zoom';

/** The window's title and the undo step's name. */
export const PHOTO_IMPORT_TITLE = 'Konumlu fotoğraflar';

/** The photos Konumlu fotoğraflar offers to open. */
export const PHOTO_FILES = { description: 'Fotoğraf (JPEG)', accept: { 'image/jpeg': ['.jpg', '.jpeg'] } };

const PREVIEW_ROWS = 12;
const SHOWN_PROBLEMS = 5;
const NEW = '__new__';

/** A photo as the window read it: its file, what its EXIF says, and its image for the library (null when it cannot be one). */
interface Read {
  readonly file: PickedFile;
  readonly exif: PhotoExif;
  readonly item: { id: string; item: LibraryAsset } | null;
}

/** Konumlu fotoğraflar: asks for the photos, then opens the window on them. */
export async function openPhotoImport(ctx: AppContext): Promise<void> {
  const picked = await ctx.files.pickManyForImport(PHOTO_FILES);
  if (!picked?.length) return;
  new PhotoImportDialog(ctx, picked);
}

/**
 * Konumlu fotoğraflar (docs/adr/0225 §3; the desktop's `exchange/photo_import.rs`): JPEG photos' EXIF read by the formats
 * worker (crates/shared/formats/src/exif.rs), each place moved from WGS 84 into the project's system the way Koordinat
 * dönüştür moves it (model/photoImport.ts); the photos show as they will be written, those without a place or out of
 * the system's reach with why. Each photo a point with its attributes and itself as its attachment, embedded in the
 * project's library (the browser keeps no file's path). One undo step “Konumlu fotoğraflar”, onto a new layer or one
 * of the drawing's. A project without a system takes none (CLAUDE.md §5).
 */
class PhotoImportDialog {
  private readonly ctx: AppContext;
  private files: PickedFile[];
  private read: Read[] = [];
  private reading = false;
  private closed = false;
  private generation = 0;
  private target = NEW;
  private plan: PhotoPlan | null = null;
  private readonly system: NamedSystem | null;
  private readonly nameInput: HTMLInputElement;
  private readonly fileHost = h('div');
  private readonly tableHost = h('div', { class: 'io-table-wrap' });
  private readonly summaryHost = h('div', { class: 'io-summary' });
  private readonly layerHost = h('div', { class: 'io-row' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'İçe aktar');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, files: PickedFile[]) {
    this.ctx = ctx;
    this.files = files;
    this.system = ownSystem(crsSettings(ctx.doc.settings));
    this.nameInput = h('input', { class: 'field', value: 'Fotoğraflar', 'aria-label': 'Yeni katmanın adı', spellcheck: 'false', dataset: { key: 'name' } });
    this.nameInput.addEventListener('input', () => this.updateButton());
    this.nameInput.addEventListener('keydown', (e) => e.key === 'Enter' && this.run());
    const other = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Başka fotoğraflar…');
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: PHOTO_IMPORT_TITLE,
      width: 960,
      className: 'dialog--io dialog--photo-import',
      content: [this.fileHost, this.tableHost, this.summaryHost, this.layerHost],
      footer: [other, this.status, cancel, this.primary],
      onClose: () => {
        this.closed = true;
      },
    });
    other.addEventListener('click', () => void this.pickAnother());
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => this.run());
    this.renderLayer();
    void this.refresh();
  }

  /** Reads every photo's EXIF and image; a late answer to an older read is dropped. */
  private async refresh(): Promise<void> {
    const gen = ++this.generation;
    this.reading = true;
    this.read = [];
    this.render();
    this.say('Fotoğraflar okunuyor…');
    const out: Read[] = [];
    for (const file of this.files) {
      let exif: PhotoExif;
      try {
        exif = await formats().readExif(file.bytes);
      } catch (e) {
        exif = { problem: `Fotoğraf okunamadı: ${e instanceof Error ? e.message : String(e)}` };
      }
      const got = await pictureItem(file.name, file.bytes, PHOTO_PATH);
      if (gen !== this.generation || this.closed) return;
      if (!got.ok && exif.latitude != null) exif = { ...exif, latitude: undefined, longitude: undefined, problem: got.error };
      out.push({ file, exif, item: got.ok ? { id: got.id, item: got.item } : null });
    }
    this.read = out;
    this.reading = false;
    this.say('');
    this.render();
  }

  private async pickAnother(): Promise<void> {
    const f = await this.ctx.files.pickManyForImport(PHOTO_FILES);
    if (!f?.length || this.closed) return;
    this.files = f;
    void this.refresh();
  }

  private place(): void {
    const to = this.system?.system;
    if (!this.read.length || !to || !this.system) {
      this.plan = null;
      return;
    }
    const photos: Photo[] = this.read.map((r) => ({ name: r.file.name, exif: r.exif, ...(r.item && { asset: r.item.id }) }));
    this.plan = placePhotos(photos, to, this.system.name, datumChoices(crsSettings(this.ctx.doc.settings)));
  }

  private render(): void {
    const n = this.files.length;
    const bytes = this.files.reduce((s, f) => s + f.bytes.length, 0);
    replaceChildren(this.fileHost, fileLine(n === 1 ? this.files[0].name : `${n} fotoğraf`, `${(bytes / (1024 * 1024)).toLocaleString('tr-TR', { maximumFractionDigits: 1 })} MB${this.reading ? ', okunuyor…' : ''}`));
    this.place();
    this.renderTable();
    this.renderSummary();
    this.updateButton();
  }

  private renderTable(): void {
    const plan = this.plan;
    if (this.reading || !plan) {
      replaceChildren(this.tableHost, h('p', { class: 'io-empty' }, this.reading ? 'Fotoğraflar okunuyor…' : 'Önizleme yok: konumlar projenin sistemine çevrilemiyor.'));
      return;
    }
    const f = this.ctx.format;
    const geographic = this.system?.system?.kind === 'geographic';
    const [first, second] = geographic ? ['Enlem', 'Boylam'] : [f.axesText('Y (sağa)'), f.axesText('X (yukarı)')];
    const place = (p: PlacedPhoto): [string, string] => (geographic ? [formatDd(p.p.y, true, 9), formatDd(p.p.x, false, 9)] : [f.coord(p.p.x), f.coord(p.p.y)]);
    const placed = new Map(plan.placed.map((p) => [p.index, p]));
    const said = new Map(plan.skipped.map((s) => [s.line, s.message]));
    const head = h('tr', null, h('th', { class: 'io-table__line' }, 'Sıra'), ['Ad', 'Tarih', first, second, 'Yön', 'Durum'].map((t) => h('th', null, t)));
    const rows = this.read.slice(0, PREVIEW_ROWS).map((r, i) => {
      const p = placed.get(i + 1);
      const [a, b] = p ? place(p) : ['—', '—'];
      return h(
        'tr',
        null,
        h('td', { class: 'io-table__line' }, String(i + 1)),
        h('td', null, r.file.name),
        h('td', null, r.exif.taken ?? ''),
        h('td', { class: 'num' }, a),
        h('td', { class: 'num' }, b),
        h('td', { class: 'num' }, p?.attrs['Yön'] ?? ''),
        h('td', { class: p ? null : 'io-table__warn', title: said.get(i + 1) ?? '' }, p ? 'Alınacak' : (r.exif.problem ?? 'Alınmayacak')),
      );
    });
    replaceChildren(this.tableHost, h('table', { class: 'io-table photo-table' }, h('thead', null, head), h('tbody', null, rows)));
  }

  private renderSummary(): void {
    if (this.reading) return void replaceChildren(this.summaryHost, summaryLine('info', 'Fotoğraflar okunuyor…'));
    const lines: HTMLElement[] = [];
    const plan = this.plan;
    if (!this.system) lines.push(summaryLine('error', "Projenin koordinat sistemi yok: fotoğrafların konumu WGS 84'tedir ve yerel sisteme çevrilemez. Proje ayarlarından bir koordinat sistemi seçin ya da özel sistem tanımlayın."));
    else if (!this.system.system) lines.push(summaryLine('error', `“${this.system.name}” sisteminin tanımı dönüşümlerde kullanılamıyor; fotoğrafların konumu ona çevrilemez.`));
    else if (plan) {
      const n = plan.placed.length;
      lines.push(n ? summaryLine('ok', `${n.toLocaleString('tr-TR')} fotoğraf nokta olarak alınacak; her biri kendi fotoğrafını ek olarak taşır (projenin kitaplığına gömülür).`) : summaryLine('warn', 'Konumu olan fotoğraf yok.'));
      const way = plan.placed[0]?.attrs['Dönüşüm'];
      if (way) lines.push(summaryLine(way.endsWith('resmî dönüşüm değil') ? 'warn' : 'info', `Dönüşüm: ${way}.`));
      if (n) lines.push(summaryLine('info', "Yükseklik deniz düzeyine göredir ve dayanağı EXIF'te yazmaz: kot olarak değil, Yükseklik özniteliği olarak yazılır."));
      if (plan.skipped.length) {
        const shown = plan.skipped.slice(0, SHOWN_PROBLEMS);
        lines.push(
          summaryLine(
            'warn',
            `${plan.skipped.length.toLocaleString('tr-TR')} fotoğraf alınmayacak:`,
            h('ul', { class: 'io-summary__list' }, shown.map((e) => h('li', null, e.message)), plan.skipped.length > shown.length ? h('li', null, `… ve ${plan.skipped.length - shown.length} fotoğraf daha.`) : null),
          ),
        );
      }
      if (n && this.system.system.kind !== 'geographic') {
        const xs = plan.placed.map((p) => p.p.x);
        const ys = plan.placed.map((p) => p.p.y);
        lines.push(extentLine(this.ctx, { minX: Math.min(...xs), minY: Math.min(...ys), maxX: Math.max(...xs), maxY: Math.max(...ys) }));
      }
    }
    replaceChildren(this.summaryHost, lines);
  }

  private renderLayer(): void {
    const layers = this.ctx.doc.layers;
    const choices = [
      { value: NEW, label: 'Yeni katman' },
      ...layers
        .leaves()
        .filter((l) => !l.service)
        .map((l) => ({ value: l.id, label: `${layers.path(l.id)}${layers.isLocked(l.id) ? ' (kilitli)' : !layers.isVisible(l.id) ? ' (gizli)' : ''}`, disabled: layers.isLocked(l.id) })),
    ];
    const pick = select('Hedef katman', choices, this.target, (v) => {
      this.target = v;
      this.renderLayer();
      this.updateButton();
    }, 'target');
    const hidden = this.target !== NEW && !layers.isVisible(this.target);
    replaceChildren(
      this.layerHost,
      field('Hedef katman', pick, hidden ? 'Katman gizli: noktalar alınır ama görünmez.' : null, 'wide'),
      this.target === NEW ? field('Yeni katmanın adı', this.nameInput, 'Bu adda bir katman varsa noktalar ona eklenir.', 'grow') : null,
    );
  }

  private layerTarget(): LayerTarget | null {
    if (this.target !== NEW) return { kind: 'existing', id: this.target };
    const name = this.nameInput.value.trim();
    if (!name) return null;
    const same = layerNamed(this.ctx.doc, name);
    return same ? { kind: 'existing', id: same.id } : { kind: 'new', name, style: POINT_LAYER_STYLE, visible: true, locked: false };
  }

  private updateButton(): void {
    const t = this.layerTarget();
    const locked = t?.kind === 'existing' && this.ctx.doc.layers.isLocked(t.id);
    this.primary.disabled = this.reading || !this.plan || this.plan.placed.length === 0 || !t || locked;
    if (locked) this.say(`“${this.ctx.doc.layers.get((t as { id: string }).id)?.name}” katmanı kilitli; kilidini Katmanlar panelinden açın ya da başka bir katman seçin.`, 'error');
    else if (this.status.dataset.kind === 'error') this.say('');
  }

  private say(text: string, kind: 'info' | 'error' = 'info'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  private run(): void {
    const plan = this.plan;
    const target = this.layerTarget();
    if (this.primary.disabled || !plan || !target) return;
    const { ctx } = this;
    // The placed photos' images in the project's library first (an edit, no undo step: docs/adr/0092), each once.
    const doc = ctx.doc;
    const wanted = new Set(plan.placed.map((p) => p.attachment.asset).filter((a): a is string => a != null));
    const items = this.read.flatMap((r) => (r.item && wanted.has(r.item.id) && !hasPicture(doc, r.item.id) ? [r.item.item] : []));
    const fresh = items.filter((it, i) => items.findIndex((o) => o.id === it.id) === i);
    if (fresh.length) doc.styles.set({ ...doc.styles.value, items: [...doc.styles.value.items, ...fresh] });
    const applied = applyImport(doc, photoEntities(plan.placed), { label: PHOTO_IMPORT_TITLE, layers: new Map([['', target]]) });
    if (!applied.ok) return void this.say(applied.error, 'error');
    const layerName = target.kind === 'new' ? target.name : (doc.layers.get(target.id)?.name ?? '');
    zoomToImported(ctx, applied.ids);
    ctx.log.success(`${plan.placed.length} konumlu fotoğraf “${layerName}” katmanına nokta olarak alındı${applied.created.length ? ' (yeni katman)' : ''}. Tek adımda geri alınabilir.`);
    if (plan.skipped.length) ctx.log.warn(`${plan.skipped.length} fotoğraf alınmadı (konumu yok ya da projenin sistemine çevrilemiyor); nedenleri pencerede yazılıydı.`);
    this.dialog.close();
  }
}
