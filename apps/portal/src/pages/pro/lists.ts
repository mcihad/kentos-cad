// A professional's dossiers (docs/adr/9000 §5): their applications and the work municipalities transferred to them,
// what waits for them, and their overview, with their registration's state when it is not verified yet.
import { professional } from '../../api/portal';
import type { DossierDirection } from '../../api/types';
import { withQuery } from '../../app/router';
import { me, professionalStatus } from '../../app/state';
import { h } from '../../lib/dom';
import { button, empty, notice, pageHeader, panel, stat, tabs } from '../../ui/kit';
import { dossierTable } from '../dossier/list';
import { searchBox } from '../org/lists';
import type { PageFn } from '../page';

const OPEN = ['draft', 'submitted', 'in_review', 'changes_requested', 'sent', 'accepted', 'delivered', 'revision_requested'];
const CLOSED = ['approved', 'rejected', 'withdrawn', 'completed', 'declined', 'cancelled'];

/** The registration's state, said at the top while it keeps the professional from working. */
export function registrationNotice(): HTMLElement | null {
  const p = me.value?.professional;
  if (!p || p.status === 'verified') return null;
  const st = professionalStatus(p.status);
  if (p.status === 'pending') return notice('warning', 'Kaydınız doğrulama bekliyor.', 'Sistem yöneticisi oda sicil numaranızı odanın kayıtlarıyla karşılaştırınca başvuru gönderebilir ve iş alabilirsiniz. Sonucu bildirimlerinizde görürsünüz.');
  return notice('danger', `Kaydınız: ${st.label}.`, p.statusNote || 'Ayrıntı için sistem yöneticisine başvurun.');
}

function verified(): boolean {
  return me.value?.professional?.status === 'verified';
}

export function proDossiers(direction: DossierDirection): PageFn {
  return async ({ query }) => {
    const filter = query.get('liste') ?? (direction === 'application' ? 'acik' : 'sira');
    const q = query.get('ara') ?? '';
    const status = filter === 'acik' ? OPEN.join(',') : filter === 'kapanan' ? CLOSED.join(',') : undefined;
    const [list, awaiting] = await Promise.all([
      professional.dossiers({ direction, status, q: q || undefined, awaiting: filter === 'sira' || undefined, archived: filter === 'arsiv' || undefined, limit: 100 }),
      professional.dossiers({ direction, awaiting: true, limit: 1 }),
    ]);
    const application = direction === 'application';
    const title = application ? 'Başvurularım' : 'Aktarılan işler';
    const body = h(
      'div',
      { class: 'page' },
      pageHeader({
        title,
        lead: application ? 'Belediyelere gönderdiğiniz başvurular ve taslaklarınız.' : 'Belediyelerin size aktardığı işler: üstlenin, hazırlayın, teslim edin.',
        actions: application ? [button('Yeni başvuru', { kind: 'primary', icon: 'plus', href: '#/mesleki/basvurular/yeni', disabled: !verified() })] : [],
      }),
      registrationNotice() ? h('div', { style: 'margin-bottom: 14px' }, registrationNotice()) : null,
      tabs(
        [
          { key: 'sira', label: 'Sıra sizde', count: awaiting.total, href: withQuery({ liste: 'sira' }) },
          { key: 'acik', label: 'Açık', href: withQuery({ liste: 'acik' }) },
          { key: 'kapanan', label: 'Kapanan', href: withQuery({ liste: 'kapanan' }) },
          { key: 'tumu', label: 'Tümü', href: withQuery({ liste: 'tumu' }) },
          { key: 'arsiv', label: 'Arşiv', href: withQuery({ liste: 'arsiv' }) },
        ],
        filter,
      ),
      h('div', { class: 'toolbar' }, searchBox(q, 'Başlık ya da dosya no…'), h('span', { class: 'toolbar-fill' }), h('span', { class: 'muted' }, `${list.total} dosya`)),
      dossierTable(
        list.items,
        'professional',
        direction,
        (d) => `#/mesleki/dosya/${d.id}`,
        empty(
          application ? 'inbox' : 'send',
          application ? 'Başvuru yok' : 'Aktarılan iş yok',
          application ? 'Yeni başvuru ile bir belediyeye başvurun; taslağınız siz gönderene kadar yalnız sizde kalır.' : 'Belediyeler size bir iş aktardığında burada görünür ve bildirim alırsınız.',
        ),
      ),
    );
    return { crumbs: [{ label: 'Meslek mensubu', href: '#/mesleki' }, { label: title }], body };
  };
}

export const proDashboard: PageFn = async () => {
  const s = await professional.summary();
  const of = (direction: string, statuses: string[]) => s.counts.filter((c) => c.direction === direction && statuses.includes(c.status)).reduce((n, c) => n + c.count, 0);
  const name = me.value?.user.displayName ?? '';
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({
      title: `Hoş geldiniz, ${name}`,
      lead: 'Başvurularınız ve size aktarılan işler tek yerde: sıranın kimde olduğu, belediyenin ne istediği, ne zaman.',
      actions: [button('Yeni başvuru', { kind: 'primary', icon: 'plus', href: '#/mesleki/basvurular/yeni', disabled: !verified() })],
    }),
    registrationNotice() ? h('div', { style: 'margin-bottom: 14px' }, registrationNotice()) : null,
    h(
      'div',
      { class: 'stats' },
      stat({ label: 'Sıra sizde', value: s.awaiting, icon: 'inbox', hint: 'Düzeltme, üstlenme ya da teslim bekleyen', tone: s.awaiting ? 'warning' : undefined }),
      stat({ label: 'Belediyede incelenen', value: of('application', ['submitted', 'in_review']), icon: 'stamp', hint: 'Gönderdiğiniz, karar bekleyen başvurular', href: '#/mesleki/basvurular?liste=acik' }),
      stat({ label: 'Açık işler', value: of('transfer', ['sent', 'accepted', 'revision_requested', 'delivered']), icon: 'send', hint: `${of('transfer', ['sent'])} iş üstlenmenizi bekliyor`, href: '#/mesleki/isler?liste=acik' }),
      stat({ label: 'Sonuçlanan', value: of('application', ['approved']) + of('transfer', ['completed']), icon: 'check', hint: 'Onaylanan başvurular ve tamamlanan işler' }),
    ),
    panel('Son değişen dosyalar', dossierTable(s.recent, 'professional', null, (d) => `#/mesleki/dosya/${d.id}`, empty('inbox', 'Henüz dosyanız yok', 'Bir belediyeye başvurduğunuzda ya da size iş aktarıldığında burada görünür.')), {
      class: 'panel-flush',
      actions: [button('Başvurularım', { kind: 'ghost', href: '#/mesleki/basvurular' })],
    }),
  );
  return { crumbs: [{ label: 'Meslek mensubu' }], body };
};
