import type { AppContext } from '../../app/context';
import type { FileKind, PickedFile } from '../../app/fileIO';
import type { CoordColumn } from '../../contracts/generated/CoordColumn';
import type { CoordDelimiter } from '../../contracts/generated/CoordDelimiter';
import type { CoordRead } from '../../contracts/generated/CoordRead';
import type { CoordReadOptions } from '../../contracts/generated/CoordReadOptions';
import type { DecimalMark } from '../../contracts/generated/DecimalMark';
import type { HeaderMode } from '../../contracts/generated/HeaderMode';
import { applyImport, layerNamed, type LayerTarget } from '../../io/apply';
import { formats } from '../../io/client';
import { COLUMN_LABEL, COLUMN_ROLES, COORD_ORDERS, DELIMITER_LABEL, orderColumns, orderOf, POINT_LAYER_STYLE } from '../../io/coords';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { baseName, checkField, CrsQuestion, extentLine, field, fileLine, select, summaryLine } from './common';
import { zoomToImported } from './zoom';

/**
 * Koordinat listesi içe aktar (Netcad NCN, TXT, CSV). The worker reads the
 * file (crates/formats); the window shows what it found (delimiter,
 * decimal mark, header, what each column holds) with the first rows, and
 * every choice re-reads the file. The coordinate system is asked, the
 * project's by default; any other blocks the import (no silent
 * reprojection, CLAUDE.md §5). The points go in as one undo step.
 */
export function openCoordImport(ctx: AppContext, file: PickedFile, kind: FileKind): void {
  new CoordImportDialog(ctx, file, kind);
}

const PREVIEW_ROWS = 12;
const NEW = '__new__';
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

