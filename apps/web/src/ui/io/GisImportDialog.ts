import type { AppContext } from '../../app/context';
import type { FileKind, PickedFile } from '../../app/fileIO';
import type { ImportLayer } from '../../contracts/generated/ImportLayer';
import type { ImportResult } from '../../contracts/generated/ImportResult';
import { applyImport, layerNamed, type LayerTarget } from '../../io/apply';
import { formats, type ImportMove } from '../../io/client';
import { shapefileSet } from '../../io/shapefile';
import { h, replaceChildren } from '../dom';
import { colorSwatch } from '../layers/swatch';
import { Dialog } from '../widgets/Dialog';
import { baseName, CrsQuestion, extentLine, fileLine, kindCounts, reportLines, reportText, summaryLine, type CrsStatement } from './common';
import { zoomToImported } from './zoom';

/**
 * GeoJSON, Shapefile ve KML/KMZ içe aktar (docs/adr/0046, 0222). The worker
 * reads the file once (crates/shared/formats: geojson, shp, kml) into points,
 * lines, paths and areas with holes, their attributes as text, and a report
 * of what was converted or left out. The window shows the source's layers
 * and where each goes (a project layer with the same name, or a new layer in
 * a group named after the file), the report, and what the file says of its
 * coordinate system: RFC 7946's WGS 84, a `crs` member, a .prj or KML's WGS
 * 84. A file in the project's system goes in untouched; one in another system
 * only when the user ticks “Projenin sistemine çevir”, its objects then moved
 * in the worker (docs/adr/0222 §2) before the window shows what will come in.
 * Nothing is transformed or guessed silently (CLAUDE.md §5); an import is one
 * undo step. The desktop's `exchange/gis_import.rs`.
 */
export function openGeoJsonImport(ctx: AppContext, file: PickedFile, kind: FileKind): void {
  new GisImportDialog(ctx, { kind: 'geojson', file }, kind);
}

/** A KML file or KMZ archive (docs/adr/0222 §3). */
export function openKmlImport(ctx: AppContext, file: PickedFile, kind: FileKind): void {
  new GisImportDialog(ctx, { kind: 'kml', file }, kind);
}

/** A Shapefile layer from the files the user chose together. */
export function openShapefileImport(ctx: AppContext, files: PickedFile[], kind: FileKind): void {
  new GisImportDialog(ctx, { kind: 'shp', files }, kind);
}

type Source = { kind: 'geojson'; file: PickedFile } | { kind: 'kml'; file: PickedFile } | { kind: 'shp'; files: PickedFile[] };

/** The window's title and the undo step's kind. */
const KIND = { geojson: ['GeoJSON içe aktar', 'GeoJSON'], kml: ['KML/KMZ içe aktar', 'KML'], shp: ['Shapefile içe aktar', 'Shapefile'] } as const;

