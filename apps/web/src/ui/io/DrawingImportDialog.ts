import type { AppContext } from '../../app/context';
import type { FileKind, PickedFile } from '../../app/fileIO';
import type { Bounds } from '../../contracts/generated/Bounds';
import type { ImportLayer } from '../../contracts/generated/ImportLayer';
import type { ReportItem } from '../../contracts/generated/ReportItem';
import { applyImport, layerNamed, type ImportPlan, type LayerTarget } from '../../io/apply';
import { formats, type ImportedDrawing } from '../../io/client';
import { AT_ONCE, ProgressiveImport, importedEntities, viewOf } from '../../io/drawingImport';
import { KcadError } from '../../io/kcad';
import { h, replaceChildren } from '../dom';
import { colorSwatch } from '../layers/swatch';
import { Dialog } from '../widgets/Dialog';
import { CrsQuestion, extentLine, fileLine, kindCounts, reportLines, reportText, summaryLine } from './common';
import { writeImport } from './importing';

/**
 * DXF içe aktar and NCZ içe aktar (docs/adr/0138). The worker reads the
 * whole file once, in the module of its format (crates/wasm/dxf-wasm,
 * crates/wasm/ncz-wasm, loaded the first time such a file is imported), and
 * says how far it is; the window shows the source's layers with their object
 * counts and where each goes (a project layer with the same name, or a new
 * layer in a group named after the file), the report, and the coordinate
 * system question (an NCZ says its system; a DXF says nothing). Everything
 * chosen goes in as one undo step: at once when it is small, a frame at a
 * time when it is large (io/drawingImport.ts), the window closing and a panel
 * counting the objects while the drawing fills in. The desktop's window is
 * the same (apps/desktop/src/exchange/drawing_import.rs).
 */

/** The file's format: what it is read with and what the window says. */
export type DrawingSource = 'dxf' | 'ncz';

const SOURCE: Record<DrawingSource, { title: string; column: string; prefix: string }> = {
  dxf: { title: 'DXF içe aktar', column: 'DXF katmanı', prefix: 'DXF' },
  ncz: { title: 'NCZ içe aktar', column: 'NCZ katmanı', prefix: 'NCZ' },
};

export function openDrawingImport(ctx: AppContext, file: PickedFile, kind: FileKind, source: DrawingSource): void {
  new DrawingImportDialog(ctx, file, kind, source);
}

/** Kept for the DXF command's loader (app/fileExchange.ts). */
export function openDxfImport(ctx: AppContext, file: PickedFile, kind: FileKind): void {
  openDrawingImport(ctx, file, kind, 'dxf');
}

