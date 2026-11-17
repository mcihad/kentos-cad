// One dossier's page, the same for both sides (docs/adr/9000 §5): where it stands in its flow, what waits for whom,
// its legal deadline, its packages, conversation and timeline, its particulars, the documents its kind is usually sent
// with, and the other side. The steps shown are the ones the server allows the reader now.
import { dossier as api } from '../../api/portal';
import type { DossierDetail } from '../../api/types';
import { withQuery } from '../../app/router';
import { chamberOf, professionLabel, professionalStatus } from '../../app/state';
import { type Child, h } from '../../lib/dom';
import { day, daysLeft, initials, moment } from '../../lib/format';
import { icon } from '../../ui/icons';
import { avatar, badge, button, facts, notice, panel, statusBadge, tabs } from '../../ui/kit';
import type { PageFn } from '../page';
import { actionButtons, deleteDraft, editDossier } from './actions';
import { packagesPanel } from './files';
import { type Reader, siteText } from './list';
import { threadPanel, timelinePanel } from './thread';

type StepState = 'done' | 'current' | 'todo' | 'stopped';
type Step = { label: string; state: StepState };

/** The flow's milestones and where the dossier is among them. */
function stepsOf(d: DossierDetail): Step[] {
  const s = d.summary.status;
  const mark = (labels: string[], reached: number, current: { at: number; label?: string; stopped?: boolean } | null): Step[] =>
    labels.map((label, i) => {
      if (current && i === current.at) return { label: current.label ?? label, state: current.stopped ? 'stopped' : 'current' };
      return { label, state: i < reached ? 'done' : 'todo' };
    });
  if (d.summary.direction === 'application') {
    const L = ['Taslak', 'Gönderildi', 'İncelemede', 'Karar'];
    switch (s) {
      case 'draft':
        return mark(L, 0, { at: 0 });
      case 'submitted':
        return mark(L, 1, { at: 1 });
      case 'in_review':
        return mark(L, 2, { at: 2 });
      case 'changes_requested':
        return mark(L, 2, { at: 2, label: 'Düzeltme istendi' });
      case 'approved':
        return mark(['Taslak', 'Gönderildi', 'İncelemede', 'Onaylandı'], 4, null);
      case 'rejected':
        return mark(L, 3, { at: 3, label: 'Reddedildi', stopped: true });
      default:
        return mark(L, 2, { at: 2, label: 'Geri çekildi', stopped: true });
    }
  }
  const L = ['Taslak', 'Gönderildi', 'Üstlenildi', 'Teslim edildi', 'Tamamlandı'];
  switch (s) {
    case 'draft':
      return mark(L, 0, { at: 0 });
    case 'sent':
      return mark(L, 2, { at: 2, label: 'Üstlenilmesi bekleniyor' });
    case 'accepted':
      return mark(L, 3, { at: 3, label: 'Hazırlanıyor' });
    case 'revision_requested':
      return mark(L, 3, { at: 3, label: 'Düzeltme istendi' });
    case 'delivered':
      return mark(L, 4, { at: 4, label: 'Onay bekleniyor' });
    case 'completed':
      return mark(L, 5, null);
    case 'declined':
      return mark(L, 2, { at: 2, label: 'Üstlenilmedi', stopped: true });
    default:
      return mark(L, 2, { at: 4, label: 'İptal edildi', stopped: true });
  }
}

function stepsStrip(d: DossierDetail): HTMLElement {
  const steps = stepsOf(d);
  const out: Child[] = [];
  steps.forEach((st, i) => {
    if (i) out.push(h('span', { class: `step-line ${st.state === 'done' || st.state === 'current' || st.state === 'stopped' ? 'is-done' : ''}` }));
    out.push(
      h(
        'span',
        { class: `step is-${st.state}` },
        h('span', { class: 'step-dot', 'aria-hidden': 'true' }, st.state === 'done' ? icon('check', 13) : st.state === 'stopped' ? icon('close', 12) : String(i + 1)),
        st.label,
      ),
    );
  });
  return h('div', { class: 'steps', role: 'list', 'aria-label': 'Dosyanın akışı' }, ...out);
}

function waiting(d: DossierDetail): string {
  const st = d.summary.status;
  if (['approved', 'rejected', 'withdrawn', 'completed', 'declined', 'cancelled'].includes(st)) return 'Dosya kapandı.';
  if (d.summary.awaitingMe) return d.side === 'municipality' ? 'Sıra belediyenizde.' : 'Sıra sizde.';
  return d.side === 'municipality' ? 'Sıra meslek mensubunda.' : 'Sıra belediyede.';
}

