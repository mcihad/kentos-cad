import type { AppContext } from '../../app/context';
import type { FileKind } from '../../app/fileIO';
import type { FieldPoint } from '../../contracts/generated/FieldPoint';
import type { FieldWrite } from '../../contracts/generated/FieldWrite';
import type { FieldWriteFormat } from '../../contracts/generated/FieldWriteFormat';
import { formats } from '../../io/client';
import type { Entity } from '../../model/entities';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { field, select, summaryLine } from './common';
import { saveExport } from './save';

/**
 * Cihaza gönder (docs/adr/0169 §4, §6; the desktop's `exchange/field_send.rs`): points written as an instrument's own
 * coordinate file by the shared writer (crates/shared/formats/src/field/write.rs, through the formats worker): Leica
 * GSI-16 and GSI-8, Topcon GTS-7 points, Trimble JobXML, Nikon RAW or CSV. The window shows the file's first lines and
 * every point the format cannot carry, with why; nothing is cut to fit. The points come from the drawing's selection,
 * Aplikasyon's table or Nokta editörü's rows; the drawing is not changed.
 */
export function openFieldSend(ctx: AppContext, points: readonly FieldPoint[], from: string): void {
  new FieldSendDialog(ctx, points, from);
}

/** A drawing's point as the writer takes it: its name (its label, else its Ad), Kod and elevation. */
export function fieldPoint(e: Entity): FieldPoint | null {
  if (e.kind !== 'point') return null;
  const code = e.attrs['Kod'];
  return {
    name: e.label ?? e.attrs['Ad'] ?? '',
    east: e.p.x,
    north: e.p.y,
    ...(e.z != null ? { elevation: e.z } : {}),
    ...(code ? { code } : {}),
  };
}

/** The formats in the list's order: their names, the files' extensions and kinds. */
const FORMATS: readonly { value: FieldWriteFormat; label: string; extension: string; kind: FileKind }[] = [
  { value: 'gsi16', label: 'Leica GSI-16', extension: '.gsi', kind: { description: 'Leica GSI', accept: { 'text/plain': ['.gsi'] } } },
  { value: 'gsi8', label: 'Leica GSI-8', extension: '.gsi', kind: { description: 'Leica GSI', accept: { 'text/plain': ['.gsi'] } } },
  { value: 'gts7', label: 'Topcon GTS-7 (noktalar)', extension: '.xyz', kind: { description: 'Topcon GTS-7 noktaları', accept: { 'text/plain': ['.xyz'] } } },
  { value: 'jobxml', label: 'Trimble JobXML', extension: '.jxl', kind: { description: 'Trimble JobXML', accept: { 'application/xml': ['.jxl'] } } },
  { value: 'nikon', label: 'Nikon RAW', extension: '.raw', kind: { description: 'Nikon RAW', accept: { 'text/plain': ['.raw'] } } },
  { value: 'csv', label: 'CSV (Ad, Y, X, Z, Kod)', extension: '.csv', kind: { description: 'CSV', accept: { 'text/csv': ['.csv'] } } },
];

/** The file's first lines the window shows. */
const PREVIEW_LINES = 14;
/** Points said in the summary; the rest are counted. */
const SHOWN = 5;

/** The last choices, kept for the session. */
const memory: { format: FieldWriteFormat } = { format: 'gsi16' };

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

