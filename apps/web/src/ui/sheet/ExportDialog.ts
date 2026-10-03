import type { AppContext } from '../../app/context';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import { DisposableStore } from '../../core/disposable';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { note, segmented, toggleSwitch } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { pdfFields, type PdfAction, type PdfExporter, type PdfState } from './exportPdf';
import type { SheetHost } from './host';
import { choice, field } from './widgets/form';

/**
 * Dışa aktar (docs/sheet/design.md §9, §9a): PDF (the core's writer: the
 * sheets chosen, a page each, maps as vectors where they can be, GeoPDF and
 * layers; saved, printed or opened in a tab: exportPdf.ts), SVG (the
 * engine's writer, each map frame a PNG at the dpi), PNG at a dpi, or the
 * `.kpafta` file of the whole book. The window shows the preflight first
 * (of every sheet going); with an error the export waits for an explicit
 * “Yine de aktar”. A PNG too large for a browser's canvas says so before it
 * is tried.
 */

export type ExportFormat = 'pdf' | 'svg' | 'png' | 'kpafta';

export interface Exporters extends PdfExporter {
  svg(dpi: number): Promise<string | null>;
  png(dpi: number): Promise<string | null>;
  kpafta(): Promise<string | null>;
}

export const DPI: readonly number[] = [96, 150, 200, 300, 400, 600];
const SEVERITY_ICON: Record<Finding['severity'], string> = { error: 'error', warning: 'warning', info: 'info' };

export interface ExportOpen {
  readonly format: ExportFormat;
  /** Yazdır: the PDF goes to the browser's print window (its button is the window's first). */
  readonly print?: boolean;
}

