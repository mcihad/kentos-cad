// The portal's frame (docs/adr/9000 §8): on the left the mark, where the person works (the platform, an organisation,
// their practice) and that place's pages; on top where they are and their notifications; the page in the middle.
import { portal } from '../api/portal';
import type { NotificationView } from '../api/types';
import { type Context, contexts, homeOf, me, professionLabel, roleLabel, unread } from '../app/state';
import { type Child, h, replaceChildren } from '../lib/dom';
import { initials, since } from '../lib/format';
import { type IconName, brandMark, icon } from '../ui/icons';
import { avatar } from '../ui/kit';

export type Crumb = { label: string; href?: string };

type NavItem = { label: string; href: string; icon: IconName; count?: number; match: RegExp };

/** Figures beside a context's pages (waiting dossiers, professionals to check), by page address. */
export const counts = new Map<string, number>();

function navOf(c: Context): NavItem[] {
  switch (c.kind) {
    case 'admin':
      return [
        { label: 'Genel bakış', href: '#/sistem', icon: 'home', match: /^#\/sistem$/ },
        { label: 'Kurumlar', href: '#/sistem/kurumlar', icon: 'building', match: /^#\/sistem\/kurumlar/ },
        { label: 'Meslek mensupları', href: '#/sistem/meslek-mensuplari', icon: 'badge', match: /^#\/sistem\/meslek-mensuplari/ },
        { label: 'Hesaplar', href: '#/sistem/hesaplar', icon: 'users', match: /^#\/sistem\/hesaplar/ },
        { label: 'Denetim kaydı', href: '#/sistem/kayit', icon: 'list', match: /^#\/sistem\/kayit/ },
        { label: 'Platform ayarları', href: '#/sistem/ayarlar', icon: 'settings', match: /^#\/sistem\/ayarlar/ },
      ];
    case 'org': {
      const base = `#/kurum/${c.org.tenantSlug}`;
      const admin = c.org.role === 'owner' || c.org.role === 'admin';
      const municipality = c.org.category === 'municipality';
      const items: NavItem[] = [{ label: 'Genel bakış', href: base, icon: 'home', match: new RegExp(`^${base}$`) }];
      if (c.org.category === 'office') {
        items.push(
          { label: 'Başvurular', href: `${base}/basvurular`, icon: 'inbox', match: new RegExp(`^${base}/(basvurular|dosya)`) },
          { label: 'Aktarılan işler', href: `${base}/isler`, icon: 'send', match: new RegExp(`^${base}/isler`) },
        );
      }
      if (municipality) {
        items.push(
          { label: 'Gelen başvurular', href: `${base}/basvurular`, icon: 'inbox', match: new RegExp(`^${base}/(basvurular|dosya)`) },
          { label: 'Proje aktarımları', href: `${base}/aktarimlar`, icon: 'send', match: new RegExp(`^${base}/aktarimlar`) },
          { label: 'Meslek mensupları', href: `${base}/meslek-mensuplari`, icon: 'badge', match: new RegExp(`^${base}/meslek-mensuplari`) },
          { label: 'Harç ve ücretler', href: `${base}/harclar`, icon: 'receipt', match: new RegExp(`^${base}/harclar`) },
        );
      }
      items.push({ label: 'Projeler', href: `${base}/projeler`, icon: 'folder', match: new RegExp(`^${base}/projeler|^#/proje/${c.org.tenantId}`) });
      if (admin) {
        items.push(
          { label: 'Üyeler', href: `${base}/uyeler`, icon: 'users', match: new RegExp(`^${base}/uyeler`) },
          { label: 'Kurum ayarları', href: `${base}/ayarlar`, icon: 'settings', match: new RegExp(`^${base}/ayarlar`) },
        );
      }
      return items;
    }
    case 'pro':
      return [
        { label: 'Genel bakış', href: '#/mesleki', icon: 'home', match: /^#\/mesleki$/ },
        { label: 'Başvurularım', href: '#/mesleki/basvurular', icon: 'inbox', match: /^#\/mesleki\/(basvurular|dosya)/ },
        { label: 'Aktarılan işler', href: '#/mesleki/isler', icon: 'send', match: /^#\/mesleki\/isler/ },
        { label: 'Projelerim', href: '#/mesleki/projeler', icon: 'folder', match: /^#\/(mesleki\/projeler|proje\/)/ },
        { label: 'Profil', href: '#/mesleki/profil', icon: 'user', match: /^#\/mesleki\/profil/ },
      ];
  }
}

function contextName(c: Context): { name: string; role: string; icon: IconName } {
  switch (c.kind) {
    case 'admin':
      return { name: 'Sistem yönetimi', role: 'Platform yöneticisi', icon: 'shield' };
    case 'org':
      return { name: c.org.tenantName, role: c.org.usable ? roleLabel(c.org.role) : 'Üyelik kullanılamıyor', icon: 'building' };
    case 'pro': {
      const p = me.value?.professional;
      return { name: 'Meslek mensubu', role: p ? professionLabel(p.profession) : '', icon: 'badge' };
    }
  }
}

function same(a: Context, b: Context): boolean {
  if (a.kind !== b.kind) return false;
  return a.kind !== 'org' || (b.kind === 'org' && a.org.tenantId === b.org.tenantId);
}

/** Closes popovers on a click outside them or on Esc. */
function popover(anchor: HTMLElement, panel: HTMLElement): () => void {
  const close = () => {
    panel.remove();
    document.removeEventListener('mousedown', outside, true);
    document.removeEventListener('keydown', esc, true);
    anchor.setAttribute('aria-expanded', 'false');
  };
  const outside = (e: MouseEvent) => {
    if (!panel.contains(e.target as Node) && !anchor.contains(e.target as Node)) close();
  };
  const esc = (e: KeyboardEvent) => {
    if (e.key === 'Escape') close();
  };
  document.addEventListener('mousedown', outside, true);
  document.addEventListener('keydown', esc, true);
  anchor.setAttribute('aria-expanded', 'true');
  return close;
}

function switcher(current: Context): HTMLElement {
  const all = me.value ? contexts(me.value) : [];
  const shown = contextName(current);
  const btn = h(
    'button',
    { class: 'switcher', type: 'button', 'aria-haspopup': 'menu', 'aria-expanded': 'false', disabled: all.length < 2 },
    h('span', { class: 'switcher-icon' }, icon(shown.icon, 18)),
    h('span', { class: 'switcher-text' }, h('span', { class: 'switcher-name' }, shown.name), h('span', { class: 'switcher-role' }, shown.role)),
    all.length > 1 ? icon('chevronDown', 16) : null,
  );
  const wrap = h('div', { class: 'switcher-wrap' }, btn);
  btn.addEventListener('click', () => {
    if (wrap.querySelector('.menu')) return;
    const menu = h(
      'div',
      { class: 'menu switcher-menu', role: 'menu' },
      h('div', { class: 'menu-title' }, 'Çalışma yeri'),
      ...all.map((c) => {
        const n = contextName(c);
        const item = h(
          'a',
          { class: `menu-item ${same(c, current) ? 'is-current' : ''}`, href: homeOf(c), role: 'menuitem' },
          icon(n.icon, 16),
          h('span', { class: 'menu-item-text' }, h('span', null, n.name), h('span', { class: 'menu-item-sub' }, n.role)),
          same(c, current) ? icon('check', 16) : null,
        );
        item.addEventListener('click', () => close());
        return item;
      }),
    );
    wrap.append(menu);
    const close = popover(btn, menu);
  });
  return wrap;
}

function theme(): 'light' | 'dark' {
  return document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light';
}

export function setTheme(t: 'light' | 'dark'): void {
  document.documentElement.dataset.theme = t;
  try {
    localStorage.setItem('kentos-portal-theme', t);
  } catch {
    // A browser that keeps nothing: the theme lasts this visit.
  }
}

function userBlock(onLogout: () => void): HTMLElement {
  const m = me.value;
  const name = m?.user.displayName ?? '';
  const btn = h(
    'button',
    { class: 'user', type: 'button', 'aria-haspopup': 'menu', 'aria-expanded': 'false' },
    avatar(name, initials(name), 'sm'),
    h('span', { class: 'user-text' }, h('span', { class: 'user-name' }, name), h('span', { class: 'user-login' }, m?.login ?? m?.user.email ?? '')),
    icon('more', 16),
  );
  const wrap = h('div', { class: 'user-wrap' }, btn);
  btn.addEventListener('click', () => {
    if (wrap.querySelector('.menu')) return;
    const dark = theme() === 'dark';
    const themeItem = h('button', { class: 'menu-item', type: 'button', role: 'menuitem' }, icon(dark ? 'sun' : 'moon', 16), dark ? 'Açık tema' : 'Koyu tema');
    const out = h('button', { class: 'menu-item', type: 'button', role: 'menuitem' }, icon('logout', 16), 'Oturumu kapat');
    const items: Child[] = [];
    items.push(h('a', { class: 'menu-item', href: '#/hesabim', role: 'menuitem' }, icon('key', 16), 'Hesabım'));
    if (m?.professional) items.push(h('a', { class: 'menu-item', href: '#/mesleki/profil', role: 'menuitem' }, icon('user', 16), 'Profilim'));
    items.push(themeItem, h('div', { class: 'menu-sep' }), out);
    const menu = h('div', { class: 'menu user-menu', role: 'menu' }, ...items);
    wrap.append(menu);
    const close = popover(btn, menu);
    themeItem.addEventListener('click', () => {
      setTheme(dark ? 'light' : 'dark');
      close();
    });
    out.addEventListener('click', () => {
      close();
      onLogout();
    });
    menu.querySelectorAll('a').forEach((a) => a.addEventListener('click', () => close()));
  });
  return wrap;
}

/** Where a notification opens: its dossier in the place it was sent to, else the page it is about. */
export function notificationHref(n: NotificationView): string {
  if (n.dossierId) return n.tenantSlug ? `#/kurum/${n.tenantSlug}/dosya/${n.dossierId}` : `#/mesleki/dosya/${n.dossierId}`;
  if (n.kind === 'professional.registered') return '#/sistem/meslek-mensuplari';
  if (n.kind.startsWith('professional.')) return '#/mesleki/profil';
  return '#/bildirimler';
}

function bell(): HTMLElement {
  const btn = h('button', { class: 'icon-btn bell', type: 'button', 'aria-label': 'Bildirimler', title: 'Bildirimler', 'aria-haspopup': 'dialog', 'aria-expanded': 'false' }, icon('bell', 18));
  const badgeEl = h('span', { class: 'icon-btn-badge num' });
  btn.append(badgeEl);
  const show = (n: number) => {
    badgeEl.textContent = n > 99 ? '99+' : String(n);
    badgeEl.hidden = n === 0;
    btn.setAttribute('aria-label', n ? `Bildirimler (${n} okunmamış)` : 'Bildirimler');
  };
  unread.subscribe(show, true);
  const wrap = h('div', { class: 'bell-wrap' }, btn);
  btn.addEventListener('click', async () => {
    if (wrap.querySelector('.menu')) return;
    const list = h('div', { class: 'notif-list' }, h('div', { class: 'loading' }, 'Yükleniyor…'));
    const markAll = h('button', { class: 'link-btn', type: 'button' }, 'Tümünü okundu say');
    const menu = h(
      'div',
      { class: 'menu notif-menu', role: 'dialog', 'aria-label': 'Bildirimler' },
      h('div', { class: 'notif-head' }, h('strong', null, 'Bildirimler'), markAll),
      list,
      h('a', { class: 'notif-all', href: '#/bildirimler' }, 'Bütün bildirimler'),
    );
    wrap.append(menu);
    const close = popover(btn, menu);
    menu.querySelector('.notif-all')?.addEventListener('click', () => close());
    const fill = (items: NotificationView[]) => {
      if (!items.length) {
        replaceChildren(list, h('div', { class: 'notif-empty' }, 'Bildirim yok. Bir dosyada size düşen bir adım olunca burada görünür.'));
        return;
      }
      replaceChildren(
        list,
        ...items.slice(0, 8).map((n) => {
          const a = h(
            'a',
            { class: `notif ${n.read ? '' : 'is-unread'}`, href: notificationHref(n) },
            h('span', { class: 'notif-dot', 'aria-hidden': 'true' }),
            h('span', { class: 'notif-text' }, h('span', { class: 'notif-title' }, n.title), n.body ? h('span', { class: 'notif-body' }, n.body) : null, h('span', { class: 'notif-time' }, since(n.at))),
          );
          a.addEventListener('click', () => {
            close();
            if (!n.read) void portal.readNotifications([n.id]).then((r) => unread.set(r.unread));
          });
          return a;
        }),
      );
    };
    try {
      const got = await portal.notifications();
      unread.set(got.unread);
      fill(got.items);
    } catch (e) {
      replaceChildren(list, h('div', { class: 'notif-empty' }, e instanceof Error ? e.message : String(e)));
    }
    markAll.addEventListener('click', async () => {
      const got = await portal.readNotifications([], true);
      unread.set(got.unread);
      fill(got.items);
    });
  });
  return wrap;
}

export type Shell = {
  /** Shows a page in `context` under `crumbs`. */
  show: (context: Context, crumbs: Crumb[], body: HTMLElement, nav?: string) => void;
  /** Draws the sidebar again (its figures changed). */
  refresh: () => void;
};

export function mountShell(root: HTMLElement, onLogout: () => void): Shell {
  const sidebar = h('aside', { class: 'sidebar' });
  const crumbsEl = h('nav', { class: 'crumbs', 'aria-label': 'Konum' });
  const topbar = h('header', { class: 'topbar' }, crumbsEl, h('div', { class: 'topbar-actions' }, bell()));
  const content = h('main', { class: 'content', id: 'content' });
  replaceChildren(root, h('div', { class: 'shell' }, sidebar, h('div', { class: 'main' }, topbar, content)));

  let shown: Context | null = null;
  let shownNav: string | undefined;
  const drawSidebar = (c: Context, active?: string) => {
    shown = c;
    shownNav = active;
    const nav = navOf(c);
    const here = active ?? location.hash.split('?')[0];
    replaceChildren(
      sidebar,
      h('a', { class: 'brand', href: homeOf(c) }, brandMark(24), h('span', { class: 'brand-word' }, h('b', null, 'Kent'), 'OS'), h('span', { class: 'brand-product' }, 'Bulut')),
      switcher(c),
      h(
        'nav',
        { class: 'nav', 'aria-label': 'Sayfalar' },
        ...nav.map((item) => {
          const n = counts.get(item.href);
          return h(
            'a',
            { class: `nav-item ${(active ? item.href === active : item.match.test(here)) ? 'is-active' : ''}`, href: item.href, 'aria-current': (active ? item.href === active : item.match.test(here)) ? 'page' : null },
            icon(item.icon, 18),
            h('span', { class: 'nav-label' }, item.label),
            n ? h('span', { class: 'nav-count num', title: 'Size düşen' }, String(n)) : null,
          );
        }),
      ),
      h('div', { class: 'sidebar-fill' }),
      userBlock(onLogout),
    );
  };

  return {
    show(context, crumbs, body, nav) {
      drawSidebar(context, nav);
      replaceChildren(
        crumbsEl,
        ...crumbs.flatMap((c, i) => [
          i ? h('span', { class: 'crumb-sep', 'aria-hidden': 'true' }, icon('chevronRight', 14)) : null,
          c.href && i < crumbs.length - 1 ? h('a', { class: 'crumb', href: c.href }, c.label) : h('span', { class: 'crumb is-current' }, c.label),
        ]),
      );
      replaceChildren(content, body);
      content.scrollTop = 0;
      document.title = `${crumbs[crumbs.length - 1]?.label ?? 'KentOS Bulut'} — KentOS Bulut`;
    },
    refresh() {
      if (shown) drawSidebar(shown, shownNav);
    },
  };
}
