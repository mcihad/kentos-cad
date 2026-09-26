import '../../styles/appmenu.css';
import type { AppContext } from '../../app/context';
import { workspaceName } from '../../app/cloud/session';
import { effectiveWorkspace } from '../../app/workspaces';
import { drawingFontById } from '../../app/appearance';
import { DisposableStore, listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { LINK_TEXT, SAVE_TEXT } from '../statusbar/cloudCells';
import { brandMark } from '../shell/brandButton';
import { recentFileRow } from '../start/recentList';

/**
 * The application menu (DESIGN.md §7.1.1), opened from the KentOS mark in
 * either shell, like AutoCAD's: the file commands down the left, and on the
 * right what the hovered one holds (import and export formats, the cloud)
 * or, at first, the open drawing. Commands come from the registry, so a
 * menu row is exactly what the File menu or the ribbon runs. Loaded on
 * first use (CLAUDE.md §20).
 */

type Pane = 'overview' | 'import' | 'export' | 'cloud';

interface NavItem {
  readonly label: string;
  readonly icon: string;
  readonly sub: (ctx: AppContext) => string;
  /** Runs this command, or shows this pane on the right. */
  readonly command?: string;
  readonly pane?: Pane;
}

const NAV: readonly NavItem[] = [
  { label: 'Yeni', icon: 'fileNew', command: 'file.new', sub: () => 'Mod, koordinat sistemi ve ölçek' },
  { label: 'Aç', icon: 'fileOpen', command: 'file.open', sub: () => '.kcad çizim dosyası' },
  { label: 'Kaydet', icon: 'save', command: 'file.save', sub: (ctx) => saveWhere(ctx) },
  { label: 'Farklı kaydet', icon: 'saveAs', command: 'file.saveAs', sub: () => 'Yeni bir .kcad dosyasına' },
  { label: 'İçe aktar', icon: 'import', pane: 'import', sub: () => 'DXF, NCN, NCZ, SHP, GeoJSON' },
  { label: 'Dışa aktar', icon: 'export', pane: 'export', sub: () => 'DXF, NCN, GeoJSON, PDF' },
  { label: 'Bulut', icon: 'cloud', pane: 'cloud', sub: (ctx) => cloudLine(ctx) },
  { label: 'Yazdır ve pafta', icon: 'print', command: 'file.print', sub: () => 'Pafta düzeni ve çıktı' },
  { label: 'Proje ayarları', icon: 'folder', command: 'file.settings', sub: () => 'Birimler, sistem, mod, yazı tipi' },
];

/** The formats of İçe aktar and Dışa aktar: the command and what it takes or gives. */
const FORMATS: Record<'import' | 'export', readonly { id: string; label: string; detail: string; badge: string }[]> = {
  import: [
    { id: 'file.import.dxf', label: 'DXF', detail: 'AutoCAD R12–2018: katmanlar, bloklar, ölçüler ve taramalar', badge: 'DXF' },
    { id: 'file.import.ncn', label: 'Koordinat listesi', detail: 'Netcad NCN, TXT ya da CSV nokta listesi', badge: 'NCN' },
    { id: 'file.import.ncz', label: 'Netcad çizimi', detail: 'Netcad NCZ dosyası', badge: 'NCZ' },
    { id: 'file.import.shp', label: 'Shapefile', detail: 'ESRI SHP, öznitelikleriyle', badge: 'SHP' },
    { id: 'file.import.geojson', label: 'GeoJSON', detail: 'Coğrafi JSON; öznitelikler dahil', badge: 'JSON' },
  ],
  export: [
    { id: 'file.export.dxf', label: 'DXF', detail: 'AutoCAD 2007: ölçüler DXF ölçüsü, Türkçe yazılar UTF-8', badge: 'DXF' },
    { id: 'file.export.ncn', label: 'Koordinat listesi', detail: 'Noktalar NCN, TXT ya da CSV olarak', badge: 'NCN' },
    { id: 'file.export.geojson', label: 'GeoJSON', detail: 'Coğrafi JSON; öznitelikler dahil', badge: 'JSON' },
    { id: 'file.export.pdf', label: 'PDF pafta', detail: 'Ölçekli pafta çıktısı', badge: 'PDF' },
  ],
};

const PANE_TITLE: Record<Pane, string> = { overview: 'Bu çizim', import: 'İçe aktar', export: 'Dışa aktar', cloud: 'Bulut' };

function saveWhere(ctx: AppContext): string {
  const p = ctx.cloud.project.value;
  if (p) return ctx.cloud.autosaves() ? 'Bulut projesi: kendiliğinden kaydediliyor' : 'Bulut projesi';
  const file = ctx.files.handle?.name;
  return file ? file : 'İlk kayıtta yer sorulur';
}

function cloudLine(ctx: AppContext): string {
  if (ctx.server.state.value !== 'online') return 'Sunucuya ulaşılamıyor';
  const me = ctx.cloud.me.value;
  if (!me) return 'Giriş yapın, projelerinizi açın ve paylaşın';
  const p = ctx.cloud.project.value;
  return p ? `${p.tenantName} › ${p.name}` : `${me.user.displayName} olarak giriş yapıldı`;
}

function ago(iso: string | number): string {
  const ms = typeof iso === 'number' ? iso : Date.parse(iso);
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 60) return 'az önce';
  if (s < 3600) return `${Math.round(s / 60)} dk önce`;
  if (s < 86400) return `${Math.round(s / 3600)} sa önce`;
  return new Date(ms).toLocaleDateString('tr-TR', { day: 'numeric', month: 'short' });
}

