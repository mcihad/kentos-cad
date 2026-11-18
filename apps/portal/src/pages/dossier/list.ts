// Dossiers in lists (docs/adr/9000 §5): the columns each side reads, the place said as surveyors say it ("412 ada 7
// parsel"), and a mark on the rows waiting for the reader.
import type { DossierDirection, DossierSite, DossierSummary } from '../../api/types';
import { professionLabel } from '../../app/state';
import { type Child, h } from '../../lib/dom';
import { day, daysLeft, daysUntil, since } from '../../lib/format';
import { icon } from '../../ui/icons';
import { type Column, clockChip, statusBadge, table, twoLine } from '../../ui/kit';

export type Reader = 'municipality' | 'professional';

/** "Cumhuriyet Mahallesi, 412 ada 7 parsel". */
export function siteText(site: DossierSite): string {
  const parts: string[] = [];
  if (site.neighbourhood) parts.push(site.neighbourhood);
  const parcel = [site.block ? `${site.block} ada` : '', site.parcel ? `${site.parcel} parsel` : ''].filter(Boolean).join(' ');
  if (parcel) parts.push(parcel);
  if (!parts.length && site.address) parts.push(site.address);
  if (!parts.length) parts.push([site.district, site.province].filter(Boolean).join(', '));
  return parts.filter(Boolean).join(', ');
}

export function numberCell(d: DossierSummary): HTMLElement {
  return h(
    'span',
    { class: `dossier-no num ${d.awaitingMe ? 'is-awaiting' : ''}`, title: d.awaitingMe ? 'Sıra sizde' : null },
    d.awaitingMe ? h('span', { class: 'await-dot', 'aria-label': 'Sıra sizde' }) : null,
    d.number ?? h('span', { class: 'faint' }, 'Taslak'),
  );
}

function dueCell(d: DossierSummary): Child {
  if (!d.dueOn) return h('span', { class: 'faint' }, '—');
  const left = daysUntil(d.dueOn);
  const closed = ['completed', 'cancelled', 'declined'].includes(d.status);
  return twoLine(day(d.dueOn), closed ? '' : h('span', { class: left < 0 ? 'tone-text-danger' : left <= 3 ? 'tone-text-warning' : '' }, daysLeft(left)));
}

function titleCell(d: DossierSummary): Child {
  const site = siteText(d.site);
  return twoLine(d.title, site ? `${d.kindLabel}, ${site}` : d.kindLabel);
}

function professionalCell(d: DossierSummary): Child {
  return twoLine(d.professionalName, professionLabel(d.profession));
}

function updatedCell(d: DossierSummary): Child {
  return h('span', { class: 'muted num' }, since(d.updatedAt));
}

export function dossierTable(items: DossierSummary[], reader: Reader, direction: DossierDirection | null, href: (d: DossierSummary) => string, emptyNode: Child): HTMLElement {
  const no: Column<DossierSummary> = { label: 'No', cell: numberCell, class: 'narrow' };
  const status: Column<DossierSummary> = { label: 'Durum', cell: (d) => statusBadge(d.status, d.statusLabel), class: 'narrow' };
  const updated: Column<DossierSummary> = { label: 'Son değişiklik', cell: updatedCell, class: 'narrow' };
  let columns: Column<DossierSummary>[];
  if (reader === 'municipality' && direction === 'application') {
    columns = [
      no,
      { label: 'Başvuru', cell: titleCell },
      { label: 'Meslek mensubu', cell: professionalCell },
      status,
      { label: 'Yasal süre', cell: (d) => (d.clock ? clockChip(d.clock) : h('span', { class: 'faint' }, '—')), class: 'narrow' },
      { label: 'Sorumlu', cell: (d) => d.assigneeName ?? h('span', { class: 'faint' }, 'Atanmadı') },
      updated,
    ];
  } else if (reader === 'municipality' && direction === 'transfer') {
    columns = [no, { label: 'İş', cell: titleCell }, { label: 'Meslek mensubu', cell: professionalCell }, status, { label: 'Teslim günü', cell: dueCell, class: 'narrow' }, updated];
  } else if (reader === 'professional' && direction === 'application') {
    columns = [no, { label: 'Başvuru', cell: titleCell }, { label: 'Belediye', cell: (d) => d.tenantName }, status, updated];
  } else if (reader === 'professional' && direction === 'transfer') {
    columns = [no, { label: 'İş', cell: titleCell }, { label: 'Belediye', cell: (d) => d.tenantName }, status, { label: 'Teslim günü', cell: dueCell, class: 'narrow' }, updated];
  } else {
    columns = [
      no,
      {
        label: 'Dosya',
        cell: (d) =>
          h(
            'div',
            { class: 'with-icon' },
            h('span', { class: 'kind-icon', title: d.direction === 'application' ? 'Başvuru' : 'Aktarım' }, icon(d.direction === 'application' ? 'inbox' : 'send', 16)),
            titleCell(d),
          ),
      },
      { label: reader === 'municipality' ? 'Meslek mensubu' : 'Belediye', cell: (d) => (reader === 'municipality' ? professionalCell(d) : d.tenantName) },
      status,
      updated,
    ];
  }
  return table(columns, items, { href, empty: emptyNode });
}
