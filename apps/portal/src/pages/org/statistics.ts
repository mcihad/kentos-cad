// A municipality's figures for a period (docs/adr/9000 §22): what came in and was decided, how fast, what waits and
// what is late, by month, kind and person in charge; its fees. The workbook (Excel, CSV) is the server's; the PDF is
// this page printed (its print style leaves the portal's frame out), so Turkish letters need no font on the server.
import { query as qs } from '../../api/client';
import { statistics as api } from '../../api/portal';
import type { PortalStatistics } from '../../api/types';
import { go } from '../../app/router';
import { orgBySlug } from '../../app/state';
import { type Child, h } from '../../lib/dom';
import { day, decimal, money } from '../../lib/format';
import { groupedColumns } from '../../ui/chart';
import { button, pageHeader, panel, stat, table } from '../../ui/kit';
import type { PageFn } from '../page';

const MONTHS = ['Ocak', 'Şubat', 'Mart', 'Nisan', 'Mayıs', 'Haziran', 'Temmuz', 'Ağustos', 'Eylül', 'Ekim', 'Kasım', 'Aralık'];

const iso = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

/** The period presets: this year, the last twelve months, this month, last year. */
function presets(): { key: string; label: string; from: string; to: string }[] {
  const now = new Date();
  const y = now.getFullYear();
  return [
    { key: 'yil', label: 'Bu yıl', from: `${y}-01-01`, to: iso(now) },
    { key: '12ay', label: 'Son 12 ay', from: iso(new Date(y, now.getMonth() - 11, 1)), to: iso(now) },
    { key: 'ay', label: 'Bu ay', from: iso(new Date(y, now.getMonth(), 1)), to: iso(now) },
    { key: 'gecen', label: 'Geçen yıl', from: `${y - 1}-01-01`, to: `${y - 1}-12-31` },
  ];
}

function days(x: number | null): string {
  return x == null ? '—' : `${decimal(x)} gün`;
}

