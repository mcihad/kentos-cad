import type { AppContext } from '../../app/context';
import type { FileKind } from '../../app/fileIO';
import type { CoordColumn } from '../../contracts/generated/CoordColumn';
import type { TextEncoding } from '../../contracts/generated/TextEncoding';
import { formats } from '../../io/client';
import { COORD_FORMATS, COORD_ORDERS, coordPoints, type CoordFormat } from '../../io/coords';
import { h, replaceChildren } from '../dom';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { checkField, exportName, field, reportText, select, summaryLine } from './common';
import { saveExport } from './save';
import { SCOPE_LABEL, scopeEntities, type ExportScope } from './scope';

/**
 * Koordinat listesi dışa aktar: the point objects of the selection, of the
 * visible layers or of the whole drawing, one per line (name, Y, X, Z) as
 * Netcad NCN, TXT or CSV. Values are written as the shortest decimal that
 * reads back to the same float64: nothing is rounded (CLAUDE.md §23).
 *
 * Opened from Noktalar (docs/adr/0153 §5) it offers the table's rows first,
 * in the table's order: its selected rows, or every row it shows (“Seçili
 * satırlar”, “Tablodaki”), in place of the drawing's selection.
 */
export function openCoordExport(ctx: AppContext, opts: { table?: TableRows } = {}): void {
  new CoordExportDialog(ctx, opts.table ?? null);
}

/** Noktalar's target rows: their ids in the table's order; `selected`: they are its selected rows. */
export interface TableRows {
  ids: readonly number[];
  selected: boolean;
}

/** What the window writes: the table's rows, or a scope of the drawing. */
type Scope = ExportScope | 'table';

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

class CoordExportDialog {
  private readonly ctx: AppContext;
  private readonly table: TableRows | null;
  private scope: Scope;
  private format: CoordFormat = COORD_FORMATS[0];
  private order: readonly CoordColumn[] = COORD_ORDERS[0].columns;
  private header = COORD_FORMATS[0].header;
  private encoding: TextEncoding = 'utf8';
  private writing = false;
  private readonly form = h('div', { class: 'io-row' });
  private readonly summary = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Dışa aktar…');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, table: TableRows | null) {
    this.ctx = ctx;
    this.table = table;
    this.scope = table ? 'table' : this.points('selection').length ? 'selection' : 'visible';
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'Koordinat listesi dışa aktar',
      width: 720,
      className: 'dialog--io',
      content: [this.form, this.summary],
      footer: [this.status, cancel, this.primary],
    });
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => void this.run());
    this.render();
  }

  private points(scope: Scope) {
    if (scope !== 'table') return coordPoints(scopeEntities(this.ctx, scope));
    const { doc } = this.ctx;
    return coordPoints((this.table?.ids ?? []).flatMap((id) => doc.get(id) ?? []));
  }

  private label(scope: Scope): string {
    if (scope !== 'table') return SCOPE_LABEL[scope];
    return this.table?.selected ? 'Seçili satırlar' : 'Tablodaki';
  }

  private render(): void {
    const focus = (document.activeElement as HTMLElement | null)?.dataset?.key;
    const scopes: Scope[] = this.table ? ['table', 'visible', 'all'] : ['selection', 'visible', 'all'];
    const counts = new Map(scopes.map((s) => [s, this.points(s).length]));
    const scope = segmented<Scope>({
      label: 'Yazılacak noktalar',
      options: scopes.map((s) => ({ value: s, label: `${this.label(s)} (${counts.get(s) ?? 0})`, disabled: !counts.get(s) })),
      value: this.scope,
      onChange: (s) => ((this.scope = s), this.render()),
    });
    const format = select('Biçim', COORD_FORMATS.map((f) => ({ value: f.id, label: f.label })), this.format.id, (id) => {
      this.format = COORD_FORMATS.find((f) => f.id === id) ?? COORD_FORMATS[0];
      this.header = this.format.header;
      this.render();
    }, 'format');
    const order = segmented<string>({
      label: 'Sütun sırası',
      options: COORD_ORDERS.map((o) => ({ value: o.id, label: o.label })),
      value: COORD_ORDERS.find((o) => o.columns === this.order)?.id ?? COORD_ORDERS[0].id,
      onChange: (id) => ((this.order = COORD_ORDERS.find((o) => o.id === id)?.columns ?? this.order), this.render()),
    });
    const header = checkField('İlk satır', 'Başlık (Ad, Y, X, Z)', this.header, (on) => ((this.header = on), this.render()), 'header');
    const encoding = segmented<TextEncoding>({
      label: 'Karakter kodlaması',
      options: [
        { value: 'utf8', label: 'UTF-8' },
        { value: 'windows1254', label: 'Windows-1254 (Türkçe)', hint: 'Eski Windows programları için; Türkçe dışındaki harfler “?” olur.' },
      ],
      value: this.encoding,
      onChange: (v) => ((this.encoding = v), this.render()),
    });
    replaceChildren(
      this.form,
      field('Yazılacak noktalar', scope, null, 'grow'),
      field('Biçim', format),
      field('Sütun sırası', order, 'Y sağa (doğu), X yukarı (kuzey).'),
      header,
      field('Karakter kodlaması', encoding),
    );
    const pts = this.points(this.scope);
    const noZ = pts.filter((p) => p.z === undefined).length;
    const unnamed = pts.filter((p) => !p.name).length;
    replaceChildren(
      this.summary,
      pts.length ? summaryLine('ok', `${pts.length} nokta yazılacak; değerler yuvarlanmadan, tam yazılır.`) : summaryLine('warn', 'Bu kapsamda nokta yok. Başka bir kapsam seçin ya da noktaları seçip yeniden açın.'),
      noZ ? summaryLine('info', `${noZ} noktanın kotu yok; Z alanı boş kalır.`) : null,
      unnamed ? summaryLine('info', `${unnamed} noktanın adı yok; ad alanı boş kalır.`) : null,
    );
    this.primary.disabled = this.writing || !pts.length;
    if (focus) this.dialog.body.querySelector<HTMLElement>(`[data-key="${focus}"]`)?.focus();
  }

  private async run(): Promise<void> {
    if (this.primary.disabled) return;
    const { ctx } = this;
    const pts = this.points(this.scope);
    this.writing = true;
    this.primary.disabled = true;
    this.status.textContent = 'Yazılıyor…';
    try {
      const out = await formats().writeCoords({ points: pts, delimiter: this.format.delimiter, columns: [...this.order], header: this.header, encoding: this.encoding });
      const kind: FileKind = { description: 'Koordinat listesi', accept: { 'text/plain': [this.format.extension] } };
      const name = await saveExport(ctx, out.bytes, exportName(ctx, this.format.extension), kind);
      if (!name) {
        this.status.textContent = '';
        return;
      }
      ctx.log.success(`“${name}” yazıldı: ${out.report.counts.point ?? 0} nokta.`);
      for (const item of out.report.notes) ctx.log.info(reportText(item));
      this.dialog.close();
    } catch (e) {
      this.status.textContent = `Yazılamadı: ${message(e)}`;
      this.status.dataset.kind = 'error';
    } finally {
      this.writing = false;
      this.primary.disabled = !pts.length;
    }
  }
}
