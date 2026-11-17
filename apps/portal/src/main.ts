// KentOS Bulut, the cloud portal (docs/adr/9000): the platform's administration, the municipalities' work and the
// professionals' practice in one web app. It signs the person in, learns who they are and the catalog's names, and
// draws the page the address names in the place (context) it belongs to.
import './styles/portal.css';
import { ApiError } from './api/client';
import { organization, platform, portal, professional } from './api/portal';
import { Routes, go } from './app/router';
import { type Context, catalog, contexts, homeOf, me, orgBySlug, unread } from './app/state';
import { h, replaceChildren } from './lib/dom';
import { adminPages } from './pages/admin/routes';
import { loginPage, signupPage } from './pages/auth';
import { accountPage } from './pages/account';
import { notificationsPage } from './pages/notifications';
import { projectPage } from './pages/project';
import { orgPages } from './pages/org/routes';
import type { PageFn } from './pages/page';
import { proPages } from './pages/pro/routes';
import { type Shell, counts, mountShell, setTheme } from './shell/shell';
import { empty, loading, notice } from './ui/kit';
import { toast } from './ui/toast';

const root = document.getElementById('app') as HTMLElement;

const routes = new Routes<PageFn>();
for (const [pattern, page] of [...adminPages, ...orgPages, ...proPages]) routes.add(pattern, page);
routes.add('bildirimler', notificationsPage);
routes.add('hesabim', accountPage);
routes.add('proje/:tenant/:project', projectPage);

let shell: Shell | null = null;
let lastContext: Context | null = null;
let turn = 0;

function signedOut(): void {
  me.set(null);
  shell = null;
  const back = location.hash && !/^#\/(giris|kayit)/.test(location.hash) ? location.hash : '';
  go(back ? `#/giris?sonra=${encodeURIComponent(back)}` : '#/giris');
}

async function logout(): Promise<void> {
  try {
    await portal.logout();
  } finally {
    me.set(null);
    shell = null;
    go('#/giris');
  }
}

/** The place an address belongs to, or why the person may not be there. */
function contextOf(hash: string): Context | string {
  const m = me.value;
  if (!m) return 'Oturum yok.';
  const [, first, second] = hash.split('?')[0].split('/');
  if (first === 'sistem') return m.platformAdmin ? { kind: 'admin' } : 'Bu sayfa yalnız sistem yöneticilerine açık.';
  if (first === 'kurum') {
    const org = orgBySlug(second ?? '');
    if (!org) return 'Bu kurumun üyesi değilsiniz ya da böyle bir kurum yok.';
    if (!org.usable) return `“${org.tenantName}” kurumundaki üyeliğiniz şu an kullanılamıyor (koltuk ya da üyelik kapalı); kurum yöneticinize başvurun.`;
    return { kind: 'org', org };
  }
  if (first === 'mesleki') return m.professional ? { kind: 'pro' } : 'Hesabınız bir meslek mensubunun değil.';
  return lastContext ?? contexts(m)[0] ?? 'Bu hesabın portalda bir çalışma yeri yok.';
}

/** The sidebar's figures for the place: what waits for the person there. */
async function refreshCounts(c: Context): Promise<void> {
  try {
    if (c.kind === 'admin') {
      const s = await platform.summary();
      counts.set('#/sistem/meslek-mensuplari', s.pendingProfessionals);
    } else if (c.kind === 'org' && c.org.category === 'municipality') {
      const s = await organization.summary(c.org.tenantId);
      const of = (direction: string, statuses: string[]) =>
        s.counts.filter((x) => x.direction === direction && statuses.includes(x.status)).reduce((n, x) => n + x.count, 0);
      counts.set(`#/kurum/${c.org.tenantSlug}/basvurular`, of('application', ['submitted', 'in_review']));
      counts.set(`#/kurum/${c.org.tenantSlug}/aktarimlar`, of('transfer', ['delivered']));
    } else if (c.kind === 'pro') {
      const s = await professional.summary();
      const of = (direction: string, statuses: string[]) =>
        s.counts.filter((x) => x.direction === direction && statuses.includes(x.status)).reduce((n, x) => n + x.count, 0);
      counts.set('#/mesleki/basvurular', of('application', ['changes_requested']));
      counts.set('#/mesleki/isler', of('transfer', ['sent', 'accepted', 'revision_requested']));
    }
    shell?.refresh();
  } catch {
    // Figures are a convenience: the pages say the same.
  }
}