export function openNczImport(ctx: AppContext, file: PickedFile, kind: FileKind): void {
  openDrawingImport(ctx, file, kind, 'ncz');
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const count = (n: number) => n.toLocaleString('tr-TR');

/** At least this much around what was imported (one point alone is shown in context). */
const MIN_SPAN = 20;

class DrawingImportDialog {
  private readonly ctx: AppContext;
  private readonly kind: FileKind;
  private readonly source: DrawingSource;
  private file: PickedFile;
  private drawing: ImportedDrawing | null = null;
  private failed: string | null = null;
  private generation = 0;
  /** How far the read under way is (0–1); null when no read is. */
  private reading: number | null = null;
  private closed = false;
  /** Source layer names left out by the user. */
  private readonly excluded = new Set<string>();
  private readonly crs: CrsQuestion;
  private readonly fileHost = h('div');
  private readonly layersHost = h('div', { class: 'io-table-wrap' });
  private readonly summaryHost = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'İçe aktar');
  private readonly dialog: Dialog;
  private readonly bar = h('span');
  private readonly readText = h('p', { class: 'io-reading__text', role: 'status' });

  constructor(ctx: AppContext, file: PickedFile, kind: FileKind, source: DrawingSource) {
    this.ctx = ctx;
    this.file = file;
    this.kind = kind;
    this.source = source;
    this.crs = new CrsQuestion(ctx, () => this.updateButton());
    const other = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Başka dosya…');
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: SOURCE[source].title,
      width: 900,
      className: 'dialog--io',
      content: [this.fileHost, this.layersHost, this.summaryHost, this.crs.el],
      footer: [other, this.status, cancel, this.primary],
      onClose: () => {
        this.closed = true;
        // A file still being read stops with the window.
        if (this.reading !== null) formats().cancel();
      },
    });
    other.addEventListener('click', () => void this.pickAnother());
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => this.run());
    void this.read();
  }

  private async read(): Promise<void> {
    // A read still under way (another file picked) ends first: one request at a time goes to the worker.
    if (this.reading !== null) formats().cancel();
    const gen = ++this.generation;
    this.drawing = null;
    this.failed = null;
    this.reading = 0;
    this.excluded.clear();
    this.render();
    this.say('');
    const progress = (p: { stage: string; done?: number; total?: number }) => {
      if (gen !== this.generation || p.stage !== 'reading' || !p.total) return;
      this.reading = Math.min(1, (p.done ?? 0) / p.total);
      this.showReading();
    };
    try {
      // The bytes go to the worker without a copy (the window does not need them again).
      const d =
        this.source === 'ncz'
          ? await formats().readNcz(this.file.bytes, { maxEntities: 0, drawingFont: this.ctx.doc.settings.drawingFont.value }, progress)
          : await formats().readDxf(this.file.bytes, { maxEntities: 0 }, progress);
      if (gen !== this.generation || this.closed) return;
      this.drawing = d;
      // An NCZ says what its coordinates are in (its projection blocks); a DXF says nothing.
      if (this.source === 'ncz') {
        const said = d.result.declaredCrs;
        this.crs.declare(said ? { srid: said.srid ?? null, text: said.text, source: 'ncz' } : null, d.result.bounds);
      }
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      // A read stopped by “Başka dosya…” says nothing.
      if (e instanceof KcadError && e.code === 'cancelled') return;
      this.failed = message(e);
    } finally {
      if (gen === this.generation) this.reading = null;
    }
    this.render();
  }

  private async pickAnother(): Promise<void> {
    const f = await this.ctx.files.pickForImport(this.kind);
    if (!f || this.closed) return;
    this.file = f;
    void this.read();
  }

  /** Where a source layer's objects go: the project layer of the same name, or a new one. */
  private target(l: ImportLayer): { existing?: string; locked: boolean } {
    const same = layerNamed(this.ctx.doc, l.name);
    return same ? { existing: same.id, locked: this.ctx.doc.layers.isLocked(same.id) } : { locked: false };
  }

  private included(): ImportLayer[] {
    return (this.drawing?.result.layers ?? []).filter((l) => !this.excluded.has(l.name) && !this.target(l).locked);
  }

  private render(): void {
    const r = this.drawing?.result;
    const facts = r?.report.source.map((f) => `${f.label}: ${f.value}`).join(', ');
    replaceChildren(this.fileHost, fileLine(this.file.name, r ? (facts ?? '') : this.failed ? 'okunamadı' : 'okunuyor…'));
    this.renderLayers();
    this.renderSummary();
    this.updateButton();
  }

  private showReading(): void {
    const share = Math.round((this.reading ?? 0) * 100);
    this.bar.style.width = `${share}%`;
    this.readText.textContent = `“${this.file.name}” okunuyor… %${share}`;
  }

  private renderLayers(): void {
    const r = this.drawing?.result;
    if (!r) {
      if (this.failed) replaceChildren(this.layersHost, h('p', { class: 'io-empty' }, 'Katman yok.'));
      else {
        this.showReading();
        replaceChildren(this.layersHost, h('div', { class: 'io-reading' }, h('div', { class: 'io-reading__bar' }, this.bar), this.readText));
      }
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
          : `yeni katman${l.visible ? '' : ', gizli'}${l.locked ? ', kilitli' : ''}`;
      return h(
        'tr',
        { dataset: t.locked ? { error: '' } : undefined },
        h('td', { class: 'io-table__line' }, box),
        h('td', null, h('span', { class: 'swatch', style: `--swatch:${colorSwatch(l.color, palette)}` }), ' ', l.name),
        h('td', { class: 'num' }, count(l.count)),
        h('td', { class: 'io-table__skip' }, where),
      );
    });
    replaceChildren(
      this.layersHost,
      h(
        'table',
        { class: 'io-table' },
        h('thead', null, h('tr', null, h('th', { class: 'io-table__line' }, all), h('th', null, SOURCE[this.source].column), h('th', null, 'Nesne'), h('th', null, 'Nereye'))),
        h('tbody', null, rows),
      ),
    );
  }

  private renderSummary(): void {
    const r = this.drawing?.result;
    if (!r) {
      replaceChildren(this.summaryHost, this.failed ? summaryLine('error', this.failed) : summaryLine('info', 'Dosya okunuyor; büyük dosyalar biraz sürebilir. Vazgeç okumayı durdurur.'));
      return;
    }
    // Counted from the layers, not the objects: a large file holds hundreds of thousands.
    const counts = new Map<string, number>();
    for (const l of this.included()) for (const [k, n] of Object.entries(l.kinds)) counts.set(k, (counts.get(k) ?? 0) + (n ?? 0));
    const total = [...counts.values()].reduce((s, n) => s + n, 0);
    const created = this.included().filter((l) => !this.target(l).existing).length;
    const lines = [
      total
        ? summaryLine('ok', `${count(total)} nesne alınacak: ${kindCounts(counts)}.${created ? ` ${created} yeni katman “${this.file.name}” grubunda kurulacak.` : ''}`)
        : summaryLine('warn', 'Alınacak nesne yok; en az bir katman seçin.'),
    ];
    if (total > AT_ONCE)
      lines.push(summaryLine('info', 'Nesneler çizime parça parça yazılır: pencere kapanır, çizim doldukça görünür, sağ alttaki panel sayar ve Durdur hepsini geri alır.'));
    lines.push(...reportLines(r.report.notes, 'info'), ...reportLines(r.report.skipped, 'warn'));
    if (r.bounds) lines.push(extentLine(this.ctx, r.bounds));
    replaceChildren(this.summaryHost, lines);
  }

  private updateButton(): void {
    this.primary.disabled = this.reading !== null || !this.drawing || !this.included().length || !this.crs.matches;
  }

  private say(text: string, kind: 'info' | 'error' = 'info'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  private run(): void {
    const d = this.drawing;
    if (this.primary.disabled || !d) return;
    const { ctx } = this;
    const name = this.file.name;
    const included = this.included();
    const layers = new Map<string, LayerTarget>();
    for (const l of included) {
      const t = this.target(l);
      layers.set(
        l.name,
        t.existing
          ? { kind: 'existing', id: t.existing }
          : { kind: 'new', name: l.name, style: { color: l.color, lineType: l.lineType, ...(l.lineWeight ? { lineWeight: l.lineWeight } : {}) }, visible: l.visible, locked: l.locked },
      );
    }
    const plan: ImportPlan = { label: `${SOURCE[this.source].prefix}: ${name}`, layers, group: name };
    const skipped = d.result.report.skipped;
    const view = viewOf(
      d.result.view,
      included.flatMap((l) => (l.bounds ? [l.bounds] : [])),
    );
    const total = included.reduce((n, l) => n + l.count, 0);

    if (total <= AT_ONCE) {
      const applied = applyImport(ctx.doc, importedEntities(d, new Set(layers.keys())), plan);
      if (!applied.ok) {
        this.say(applied.error, 'error');
        return;
      }
      this.dialog.close();
      if (view) zoomTo(ctx, view);
      said(ctx, name, applied.ids.length, layers.size, applied.created, skipped);
      return;
    }

    // A large file: the layers now, the objects a frame at a time, one undo step.
    const work = ProgressiveImport.start(ctx.doc, d, plan);
    if ('error' in work) {
      this.say(work.error, 'error');
      return;
    }
    this.drawing = null;
    this.dialog.close();
    // The view goes where the file is first, so the drawing fills in before the user's eyes.
    if (view) zoomTo(ctx, view);
    writeImport(ctx, name, work, (w) => {
      if (w.kind === 'done') said(ctx, name, w.objects, layers.size, work.created, skipped);
      else if (w.kind === 'stopped') ctx.log.warn(`“${name}” içe aktarılması durduruldu; çizim olduğu gibi kaldı.`);
      else ctx.log.error(w.error);
    });
  }
}

/** Shows `b`, never smaller than a few metres across. */
function zoomTo(ctx: AppContext, b: Bounds): void {
  const grow = (lo: number, hi: number) => {
    const pad = Math.max(0, (MIN_SPAN - (hi - lo)) / 2);
    return [lo - pad, hi + pad];
  };
  const [minX, maxX] = grow(b.minX, b.maxX);
  const [minY, maxY] = grow(b.minY, b.maxY);
  ctx.view.camera.fit({ minX, minY, maxX, maxY });
}

/** What went in, said in the message log (the desktop's `import_said`). */
function said(ctx: AppContext, name: string, objects: number, layers: number, created: readonly string[], skipped: readonly ReportItem[]): void {
  const into = created.length ? `; ${created.length} yeni katman “${name}” grubunda` : '';
  ctx.log.success(`“${name}”: ${count(objects)} nesne ${layers} katmana alındı${into}. Tek adımda geri alınabilir.`);
  if (skipped.length) ctx.log.warn(`“${name}” içinde alınmayanlar: ${skipped.map(reportText).join(' ')}`);
}
