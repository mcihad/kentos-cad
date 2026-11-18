// An office's pages (docs/adr/9000 §18): the dossiers its professionals share with it, read and worked on by its
// colleagues by role. The office is on the professional's side: the other side of each dossier is a municipality.
import { organization } from '../../api/portal';
import type { DossierDirection, DossierSummary } from '../../api/types';
import { withQuery } from '../../app/router';
import { orgBySlug } from '../../app/state';
import { h } from '../../lib/dom';
import { empty, pageHeader, panel, stat, tabs } from '../../ui/kit';
import { dossierTable } from '../dossier/list';
import type { PageFn } from '../page';
import { searchBox } from './lists';

const OPEN = ['draft', 'submitted', 'in_review', 'changes_requested', 'sent', 'accepted', 'delivered', 'revision_requested'];
const CLOSED = ['approved', 'rejected', 'withdrawn', 'completed', 'declined', 'cancelled'];

export function officeDossiers(direction: DossierDirection): PageFn {
  return async ({ params, query }) => {
    const org = orgBySlug(params.slug);
    if (!org) throw new Error('Kurum bulunamadı.');
    const filter = query.get('liste') ?? 'sira';
    const q = query.get('ara') ?? '';
    const status = filter === 'acik' ? OPEN.join(',') : filter === 'kapanan' ? CLOSED.join(',') : undefined;
    const [list, all, awaiting] = await Promise.all([
      organization.dossiers(org.tenantId, { direction, status, q: q || undefined, awaiting: filter === 'sira' || undefined, limit: 100 }),
      organization.dossiers(org.tenantId, { direction, limit: 1 }),
      organization.dossiers(org.tenantId, { direction, awaiting: true, limit: 1 }),
    ]);
    const application = direction === 'application';
    const base = `#/kurum/${org.tenantSlug}`;
    const listHref = `${base}/${application ? 'basvurular' : 'isler'}`;
    const title = application ? 'Büronun başvuruları' : 'Büroya aktarılan işler';
    const head = pageHeader({
      title,
      lead: application
        ? 'Büronun meslek mensuplarının büroyla paylaştığı başvurular: belgeleri hazırlayın, yazışın; proje yöneticileri gönderir.'
        : 'Belediyelerin büronun meslek mensuplarına aktardığı ve büroyla paylaşılan işler.',
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
    const toolbar = h('div', { class: 'toolbar' }, searchBox(q, 'Başlık, dosya no, parsel…'), h('span', { class: 'toolbar-fill' }), h('span', { class: 'muted' }, `${list.total} dosya`));
    const emptyNode = empty(
      application ? 'inbox' : 'send',
      filter === 'sira' ? 'Sırası büroda olan dosya yok' : 'Dosya yok',
      'Meslek mensupları dosyalarını büroyla paylaştıkça ya da büro adına başvurdukça burada görünür.',
    );
    const body = h('div', { class: 'page' }, head, nav, toolbar, dossierTable(list.items, 'professional', direction, (d: DossierSummary) => `${base}/dosya/${d.id}`, emptyNode));
    return { crumbs: [{ label: org.tenantName, href: base }, { label: title, href: listHref }], body };
  };
}

export const officeDashboard: PageFn = async ({ params }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  const [s, awaiting] = await Promise.all([organization.summary(org.tenantId), organization.dossiers(org.tenantId, { awaiting: true, limit: 8 })]);
  const of = (direction: string, statuses: string[]) => s.counts.filter((c) => c.direction === direction && statuses.includes(c.status)).reduce((n, c) => n + c.count, 0);
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: org.tenantName, lead: 'Büronun meslek mensuplarının büroyla paylaştığı başvurular ve aktarılan işler.' }),
    h(
      'div',
      { class: 'stats' },
      stat({ label: 'Sırası büroda', value: s.awaiting, icon: 'inbox', hint: 'Hazırlanacak, düzeltilecek ya da teslim edilecek', href: `${base}/basvurular` }),
      stat({ label: 'Yasal süresi yaklaşan', value: s.dueSoon, icon: 'clock', tone: s.dueSoon ? 'warning' : undefined, hint: 'Üç gün ya da daha az kalan, geçen' }),
      stat({ label: 'Açık başvurular', value: of('application', ['draft', 'submitted', 'in_review', 'changes_requested']), icon: 'stamp', hint: `${of('application', ['changes_requested'])} başvuruda düzeltme isteniyor`, href: `${base}/basvurular?liste=acik` }),
      stat({ label: 'Açık işler', value: of('transfer', ['sent', 'accepted', 'delivered', 'revision_requested']), icon: 'send', href: `${base}/isler?liste=acik` }),
    ),
    h(
      'div',
      { class: 'stack' },
      panel('Sırası büroda olanlar', dossierTable(awaiting.items, 'professional', null, (d) => `${base}/dosya/${d.id}`, empty('check', 'Bekleyen dosya yok', 'Sırası büroda olan bütün dosyalar işlendi.')), { class: 'panel-flush' }),
      panel('Son değişen dosyalar', dossierTable(s.recent, 'professional', null, (d) => `${base}/dosya/${d.id}`, empty('inbox', 'Henüz dosya yok', 'Meslek mensupları dosyalarını büroyla paylaştıkça burada görünür.')), { class: 'panel-flush' }),
    ),
  );
  return { crumbs: [{ label: org.tenantName }], body };
};