export const orgStatistics: PageFn = async ({ params, query }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  const choice = presets().find((p) => p.key === (query.get('donem') ?? 'yil'));
  const from = query.get('baslangic') ?? choice?.from ?? presets()[0].from;
  const to = query.get('bitis') ?? choice?.to ?? presets()[0].to;
  const s: PortalStatistics = await api.page(org.tenantId, from, to);

  const periodSelect = h(
    'select',
    { class: 'input', 'aria-label': 'Dönem', style: 'width: 170px' },
    ...presets().map((p) => h('option', { value: p.key, selected: !query.get('baslangic') && p.key === (query.get('donem') ?? 'yil') }, p.label)),
    h('option', { value: 'ozel', selected: !!query.get('baslangic') }, 'Özel dönem'),
  );
  const fromInput = h('input', { class: 'input', type: 'date', value: s.from, 'aria-label': 'Başlangıç' });
  const toInput = h('input', { class: 'input', type: 'date', value: s.to, 'aria-label': 'Bitiş' });
  periodSelect.addEventListener('change', () => {
    if (periodSelect.value === 'ozel') return;
    go(`${base}/istatistik?donem=${periodSelect.value}`);
  });
  const apply = button('Uygula', { kind: 'ghost', onClick: () => go(`${base}/istatistik?baslangic=${fromInput.value}&bitis=${toInput.value}`) });
  const file = (format: string) => `/v1/tenants/${org.tenantId}/portal/statistics/export${qs({ format, from: s.from, to: s.to })}`;

  const months = s.byMonth.map((m) => {
    const [y, mo] = m.month.split('-').map(Number);
    return { label: MONTHS[mo - 1].slice(0, 3), title: `${MONTHS[mo - 1]} ${y}`, values: [m.received, m.decided] };
  });
  const chart = groupedColumns(
    months,
    [
      { name: 'Gelen başvuru', token: '--viz-1' },
      { name: 'Karara bağlanan', token: '--viz-2' },
    ],
    { label: 'Aylara göre gelen ve karara bağlanan başvurular' },
  );
  const tiles: Child[] = [
    stat({ label: 'Gelen başvuru', value: s.received, icon: 'inbox', hint: `${day(s.from)} – ${day(s.to)}` }),
    stat({ label: 'Karara bağlanan', value: s.decided, icon: 'stamp', hint: `${s.approved} onay · ${s.rejected} ret · ${s.withdrawn} geri çekildi` }),
    stat({ label: 'Ortalama karar süresi', value: days(s.averageDecisionDays), icon: 'clock', hint: `ortanca ${days(s.medianDecisionDays)} · ${s.averageRounds == null ? '—' : decimal(s.averageRounds)} tur` }),
    stat({ label: 'Açık başvuru', value: s.openNow, icon: 'hourglass', tone: s.overdueNow ? 'danger' : undefined, hint: s.overdueNow ? `${s.overdueNow} başvurunun yasal süresi geçti` : 'Yasal süresi geçen yok' }),
    stat({ label: 'Harç ve ücret', value: money(s.feesAssessed), icon: 'receipt', hint: `${money(s.feesPaid)} tahsil edildi` }),
  ];
  const body = h(
    'div',
    { class: 'page stats-page' },
    pageHeader({
      title: 'İstatistikler',
      lead: `${org.tenantName}: dönemde gelen ve karara bağlanan başvurular, süreler, açık işler ve harçlar.`,
      actions: [
        button('Excel', { kind: 'ghost', icon: 'download', href: file('xlsx') }),
        button('CSV', { kind: 'ghost', icon: 'download', href: file('csv') }),
        button('Yazdır / PDF', { kind: 'secondary', icon: 'file', onClick: () => window.print() }),
      ],
    }),
    h('div', { class: 'toolbar no-print' }, periodSelect, fromInput, h('span', { class: 'muted' }, '–'), toInput, apply),
    h('div', { class: 'print-only print-title' }, `${org.tenantName} · İstatistikler · ${day(s.from)} – ${day(s.to)}`),
    h('div', { class: 'stats' }, ...tiles),
    panel(
      'Aylara göre',
      h(
        'div',
        { class: 'stack' },
        chart,
        h('details', { class: 'table-toggle' }, h('summary', null, 'Tablo olarak'), table(
          [
            { label: 'Ay', cell: (m: (typeof months)[number]) => m.title },
            { label: 'Gelen', cell: (m: (typeof months)[number]) => h('span', { class: 'num' }, String(m.values[0])), class: 'narrow right' },
            { label: 'Karara bağlanan', cell: (m: (typeof months)[number]) => h('span', { class: 'num' }, String(m.values[1])), class: 'narrow right' },
          ],
          months,
          { class: 'stats-month-table' },
        )),
      ),
    ),
    h(
      'div',
      { class: 'cols-2' },
      panel(
        'Türlere göre',
        table(
          [
            { label: 'Tür', cell: (k) => k.label },
            { label: 'Gelen', cell: (k) => h('span', { class: 'num' }, String(k.received)), class: 'narrow right' },
            { label: 'Onay', cell: (k) => h('span', { class: 'num' }, String(k.approved)), class: 'narrow right' },
            { label: 'Ret', cell: (k) => h('span', { class: 'num' }, String(k.rejected)), class: 'narrow right' },
            { label: 'Ort. karar', cell: (k) => days(k.averageDecisionDays), class: 'narrow right' },
          ],
          s.byKind,
          { empty: h('div', { class: 'muted' }, 'Bu dönemde başvuru yok.') },
        ),
      ),
      panel(
        'Sorumlulara göre',
        table(
          [
            { label: 'Sorumlu', cell: (p) => p.name },
            { label: 'Açık', cell: (p) => h('span', { class: 'num' }, String(p.open)), class: 'narrow right' },
            { label: 'Karara bağlanan', cell: (p) => h('span', { class: 'num' }, String(p.decided)), class: 'narrow right' },
          ],
          s.byAssignee,
          { empty: h('div', { class: 'muted' }, 'Açık ya da karara bağlanmış başvuru yok.') },
        ),
        { note: `Aktarım: ${s.transfersSent} gönderildi, ${s.transfersCompleted} tamamlandı` },
      ),
    ),
  );
  return { crumbs: [{ label: org.tenantName, href: base }, { label: 'İstatistikler' }], body };
};