class FieldSendDialog {
  private readonly ctx: AppContext;
  private readonly points: readonly FieldPoint[];
  private written: FieldWrite | null = null;
  private failed: string | null = null;
  private generation = 0;
  private closed = false;
  private readonly jobInput: HTMLInputElement;
  private readonly optionsHost = h('div', { class: 'io-row' });
  private readonly previewHost = h('div');
  private readonly summaryHost = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet…');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, points: readonly FieldPoint[], from: string) {
    this.ctx = ctx;
    this.points = points;
    const job = ctx.doc.name.value.replace(/\.kcad$/i, '') || 'KentOS';
    this.jobInput = h('input', { class: 'field', value: job, 'aria-label': 'İş adı', spellcheck: 'false', dataset: { key: 'job' } });
    this.jobInput.addEventListener('input', () => void this.refresh());
    const head = h(
      'div',
      { class: 'io-file' },
      icon('fieldSend', 18),
      h('div', { class: 'io-file__text' }, h('span', { class: 'io-file__name' }, `${points.length.toLocaleString('tr-TR')} nokta`), h('span', { class: 'io-file__meta' }, from)),
    );
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'Cihaza gönder',
      width: 760,
      className: 'dialog--io',
      content: [head, this.optionsHost, this.previewHost, this.summaryHost],
      footer: [this.status, cancel, this.primary],
      onClose: () => {
        this.closed = true;
      },
    });
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => void this.save());
    this.renderOptions();
    void this.refresh();
  }

  private get format() {
    return FORMATS.find((f) => f.value === memory.format) ?? FORMATS[0]!;
  }

  private renderOptions(): void {
    const pick = select('Biçim', FORMATS.map((f) => ({ value: f.value, label: f.label })), memory.format, (v) => {
      memory.format = v;
      void this.refresh();
    }, 'format');
    replaceChildren(
      this.optionsHost,
      field('Biçim', pick, null, 'wide'),
      field('İş adı', this.jobInput, 'Trimble JobXML ve Nikon RAW dosyasına yazılır.', 'grow'),
    );
  }

  /** Writes the file again with the current choices; a late answer to an older write is dropped. */
  private async refresh(): Promise<void> {
    const gen = ++this.generation;
    try {
      const out = await formats().writeField(this.points, { format: memory.format, job: this.jobInput.value, stamp: stamp() });
      if (gen !== this.generation || this.closed) return;
      this.written = out;
      this.failed = null;
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      this.written = null;
      this.failed = `Dosya yazılamadı: ${message(e)}`;
    }
    this.render();
  }

  private render(): void {
    const w = this.written;
    if (!w || w.error || w.written === 0) {
      replaceChildren(this.previewHost, h('p', { class: 'io-empty' }, 'Önizleme yok.'));
    } else {
      const lines = w.text.split('\r\n').filter((l, i, all) => l || i < all.length - 1);
      const shown = lines.slice(0, PREVIEW_LINES).join('\n');
      const more = lines.length > PREVIEW_LINES ? `\n… ve ${(lines.length - PREVIEW_LINES).toLocaleString('tr-TR')} satır daha` : '';
      replaceChildren(this.previewHost, h('pre', { class: 'send-preview', 'aria-label': 'Dosyanın ilk satırları' }, shown + more));
    }
    const lines: HTMLElement[] = [];
    if (this.failed) lines.push(summaryLine('error', this.failed));
    else if (w?.error) lines.push(summaryLine('error', w.error));
    else if (w) {
      lines.push(
        w.written
          ? summaryLine('ok', `${w.written.toLocaleString('tr-TR')} nokta ${this.format.label} olarak yazılacak; değerler milimetreye yuvarlanır.`)
          : summaryLine('warn', 'Yazılacak nokta yok.'),
      );
      if (w.skipped.length) {
        const shown = w.skipped.slice(0, SHOWN);
        lines.push(
          summaryLine(
            'warn',
            `${w.skipped.length.toLocaleString('tr-TR')} nokta bu biçimde taşınamıyor; yazılmayacak:`,
            h('ul', { class: 'io-summary__list' }, shown.map((s) => h('li', null, s.problem)), w.skipped.length > shown.length ? h('li', null, `… ve ${w.skipped.length - shown.length} nokta daha.`) : null),
          ),
        );
      }
    }
    replaceChildren(this.summaryHost, lines);
    this.primary.disabled = !w || !!w.error || w.written === 0;
  }

  private async save(): Promise<void> {
    const w = this.written;
    if (this.primary.disabled || !w) return;
    const { ctx } = this;
    const job = this.jobInput.value.trim() || 'KentOS';
    const name = await saveExport(ctx, new TextEncoder().encode(w.text), `${job}${this.format.extension}`, this.format.kind);
    if (!name) return;
    const left = w.skipped.length ? ` ${w.skipped.length} nokta biçimde taşınamadığı için yazılmadı (nedenleri pencerede).` : '';
    ctx.log.success(`“${name}”: ${w.written} nokta ${this.format.label} olarak yazıldı.${left}`);
    this.dialog.close();
  }
}

/** The time stamp JobXML carries: local time, to the second. */
function stamp(): string {
  const d = new Date();
  const two = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())}T${two(d.getHours())}:${two(d.getMinutes())}:${two(d.getSeconds())}`;
}