function statusPanel(d: DossierDetail): HTMLElement {
  const c = d.summary.clock;
  return panel(
    'Durum',
    h(
      'div',
      { class: 'stack', style: 'gap: 12px' },
      h('div', { class: 'party' }, statusBadge(d.summary.status, d.summary.statusLabel), h('span', { class: d.summary.awaitingMe ? '' : 'muted' }, waiting(d))),
      c
        ? h(
            'div',
            { class: `clock-box notice tone-${c.daysLeft < 0 ? 'danger' : c.daysLeft <= 3 ? 'warning' : 'info'}` },
            h('div', { class: 'notice-text' }, h('strong', null, `${c.label}: ${day(c.dueOn)}`), h('div', { class: 'big num' }, daysLeft(c.daysLeft)), h('div', { class: 'field-hint' }, c.basis)),
          )
        : null,
      d.summary.dueOn && d.summary.direction === 'transfer' ? facts([['İstenen teslim', day(d.summary.dueOn)]]) : null,
    ),
  );
}

function factsPanel(d: DossierDetail): HTMLElement {
  const s = d.summary;
  const rows: [string, Child][] = [
    ['Dosya no', s.number ? h('span', { class: 'num' }, s.number) : 'Gönderilince verilir'],
    ['Tür', s.kindLabel],
    ['Yer', [s.site.province, s.site.district].filter(Boolean).join(', ') || '—'],
    ['Mahalle', s.site.neighbourhood ?? '—'],
    ['Ada / parsel', s.site.block || s.site.parcel ? `${s.site.block ?? '—'} ada ${s.site.parcel ?? '—'} parsel` : '—'],
  ];
  if (s.site.address) rows.push(['Adres', s.site.address]);
  if (s.direction === 'application') rows.push(['Yapı sahibi', d.ownerName ?? '—']);
  rows.push(['Sorumlu', s.assigneeName ?? (d.side === 'municipality' ? 'Atanmadı' : '—')]);
  rows.push(['Açıldı', moment(s.createdAt)]);
  if (s.sentAt) rows.push(['Gönderildi', moment(s.sentAt)]);
  return panel('Bilgiler', facts(rows));
}

function checklistPanel(d: DossierDetail): HTMLElement | null {
  const c = d.checklist;
  if (!c) return null;
  const missing = c.items.filter((i) => !i.present).length;
  return panel(
    'Belge listesi',
    h(
      'div',
      { class: 'stack', style: 'gap: 10px' },
      h(
        'ul',
        { class: 'checklist' },
        ...c.items.map((i) => h('li', { class: i.present ? 'is-present' : '' }, h('span', { class: 'mark', 'aria-hidden': 'true' }, i.present ? icon('check', 12) : null), h('span', null, i.label, i.present ? null : h('span', { class: 'visually-hidden' }, ' (yok)')))),
      ),
      h('div', { class: 'field-hint' }, c.basis),
    ),
    { note: missing ? `${missing} belge yok` : 'Hepsi var' },
  );
}

function partyPanel(d: DossierDetail): HTMLElement {
  if (d.side === 'municipality' && d.professional) {
    const p = d.professional;
    const st = professionalStatus(p.status);
    return panel(
      'Meslek mensubu',
      h(
        'div',
        { class: 'stack', style: 'gap: 12px' },
        h('div', { class: 'party' }, avatar(p.displayName, initials(p.displayName)), h('div', { class: 'party-text' }, h('span', { class: 'party-name' }, p.displayName), h('span', { class: 'party-sub' }, professionLabel(p.profession)), badge(st.tone, st.label))),
        facts([
          ['Oda', chamberOf(p.profession)],
          ['Sicil no', h('span', { class: 'num' }, p.chamberNo)],
          ['Büro', p.officeName ?? '—'],
          ['Telefon', p.phone ?? '—'],
          ['E-posta', p.email ?? '—'],
        ]),
      ),
    );
  }
  const o = d.organization;
  return panel(
    'Belediye',
    h(
      'div',
      { class: 'stack', style: 'gap: 12px' },
      h('div', { class: 'party' }, h('span', { class: 'file-icon is-project' }, icon('building', 18)), h('div', { class: 'party-text' }, h('span', { class: 'party-name' }, o.name), h('span', { class: 'party-sub' }, [o.province, o.district].filter(Boolean).join(', ')))),
      facts([
        ['E-posta', o.contactEmail ?? '—'],
        ['Telefon', o.contactPhone ?? '—'],
      ]),
    ),
  );
}

