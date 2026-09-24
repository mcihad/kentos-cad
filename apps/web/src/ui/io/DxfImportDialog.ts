import type { AppContext } from '../../app/context';
import type { FileKind, PickedFile } from '../../app/fileIO';
import type { ImportLayer } from '../../contracts/generated/ImportLayer';
import type { ImportResult } from '../../contracts/generated/ImportResult';
import { applyImport, layerNamed, type LayerTarget } from '../../io/apply';
import { formats } from '../../io/client';
import { ENTITY_KIND_LABEL, type EntityKind } from '../../model/entities';
import { h, replaceChildren } from '../dom';
import { colorSwatch } from '../layers/swatch';
import { Dialog } from '../widgets/Dialog';
import { CrsQuestion, extentLine, fileLine, reportLines, reportText, summaryLine } from './common';
import { zoomToImported } from './zoom';

/**
 * DXF içe aktar. The worker reads the whole file once (crates/shared/formats:
 * blocks exploded, object coordinate systems applied, a report of what was
 * converted or left out); the window shows the source's layers with their
 * object counts and where each goes (a project layer with the same name,
 * or a new layer in a group named after the file), the report, and the
 * coordinate system question. Everything chosen goes in as one undo step.
 */
export function openDxfImport(ctx: AppContext, file: PickedFile, kind: FileKind): void {
  new DxfImportDialog(ctx, file, kind);
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Plural-free Turkish counts ("12 çizgi, 3 yay"). */
function kindCounts(counts: ReadonlyMap<string, number>): string {
  return [...counts]
    .sort((a, b) => b[1] - a[1])
    .map(([k, n]) => `${n} ${(ENTITY_KIND_LABEL[k as EntityKind] ?? k).toLocaleLowerCase('tr-TR')}`)
    .join(', ');
}

class DxfImportDialog {
  private readonly ctx: AppContext;
  private readonly kind: FileKind;
  private file: PickedFile;
  private result: ImportResult | null = null;
  private failed: string | null = null;
  private generation = 0;
  private reading = false;
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

  constructor(ctx: AppContext, file: PickedFile, kind: FileKind) {
    this.ctx = ctx;
    this.file = file;
    this.kind = kind;
    this.crs = new CrsQuestion(ctx, () => this.updateButton());
    const other = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Başka dosya…');
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'DXF içe aktar',
      width: 900,
      className: 'dialog--io',
      content: [this.fileHost, this.layersHost, this.summaryHost, this.crs.el],
      footer: [other, this.status, cancel, this.primary],
      onClose: () => {
        this.closed = true;
        // A large file still being read stops with the window.
        if (this.reading) formats().cancel();
      },
    });
    other.addEventListener('click', () => void this.pickAnother());
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => this.run());
    void this.read();
  }

  private async read(): Promise<void> {
    const gen = ++this.generation;
    this.result = null;
    this.failed = null;
    this.reading = true;
    this.excluded.clear();
    this.render();
    this.say(`“${this.file.name}” okunuyor…`);
    try {
      // The bytes go to the worker without a copy (the window does not need them again).
      const r = await formats().readDxf(this.file.bytes, { maxEntities: 0 });
      if (gen !== this.generation || this.closed) return;
      this.result = r;
      this.say('');
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      this.failed = message(e);
      this.say('');
    } finally {
      if (gen === this.generation) this.reading = false;
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
    return (this.result?.layers ?? []).filter((l) => !this.excluded.has(l.name) && !this.target(l).locked);
  }

  private render(): void {
    const r = this.result;
    const facts = r?.report.source.map((f) => `${f.label}: ${f.value}`).join(', ');
    replaceChildren(this.fileHost, fileLine(this.file.name, r ? (facts ?? '') : this.failed ? 'okunamadı' : 'okunuyor…'));
    this.renderLayers();
    this.renderSummary();
    this.updateButton();
  }

  private renderLayers(): void {
    const r = this.result;
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
          : `yeni katman${l.visible ? '' : ', gizli'}${l.locked ? ', kilitli' : ''}`;
      return h(
        'tr',
        { dataset: t.locked ? { error: '' } : undefined },
        h('td', { class: 'io-table__line' }, box),
        h('td', null, h('span', { class: 'swatch', style: `--swatch:${colorSwatch(l.color, palette)}` }), ' ', l.name),
        h('td', { class: 'num' }, String(l.count)),
        h('td', { class: 'io-table__skip' }, where),
      );
    });
    replaceChildren(
      this.layersHost,
      h('table', { class: 'io-table' }, h('thead', null, h('tr', null, h('th', { class: 'io-table__line' }, all), h('th', null, 'DXF katmanı'), h('th', null, 'Nesne'), h('th', null, 'Nereye'))), h('tbody', null, rows)),
    );
  }

  private renderSummary(): void {
    const r = this.result;
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
        ? summaryLine('ok', `${total} nesne alınacak: ${kindCounts(counts)}.${created ? ` ${created} yeni katman “${this.file.name}” grubunda kurulacak.` : ''}`)
        : summaryLine('warn', 'Alınacak nesne yok; en az bir katman seçin.'),
      ...reportLines(r.report.notes, 'info'),
      ...reportLines(r.report.skipped, 'warn'),
    ];
    if (r.bounds) lines.push(extentLine(this.ctx, r.bounds));
    replaceChildren(this.summaryHost, lines);
  }

  private updateButton(): void {
    this.primary.disabled = this.reading || !this.result || !this.included().length || !this.crs.matches;
  }

  private say(text: string, kind: 'info' | 'error' = 'info'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  private run(): void {
    const r = this.result;
    if (this.primary.disabled || !r) return;
    const { ctx } = this;
    const layers = new Map<string, LayerTarget>();
    for (const l of this.included()) {
      const t = this.target(l);
      layers.set(
        l.name,
        t.existing
          ? { kind: 'existing', id: t.existing }
          : { kind: 'new', name: l.name, style: { color: l.color, lineType: l.lineType, ...(l.lineWeight ? { lineWeight: l.lineWeight } : {}) }, visible: l.visible, locked: l.locked },
      );
    }
    const applied = applyImport(ctx.doc, r.entities, { label: `DXF: ${this.file.name}`, layers, group: this.file.name });
    if (!applied.ok) {
      this.say(applied.error, 'error');
      return;
    }
    zoomToImported(ctx, applied.ids);
    const into = applied.created.length ? `; ${applied.created.length} yeni katman “${this.file.name}” grubunda` : '';
    ctx.log.success(`“${this.file.name}”: ${applied.ids.length} nesne ${layers.size} katmana alındı${into}. Tek adımda geri alınabilir.`);
    if (r.report.skipped.length) ctx.log.warn(`“${this.file.name}” içinde alınmayanlar: ${r.report.skipped.map(reportText).join(' ')}`);
    this.dialog.close();
  }
}
