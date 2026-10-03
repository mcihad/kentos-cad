import type { AppContext } from '../../app/context';
import type { DisposableStore } from '../../core/disposable';
import { fixed } from '../../core/displayNumber';
import { watchAll } from '../../core/signal';
import { boundsOf } from '../../render/sheet/paperPainter';
import { h } from '../dom';
import { icon } from '../icons';
import { tooltip } from '../widgets/tooltip';
import type { SheetHost } from './host';
import type { SheetStage } from './SheetStage';

/**
 * The status bar's cells while a sheet is in front (docs/sheet/design.md
 * §11): where the pointer is on the paper (Sol, Üst in mm: X and Y are ground
 * coordinates in KentOS), the chosen items' size, the zoom, which sheet of
 * how many, and the preflight's findings. They stand where the drawing's
 * coordinate, selection, aid and scale cells were (sheet.css hides those).
 */
export function sheetStatusCells(ctx: AppContext, host: SheetHost, stage: SheetStage, d: DisposableStore): HTMLElement {
  const { state } = host;
  const left = h('span', { class: 'sheet-status__value num' }, '—');
  const top = h('span', { class: 'sheet-status__value num' }, '—');
  const cursor = h(
    'div',
    { class: 'sheet-status__cell sheet-status__cursor' },
    h('span', { class: 'sheet-status__axis' }, 'Sol'),
    left,
    h('span', { class: 'sheet-status__axis' }, 'Üst'),
    top,
    h('span', { class: 'sheet-status__unit' }, 'mm'),
  );
  const sel = h('div', { class: 'sheet-status__cell sheet-status__sel num', hidden: true });
  const zoom = h('div', { class: 'sheet-status__cell num' });
  const page = h('div', { class: 'sheet-status__cell num' });
  const pfText = h('span');
  const preflight = h('button', { class: 'sheet-status__cell sheet-status__btn sheet-status__preflight', type: 'button' }, icon('sheetPreflight', 14), pfText);
  preflight.addEventListener('click', () => ctx.commands.execute('sheet.preflight'));
  // The template library's sync (design §13: “durum çubuğu ve galeri bunu söyler”): shown while it is not in step.
  const library = h('button', { class: 'sheet-status__cell sheet-status__btn sheet-status__library', type: 'button', hidden: true });
  library.addEventListener('click', () => ctx.commands.execute('sheet.fromTemplate'));
  d.add(
    host.cloudLine.subscribe((line) => {
      library.hidden = line.state === 'none' || line.state === 'signedOut' || line.state === 'synced';
      const words = { offline: 'Şablonlar: çevrimdışı', syncing: 'Şablonlar eşitleniyor…', failed: 'Şablonlar eşitlenemedi' } as Record<string, string>;
      library.replaceChildren(icon(line.state === 'syncing' ? 'cloud' : 'warning', 14), h('span', null, words[line.state] ?? ''));
      library.dataset.level = line.state === 'syncing' ? 'info' : 'warning';
    }, true),
  );

  d.add(
    stage.cursor.subscribe((p) => {
      left.textContent = p ? fixed(p.x, 1) : '—';
      top.textContent = p ? fixed(p.y, 1) : '—';
    }, true),
  );
  d.add(stage.zoom.subscribe((z) => (zoom.textContent = `%${z}`), true));
  const sync = () => {
    const chosen = state.chosen;
    const b = boundsOf(chosen);
    sel.hidden = !b;
    sel.textContent = b ? `${chosen.length > 1 ? `${chosen.length} öğe · ` : 'Seçim '}${fixed(b.width, 1)} × ${fixed(b.height, 1)} mm` : '';
    const sheets = state.book.value.sheets;
    const at = sheets.findIndex((s) => s.id === state.open.value);
    page.textContent = at >= 0 ? `Sayfa ${at + 1}/${sheets.length}` : '';
    const f = host.findings.value;
    const errors = f.filter((x) => x.severity === 'error').length;
    const warnings = f.filter((x) => x.severity === 'warning').length;
    pfText.textContent = errors ? `${errors} hata` : warnings ? `${warnings} uyarı` : 'Ön denetim temiz';
    preflight.dataset.level = errors ? 'error' : warnings ? 'warning' : 'ok';
    preflight.replaceChildren(icon(errors ? 'error' : warnings ? 'warning' : 'success', 14), pfText);
  };
  d.add(watchAll([state.selection, state.book, state.open, state.engine, host.findings], sync));
  sync();
  d.add(tooltip(cursor, () => ({ title: 'İmleç kâğıtta', description: 'Kâğıdın sol üst köşesinden milimetre: Sol sağa, Üst aşağı doğru.' }), 'top'));
  d.add(tooltip(sel, () => ({ title: 'Seçimin boyu', description: 'Seçili öğelerin çerçevelerini kapsayan kutunun genişliği ve yüksekliği.' }), 'top'));
  d.add(tooltip(zoom, () => ({ title: 'Yakınlaştırma', description: 'Gerçek boya göre. Ctrl+0 sayfayı sığdırır, Ctrl+1 gerçek boyut.' }), 'top'));
  d.add(tooltip(page, () => ({ title: 'Pafta', description: 'Öndeki paftanın sırası ve paftaların sayısı.' }), 'top'));
  d.add(
    tooltip(
      preflight,
      () => {
        const f = host.findings.value;
        return { title: 'Ön denetim', description: f.length ? `${f.length} bulgu: denetçinin Ön denetim sekmesinde, çözümleriyle.` : 'Dışa aktarmadan önce: sayfa dışında kalan, örtülen, bağı kopuk öğeler, sığmayan yazılar, düşük çözünürlük …', note: state.whyNoEngine() ?? undefined };
      },
      'top',
    ),
  );
  d.add(tooltip(library, () => ({ title: 'Pafta şablonları', description: host.cloudLine.value.text, note: 'Tıklayınca şablon galerisi açılır.' }), 'top'));
  return h('div', { class: 'sheet-status', role: 'group', 'aria-label': 'Pafta' }, cursor, sel, zoom, page, library, preflight);
}