export function openExportDialog(ctx: AppContext, host: SheetHost, sheetId: string, o: ExportOpen, run: Exporters): void {
  const book = host.book()?.book;
  const sheet = book?.sheets.find((s) => s.id === sheetId);
  if (!book || !sheet) return;
  const crs = ctx.doc.crs.value;
  const s = { format: o.format, dpi: DPI.includes(sheet.export.dpi) ? sheet.export.dpi : 300, anyway: false, busy: false };
  const pdf: PdfState = { scope: 'this', chosen: new Set([sheetId]), geo: run.pdfGeoReady(), layers: true };
  const d = new DisposableStore();
  const body = h('div', { class: 'sheet-form sheet-export' });
  const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Aktar');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const printBtn = h('button', { class: 'btn', type: 'button' }, icon('print', 15), 'Yazdır');
  const tabBtn = h('button', { class: 'btn', type: 'button' }, 'Yeni sekmede aç');
  const pixels = () => ({ width: Math.round((sheet.page.size.width / 25_400) * s.dpi), height: Math.round((sheet.page.size.height / 25_400) * s.dpi) });
  const sheetsGoing = () => (s.format !== 'pdf' || pdf.scope === 'this' ? [sheetId] : pdf.scope === 'all' ? book.sheets.map((x) => x.id) : book.sheets.filter((x) => pdf.chosen.has(x.id)).map((x) => x.id));
  const findingsOf = () => {
    if (s.format === 'kpafta') return [];
    const ids = sheetsGoing();
    return ids.flatMap((id) => host.preflight(id, s.dpi).map((f) => (ids.length > 1 ? { ...f, message: `${book.sheets.find((x) => x.id === id)?.name}: ${f.message}` } : f)));
  };
  const render = () => {
    d.dispose();
    const findings = findingsOf();
    const errors = findings.filter((f) => f.severity === 'error');
    const warnings = findings.filter((f) => f.severity === 'warning');
    const px = pixels();
    const huge = s.format === 'png' && (px.width > 16_384 || px.height > 16_384 || px.width * px.height > 120_000_000);
    const none = s.format === 'pdf' && !sheetsGoing().length;
    const blocked = s.busy || huge || none || (errors.length > 0 && !s.anyway);
    for (const b of [ok, printBtn, tabBtn]) b.disabled = blocked;
    ok.textContent = s.busy ? 'Hazırlanıyor…' : s.format === 'pdf' ? 'Kaydet' : 'Aktar';
    printBtn.hidden = tabBtn.hidden = s.format !== 'pdf';
    // One primary: Yazdır when the window was opened to print, else Kaydet / Aktar.
    const printing = !!o.print && s.format === 'pdf';
    ok.className = printing ? 'btn' : 'btn btn--primary';
    printBtn.className = printing ? 'btn btn--primary' : 'btn';
    replaceChildren(
      body,
      field(
        'Biçim',
        segmented<ExportFormat>({
          label: 'Biçim',
          value: s.format,
          options: [
            { value: 'pdf', label: 'PDF' },
            { value: 'svg', label: 'SVG' },
            { value: 'png', label: 'PNG' },
            { value: 'kpafta', label: '.kpafta' },
          ],
          onChange: (v) => ((s.format = v), render()),
        }),
        s.format === 'pdf'
          ? 'Baskı ve paylaşım için: yazılar seçilebilir ve aranabilir, çizgiler vektör; konumlu haritalar GeoPDF olur.'
          : s.format === 'svg'
            ? 'Çizgiler ve yazılar vektör; harita içleri seçilen çözünürlükte PNG olarak gömülür.'
            : s.format === 'png'
              ? 'Kâğıdın boyunda bir resim; çizgi kalınlıkları gerçek değerleriyle.'
              : 'Bütün paftalar ve resimleri tek dosyada: başka bir projeye ya da cihaza taşımak için.',
      ),
      findings.length
        ? h(
            'div',
            { class: 'sheet-export__findings' },
            h('h3', { class: 'sheet-form__head' }, `Ön denetim: ${errors.length} hata, ${warnings.length} uyarı`),
            h(
              'ul',
              { class: 'sheet-findings sheet-findings--short' },
              [...errors, ...warnings].slice(0, 6).map((f) => h('li', { class: `sheet-finding sheet-finding--${f.severity}` }, icon(SEVERITY_ICON[f.severity], 14), h('span', null, f.message))),
            ),
            errors.length + warnings.length > 6 ? h('p', { class: 'sheet-insp__hint' }, 'Hepsi denetçinin Ön denetim sekmesinde, çözümleriyle.') : null,
          )
        : s.format === 'kpafta'
          ? null
          : note('info', `Ön denetim temiz: aktarılacak ${sheetsGoing().length > 1 ? 'paftalarda' : 'paftada'} hata ya da uyarı yok.`),
      errors.length
        ? h(
            'div',
            { class: 'sheet-insp__row' },
            h('span', { class: 'sheet-insp__label' }, 'Hatalar varken yine de aktar'),
            toggleSwitch({ label: 'Hatalar varken yine de aktar', checked: s.anyway, onChange: (v) => ((s.anyway = v), render()) }),
          )
        : null,
      ...(s.format === 'pdf'
        ? pdfFields(
            {
              book,
              sheetId,
              state: pdf,
              crsName: crs?.name ?? null,
              geoReady: run.pdfGeoReady(),
              ways: () => run.pdfWays({ sheets: sheetsGoing(), dpi: s.dpi }),
              notes: () => run.pdfNotes({ sheets: sheetsGoing(), dpi: s.dpi }),
              name: () => run.pdfName({ sheets: sheetsGoing(), dpi: s.dpi, geo: pdf.geo, layers: pdf.layers }),
              dpi: s.dpi,
              rerender: render,
            },
            d,
          )
        : []),
      s.format === 'kpafta'
        ? null
        : choice({ label: s.format === 'pdf' ? 'Yedek resimlerin çözünürlüğü' : 'Çözünürlük', key: 'dpi', value: s.dpi, options: DPI.map((x) => ({ value: x, label: `${x} dpi`, detail: x === 300 ? 'baskı' : x === 96 ? 'ekran' : undefined })), onChange: (v) => ((s.dpi = v), render()) }, d),
      s.format === 'png' ? h('p', { class: 'sheet-insp__hint num' }, `${px.width} × ${px.height} piksel`) : null,
      huge ? note('warn', 'Bu boyutta bir PNG tarayıcının çizebileceğinden büyük. Daha düşük bir dpi seçin ya da SVG olarak aktarın.') : null,
    );
  };
  render();
  const dialog = new Dialog({ title: o.print ? `Yazdır: ${sheet.name}` : `Dışa aktar: ${sheet.name}`, width: 560, className: 'sheet-dialog sheet-export-dialog', content: [body], footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, tabBtn, printBtn, ok], onClose: () => d.dispose() });
  cancel.addEventListener('click', () => dialog.close());
  const go = (action: PdfAction | null) => {
    // A tab is opened at the click itself: a browser lets a page open one only while it handles a click.
    const tab = action === 'open' ? window.open('', '_blank') : null;
    s.busy = true;
    render();
    const job =
      s.format === 'pdf'
        ? run.pdf({ sheets: sheetsGoing(), dpi: s.dpi, geo: pdf.geo, layers: pdf.layers }, action ?? 'save', tab)
        : s.format === 'svg'
          ? run.svg(s.dpi)
          : s.format === 'png'
            ? run.png(s.dpi)
            : run.kpafta();
    void job.then((name) => {
      s.busy = false;
      if (name) {
        if (action !== 'print' && action !== 'open') ctx.log.success(`“${name}” kaydedildi.`);
        dialog.close();
      } else {
        tab?.close();
        render();
      }
    });
  };
  ok.addEventListener('click', () => go(s.format === 'pdf' ? 'save' : null));
  printBtn.addEventListener('click', () => go('print'));
  tabBtn.addEventListener('click', () => go('open'));
}
