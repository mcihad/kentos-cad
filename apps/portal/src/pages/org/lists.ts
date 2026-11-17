// A municipality's dossier lists (docs/adr/9000 §5): incoming applications and the transfers it opened, filtered by
// what waits for it, what is open or closed, what is assigned to the reader, and a search; its overview's figures.
import { organization } from '../../api/portal';
import type { DossierDirection, DossierSummary } from '../../api/types';
import { go, withQuery } from '../../app/router';
import { orgBySlug } from '../../app/state';
import { h } from '../../lib/dom';
import { icon } from '../../ui/icons';
import { button, empty, pageHeader, panel, stat, tabs } from '../../ui/kit';
import { dossierTable } from '../dossier/list';
import type { PageFn } from '../page';

const OPEN = ['submitted', 'in_review', 'changes_requested', 'sent', 'accepted', 'delivered', 'revision_requested'];
const CLOSED = ['approved', 'rejected', 'withdrawn', 'completed', 'declined', 'cancelled'];

export function searchBox(value: string, placeholder: string): HTMLElement {
  const input = h('input', { class: 'input', type: 'search', name: 'q', value, placeholder, 'aria-label': placeholder });
  const f = h('form', { class: 'search', role: 'search' }, icon('search', 16), input);
  f.addEventListener('submit', (e) => {
    e.preventDefault();
    go(withQuery({ ara: input.value.trim() || null }));
  });
  return f;
}

export function orgDossiers(direction: DossierDirection): PageFn {
  return async ({ params, query }) => {
    const org = orgBySlug(params.slug);
    if (!org) throw new Error('Kurum bulunamadı.');
    const filter = query.get('liste') ?? 'sira';
    const q = query.get('ara') ?? '';
    const mine = query.get('bana') === '1';
    const status = filter === 'acik' ? OPEN.join(',') : filter === 'kapanan' ? CLOSED.join(',') : undefined;
    const [list, all, awaiting] = await Promise.all([
      organization.dossiers(org.tenantId, { direction, status, q: q || undefined, mine: mine || undefined, awaiting: filter === 'sira' || undefined, limit: 100 }),
      organization.dossiers(org.tenantId, { direction, limit: 1 }),
      organization.dossiers(org.tenantId, { direction, awaiting: true, limit: 1 }),
    ]);
    const application = direction === 'application';
    const base = `#/kurum/${org.tenantSlug}`;
    const listHref = `${base}/${application ? 'basvurular' : 'aktarimlar'}`;
    const manager = ['owner', 'admin', 'project_manager'].includes(org.role);
    const head = pageHeader({
      title: application ? 'Gelen başvurular' : 'Proje aktarımları',
      lead: application
        ? 'Meslek mensuplarının belediyenize gönderdiği başvurular: inceleyin, eksikleri bir defada bildirin, karar verin.'
        : 'Meslek mensuplarına aktardığınız işler: projeyi ve belgeleri gönderin, teslimi inceleyin.',
      actions: !application && manager ? [button('Yeni aktarım', { kind: 'primary', icon: 'plus', href: `${base}/aktarimlar/yeni` })] : [],
    });
    const nav = tabs(
      [
        { key: 'sira', label: 'Sıra bizde', count: awaiting.total, href: withQuery({ liste: null }) },
        { key: 'acik', label: 'Açık', href: withQuery({ liste: 'acik' }) },
        { key: 'kapanan', label: 'Kapanan', href: withQuery({ liste: 'kapanan' }) },
        { key: 'tumu', label: 'Tümü', count: all.total, href: withQuery({ liste: 'tumu' }) },
      ],
      filter,
    );
    const mineLink = h('a', { class: `chip ${mine ? 'is-on' : ''}`, href: withQuery({ bana: mine ? null : '1' }) }, icon(mine ? 'check' : 'user', 14), 'Bana atananlar');
    const toolbar = h('div', { class: 'toolbar' }, searchBox(q, application ? 'Başlık, dosya no, meslek mensubu, parsel…' : 'Başlık, dosya no, meslek mensubu…'), mineLink, h('span', { class: 'toolbar-fill' }), h('span', { class: 'muted' }, `${list.total} dosya`));
    const emptyNode = empty(
      application ? 'inbox' : 'send',
      filter === 'sira' ? 'Sırası sizde olan dosya yok' : 'Dosya yok',
      application ? 'Meslek mensupları başvuru gönderdikçe burada görünür.' : manager ? 'Bir meslek mensubuna proje aktarmak için Yeni aktarım ile başlayın.' : 'Kurumunuzun aktardığı işler burada görünür.',
    );
    const body = h('div', { class: 'page' }, head, nav, toolbar, dossierTable(list.items, 'municipality', direction, (d: DossierSummary) => `${base}/dosya/${d.id}`, emptyNode));
    return { crumbs: [{ label: org.tenantName, href: base }, { label: application ? 'Gelen başvurular' : 'Proje aktarımları', href: listHref }], body };
  };
}