/** What the file says of its coordinate system, for the question. */
function statement(r: ImportResult, source: Source): CrsStatement {
  const d = r.declaredCrs;
  if (!d) return { srid: null, text: '', source: source.kind === 'shp' ? 'none' : 'rfc7946' };
  return { srid: d.srid ?? null, text: d.text, source: d.source };
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

class GisImportDialog {
  private readonly ctx: AppContext;
  private readonly kind: FileKind;
  private source: Source;
  private result: ImportResult | null = null;
  private failed: string | null = null;
  private generation = 0;
  private reading = false;
  private closed = false;
  /** Source layer names left out by the user. */
  private readonly excluded = new Set<string>();
  private readonly crs: CrsQuestion;
  /** The objects moved into the project's system, for the source system they were moved from (docs/adr/0222 §2). */
  private moved: { srid: number; result: ImportResult; move: ImportMove } | null = null;
  /** The move running: the read it moves and the source system it moves from. Another system chosen meanwhile is not waiting for it. */
  private inFlight: { gen: number; srid: number } | null = null;
  private readonly fileHost = h('div');
  private readonly layersHost = h('div', { class: 'io-table-wrap' });
  private readonly summaryHost = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'İçe aktar');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, source: Source, kind: FileKind) {
    this.ctx = ctx;
    this.source = source;
    this.kind = kind;
    this.crs = new CrsQuestion(ctx, () => this.crsChanged(), { convertible: true });
    const other = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Başka dosya…');
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: KIND[source.kind][0],
      width: 900,
      className: 'dialog--io',
      // The system before the report: whether the file can go in at all is decided there.
      content: [this.fileHost, this.layersHost, this.crs.el, this.summaryHost],
      footer: [other, this.status, cancel, this.primary],
      onClose: () => {
        this.closed = true;
        if (this.reading) formats().cancel();
      },
    });
    other.addEventListener('click', () => void this.pickAnother());
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => this.run());
    void this.read();
  }

  /** The name the import goes by (the undo step, the new layers' group). */
  private get name(): string {
    if (this.source.kind !== 'shp') return this.source.file.name;
    const set = shapefileSet(this.source.files);
    return 'error' in set ? (this.source.files[0]?.name ?? 'Shapefile') : `${set.name}.shp`;
  }

  private async read(): Promise<void> {
    const gen = ++this.generation;
    this.result = null;
    this.failed = null;
    this.moved = null;
    this.excluded.clear();
    const src = this.source;
    const set = src.kind === 'shp' ? shapefileSet(src.files) : null;
    if (set && 'error' in set) {
      this.failed = set.error;
      this.crs.declare(null);
      this.render();
      return;
    }
    this.reading = true;
    this.render();
    this.say(`“${this.name}” okunuyor…`);
    try {
      const r =
        src.kind === 'geojson'
          ? // The bytes go to the worker without a copy (the window does not need them again).
            await formats().readGeoJson(src.file.bytes, { layer: baseName(src.file.name), maxEntities: 0 })
          : src.kind === 'kml'
            ? await formats().readKml(src.file.bytes)
            : await formats().readShapefile(set!.files, { layer: set!.name, maxEntities: 0 });
      if (gen !== this.generation || this.closed) return;
      this.result = r;
      this.crs.declare(statement(r, src), r.bounds ?? null);
      this.say('');
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      this.failed = message(e);
      this.crs.declare(null);
      this.say('');
    } finally {
      if (gen === this.generation) this.reading = false;
    }
    this.render();
    if (this.crs.convert) void this.move();
  }

  /** The question changed: the move again when it is asked for a system not yet moved from. */
  private crsChanged(): void {
    if (this.crs.convert) void this.move();
    this.render();
  }

  /** The objects moved into the project's system in the worker (docs/adr/0222 §2), once for each source system chosen. */
  private async move(): Promise<void> {
    const r = this.result;
    const srid = this.crs.chosen;
    const m = this.crs.moveBetween();
    const gen = this.generation;
    if (!r || srid === null || !m || this.moved?.srid === srid || this.isMoving(gen, srid)) return;
    this.inFlight = { gen, srid };
    this.render();
    try {
      const done = await formats().reprojectImport(r, m.from, m.to, m.choices);
      if (gen !== this.generation || this.closed) return;
      if (srid === this.crs.chosen) this.moved = { srid, result: done.result, move: done.moved };
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      if (srid === this.crs.chosen) this.say(message(e), 'error');
    } finally {
      if (this.isMoving(gen, srid)) this.inFlight = null;
    }
    this.render();
  }

  private isMoving(gen: number, srid: number): boolean {
    return this.inFlight !== null && this.inFlight.gen === gen && this.inFlight.srid === srid;
  }

  /** Whether the objects are being moved from the system chosen now. */
  private get moving(): boolean {
    return this.crs.chosen !== null && this.isMoving(this.generation, this.crs.chosen);
  }

  /** The objects that will come in: moved when the move is asked and done for the system chosen now, else as read. */
  private get effective(): ImportResult | null {
    const m = this.moved;
    return this.crs.convert && m && m.srid === this.crs.chosen ? m.result : this.result;
  }

  private async pickAnother(): Promise<void> {
    if (this.source.kind !== 'shp') {
      const f = await this.ctx.files.pickForImport(this.kind);
      if (!f || this.closed) return;
      this.source = { kind: this.source.kind, file: f };
    } else {
      const files = await this.ctx.files.pickManyForImport(this.kind);
      if (!files || this.closed) return;
      this.source = { kind: 'shp', files };
    }
    void this.read();
  }

  /** Where a source layer's objects go: the project layer of the same name, or a new one. */
  private target(l: ImportLayer): { existing?: string; locked: boolean } {
    const same = layerNamed(this.ctx.doc, l.name);
    return same ? { existing: same.id, locked: this.ctx.doc.layers.isLocked(same.id) } : { locked: false };
  }

  private included(): ImportLayer[] {
    return (this.effective?.layers ?? []).filter((l) => !this.excluded.has(l.name) && !this.target(l).locked);
  }

  private render(): void {
    const r = this.result;
    const src = this.source;
    let meta = r ? r.report.source.map((f) => `${f.label}: ${f.value}`).join(', ') : this.failed ? 'okunamadı' : 'okunuyor…';
    if (src.kind === 'shp') {
      const set = shapefileSet(src.files);
      if (!('error' in set)) meta = `${set.parts.join(', ')}${meta ? ` · ${meta}` : ''}`;
    }
    replaceChildren(this.fileHost, fileLine(this.name, meta));
    this.renderLayers();
    this.renderSummary();
    this.updateButton();
  }

  private renderLayers(): void {
    const r = this.effective;
    if (!r) {
      replaceChildren(this.layersHost, h('p', { class: 'io-empty' }, this.failed ? 'Katman yok.' : 'Dosya okunuyor…'));
      return;
    }
    if (!r.layers.length) {
      replaceChildren(this.layersHost, h('p', { class: 'io-empty' }, 'Dosyada alınacak nesne yok.'));
      return;
    }
    const all = h('input', { type: 'checkbox', 'aria-label': 'Bütün katmanlar', checked: this.excluded.size === 0 });
    all.indeterminate = this.excluded.size > 0 && this.excluded.size < r.layers.length;
    all.addEventListener('change', () => {
      this.excluded.clear();
      if (!all.checked) for (const l of r.layers) this.excluded.add(l.name);
      this.render();
    });
    const palette = this.ctx.view.palette;
    const rows = r.layers.map((l) => {
      const t = this.target(l);
      const box = h('input', { type: 'checkbox', 'aria-label': `“${l.name}” katmanını al`, checked: !this.excluded.has(l.name) && !t.locked, disabled: t.locked });
      box.addEventListener('change', () => {
        if (box.checked) this.excluded.delete(l.name);
        else this.excluded.add(l.name);
        this.render();
      });
      const where = t.locked
        ? `“${this.ctx.doc.layers.get(t.existing!)?.name}” katmanı kilitli; alınmaz. Kilidini Katmanlar panelinden açın.`
        : t.existing
          ? `“${this.ctx.doc.layers.path(t.existing)}” katmanına eklenir`
          : 'yeni katman';
      return h(
        'tr',
        { dataset: t.locked ? { error: '' } : undefined },
        h('td', { class: 'io-table__line' }, box),
        h('td', { class: 'io-table__name' }, h('span', { class: 'swatch', style: `--swatch:${colorSwatch(l.color, palette)}` }), ' ', l.name),
        h('td', { class: 'num' }, String(l.count)),
        h('td', { class: 'io-table__skip' }, where),
      );
    });
    replaceChildren(
      this.layersHost,
      h('table', { class: 'io-table' }, h('thead', null, h('tr', null, h('th', { class: 'io-table__line' }, all), h('th', null, 'Katman'), h('th', null, 'Nesne'), h('th', null, 'Nereye'))), h('tbody', null, rows)),
    );
  }

  private renderSummary(): void {
    const r = this.effective;
    if (!r) {
      replaceChildren(this.summaryHost, this.failed ? summaryLine('error', this.failed) : summaryLine('info', 'Dosya okunuyor; büyük dosyalar biraz sürebilir.'));
      return;
    }
    const chosen = new Set(this.included().map((l) => l.name));
    const counts = new Map<string, number>();
    for (const e of r.entities) if (chosen.has(e.layerId)) counts.set(e.kind, (counts.get(e.kind) ?? 0) + 1);
    const total = [...counts.values()].reduce((s, n) => s + n, 0);
    const created = this.included().filter((l) => !this.target(l).existing).length;
    const lines = [
      total
        ? summaryLine('ok', `${total} nesne alınacak: ${kindCounts(counts)}.${created ? ` ${created} yeni katman “${this.name}” grubunda kurulacak.` : ''}`)
        : summaryLine('warn', 'Alınacak nesne yok; en az bir katman seçin.'),
      this.crs.matches
        ? null
        : this.crs.convert && this.crs.ready
          ? this.moving
            ? summaryLine('info', 'Koordinatlar projenin sistemine çevriliyor…')
            : summaryLine('ok', 'Koordinatlar projenin sistemine çevrildi; aşağıdaki sayılar çevrilmiş nesnelerindir.')
          : summaryLine('warn', 'Koordinatların sistemi projeninki değil: içe aktarma kapalı (yukarıdaki koordinat sistemi notuna bakın).'),
      ...reportLines(r.report.notes, 'info'),
      ...reportLines(r.report.skipped, 'warn'),
    ];
    if (this.source.kind === 'shp') {
      const set = shapefileSet(this.source.files);
      if (!('error' in set) && set.unused.length) lines.push(summaryLine('info', `Kullanılmayan dosyalar (başka katmanın ya da tanınmayan): ${set.unused.join(', ')}.`));
    }
    if (r.bounds) lines.push(extentLine(this.ctx, r.bounds));
    replaceChildren(this.summaryHost, lines);
  }

  private updateButton(): void {
    // Moved: the objects moved for the system chosen now.
    const placed = this.crs.matches || (this.crs.ready && !this.moving && this.moved?.srid === this.crs.chosen);
    this.primary.disabled = this.reading || !this.result || !this.included().length || !placed;
  }

  private say(text: string, kind: 'info' | 'error' = 'info'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  private run(): void {
    const r = this.effective;
    if (this.primary.disabled || !r) return;
    const { ctx } = this;
    const name = this.name;
    const layers = new Map<string, LayerTarget>();
    for (const l of this.included()) {
      const t = this.target(l);
      layers.set(
        l.name,
        t.existing
          ? { kind: 'existing', id: t.existing }
          : { kind: 'new', name: l.name, style: { color: l.color, lineType: l.lineType }, visible: l.visible, locked: l.locked, fields: l.fields ?? [] },
      );
    }
    const label = `${KIND[this.source.kind][1]}: ${name}`;
    const applied = applyImport(ctx.doc, r.entities, { label, layers, group: name });
    if (!applied.ok) {
      this.say(applied.error, 'error');
      return;
    }
    zoomToImported(ctx, applied.ids);
    const into = applied.created.length ? `; ${applied.created.length} yeni katman “${name}” grubunda` : '';
    ctx.log.success(`“${name}”: ${applied.ids.length} nesne ${layers.size} katmana alındı${into}. Tek adımda geri alınabilir.`);
    if (r.report.skipped.length) ctx.log.warn(`“${name}” içinde alınmayanlar: ${r.report.skipped.map(reportText).join(' ')}`);
    this.dialog.close();
  }
}