const initials = (name: string) =>
  name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]!.toLocaleUpperCase('tr-TR'))
    .join('');

let current: { close(): void } | null = null;

export function openAppMenu(ctx: AppContext, anchor: HTMLElement, keyboard = false): void {
  current?.close();
  const d = new DisposableStore();
  const pane = h('section', { class: 'appmenu__pane', 'aria-live': 'polite' });
  const nav = h('nav', { class: 'appmenu__nav', role: 'menu', 'aria-label': 'Dosya' });
  const foot = h('footer', { class: 'appmenu__foot' });
  const el = h(
    'div',
    { class: 'appmenu', role: 'dialog', 'aria-label': 'KentOS uygulama menüsü' },
    h(
      'header',
      { class: 'appmenu__head' },
      brandMark(28),
      h('div', { class: 'appmenu__title' }, h('span', { class: 'appmenu__product' }, 'KentOS', h('span', { class: 'appmenu__cad' }, ' CAD')), h('span', { class: 'appmenu__doc' })),
    ),
    h('div', { class: 'appmenu__body' }, nav, pane),
    foot,
  );
  const docLine = el.querySelector<HTMLElement>('.appmenu__doc')!;
  let shown: Pane = 'overview';

  const close = () => {
    if (current !== api) return;
    current = null;
    d.dispose();
    el.remove();
    anchor.setAttribute('aria-expanded', 'false');
  };
  const api = { close };
  current = api;

  /** Runs a command after the menu closes (a dialog it opens gets the focus). */
  const run = (id: string, args?: unknown) => {
    close();
    ctx.commands.execute(id, args);
  };

  // ── Left column ────────────────────────────────────────────────────
  const navButtons: HTMLButtonElement[] = [];
  for (const item of NAV) {
    const cmd = item.command ? ctx.commands.get(item.command) : undefined;
    const sub = h('span', { class: 'appmenu__sub' });
    const b = h(
      'button',
      { class: 'appmenu__item', type: 'button', role: 'menuitem', dataset: { pane: item.pane ?? '', command: item.command ?? '' }, 'aria-haspopup': item.pane ? 'true' : null },
      h('span', { class: 'appmenu__icon' }, icon(item.icon, 22)),
      h('span', { class: 'appmenu__text' }, h('span', { class: 'appmenu__label' }, item.label), sub),
      item.pane ? h('span', { class: 'appmenu__arrow' }, icon('chevronRight', 14)) : cmd ? h('span', { class: 'appmenu__key' }, ctx.keymap.chordFor(item.command!) ?? '') : null,
    );
    const sync = () => {
      sub.textContent = item.sub(ctx);
      if (item.command) b.disabled = !ctx.commands.isEnabled(item.command);
      if (cmd?.pending) b.dataset.pending = '';
    };
    sync();
    d.add(watchAll([ctx.cloud.me, ctx.cloud.project, ctx.server.state, ...(cmd?.watch ?? [])], sync));
    if (item.pane) {
      const show = () => showPane(item.pane!);
      d.add(listen(b, 'pointerenter', show));
      d.add(listen(b, 'focus', show));
      d.add(listen(b, 'click', show));
    } else {
      // Hovering a command brings the overview back, so the right side never describes something else.
      d.add(listen(b, 'pointerenter', () => showPane('overview')));
      d.add(listen(b, 'click', () => run(item.command!)));
    }
    navButtons.push(b);
    nav.append(b);
  }

  // ── Footer: the app's own settings ─────────────────────────────────
  for (const [id, label, ic] of [
    ['tools.options', 'Uygulama ayarları', 'settings'],
    ['help.shortcuts', 'Kısayollar', 'keyboard'],
    ['help.about', 'Hakkında', 'info'],
  ] as const) {
    const b = h('button', { class: 'appmenu__footbtn', type: 'button', dataset: { command: id } }, icon(ic, 16), h('span', null, label));
    d.add(listen(b, 'click', () => run(id)));
    foot.append(b);
  }
  foot.append(h('span', { class: 'appmenu__version' }, 'KentOS CAD'));

  // ── Right pane ─────────────────────────────────────────────────────
  const paneHead = (title: string, lead?: string): Child => h('div', { class: 'appmenu__panehead' }, h('h3', null, title), lead ? h('p', null, lead) : null);

  const formatRow = (f: (typeof FORMATS)['import'][number]) => {
    const cmd = ctx.commands.get(f.id);
    const pending = !!cmd?.pending;
    const b = h(
      'button',
      { class: 'appmenu__row', type: 'button', role: 'menuitem', dataset: { command: f.id }, disabled: !ctx.commands.isEnabled(f.id) },
      h('span', { class: 'appmenu__badge' }, f.badge),
      h('span', { class: 'appmenu__text' }, h('span', { class: 'appmenu__label' }, f.label), h('span', { class: 'appmenu__sub' }, pending ? 'Geliştirme aşamasında' : f.detail)),
      pending ? h('span', { class: 'appmenu__soon' }, 'Yakında') : h('span', { class: 'appmenu__key' }, ctx.keymap.chordFor(f.id) ?? ''),
    );
    if (pending) b.dataset.pending = '';
    b.addEventListener('click', () => run(f.id));
    return b;
  };

  const overview = (): Child[] => {
    const doc = ctx.doc;
    const s = doc.settings;
    const cloud = ctx.cloud.project.value;
    const where = cloud ? `Bulut · ${cloud.tenantName}` : ctx.files.handle ? `Dosya · ${ctx.files.handle.name}` : 'Bir dosyaya bağlı değil; ilk kayıtta yer sorulur';
    const chip = (ic: string, text: string) => h('span', { class: 'appmenu__chip' }, icon(ic, 13), text);
    const save = h('button', { class: 'btn btn--primary btn--small', type: 'button', dataset: { command: 'file.save' } }, 'Kaydet');
    save.addEventListener('click', () => run('file.save'));
    const mode = effectiveWorkspace(s.workspace.value);
    return [
      paneHead('Bu çizim'),
      h(
        'div',
        { class: 'appmenu__card appmenu__project' },
        h('div', { class: 'appmenu__projname' }, doc.name.value),
        h('div', { class: 'appmenu__projwhere' }, where),
        h(
          'div',
          { class: 'appmenu__chips' },
          chip('crs', s.crs.value.name),
          chip(mode.icon, mode.label),
          chip('sheet', `1:${s.plotScale.value.toLocaleString('tr-TR')}`),
          chip('text', drawingFontById(s.drawingFont.value).label),
        ),
        h(
          'div',
          { class: 'appmenu__stats' },
          h('span', null, h('b', { class: 'num' }, doc.size.toLocaleString('tr-TR')), ' nesne'),
          h('span', null, h('b', { class: 'num' }, String(doc.layers.leaves().length)), ' katman'),
        ),
        doc.dirty.value && !ctx.cloud.autosaves()
          ? h('div', { class: 'appmenu__dirty' }, h('span', { class: 'appmenu__dot' }), h('span', null, 'Kaydedilmemiş değişiklikler var'), save)
          : h('div', { class: 'appmenu__clean' }, icon('check', 14), cloud && ctx.cloud.autosaves() ? 'Değişiklikler buluta kendiliğinden kaydediliyor' : ctx.files.handle ? 'Tüm değişiklikler kaydedildi' : 'Kaydedilmemiş değişiklik yok'),
      ),
      h(
        'div',
        { class: 'appmenu__tiles' },
        ...(
          [
            ['file.new', 'fileNew', 'Yeni proje'],
            ['file.open', 'fileOpen', 'Dosya aç'],
            ['cloud.open', 'cloud', 'Bulut projesi'],
            ['file.import.dxf', 'import', 'DXF içe aktar'],
          ] as const
        ).map(([id, ic, label]) => {
          const t = h('button', { class: 'appmenu__tile', type: 'button', dataset: { command: id } }, icon(ic, 22), h('span', null, label));
          t.addEventListener('click', () => run(id));
          return t;
        }),
      ),
      ...(ctx.files.recent.list.value.length
        ? [h('div', { class: 'appmenu__section' }, 'Son dosyalar'), h('div', { class: 'appmenu__recentfiles', role: 'list' }, ...ctx.files.recent.list.value.slice(0, 5).map((f) => recentFileRow(ctx, f, close)))]
        : []),
    ];
  };

  const cloudPane = (): Child[] => {
    const server = ctx.server.state.value;
    const me = ctx.cloud.me.value;
    const art = h('div', { class: 'appmenu__cloudart', 'aria-hidden': 'true' }, cloudArt());
    const action = (id: string, label: string, ic: string, primary = false) => {
      const b = h('button', { class: `appmenu__cta${primary ? ' appmenu__cta--primary' : ''}`, type: 'button', dataset: { command: id }, disabled: !ctx.commands.isEnabled(id) }, icon(ic, 16), h('span', null, label));
      b.addEventListener('click', () => run(id));
      return b;
    };
    if (server !== 'online') {
      return [
        paneHead('Bulut'),
        h(
          'div',
          { class: 'appmenu__cloud appmenu__cloud--off' },
          art,
          h('h4', null, server === 'incompatible' ? 'Sunucu bu sürümle uyumsuz' : server === 'checking' ? 'Sunucu denetleniyor…' : 'Bulut sunucusuna ulaşılamıyor'),
          h('p', null, 'Çizim bu cihazda çalışmaya devam eder; yerel dosyaya kaydedebilirsiniz. Sunucu dönünce projeleriniz burada görünür.'),
          h('div', { class: 'appmenu__ctas' }, action('server.check', 'Yeniden dene', 'server', true)),
        ),
      ];
    }
    if (!me) {
      return [
        paneHead('Bulut'),
        h(
          'div',
          { class: 'appmenu__cloud' },
          art,
          h('h4', null, 'Projeleriniz her yerde'),
          h('p', null, 'Giriş yapın: kurumunuzun projelerini açın, çiziminizi buluta yükleyin. Değişiklikler kendiliğinden kaydedilir, ekip aynı projeyi canlı görür.'),
          h('div', { class: 'appmenu__ctas' }, action('cloud.signIn', 'Giriş yap', 'signIn', true)),
        ),
      ];
    }
    const p = ctx.cloud.project.value;
    const sync = ctx.cloud.sync.value;
    const memberships = me.memberships.filter((m) => m.active);
    const account = h(
      'div',
      { class: 'appmenu__card appmenu__account' },
      h('span', { class: 'appmenu__avatar' }, initials(me.user.displayName)),
      h('div', { class: 'appmenu__who' }, h('b', null, me.user.displayName), h('span', null, memberships.map((m) => workspaceName(m.tenantKind, m.tenantName, true)).join(' · ') || 'Kurum üyeliği yok')),
      (() => {
        const out = h('button', { class: 'appmenu__link', type: 'button', dataset: { command: 'cloud.signOut' } }, 'Çıkış');
        out.addEventListener('click', () => run('cloud.signOut'));
        return out;
      })(),
    );
    const open = p
      ? h(
          'div',
          { class: 'appmenu__card appmenu__open' },
          h('span', { class: 'appmenu__lamp', dataset: { state: sync?.state.value ?? '' } }),
          h(
            'div',
            { class: 'appmenu__who' },
            h('b', null, p.name),
            h('span', null, `${p.tenantName} · ${sync ? SAVE_TEXT[sync.state.value](sync.state.value === 'conflict' ? sync.conflicts.value.length : sync.pending.value) : ''}${LINK_TEXT[ctx.cloud.link.value] ? ` · ${LINK_TEXT[ctx.cloud.link.value]}` : ''}`),
          ),
        )
      : null;
    const recent = h('div', { class: 'appmenu__recent' }, h('div', { class: 'appmenu__skeleton' }), h('div', { class: 'appmenu__skeleton' }), h('div', { class: 'appmenu__skeleton' }));
    const tenant = memberships.find((m) => m.seat && m.tenantId === p?.tenantId) ?? memberships.find((m) => m.seat);
    if (tenant) {
      void ctx.cloud
        .projects(tenant.tenantId)
        .then(({ projects }) => {
          if (!recent.isConnected) return;
          const top = [...projects].sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt)).slice(0, 5);
          replaceChildren(
            recent,
            top.length
              ? top.map((x) => {
                  const r = h(
                    'button',
                    { class: 'appmenu__row appmenu__row--project', type: 'button', role: 'menuitem', dataset: { project: x.id } },
                    h('span', { class: 'appmenu__projicon' }, icon('cloud', 16)),
                    h('span', { class: 'appmenu__text' }, h('span', { class: 'appmenu__label' }, x.name), h('span', { class: 'appmenu__sub' }, `${workspaceName(tenant.tenantKind, tenant.tenantName, true)} · ${ago(x.updatedAt)}`)),
                    x.id === p?.projectId ? h('span', { class: 'appmenu__soon appmenu__soon--open' }, 'Açık') : null,
                  );
                  r.addEventListener('click', () => run('cloud.open', { tenantId: tenant.tenantId, projectId: x.id }));
                  return r;
                })
              : h('p', { class: 'appmenu__empty' }, tenant.tenantKind === 'personal' ? 'Kişisel alanınızda henüz proje yok. Açık çizimi “Buluta yükle” ile gönderin.' : 'Bu kurumda size açık bir proje yok. Açık çizimi “Buluta yükle” ile gönderin.'),
          );
        })
        .catch(() => recent.isConnected && replaceChildren(recent, h('p', { class: 'appmenu__empty' }, 'Projeler okunamadı.')));
    } else replaceChildren(recent, h('p', { class: 'appmenu__empty' }, 'Koltuk atanmış bir kurum üyeliğiniz yok; kurum yöneticinize başvurun.'));
    return [
      paneHead('Bulut'),
      account,
      open,
      h(
        'div',
        { class: 'appmenu__ctas' },
        action('cloud.open', 'Proje aç', 'cloud', true),
        action('cloud.upload', 'Buluta yükle', 'cloudUpload'),
        ...(p ? [action('cloud.share', 'Paylaş', 'share'), action('cloud.rename', 'Yeniden adlandır', 'edit'), action('cloud.delete', 'Sil', 'trash')] : []),
      ),
      h('div', { class: 'appmenu__section' }, 'Son projeler'),
      recent,
    ];
  };

  const showPane = (which: Pane) => {
    if (which === shown && pane.childElementCount) return;
    shown = which;
    for (const b of navButtons) b.toggleAttribute('data-current', b.dataset.pane === which && which !== 'overview');
    pane.dataset.pane = which;
    pane.setAttribute('aria-label', PANE_TITLE[which]);
    replaceChildren(
      pane,
      ...(which === 'overview'
        ? overview()
        : which === 'cloud'
          ? cloudPane()
          : [paneHead(PANE_TITLE[which], which === 'import' ? 'Dosyadaki nesneler tek geri alma adımıyla eklenir.' : 'Çizim kaydedilmiş sayılmaz; dışa aktarma çizime dokunmaz.'), h('div', { class: 'appmenu__rows' }, ...FORMATS[which].map(formatRow))]),
    );
  };
  const refresh = () => {
    docLine.textContent = `${ctx.doc.name.value}${ctx.doc.dirty.value ? ' •' : ''}`;
    const keep = shown;
    shown = 'overview';
    pane.replaceChildren();
    showPane(keep);
  };
  d.add(watchAll([ctx.doc.name, ctx.doc.dirty, ctx.cloud.me, ctx.cloud.project, ctx.server.state, ctx.cloud.link, ctx.files.recent.list], refresh));
  refresh();

  // ── Place, focus, dismiss ──────────────────────────────────────────
  document.body.append(el);
  const r = anchor.getBoundingClientRect();
  el.style.left = `${Math.max(8, Math.round(r.left))}px`;
  el.style.top = `${Math.round(r.bottom + 4)}px`;
  anchor.setAttribute('aria-expanded', 'true');
  if (keyboard) navButtons[0]?.focus();
  else el.focus?.();

  d.add(
    listen<PointerEvent>(
      document,
      'pointerdown',
      (e) => {
        if (el.contains(e.target as Node)) return;
        // The mark itself toggles: a press on it closes.
        if (anchor.contains(e.target as Node)) e.preventDefault();
        close();
      },
      { capture: true },
    ),
  );
  d.add(listen(window, 'resize', close));
  d.add(ctx.commands.events.on('executed', () => close()));
  d.add(
    listen<KeyboardEvent>(el, 'keydown', (e) => {
      const items = [...navButtons].filter((b) => !b.disabled);
      const rows = [...pane.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
      const inNav = navButtons.includes(document.activeElement as HTMLButtonElement);
      const inPane = pane.contains(document.activeElement);
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        close();
        anchor.focus();
      } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const list = inPane ? rows : items;
        const at = list.indexOf(document.activeElement as HTMLButtonElement);
        list[(at + (e.key === 'ArrowDown' ? 1 : -1) + list.length) % list.length]?.focus();
      } else if (e.key === 'ArrowRight' && inNav) {
        e.preventDefault();
        rows[0]?.focus();
      } else if (e.key === 'ArrowLeft' && inPane) {
        e.preventDefault();
        (navButtons.find((b) => b.dataset.pane === shown) ?? navButtons[0]).focus();
      }
    }),
  );
}