async function render(): Promise<void> {
  const mine = ++turn;
  const hash = location.hash || '#/';
  if (/^#\/(giris|kayit)/.test(hash)) {
    if (me.value && /^#\/giris/.test(hash)) {
      go(homeOf(contexts(me.value)[0] ?? { kind: 'pro' }));
      return;
    }
    shell = null;
    const page = /^#\/kayit/.test(hash) ? await signupPage() : await loginPage(new URLSearchParams(hash.split('?')[1] ?? ''));
    if (mine === turn) replaceChildren(root, page);
    return;
  }
  if (!me.value) {
    signedOut();
    return;
  }
  if (hash === '#/' || hash === '#') {
    const first = contexts(me.value)[0];
    if (first) go(homeOf(first));
    else replaceChildren(root, h('div', { class: 'auth' }, empty('lock', 'Çalışma yeri yok', 'Hesabınız henüz bir kuruma üye değil ve meslek mensubu kaydı da yok. Sistem yöneticisine başvurun.')));
    return;
  }
  shell ??= mountShell(root, () => void logout());
  const context = contextOf(hash);
  const match = routes.match(hash);
  if (typeof context === 'string' || !match) {
    const c = lastContext ?? contexts(me.value)[0];
    if (c) shell.show(c, [{ label: 'Bulunamadı' }], h('div', { class: 'page' }, notice('danger', typeof context === 'string' ? context : 'Bu adreste bir sayfa yok.')));
    return;
  }
  const changed = !lastContext || lastContext.kind !== context.kind || (context.kind === 'org' && lastContext.kind === 'org' && lastContext.org.tenantId !== context.org.tenantId);
  lastContext = context;
  shell.show(context, [{ label: '…' }], loading());
  if (changed) void refreshCounts(context);
  try {
    const page = await match.value({ params: match.params, query: match.query, reload: () => void render() });
    if (mine !== turn) return;
    shell.show(context, page.crumbs, page.body, page.nav);
    if (!changed) void refreshCounts(context);
  } catch (e) {
    if (mine !== turn) return;
    if (e instanceof ApiError && e.status === 401) {
      signedOut();
      return;
    }
    const message = e instanceof Error ? e.message : String(e);
    shell.show(context, [{ label: 'Hata' }], h('div', { class: 'page' }, notice('danger', 'Sayfa açılamadı', message, h('button', { class: 'btn btn-secondary', type: 'button', onClick: () => void render() }, 'Yeniden dene'))));
  }
}

async function boot(): Promise<void> {
  try {
    const saved = localStorage.getItem('kentos-portal-theme');
    if (saved === 'dark' || saved === 'light') setTheme(saved);
  } catch {
    // No storage: the light theme.
  }
  try {
    catalog.set(await portal.catalog());
  } catch {
    // Without the catalog the codes show as they are; the pages still work.
  }
  try {
    const m = await portal.me();
    me.set(m);
    unread.set(m.unread);
  } catch (e) {
    if (!(e instanceof ApiError && e.status === 401)) {
      replaceChildren(root, h('div', { class: 'auth' }, notice('danger', 'Sunucuya ulaşılamıyor', e instanceof Error ? e.message : String(e))));
      return;
    }
  }
  window.addEventListener('hashchange', () => void render());
  // Notifications as they happen (docs/adr/9000 §23): the server's stream pushes the unread count; the browser opens
  // it again by itself when it drops. Asking every five minutes stays as the fallback.
  let live: EventSource | null = null;
  const follow = () => {
    if (!me.value) {
      live?.close();
      live = null;
      return;
    }
    if (live || typeof EventSource === 'undefined') return;
    live = new EventSource('/v1/me/notifications/stream');
    live.addEventListener('unread', (e) => {
      const n = Number(JSON.parse((e as MessageEvent).data).unread);
      if (Number.isFinite(n)) {
        if (n > unread.value) toast('Yeni bildiriminiz var.', 'info');
        unread.set(n);
      }
    });
  };
  me.subscribe(follow, true);
  setInterval(() => {
    if (!me.value) return;
    void portal
      .notifications(true)
      .then((n) => unread.set(n.unread))
      .catch(() => undefined);
  }, 300_000);
  await render();
}

void boot();
