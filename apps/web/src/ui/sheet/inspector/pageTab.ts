import type { AppContext } from '../../../app/context';
import type { Sheet } from '../../../contracts/generated/sheet/Sheet';
import type { DisposableStore } from '../../../core/disposable';
import { fixed } from '../../../core/displayNumber';
import { copyItems, withChildren } from '../../../product/sheet/ops';
import type { SheetView } from '../../../product/sheet/view';
import { h, type Child } from '../../dom';
import { icon } from '../../icons';
import { tooltip } from '../../widgets/tooltip';
import type { SheetHost } from '../host';
import { numberField } from '../widgets/fields';
import { choice } from '../widgets/form';
import { flag, type SectionCtx } from './parts';
import { missingMark } from '../../../product/sheet/marks';
import { variantsSection } from './variantsSection';

/**
 * Denetçi's Sayfa tab (docs/sheet/design.md §3.2, §3.3, §5, §9, §12): the
 * paper (changed in Sayfa ayarları, where the items follow their
 * constraints), the master page drawn under the sheet (chosen, or let go
 * with its items copied onto the sheet), the snapping grid, the guides, the
 * export's defaults, the sheet's values and the template it was made from.
 */

type Section = (id: string, title: string, body: Child[]) => HTMLElement;

/** A button of the tab that runs a command (its tooltip says why when it cannot). */
function commandButton(ctx: AppContext, d: DisposableStore, id: string, label: string, iconName: string): HTMLButtonElement {
  const b = h('button', { class: 'btn btn--small', type: 'button', disabled: !ctx.commands.isEnabled(id) }, icon(iconName, 14), label);
  b.addEventListener('click', () => ctx.commands.execute(id));
  d.add(tooltip(b, () => ({ title: ctx.commands.get(id)?.title ?? label, description: ctx.commands.get(id)?.description, note: ctx.commands.get(id)?.whyDisabled?.() ?? undefined })));
  return b;
}