/** What the reader should know before anything else on the page. */
function notices(d: DossierDetail): HTMLElement[] {
  const s = d.summary;
  const out: HTMLElement[] = [];
  const last = (kind: string) => [...d.messages].reverse().find((m) => m.kind === kind);
  if (s.status === 'draft' && d.side === 'professional') out.push(notice('info', 'Bu bir taslak; belediye henüz görmüyor.', 'Paketi hazırlayın, belge listesine bakın ve başvuruyu gönderin.'));
  if (s.status === 'draft' && d.side === 'municipality') out.push(notice('info', 'Bu bir taslak; meslek mensubu henüz görmüyor.', 'Aktaracağınız projeyi ve belgeleri pakete ekleyip işi gönderin.'));
  if (s.status === 'changes_requested' || s.status === 'revision_requested') {
    const m = last('deficiency');
    out.push(
      notice(
        'warning',
        d.side === 'professional' ? 'Belediye düzeltme istedi.' : 'Düzeltme istendi; meslek mensubunun yeni sürümü bekleniyor.',
        m ? h('div', { class: 'msg-body' }, m.body) : null,
      ),
    );
  }
  if (['approved', 'completed'].includes(s.status)) {
    const m = last('decision');
    out.push(notice('success', s.status === 'approved' ? 'Başvuru onaylandı.' : 'İş tamamlandı.', m ? m.body : null));
  }
  if (['rejected', 'declined', 'cancelled'].includes(s.status)) {
    const m = last('decision');
    out.push(notice('danger', s.statusLabel, m ? m.body : null));
  }
  return out;
}

export function dossierPage(reader: Reader): PageFn {
  return async ({ params, query, reload }) => {
    const d = await api.get(params.id);
    const s = d.summary;
    const application = s.direction === 'application';
    const base = reader === 'municipality' ? `#/kurum/${params.slug}` : '#/mesleki';
    const listHref = reader === 'municipality' ? `${base}/${application ? 'basvurular' : 'aktarimlar'}` : `${base}/${application ? 'basvurular' : 'isler'}`;
    const listLabel = reader === 'municipality' ? (application ? 'Gelen başvurular' : 'Proje aktarımları') : application ? 'Başvurularım' : 'Aktarılan işler';
    const tab = query.get('sekme') ?? 'paket';
    const extra: HTMLElement[] = [];
    if (d.canEdit) extra.push(button('Bilgileri düzenle', { kind: 'ghost', icon: 'settings', onClick: () => editDossier(d, reload) }));
    if (d.canDelete) extra.push(button('Taslağı sil', { kind: 'ghost', icon: 'trash', onClick: () => void deleteDraft(d, listHref) }));
    const counterpart = reader === 'municipality' ? `${s.professionalName}, ${professionLabel(s.profession)}` : s.tenantName;
    const site = siteText(s.site);
    const head = h(
      'div',
      { class: 'dossier-head' },
      h(
        'div',
        { class: 'dossier-title' },
        h('a', { class: 'page-back', href: listHref }, icon('arrowLeft', 16), listLabel),
        h('div', { class: 'dossier-number num' }, s.number ? `${application ? 'Başvuru' : 'Aktarım'} ${s.number}` : `${application ? 'Başvuru' : 'Aktarım'} taslağı`, `, ${s.kindLabel}`),
        h('h1', null, s.title),
        h(
          'div',
          { class: 'dossier-meta' },
          statusBadge(s.status, s.statusLabel),
          h('span', { class: 'meta' }, icon(reader === 'municipality' ? 'badge' : 'building', 15), counterpart),
          site ? h('span', { class: 'meta' }, icon('pin', 15), site) : null,
          h('span', { class: 'meta' }, icon('clock', 15), `Son değişiklik ${moment(s.updatedAt)}`),
        ),
      ),
      h('div', { class: 'page-actions' }, ...extra, ...actionButtons(d, reload)),
    );
    const body =
      tab === 'yazisma' ? threadPanel(d, reload) : tab === 'gecmis' ? timelinePanel(d) : h('div', { class: 'stack' }, d.description ? panel('Açıklama', h('div', { class: 'msg-body' }, d.description)) : null, packagesPanel(d, reload));
    const page = h(
      'div',
      { class: 'page' },
      head,
      stepsStrip(d),
      ...notices(d).map((n) => h('div', { style: 'margin-bottom: 14px' }, n)),
      h(
        'div',
        { class: 'dossier-grid' },
        h(
          'div',
          null,
          tabs(
            [
              { key: 'paket', label: 'Paketler ve belgeler', href: withQuery({ sekme: null }) },
              { key: 'yazisma', label: 'Yazışma', count: d.messages.length, href: withQuery({ sekme: 'yazisma' }) },
              { key: 'gecmis', label: 'Geçmiş', count: d.events.length, href: withQuery({ sekme: 'gecmis' }) },
            ],
            tab,
          ),
          body,
        ),
        h('aside', { class: 'dossier-side' }, statusPanel(d), checklistPanel(d), factsPanel(d), partyPanel(d)),
      ),
    );
    const crumbs =
      reader === 'municipality'
        ? [{ label: d.organization.name, href: base }, { label: listLabel, href: listHref }, { label: s.number ?? 'Taslak' }]
        : [{ label: 'Meslek mensubu', href: base }, { label: listLabel, href: listHref }, { label: s.number ?? 'Taslak' }];
    return { crumbs, body: page, nav: listHref };
  };
}
