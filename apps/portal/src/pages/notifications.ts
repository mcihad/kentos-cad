// Every notification of the person (docs/adr/9000 §6), newest first; opening one marks it read.
import { portal } from '../api/portal';
import { unread } from '../app/state';
import { h } from '../lib/dom';
import { moment } from '../lib/format';
import { notificationHref } from '../shell/shell';
import { button, empty, pageHeader, panel } from '../ui/kit';
import { toast } from '../ui/toast';
import type { PageFn } from './page';

export const notificationsPage: PageFn = async ({ reload }) => {
  const got = await portal.notifications();
  unread.set(got.unread);
  const markAll = async () => {
    const r = await portal.readNotifications([], true);
    unread.set(r.unread);
    toast('Bildirimler okundu sayıldı.');
    reload();
  };
  const list = got.items.length
    ? h(
        'div',
        null,
        ...got.items.map((n) => {
          const a = h(
            'a',
            { class: `notif ${n.read ? '' : 'is-unread'}`, href: notificationHref(n) },
            h('span', { class: 'notif-dot', 'aria-hidden': 'true' }),
            h('span', { class: 'notif-text' }, h('span', { class: 'notif-title' }, n.title), n.body ? h('span', { class: 'notif-body' }, n.body) : null, h('span', { class: 'notif-time' }, moment(n.at))),
          );
          a.addEventListener('click', () => {
            if (!n.read) void portal.readNotifications([n.id]).then((r) => unread.set(r.unread));
          });
          return a;
        }),
      )
    : h('div', { class: 'panel-body' }, empty('bell', 'Bildirim yok', 'Bir dosyada size düşen bir adım ya da yeni bir ileti olunca burada görünür.'));
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Bildirimler', lead: `${got.unread} okunmamış bildirim.`, actions: got.unread ? [button('Tümünü okundu say', { onClick: () => void markAll() })] : [] }),
    panel(null, list, { class: 'panel-flush' }),
  );
  return { crumbs: [{ label: 'Bildirimler' }], body };
};