export function pageTab(ctx: AppContext, host: SheetHost, view: SheetView, d: DisposableStore, section: Section): Child[] {
  const sheet: Sheet = view.source;
  const book = host.book()?.book;
  const readOnly = host.whyReadOnly();
  const c: SectionCtx = { ctx, host, items: [], sheet, readOnly, d };
  const p = view.paper;
  const orient = p.orientation === 'landscape' ? 'yatay' : 'dikey';
  const owner = { kind: 'sheet' as const, id: sheet.id };
  const masters = book?.masters ?? [];
  const master = sheet.master ? masters.find((m) => m.id === sheet.master) : undefined;
  const detach = h('button', { class: 'btn btn--small', type: 'button', disabled: !master || readOnly !== null }, 'Ana sayfadan ayır');
  detach.addEventListener('click', () => master && host.apply([{ op: 'detachMaster', sheet: sheet.id, itemIds: master.items.map(() => host.newId()) }], 'Ana sayfadan ayır'));
  d.add(tooltip(detach, () => ({ title: 'Ana sayfadan ayır', description: 'Ana sayfanın öğeleri bu paftaya kopyalanır (kendi öğelerinin altına); pafta ana sayfadan ayrılır.' })));
  // The chosen items as a master page of their own (the sheet draws it under its items, as before).
  const chosen = [...host.state.selection.value];
  const makeMaster = h('button', { class: 'btn btn--small', type: 'button', disabled: readOnly !== null || !chosen.length || !!master }, 'Seçili öğelerden ana sayfa yap');
  makeMaster.addEventListener('click', () => {
    const id = host.newId();
    const items = copyItems(sheet, chosen, () => host.newId(), 0);
    const name = `Ana sayfa ${masters.length + 1}`;
    if (host.apply([{ op: 'addMaster', master: { id, name, page: sheet.page, items, guides: [], variants: [] } }, { op: 'removeItems', ids: withChildren(sheet, chosen) }, { op: 'setSheetMaster', sheet: sheet.id, master: id }], 'Ana sayfa yap')) host.state.clearSelection();
  });
  d.add(tooltip(makeMaster, () => ({ title: 'Seçili öğelerden ana sayfa yap', description: 'Seçili öğeler (antet, çerçeve, logo) bir ana sayfaya taşınır; bu pafta onu kullanır, başka paftalar da seçebilir.', note: master ? 'Bu paftanın zaten ana sayfası var: önce ayırın ya da Yok seçin.' : chosen.length ? (readOnly ?? undefined) : 'Önce Öğe sekmesinde ya da paftada öğeleri seçin.' })));
  const clearGuides = h('button', { class: 'btn btn--small', type: 'button', disabled: !sheet.guides.length || readOnly !== null }, 'Kılavuzları kaldır');
  clearGuides.addEventListener('click', () => host.apply(sheet.guides.map((g) => ({ op: 'removeGuide' as const, owner, id: g.id })), 'Kılavuzları kaldır'));
  const grid = sheet.snapGrid;
  const missing = [...sheet.variables, ...(book?.variables ?? [])].filter((v) => v.value === null).length;
  const exp = sheet.export;
  return [
    h('div', { class: 'sheet-insp__summary' }, icon('sheetLayout', 20), h('div', null, h('span', { class: 'sheet-insp__kind' }, view.name), h('span', { class: 'sheet-insp__sub num' }, `${p.name} ${orient} · ${fixed(p.widthMm, 0)} × ${fixed(p.heightMm, 0)} mm`))),
    section('paper', 'Kâğıt', [
      h('div', { class: 'sheet-insp__row' }, h('span', { class: 'sheet-insp__label' }, 'Boy ve yön'), h('span', null, `${p.name}, ${orient}`)),
      h('div', { class: 'sheet-insp__row' }, h('span', { class: 'sheet-insp__label' }, 'Kenar boşlukları'), h('span', { class: 'num' }, `${fixed(p.margins.left, 0)} · ${fixed(p.margins.top, 0)} · ${fixed(p.margins.right, 0)} · ${fixed(p.margins.bottom, 0)} mm`)),
      h('div', { class: 'sheet-insp__actions' }, commandButton(ctx, d, 'sheet.pageSetup', 'Sayfa ayarları…', 'sheetPage')),
      h('p', { class: 'sheet-insp__hint' }, 'Kâğıt değişince öğeler kısıtlarına göre yeniden yerleşir: bir şablon her kâğıtta kullanılır.'),
    ]),
    section('variants', 'Yerleşim düzenleri', variantsSection(host, sheet, d, p.name)),
    section('master', 'Ana sayfa', [
      choice({ label: 'Ana sayfa', key: 'master', value: sheet.master ?? '', readOnly, options: [{ value: '', label: 'Yok' }, ...masters.map((m) => ({ value: m.id, label: m.name, detail: `${m.items.length} öğe` }))], onChange: (id) => host.apply([{ op: 'setSheetMaster', sheet: sheet.id, ...(id ? { master: id } : {}) }], 'Ana sayfa') }, d),
      h('div', { class: 'sheet-insp__actions' }, makeMaster, detach),
      h('p', { class: 'sheet-insp__hint' }, 'Antet, çerçeve ve logo ana sayfada bir kez tanımlanır; ona bağlı her paftanın altında, kilitli çizilir.'),
    ]),
    section('grid', 'Izgara ve kılavuzlar', [
      numberField({ label: 'Izgara aralığı', key: 'grid.spacing', value: grid.spacing / 1000, unit: 'mm', decimals: 1, min: 0.5, max: 100, readOnly, onCommit: (v) => host.apply([{ op: 'setSnapGrid', sheet: sheet.id, grid: { ...grid, spacing: Math.round(v * 1000) } }], 'Izgara') }, d),
      flag(c, 'Izgara görünsün', grid.visible, (visible) => host.apply([{ op: 'setSnapGrid', sheet: sheet.id, grid: { ...grid, visible } }], 'Izgara')),
      flag(c, 'Izgaraya yapış', grid.enabled, (enabled) => host.apply([{ op: 'setSnapGrid', sheet: sheet.id, grid: { ...grid, enabled } }], 'Izgara')),
      h('div', { class: 'sheet-insp__row' }, h('span', { class: 'sheet-insp__label' }, `${sheet.guides.length} kılavuz`), clearGuides),
      h('p', { class: 'sheet-insp__hint' }, 'Kılavuz cetvelden kâğıda sürüklenir; cetvele geri sürüklenince kalkar.'),
    ]),
    section('export', 'Dışa aktarma', [
      choice({ label: 'Biçim', key: 'export.format', value: exp.format, readOnly, options: [{ value: 'svg' as const, label: 'SVG' }, { value: 'png' as const, label: 'PNG' }], onChange: (format) => host.apply([{ op: 'setExport', sheet: sheet.id, export: { ...exp, format } }], 'Dışa aktarma') }, d),
      numberField({ label: 'Çözünürlük', key: 'export.dpi', value: exp.dpi, unit: 'dpi', decimals: 0, min: 72, max: 1200, readOnly, onCommit: (dpi) => host.apply([{ op: 'setExport', sheet: sheet.id, export: { ...exp, dpi: Math.round(dpi) } }], 'Dışa aktarma') }, d),
    ]),
    section('variables', 'Değişkenler', [
      h('p', { class: missing ? 'sheet-insp__hint sheet-insp__hint--warn' : 'sheet-insp__hint' }, missing ? `${missing} değişkenin değeri yok: kâğıda ${missingMark('ad')} yazılıyor.` : `${sheet.variables.length} pafta, ${book?.variables.length ?? 0} proje değişkeni; hepsinin değeri var.`),
      h('div', { class: 'sheet-insp__actions' }, commandButton(ctx, d, 'sheet.variables', 'Değişkenler…', 'sheetVariables')),
    ]),
    section('template', 'Şablon', [
      h('div', { class: 'sheet-insp__row' }, h('span', { class: 'sheet-insp__label' }, 'Kaynak'), h('span', null, view.template ? view.template.name : 'Şablonsuz', view.template?.newer ? h('span', { class: 'tbadge tbadge--newer' }, 'Yeni sürüm var') : null)),
      view.template?.newer ? h('p', { class: 'sheet-insp__hint' }, 'Şablonun daha yeni bir sürümü kaydedildi. Pafta kendiliğinden değişmez; güncellemeyi uygulamak sonraki aşamada gelecek.') : null,
      h('div', { class: 'sheet-insp__actions' }, commandButton(ctx, d, 'sheet.saveTemplate', 'Şablon olarak kaydet', 'sheetSaveTemplate')),
    ]),
  ];
}