/** A soft cloud over a sheet with a synced drawing: the cloud pane's picture. */
function cloudArt(): SVGSVGElement {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 160 96');
  svg.setAttribute('width', '160');
  svg.setAttribute('height', '96');
  svg.innerHTML =
    '<defs><linearGradient id="am-cloud" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--c-accent)" stop-opacity=".95"/><stop offset="1" stop-color="var(--c-accent)" stop-opacity=".55"/></linearGradient>' +
    '<linearGradient id="am-sheet" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--c-panel-head)"/><stop offset="1" stop-color="var(--c-field)"/></linearGradient></defs>' +
    '<rect x="34" y="40" width="92" height="50" rx="6" fill="url(#am-sheet)" stroke="var(--c-line-strong)"/>' +
    '<path d="M44 78 60 60l14 10 18-18 22 16" fill="none" stroke="var(--c-text-3)" stroke-width="1.6" stroke-linejoin="round"/>' +
    '<circle cx="60" cy="60" r="2.4" fill="var(--c-text-2)"/><circle cx="92" cy="52" r="2.4" fill="var(--c-text-2)"/>' +
    '<path d="M58 40a16 16 0 0 1 30-8 13 13 0 0 1 22 9 11 11 0 0 1-1 22H60a12 12 0 0 1-2-23Z" fill="url(#am-cloud)"/>' +
    '<path d="M80 46v-12m-5 5 5-5 5 5" fill="none" stroke="var(--c-accent-ink)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>';
  return svg;
}