class CoordImportDialog {
  private readonly ctx: AppContext;
  private readonly kind: FileKind;
  private file: PickedFile;
  private delimiter: CoordDelimiter = 'auto';
  private decimal: DecimalMark = 'auto';
  private header: HeaderMode = 'auto';
  /** What each column holds, once the user chose; empty lets the reader suggest. */
  private columns: CoordColumn[] = [];
  private read: CoordRead | null = null;
  private failed: string | null = null;
  private generation = 0;
  private importing = false;
  private closed = false;
  private target = NEW;
  private readonly crs: CrsQuestion;
  private readonly nameInput: HTMLInputElement;
  private readonly fileHost = h('div');
  private readonly optionsHost = h('div', { class: 'io-row' });
  private readonly tableHost = h('div', { class: 'io-table-wrap' });
  private readonly summaryHost = h('div', { class: 'io-summary' });
  private readonly layerHost = h('div', { class: 'io-row' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'İçe aktar');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, file: PickedFile, kind: FileKind) {
    this.ctx = ctx;
    this.file = file;
    this.kind = kind;
    this.crs = new CrsQuestion(ctx, () => this.updateButton());
    this.nameInput = h('input', { class: 'field', value: baseName(file.name), 'aria-label': 'Yeni katmanın adı', spellcheck: 'false', dataset: { key: 'name' } });
    this.nameInput.addEventListener('input', () => this.updateButton());
    this.nameInput.addEventListener('keydown', (e) => e.key === 'Enter' && void this.run());
    const other = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Başka dosya…');
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'Koordinat listesi içe aktar',
      width: 860,
      className: 'dialog--io',
      content: [this.fileHost, this.optionsHost, this.tableHost, this.summaryHost, this.crs.el, this.layerHost],
      footer: [other, this.status, cancel, this.primary],
      onClose: () => {
        this.closed = true;
        // A read of a large file stops with the window.
        if (this.importing) formats().cancel();
      },
    });
    other.addEventListener('click', () => void this.pickAnother());
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => void this.run());
    this.renderLayer();
    void this.refresh();
  }

  private options(entities: boolean): CoordReadOptions {
    return { delimiter: this.delimiter, decimal: this.decimal, header: this.header, columns: this.columns, previewRows: PREVIEW_ROWS, entities };
  }

  /** Reads the file again with the current choices; a late answer to an older read is dropped. */
  private async refresh(): Promise<void> {
    const gen = ++this.generation;
    this.say('Dosya okunuyor…');
    try {
      const r = await formats().readCoords(this.file.bytes, this.options(false));
      if (gen !== this.generation || this.closed) return;
      this.read = r;
      this.failed = null;
      this.say('');
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      this.read = null;
      this.failed = `Dosya okunamadı: ${message(e)}`;
      this.say('');
    }
    this.render();
  }

  private async pickAnother(): Promise<void> {
    const f = await this.ctx.files.pickForImport(this.kind);
    if (!f || this.closed) return;
    this.file = f;
    this.delimiter = 'auto';
    this.decimal = 'auto';
    this.header = 'auto';
    this.columns = [];
    this.nameInput.value = baseName(f.name);
    void this.refresh();
  }

  private render(): void {
    const focus = (document.activeElement as HTMLElement | null)?.dataset?.key;
    this.renderFile();
    this.renderOptions();
    this.renderTable();
    this.renderSummary();
    this.updateButton();
    if (focus) this.dialog.body.querySelector<HTMLElement>(`[data-key="${focus}"]`)?.focus();
  }

  private renderFile(): void {
    const r = this.read;
    const meta = r ? `${r.dataLines} veri satırı, ${r.encoding}${r.header ? ', ilk satır başlık' : ''}` : 'okunuyor…';
    replaceChildren(this.fileHost, fileLine(this.file.name, meta));
  }

  private renderOptions(): void {
    const r = this.read;
    const delimiters = (['auto', 'space', 'tab', 'semicolon', 'comma'] as CoordDelimiter[]).map((d) => ({
      value: d,
      label: d === 'auto' && r ? `Otomatik: ${DELIMITER_LABEL[r.delimiter].toLocaleLowerCase('tr-TR')}` : DELIMITER_LABEL[d],
    }));
    const delimiter = select('Ayırıcı', delimiters, this.delimiter, (d) => {
      this.delimiter = d;
      // Other fields, other columns: let the reader suggest again.
      this.columns = [];
      void this.refresh();
    }, 'delimiter');
    const resolved = r?.decimal ?? 'point';
    const decimal = segmented<DecimalMark>({
      label: 'Ondalık ayırıcı',
      options: [
        { value: 'point', label: 'Nokta' },
        { value: 'comma', label: 'Virgül', disabled: r?.delimiter === 'comma', hint: 'Virgül ayırıcı olduğunda ondalık virgül okunamaz.' },
      ],
      value: resolved,
      onChange: (v) => {
        this.decimal = v;
        void this.refresh();
      },
    });
    const header = checkField('İlk satır', 'Başlık (sütun adları)', !!r?.header, (on) => {
      this.header = on ? 'yes' : 'no';
      void this.refresh();
    }, 'header');
    const count = r?.columns.length ?? 0;
    const order = segmented<string>({
      label: 'Sütun sırası',
      options: COORD_ORDERS.map((o) => ({ value: o.id, label: o.label, disabled: count > 0 && o.columns.length > count })),
      value: (r && orderOf(r.columns)) ?? '',
      onChange: (id) => {
        const o = COORD_ORDERS.find((x) => x.id === id);
        if (!o) return;
        this.columns = orderColumns(o.columns, count);
        void this.refresh();
      },
    });
    replaceChildren(
      this.optionsHost,
      field('Ayırıcı', delimiter),
      field('Ondalık ayırıcı', decimal),
      header,
      field('Sütun sırası', order, 'Y sağa (doğu), X yukarı (kuzey) değerdir. Her sütunu tablonun başlığından da seçebilirsiniz.', 'grow'),
    );
  }

  private renderTable(): void {
    const r = this.read;
    if (!r) {
      replaceChildren(this.tableHost, h('p', { class: 'io-empty' }, this.failed ? 'Önizleme yok.' : 'Dosya okunuyor…'));
      return;
    }
    if (!r.preview.length) {
      replaceChildren(this.tableHost, h('p', { class: 'io-empty' }, 'Dosyada nokta satırı yok.'));
      return;
    }
    const roles = COLUMN_ROLES.map((c) => ({ value: c, label: COLUMN_LABEL[c] }));
    const head = h(
      'tr',
      null,
      h('th', { class: 'io-table__line' }, 'Satır'),
      r.columns.map((role, i) => {
        const pick = select(`${i + 1}. sütun`, roles, role, (v) => {
          const next = [...r.columns];
          next[i] = v;
          this.columns = next;
          void this.refresh();
        }, `col-${i}`);
        return h('th', null, r.header ? h('span', { class: 'io-table__source' }, r.headerFields[i] ?? '') : null, pick);
      }),
      h('th', { class: 'io-table__status' }, 'Durum'),
    );
    const rows = r.preview.map((row) =>
      h(
        'tr',
        { dataset: row.error ? { error: '' } : undefined },
        h('td', { class: 'io-table__line' }, String(row.line)),
        r.columns.map((role, i) => h('td', { class: role === 'skip' ? 'io-table__skip' : null }, row.fields[i] ?? '')),
        h('td', { class: 'io-table__status' }, row.error ? [icon('warning', 14), ' ', row.error] : icon('success', 14)),
      ),
    );
    replaceChildren(this.tableHost, h('table', { class: 'io-table' }, h('thead', null, head), h('tbody', null, rows)));
  }

  private renderSummary(): void {
    const r = this.read;
    if (!r) {
      replaceChildren(this.summaryHost, this.failed ? summaryLine('error', this.failed) : summaryLine('info', 'Dosya okunuyor…'));
      return;
    }
    const lines = [];
    lines.push(r.points ? summaryLine('ok', `${r.points} nokta alınacak.`) : summaryLine('warn', 'Alınacak nokta yok; ayırıcıyı ve sütunları denetleyin.'));
    if (r.errorCount) {
      const shown = r.errors.slice(0, 5);
      lines.push(
        summaryLine(
          'warn',
          `${r.errorCount} satır nokta değil; alınmayacak:`,
          h('ul', { class: 'io-summary__list' }, shown.map((e) => h('li', null, e.message)), r.errorCount > shown.length ? h('li', null, `… ve ${r.errorCount - shown.length} satır daha.`) : null),
        ),
      );
    }
    if (r.duplicateNames) lines.push(summaryLine('info', `${r.duplicateNames} nokta başka bir noktayla aynı adı taşıyor; hepsi alınır.`));
    for (const hint of r.hints) lines.push(summaryLine('warn', hint));
    if (r.bounds) lines.push(extentLine(this.ctx, r.bounds));
    replaceChildren(this.summaryHost, lines);
  }

  private renderLayer(): void {
    const layers = this.ctx.doc.layers;
    const choices = [
      { value: NEW, label: 'Yeni katman' },
      ...layers.leaves().map((l) => ({ value: l.id, label: `${layers.path(l.id)}${layers.isLocked(l.id) ? ' (kilitli)' : !layers.isVisible(l.id) ? ' (gizli)' : ''}`, disabled: layers.isLocked(l.id) })),
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

  /** The layer the points go to. */
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
    this.primary.disabled = this.importing || !this.read || this.read.points === 0 || !this.crs.matches || !t || locked;
    if (locked && !this.importing) this.say(`“${this.ctx.doc.layers.get((t as { id: string }).id)?.name}” katmanı kilitli; kilidini Katmanlar panelinden açın ya da başka bir katman seçin.`, 'error');
  }

  private say(text: string, kind: 'info' | 'error' = 'info'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  private async run(): Promise<void> {
    const r = this.read;
    const target = this.layerTarget();
    if (this.primary.disabled || !r || !target) return;
    this.importing = true;
    this.updateButton();
    this.say(`${r.points} nokta okunuyor…`);
    let result;
    try {
      // Exactly the choices the preview showed.
      const full = await formats().readCoords(this.file.bytes, { delimiter: r.delimiter, decimal: r.decimal, header: r.header ? 'yes' : 'no', columns: r.columns, previewRows: 0, entities: true });
      result = full.result;
      if (!result) throw new Error('okuyucu nesne döndürmedi');
    } catch (e) {
      if (this.closed) return;
      this.importing = false;
      this.say(`Noktalar okunamadı: ${message(e)}`, 'error');
      this.updateButton();
      return;
    }
    if (this.closed) return;
    const { ctx } = this;
    const applied = applyImport(ctx.doc, result.entities, { label: `Koordinat listesi: ${this.file.name}`, layers: new Map([['', target]]) });
    this.importing = false;
    if (!applied.ok) {
      this.say(applied.error, 'error');
      this.updateButton();
      return;
    }
    const layerName = target.kind === 'new' ? target.name : (ctx.doc.layers.get(target.id)?.name ?? '');
    zoomToImported(ctx, applied.ids);
    ctx.log.success(`“${this.file.name}”: ${applied.ids.length} nokta “${layerName}” katmanına alındı${applied.created.length ? ' (yeni katman)' : ''}. Tek adımda geri alınabilir.`);
    if (r.errorCount) ctx.log.warn(`“${this.file.name}”: ${r.errorCount} satır nokta olmadığı için alınmadı (${r.errors.slice(0, 3).map((e) => `satır ${e.line}`).join(', ')}${r.errorCount > 3 ? ' …' : ''}).`);
    this.dialog.close();
  }
}