export const orgDashboard: PageFn = async ({ params }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  if (org.category !== 'municipality') {
    return {
      crumbs: [{ label: org.tenantName }],
      body: h('div', { class: 'page' }, pageHeader({ title: org.tenantName, lead: 'Bu kurum portalda belediye olarak tanımlı değil; başvuru ve aktarım belediyelerin işidir.' }), empty('building', 'Kurum sayfası', 'Üyeleri ve ayarları sol menüden yönetin.')),
    };
  }
  const [s, awaiting] = await Promise.all([organization.summary(org.tenantId), organization.dossiers(org.tenantId, { awaiting: true, limit: 8 })]);
  const of = (direction: string, statuses: string[]) => s.counts.filter((c) => c.direction === direction && statuses.includes(c.status)).reduce((n, c) => n + c.count, 0);
  const manager = ['owner', 'admin', 'project_manager'].includes(org.role);
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({
      title: org.tenantName,
      lead: 'Belediyenize gelen başvurular ve meslek mensuplarına aktardığınız işler.',
      actions: manager ? [button('Yeni aktarım', { kind: 'primary', icon: 'plus', href: `${base}/aktarimlar/yeni` })] : [],
    }),
    h(
      'div',
      { class: 'stats' },
      stat({ label: 'Sırası bizde', value: s.awaiting, icon: 'inbox', hint: 'İnceleme ya da karar bekleyen dosyalar', href: `${base}/basvurular` }),
      stat({ label: 'Yasal süresi yaklaşan', value: s.dueSoon, icon: 'clock', tone: s.dueSoon ? 'warning' : undefined, hint: 'Üç gün ya da daha az kalan, geçen' }),
      stat({ label: 'Bana atanan', value: s.mine, icon: 'user', hint: 'Sorumlusu olduğunuz açık dosyalar', href: `${base}/basvurular?liste=acik&bana=1` }),
      stat({ label: 'İncelemedeki başvurular', value: of('application', ['submitted', 'in_review']), icon: 'stamp', hint: `${of('application', ['changes_requested'])} başvuruda düzeltme bekleniyor` }),
      stat({ label: 'Açık aktarımlar', value: of('transfer', ['sent', 'accepted', 'delivered', 'revision_requested']), icon: 'send', hint: `${of('transfer', ['delivered'])} teslim onay bekliyor`, href: `${base}/aktarimlar?liste=acik` }),
    ),
    h(
      'div',
      { class: 'stack' },
      panel('Sırası bizde olanlar', dossierTable(awaiting.items, 'municipality', null, (d) => `${base}/dosya/${d.id}`, empty('check', 'Bekleyen dosya yok', 'Sırası belediyenizde olan bütün dosyalar işlendi.')), {
        class: 'panel-flush',
        actions: [button('Tümünü gör', { kind: 'ghost', href: `${base}/basvurular` })],
      }),
      panel('Son değişen dosyalar', dossierTable(s.recent, 'municipality', null, (d) => `${base}/dosya/${d.id}`, empty('inbox', 'Henüz dosya yok', 'Başvurular ve aktarımlar geldikçe burada görünür.')), { class: 'panel-flush' }),
    ),
  );
  return { crumbs: [{ label: org.tenantName }], body };
};
