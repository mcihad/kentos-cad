// Reference pictures of the cloud windows for porting (not a test), against the real kentosd on a throwaway
// database (examples/e2e_database: every migration of this build and the development accounts; kentos_cad and
// kentos are not touched). Seeds projects of every kind, then at each size signs in as ayse and pictures each
// scene in the dark and the light theme: the catalog's lists, details, tabs, selects and empty search; the history
// tab and its forms; the share dialog with the find box, invitations and removing access; the project forms and
// questions; renaming the open project; a file project's conflict; a file project's revisions (someone else's
// revision heard, over unsaved work, the questions, the conflict, the history's marks, the log line, offline); the
// access-lost notice; the status bar's cloud cells and the server cell's account menu; a block definition's
// conflict and the Bloklar panel after it (docs/adr/0144 §5); an invitation's page; Kaynaklar's KentOS projects, one
// opened and one of its layers taken (docs/adr/0199 §7).
// Pictures go to scripts/e2e/out/shots/cloud/<scene>-<theme>-<width>.png.
//
//   cargo build -q -p kentos-api --bin kentosd --example e2e_database
//   node scripts/e2e/cloud-shots.mjs [--only a,b] [--sizes 1440x900,1100x650]
//
// A native <select>'s list is not drawn in a headless screenshot: an "…-open" scene shows the same options in
// a list box put where the list would open.
import { spawn, spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdirSync, readFileSync } from 'node:fs';
import net from 'node:net';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const args = process.argv.slice(2);
const opt = (name) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1].split(',') : null);
const only = opt('only');
const sizes = (opt('sizes') ?? ['1440x900', '1100x650']).map((s) => s.split('x').map(Number));
const THEMES = ['dark', 'light'];

const ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
const SERVER = process.env.KENTOS_E2E_SERVER ?? `${ROOT}target/debug`;
const DIR = join(OUT, 'shots', 'cloud');
mkdirSync(DIR, { recursive: true });

const env = Object.fromEntries(
  readFileSync(`${ROOT}.env.local`, 'utf8')
    .split('\n')
    .filter((l) => l && !l.startsWith('#') && l.includes('='))
    .map((l) => [l.slice(0, l.indexOf('=')), l.slice(l.indexOf('=') + 1)]),
);
if (!env.KENTOS_DEV_PASSWORD) throw new Error('.env.local içinde KENTOS_DEV_PASSWORD yok: önce `pnpm db:setup` çalıştırın.');
const PASSWORD = env.KENTOS_DEV_PASSWORD;

// ── The throwaway database, the server and the app ───────────────────────────────────────────────────────────
const scratch = spawn(`${SERVER}/examples/e2e_database`, [], { cwd: ROOT, env: { ...process.env, KENTOS_DEV_PASSWORD: PASSWORD }, stdio: ['pipe', 'pipe', 'inherit'] });
const dbName = await new Promise((resolve, reject) => {
  let out = '';
  scratch.stdout.on('data', (d) => {
    out += d;
    if (out.includes('\n')) resolve(out.slice(0, out.indexOf('\n')).trim());
  });
  scratch.once('exit', (code) => reject(new Error(`geçici veritabanı açılamadı (${code})`)));
});
if (!/^kentos_cad_test_[0-9a-z_]+$/.test(dbName)) throw new Error('geçici veritabanının adı beklenmedik');
const scratchUrl = (() => {
  const u = new URL(env.KENTOS_DATABASE_URL);
  u.pathname = `/${dbName}`;
  return u.toString();
})();
// The admin CLI (a second organisation, a guest account) runs only on the throwaway database's owner address.
const ownerUrl = env.KENTOS_DATABASE_OWNER_URL
  ? (() => {
      const u = new URL(env.KENTOS_DATABASE_OWNER_URL);
      u.pathname = `/${dbName}`;
      return u.toString();
    })()
  : null;
console.log(`geçici veritabanı: ${dbName} (kentos_cad'e dokunulmuyor)`);

const freePort = () => new Promise((resolve) => { const s = net.createServer().listen(0, '127.0.0.1', () => { const { port } = s.address(); s.close(() => resolve(port)); }); });
const apiPort = await freePort();
process.env.KENTOS_API_PORT = String(apiPort);
const vite = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await vite.listen();
const url = vite.resolvedUrls.local[0];
const api = spawn(`${SERVER}/kentosd`, ['serve'], {
  cwd: ROOT,
  env: { ...process.env, KENTOS_API_PORT: String(apiPort), KENTOS_PUBLIC_URL: url.replace(/\/$/, ''), KENTOS_LOG: 'warn', KENTOS_DATABASE_URL: scratchUrl, ...(ownerUrl ? { KENTOS_DATABASE_OWNER_URL: ownerUrl } : {}) },
  stdio: ['ignore', 'ignore', 'inherit'],
});
for (let i = 0; ; i++) {
  try {
    if ((await fetch(`http://127.0.0.1:${apiPort}/v1/health`)).ok) break;
  } catch {}
  if (i > 100) throw new Error('kentosd başlamadı');
  await sleep(100);
}

const cleanup = async () => {
  await vite.close().catch(() => {});
  if (api.exitCode === null) {
    api.kill('SIGINT');
    await new Promise((r) => api.once('exit', r));
  }
  if (scratch.exitCode === null) {
    scratch.stdin.end();
    await new Promise((r) => scratch.once('exit', r));
  }
};

// ── Seeding through the API, as the accounts themselves ─────────────────────────────────────────────────────
const client = () => ({
  cookie: '',
  async call(method, path, body) {
    const res = await fetch(`http://127.0.0.1:${apiPort}${path}`, {
      method,
      headers: { 'content-type': 'application/json', 'x-kentos-client': 'web', ...(this.cookie ? { cookie: this.cookie } : {}) },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const set = res.headers.getSetCookie?.()[0];
    if (set) this.cookie = set.split(';')[0];
    const out = { status: res.status, body: res.status === 204 ? null : await res.json().catch(() => null) };
    if (res.status >= 400) throw new Error(`${method} ${path}: ${res.status} ${JSON.stringify(out.body)?.slice(0, 300)}`);
    return out;
  },
  async raw(method, path, bytes) {
    const res = await fetch(`http://127.0.0.1:${apiPort}${path}`, {
      method,
      headers: { 'x-kentos-client': 'web', cookie: this.cookie, ...(bytes ? { 'content-type': 'application/octet-stream' } : {}) },
      body: bytes,
    });
    if (res.status >= 400) throw new Error(`${method} ${path}: ${res.status}`);
    return new Uint8Array(await res.arrayBuffer());
  },
  run(p, commandName, input = {}, expected = {}) {
    return this.call('POST', `/v1/tenants/${p.tenantId}/projects/${p.projectId}/commands`, {
      commandName, version: 1, tenantId: p.tenantId, projectId: p.projectId, requestId: `shots-${randomUUID()}`, idempotencyKey: randomUUID(), expectedVersions: expected, input,
    });
  },
});
const ayse = client();
const mehmet = client();
const zeynep = client();
const login = async (c, name) => (await c.call('POST', '/v1/auth/login', { login: name, password: PASSWORD })).body;

const GUEST = { login: 'misafir', name: 'Misafir Kişi', email: 'misafir@misafir.example' };
const cli = (...a) => spawnSync(`${SERVER}/kentosd`, a, { cwd: ROOT, env: { ...process.env, KENTOS_DEV_PASSWORD: PASSWORD, KENTOS_DATABASE_URL: scratchUrl, KENTOS_DATABASE_OWNER_URL: ownerUrl }, encoding: 'utf8' });

const O = { x: 486500, y: 4420200 };
const at = (dx, dy) => ({ x: O.x + dx, y: O.y + dy });
const LAYERS = [
  { id: 'parsel', name: 'Parsel', type: 'layer', visible: true, locked: false, expanded: true, style: { color: '#E5484D', lineType: 'continuous', lineWeight: 0.35, fill: '#E5484D1F' }, children: [] },
  { id: 'yol', name: 'Yol', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'fg', lineType: 'continuous', lineWeight: 0.5 }, children: [] },
  { id: 'nokta', name: 'Poligon noktaları', type: 'layer', visible: true, locked: false, expanded: true, style: { color: '#3E9BF0', lineType: 'continuous', lineWeight: 0.25, point: { symbol: 'cross', size: 7 } }, children: [] },
];
const base = (extra = {}) => ({
  // A CBS project: opening it asks no project type (docs/adr/0165).
  settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'gis' },
  origin: O,
  layers: LAYERS,
  activeLayer: 'parsel',
  styles: { items: [], categories: [] },
  ...extra,
});
const parcel = (x0, x1, attrs) => ({ kind: 'polygon', layerId: 'parsel', label: attrs.Parsel, attrs, pts: [at(x0, 0), at(x1, 0), at(x1, 30), at(x0, 30)] });
const OBJECTS = [
  parcel(0, 20, { Ada: '1246', Parsel: '1', Nitelik: 'Arsa' }),
  parcel(20, 45, { Ada: '1246', Parsel: '2', Nitelik: 'Arsa' }),
  parcel(45, 60, { Ada: '1246', Parsel: '3', Nitelik: 'Tarla' }),
  { kind: 'line', layerId: 'yol', attrs: { Tür: 'Yol aksı' }, a: at(-10, -8), b: at(70, -8) },
  { kind: 'point', layerId: 'nokta', label: 'P.101', attrs: { Nokta: 'P.101' }, p: at(0, 30) },
  { kind: 'point', layerId: 'nokta', label: 'P.102', attrs: { Nokta: 'P.102' }, p: at(60, 30) },
];
const creates = (list, from = 1) => list.map((entity, i) => ({ op: 'create', id: randomUUID(), entity: { ...entity, id: from + i } }));
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');

/** A new project in `tenantId`: the answer's ref. */
async function project(c, tenantId, body) {
  const made = await c.call('POST', `/v1/tenants/${tenantId}/projects`, { ...base(), ...body });
  return { tenantId, projectId: made.body.id, name: body.name };
}
/** A file project's revision from `bytes`, over `from` ('0' for the first). */
async function commitFile(c, p, bytes, from) {
  const pb = `/v1/tenants/${p.tenantId}/projects/${p.projectId}`;
  const up = await c.call('POST', `${pb}/uploads`, { size: bytes.length, sha256: sha(bytes) });
  await c.raw('PUT', `${pb}/uploads/${up.body.id}`, bytes);
  return c.run(p, 'project.file.commit', { uploadId: up.body.id }, { '@file': from });
}

const ayseMe = await login(ayse, 'ayse');
const mehmetMe = await login(mehmet, 'mehmet');
await login(zeynep, 'zeynep');
const orgOf = (me) => me.memberships.find((m) => m.tenantKind === 'organization')?.tenantId;
const personalOf = (me) => me.memberships.find((m) => m.tenantKind === 'personal')?.tenantId;
const ORG = orgOf(ayseMe);

// A second organisation with a seat for her: the catalog's Kurum list then has a choice.
let org2 = null;
if (ownerUrl) {
  cli('tenant', 'add', '--slug', 'ikinci-buro', '--name', 'İkinci Harita Bürosu', '--seats', '3');
  cli('member', 'add', '--tenant', 'ikinci-buro', '--user', 'ayse', '--role', 'project_manager', '--seat');
  cli('user', 'add', '--login', GUEST.login, '--name', GUEST.name, '--email', GUEST.email, '--password-env', 'KENTOS_DEV_PASSWORD');
  const again = await login(ayse, 'ayse');
  org2 = again.memberships.find((m) => m.tenantKind === 'organization' && m.tenantId !== ORG)?.tenantId ?? null;
} else console.log('KENTOS_DATABASE_OWNER_URL yok: ikinci kurum ve misafir hesabı açılmadı (Kurum seçimi ve davet sayfası resimleri eksik kalır).');

const P = {};
P.survey = await project(ayse, ORG, {
  name: 'Ada 1246 ölçü projesi',
  projectType: 'subdivision',
  tags: ['Kadıköy', 'ifraz', 'ölçü'],
  description: '1246 ada 1–3 parsellerin ifraz ölçüleri; belediye teslimine hazırlanıyor.',
});
await ayse.run(P.survey, 'project.changes', { features: creates(OBJECTS) });
await ayse.run(P.survey, 'project.checkpoint.create', { name: 'İlk ölçüm', note: 'Arazi ölçümünün ilk hâli.' });
await ayse.run(P.survey, 'project.checkpoint.create', { name: 'Belediyeye teslim', note: 'İmar müdürlüğüne gönderilen sürüm.' });
P.sheet = await project(ayse, ORG, { name: 'Kadastro paftası 2026', projectType: 'cad', tags: ['pafta'], description: 'Paftanın dosya olarak saklanan çizimi.', storage: 'file' });
const snap1 = await ayse.raw('GET', `/v1/tenants/${P.survey.tenantId}/projects/${P.survey.projectId}/snapshot`);
await commitFile(ayse, P.sheet, snap1, '0');
await ayse.run(P.survey, 'project.changes', { features: creates([{ kind: 'point', layerId: 'nokta', label: 'P.103', attrs: { Nokta: 'P.103' }, p: at(30, 45) }], 50) });
const snap2 = await ayse.raw('GET', `/v1/tenants/${P.survey.tenantId}/projects/${P.survey.projectId}/snapshot`);
await commitFile(ayse, P.sheet, snap2, '1');
await commitFile(ayse, P.sheet, snap1, '2');
await ayse.run(P.sheet, 'project.checkpoint.create', { name: 'Onaylı pafta', note: 'Kadastro müdürlüğünün onayladığı revizyon.', fileRevision: '2' });
P.zoning = await project(ayse, ORG, { name: 'İmar planı taslağı', projectType: 'zoningPlan', tags: ['imar'], description: '1/1000 uygulama imar planı taslağı.' });
P.road = await project(ayse, ORG, { name: 'Yol güzergâhı', projectType: 'road', tags: ['yol'] });
P.old = await project(ayse, ORG, { name: 'Eski ifraz 2024', projectType: 'subdivision', tags: ['arşiv'] });
await ayse.run(P.old, 'project.archive');
P.trial = await project(ayse, ORG, { name: 'Deneme çizimi', projectType: 'cad' });
await ayse.run(P.trial, 'project.trash');
P.oldSheet = await project(ayse, ORG, { name: 'Eski pafta kopyası', projectType: 'cad' });
await ayse.run(P.oldSheet, 'project.trash');
// Shared: her survey and the file project with mehmet (an editor), an invitation waiting on the survey.
await ayse.run(P.survey, 'project.share', { userId: mehmetMe.user.id, role: 'editor' });
await ayse.run(P.sheet, 'project.share', { userId: mehmetMe.user.id, role: 'editor' });
await ayse.run(P.survey, 'project.invite', { email: 'kadastro@belediye.example', role: 'viewer' });
// Shared with her: mehmet's personal project (a viewer), zeynep's organisation project (an editor).
P.his = await project(mehmet, personalOf(mehmetMe), { name: "Mehmet'in CBS katmanları", projectType: 'gis', tags: ['CBS'], description: 'Mahalle sınırları ve adres noktaları.' });
await mehmet.run(P.his, 'project.share', { userId: ayseMe.user.id, role: 'viewer' });
P.net = await project(zeynep, ORG, { name: 'Kurum yol ağı', projectType: 'road', tags: ['yol', 'kurum'] });
await zeynep.run(P.net, 'project.share', { userId: ayseMe.user.id, role: 'editor' });
if (org2) P.second = await project(ayse, org2, { name: 'Arazi toplulaştırma (ikinci büro)', projectType: 'landReadjustment', tags: ['toplulaştırma'] });
// Favourites: hers and one shared with her.
for (const p of [P.survey, P.zoning, P.his]) await ayse.run(p, 'project.favorite', { favorite: true });
// An invitation for the guest per size (accepting one uses it up), each on a project of its own.
const invites = {};
if (ownerUrl) {
  const names = ['Mahalle imar projesi', 'Köy kadastro projesi'];
  for (const [i, [w]] of sizes.entries()) {
    const p = await project(ayse, ORG, { name: names[i] ?? `Davet ${i + 1}`, projectType: 'cad' });
    invites[w] = (await ayse.run(p, 'project.invite', { email: GUEST.email, role: 'editor' })).body.token;
  }
}
console.log(`tohum: ${Object.keys(P).length} proje`);

// ── The scenes ───────────────────────────────────────────────────────────────────────────────────────────────
const catalogReady = `!!document.querySelector('.catalog-list > *') && !document.querySelector('.catalog-list')?.textContent.includes('yükleniyor')`;
const detailsReady = `!!document.querySelector('.catalog-details__facts') && !document.querySelector('.catalog-details')?.textContent.includes('Hesaplanıyor')`;
const historyReady = `!!document.querySelector('.catalog-history__list, .catalog-details .cloud-empty') && !document.querySelector('.catalog-details')?.textContent.includes('yükleniyor')`;

/** Actions the scenes use on page `b`, and `snap`, which pictures both themes. */
function helpers(b, w) {
  /** An element by selector and leading text; `top`: inside the topmost window only (a question over the catalog). */
  const find = (sel, text, top = false) =>
    `[...(${top ? `[...document.querySelectorAll('.dialog')].at(-1)` : 'document'}).querySelectorAll(${JSON.stringify(sel)})].find((e) => !e.closest('[hidden]') && e.textContent.trim().startsWith(${JSON.stringify(text ?? '')}))`;
  const centre = (sel, text, top) =>
    b.eval(`(() => { const el = ${find(sel, text, top)}; if (!el) return null; el.scrollIntoView({ block: 'nearest' }); const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  const ui = {
    b,
    eval: (e) => b.eval(e),
    waitFor: (e, ms = 10000) => b.waitFor(e, ms),
    run: (id) => b.eval(`window.kentos.commands.execute(${JSON.stringify(id)})`),
    async press(sel, text, top = false) {
      const p = await centre(sel, text, top);
      if (!p) throw new Error(`yok: ${sel} “${text ?? ''}”`);
      await b.click(...p);
      await sleep(200);
    },
    /** A button of the topmost window. */
    pressTop: (sel, text) => ui.press(sel, text, true),
    /** The pointer rests on an element until its tooltip shows. */
    async hover(sel) {
      const p = await centre(sel);
      if (!p) throw new Error(`yok: ${sel}`);
      await b.move(2, 2);
      await sleep(100);
      await b.move(...p);
      await b.waitFor(`!!document.querySelector('.tooltip[data-open]')`, 4000).catch(() => {});
      await sleep(250);
    },
    async type(text) {
      await b.type(text);
      await sleep(150);
    },
    async escape(n = 1) {
      for (let i = 0; i < n; i++) {
        await b.key('Escape');
        await sleep(150);
      }
    },
    /** Closes every window (dialogs and questions stack). */
    async closeAll() {
      for (let i = 0; i < 6 && (await b.eval(`!!document.querySelector('.dialog')`)); i++) await ui.escape();
    },
    async snap(id) {
      if (only && !only.includes(id)) return;
      for (const theme of THEMES) {
        await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
        await sleep(300);
        written.push(await b.shot(`${id}-${theme}-${w}`, undefined, DIR));
      }
      await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
      await sleep(150);
    },
    // The catalog
    async openCatalog() {
      await ui.run('cloud.open');
      await ui.waitFor(catalogReady);
      await sleep(200);
    },
    async list(label) {
      await ui.press('.catalog-nav__item', label);
      await ui.waitFor(`document.querySelector('.catalog-nav [aria-selected="true"]')?.textContent === ${JSON.stringify(label)} && ${catalogReady}`);
      await sleep(250);
    },
    async pick(name) {
      await ui.press('.catalog-row', name);
      await ui.waitFor(`document.querySelector('.catalog-row[aria-selected="true"]')?.textContent.startsWith(${JSON.stringify(name)})`);
      await ui.waitFor(`${detailsReady} || !!document.querySelector('.catalog-history__head')`);
      await sleep(250);
    },
    async tab(label) {
      await ui.press('.catalog-details__tab', label);
      await ui.waitFor(label === 'Geçmiş' ? historyReady : detailsReady);
      await sleep(250);
    },
    /** A window over the catalog (a form or a question) is up. */
    async windowUp(n = 2) {
      await ui.waitFor(`document.querySelectorAll('.dialog').length >= ${n}`);
      await sleep(300);
    },
    /** A native select's options as a list box where its list opens (headless pictures do not draw the real one). */
    async openSelect(label) {
      await b.eval(`(() => {
        const s = document.querySelector('select[aria-label=${JSON.stringify(label)}]');
        const r = s.getBoundingClientRect();
        const box = s.cloneNode(true);
        box.size = Math.min(s.options.length, 9);
        box.value = s.value;
        box.dataset.shotStandIn = '';
        box.removeAttribute('aria-label');
        Object.assign(box.style, { position: 'fixed', left: r.left + 'px', top: r.bottom + 2 + 'px', width: 'max-content', minWidth: r.width + 'px', maxWidth: '360px', height: 'auto', padding: '4px 0', zIndex: 10000, boxShadow: 'var(--shadow-2, 0 6px 20px rgba(0,0,0,.35))' });
        document.body.append(box);
        s.focus();
      })()`);
      await sleep(200);
    },
    closeSelect: () => b.eval(`document.querySelectorAll('[data-shot-stand-in]').forEach((e) => e.remove())`),
  };
  return ui;
}

const ids = (p) => `{ tenantId: ${JSON.stringify(p.tenantId)}, projectId: ${JSON.stringify(p.projectId)} }`;

/** Scenes in order; each leaves the app as the next expects (the catalog closed unless said). */
const SCENES = [
  // ── Kaynaklar (docs/adr/0199 §7): her projects in the dock, one opened (its drawing downloaded, its layers listed),
  // a layer taken with its objects into the open drawing as one undo step, then taken back ──
  async (ui) => {
    if (only && !only.some((id) => id.startsWith('sources-'))) return;
    const row = (name) => `[...document.querySelectorAll('.panel--sources .tree__row')].find((r) => r.querySelector('.tree__name')?.textContent === ${JSON.stringify(name)})`;
    const centreOf = (name) =>
      ui.eval(`(() => { const r = ${row(name)}; if (!r) return null; r.scrollIntoView({ block: 'nearest' }); const b = r.getBoundingClientRect(); return [Math.round(b.left + b.width / 2), Math.round(b.top + b.height / 2)]; })()`);
    // The scenes after this one open projects: the drawing is left as clean as it was found.
    const clean = await ui.eval(`!window.kentos.doc.dirty.value`);
    await ui.run('data.sources');
    await sleep(300);
    await ui.b.click(...(await centreOf('Projelerim')));
    await ui.waitFor(`!!${row(P.survey.name)}`);
    await ui.b.click(...(await centreOf(P.survey.name)));
    await ui.waitFor(`!!${row('Poligon noktaları')}`, 20000);
    await ui.b.click(...(await centreOf('Parsel')));
    await sleep(300);
    await ui.snap('sources-project');
    await ui.eval(`${row('Parsel')}.querySelector('.src__add').click()`);
    await ui.waitFor(`window.kentos.doc.layers.leaves().some((l) => l.name === 'Parsel')`);
    await ui.eval(`window.kentos.ui.dockTab.set('layers')`);
    await sleep(400);
    await ui.snap('sources-added');
    await ui.eval(`(() => { const k = window.kentos; k.doc.undo(); k.ui.dockTab.set('layers'); if (${clean}) k.doc.markSaved(k.doc.revision); })()`);
  },
  // ── The catalog: a project's details, its history, the forms and questions on it ──
  async (ui) => {
    await ui.openCatalog();
    await ui.list('Projelerim');
    await ui.pick(P.survey.name);
    await ui.snap('catalog-details-info');
    await ui.tab('Geçmiş');
    await ui.snap('catalog-details-history-database');
    // Kontrol noktası oluştur…: the form, filled, then the new point in the list; removed again after its question.
    await ui.press('.catalog-details .btn', 'Kontrol noktası oluştur');
    await ui.windowUp();
    await ui.eval(`document.querySelector('input[aria-label="Kontrol noktasının adı"]').focus()`);
    await ui.type('Teslim öncesi');
    await ui.eval(`document.querySelector('textarea[aria-label="Not"]').focus()`);
    await ui.type('Son düzeltmelerden önce.');
    await ui.snap('history-checkpoint-form');
    await ui.pressTop('.btn--primary', 'Kontrol noktası oluştur');
    await ui.waitFor(`document.querySelectorAll('.dialog').length === 1 && document.querySelector('.catalog-history__list')?.textContent.includes('Teslim öncesi')`);
    await sleep(300);
    await ui.snap('history-checkpoint-made');
    await ui.eval(`[...document.querySelectorAll('.catalog-history__list[aria-label="Kontrol noktaları"] > *')].find((r) => r.textContent.includes('Teslim öncesi')).querySelector('.btn--danger').click()`);
    await ui.windowUp();
    await ui.snap('history-checkpoint-delete-question');
    await ui.pressTop('.btn--danger');
    // The list asked again after the removal: its rows, not its loading line.
    await ui.waitFor(`document.querySelectorAll('.dialog').length === 1 && ${historyReady} && !!document.querySelector('.catalog-history__list') && !document.querySelector('.catalog-history__list').textContent.includes('Teslim öncesi')`);
    // Yeni proje olarak geri yükle… on a checkpoint.
    await ui.press('.catalog-history__list .btn', 'Yeni proje olarak geri yükle');
    await ui.windowUp();
    await ui.snap('history-restore-checkpoint-form');
    await ui.escape();
    // The file project: its revisions and its checkpoint; a revision restored; its facts; PostGIS'e aktar…
    await ui.tab('Bilgiler');
    await ui.pick(P.sheet.name);
    await ui.snap('catalog-details-info-file');
    await ui.tab('Geçmiş');
    await ui.snap('catalog-details-history-file');
    await ui.eval(`[...document.querySelectorAll('.catalog-history__head')].find((e) => e.textContent.startsWith('Revizyonlar')).scrollIntoView({ block: 'start' })`);
    await sleep(250);
    await ui.snap('catalog-details-history-file-revisions');
    await ui.eval(`document.querySelector('.catalog-details__body').scrollTop = 0`);
    await ui.eval(`[...[...document.querySelectorAll('.catalog-history__list[aria-label="Revizyonlar"] > *')].find((r) => r.textContent.includes('Revizyon 2')).querySelectorAll('.btn')].find((x) => x.textContent.includes('geri yükle')).click()`);
    await ui.windowUp();
    await ui.snap('history-restore-revision-form');
    await ui.escape();
    await ui.tab('Bilgiler');
    await ui.press('.catalog-details__actions .btn', "PostGIS'e aktar");
    await ui.windowUp();
    await ui.snap('form-convert-to-database');
    await ui.escape();
    // The survey's forms: to a file project, a copy, its information.
    await ui.pick(P.survey.name);
    for (const [label, id] of [
      ['Dosya projesine çevir', 'form-convert-to-file'],
      ['Kopyasını oluştur', 'form-duplicate'],
      ['Bilgileri düzenle', 'form-metadata'],
    ]) {
      await ui.press('.catalog-details__actions .btn', label);
      await ui.windowUp();
      await ui.snap(id);
      await ui.escape();
      await sleep(200);
    }
    // Questions: archive and move to the trash.
    await ui.pick(P.zoning.name);
    for (const [label, id] of [
      ['Arşivle', 'question-archive'],
      ['Çöpe taşı', 'question-trash'],
    ]) {
      await ui.press('.catalog-details__actions .btn', label);
      await ui.windowUp();
      await ui.snap(id);
      await ui.escape();
      await sleep(200);
    }
  },
  // ── Every list, its first project selected ──
  async (ui) => {
    for (const [label, id] of [
      ['Son kullanılanlar', 'catalog-recent'],
      ['Favoriler', 'catalog-favorites'],
      ['Projelerim', 'catalog-mine'],
      ['Kurum projeleri', 'catalog-organization'],
      ['Benimle paylaşılanlar', 'catalog-shared'],
      ['Arşivlenmişler', 'catalog-archived'],
      ['Çöp kutusu', 'catalog-trash'],
    ]) {
      await ui.list(label);
      const first = await ui.eval(`document.querySelector('.catalog-row__title')?.textContent ?? null`);
      if (first) await ui.pick(first);
      await ui.snap(id);
    }
    // In the trash: removed for good, after a question.
    await ui.press('.catalog-details__actions .btn', 'Kalıcı olarak sil');
    await ui.windowUp();
    await ui.snap('question-purge');
    await ui.escape();
  },
  // ── The selects open, and a search that finds nothing ──
  async (ui) => {
    await ui.list('Projelerim');
    for (const [label, id] of [
      ['İş türü', 'catalog-select-type-open'],
      ['Sıralama', 'catalog-select-sort-open'],
    ]) {
      await ui.openSelect(label);
      await ui.snap(id);
      await ui.closeSelect();
    }
    // The organisation's choice shows on its own list.
    await ui.list('Kurum projeleri');
    await ui.openSelect('Kurum');
    await ui.snap('catalog-select-organization-open');
    await ui.closeSelect();
    await ui.list('Projelerim');
    await ui.eval(`document.querySelector('.catalog-search input').focus()`);
    await ui.type('yokboyleproje');
    await ui.waitFor(`!!document.querySelector('.catalog-list')?.textContent.includes('uyan proje yok')`);
    await sleep(300);
    await ui.snap('catalog-search-empty');
    await ui.eval(`(() => { const i = document.querySelector('.catalog-search input'); i.value = ''; i.dispatchEvent(new Event('input')); })()`);
    await ui.waitFor(catalogReady);
    await ui.closeAll();
  },
  // ── Sharing: who has access, the find box, an invitation and its one-time link, removing access ──
  async (ui) => {
    await ui.openCatalog();
    await ui.list('Projelerim');
    await ui.pick(P.survey.name);
    await ui.press('.catalog-details__actions .btn', 'Paylaş');
    await ui.waitFor(`document.querySelectorAll('.dialog--share .share-row').length >= 3 && !document.querySelector('.dialog--share input[aria-label="Paylaşılacak kişi"]').disabled`);
    await sleep(300);
    await ui.snap('share-people');
    await ui.eval(`document.querySelector('.dialog--share input[aria-label="Paylaşılacak kişi"]').focus()`);
    await ui.type('ay');
    await ui.waitFor(`document.querySelectorAll('.dialog--share .share-suggest__item').length >= 1`);
    await sleep(300);
    await ui.snap('share-find');
    await ui.eval(`(() => { const i = document.querySelector('.dialog--share input[aria-label="Paylaşılacak kişi"]'); i.select(); })()`);
    await ui.type('harita.muhendisi@ornek.example');
    await ui.waitFor(`!!document.querySelector('.dialog--share .share-suggest__invite')`);
    await sleep(300);
    await ui.snap('share-find-invite');
    await ui.press('.dialog--share .share-suggest__invite');
    await ui.waitFor(`document.querySelector('.dialog--share .share-tabs [aria-selected="true"]')?.textContent === 'Davetler' && !!document.querySelector('.dialog--share .invite-list') && !document.querySelector('.dialog--share .invite-list').textContent.includes('yükleniyor')`);
    await sleep(300);
    await ui.snap('share-invite-form');
    await ui.press('.dialog--share .btn', 'Davet et');
    await ui.waitFor(`!!document.querySelector('.dialog--share .invite-link__url')`);
    await sleep(300);
    await ui.snap('share-invite-link');
    // Withdrawn again, after its question: the list stays as it was for the next size.
    await ui.press('.dialog--share .share-remove[aria-label^="harita.muhendisi@ornek.example"]');
    await ui.windowUp(3);
    await ui.snap('share-invite-withdraw-question');
    await ui.pressTop('.btn--danger', 'Daveti geri al');
    await sleep(600);
    await ui.press('.dialog--share .share-tabs [role="tab"]', 'Kişiler');
    await sleep(300);
    await ui.press('.dialog--share .share-remove[aria-label^="Mehmet Demir"]');
    await ui.windowUp(3);
    await ui.snap('share-remove-question');
    await ui.closeAll();
  },
  // ── The open project renamed ──
  async (ui) => {
    await ui.eval(`import('/src/ui/cloud/CatalogDialog.ts').then((m) => m.openCatalog(window.kentos, ${ids(P.zoning)}))`);
    await ui.waitFor(`document.querySelector('.catalog-row[aria-selected="true"]')?.textContent.startsWith(${JSON.stringify(P.zoning.name)})`);
    await ui.press('.dialog__foot .btn--primary', 'Aç');
    await ui.waitFor(`window.kentos.cloud.project.value?.projectId === ${JSON.stringify(P.zoning.projectId)} && !document.querySelector('.dialog')`, 30000);
    await ui.run('cloud.rename');
    await ui.waitFor(`!!document.querySelector('.dialog input[aria-label="Yeni ad"]')`);
    await sleep(300);
    await ui.snap('project-rename');
    await ui.closeAll();
  },
  // ── A file project saved over a newer revision: the conflict question ──
  async (ui) => {
    await ui.eval(`import('/src/ui/cloud/CatalogDialog.ts').then((m) => m.openCatalog(window.kentos, ${ids(P.sheet)}))`);
    await ui.waitFor(`document.querySelector('.catalog-row[aria-selected="true"]')?.textContent.startsWith(${JSON.stringify(P.sheet.name)})`);
    await ui.press('.dialog__foot .btn--primary', 'Aç');
    await ui.waitFor(`window.kentos.cloud.file.value && window.kentos.cloud.link.value === 'online' && !document.querySelector('.dialog')`, 30000);
    // Another revision saved elsewhere while nothing waits here: heard and offered.
    let rev = await ui.eval('window.kentos.cloud.file.value.base.value');
    await commitFile(mehmet, P.sheet, snap2, rev);
    rev = String(Number(rev) + 1);
    await ui.waitFor(`[...document.querySelectorAll('.dialog')].some((d) => d.textContent.includes('Son revizyonu aç'))`, 15000).catch(() => {});
    if (await ui.eval(`!!document.querySelector('.dialog')`)) {
      await sleep(300);
      await ui.snap('notice-newer-revision');
      await ui.closeAll();
    }
    // An edit here, one more revision elsewhere, then Kaydet: the conflict question.
    await ui.eval(`(() => { const d = window.kentos.doc; d.add({ kind: 'point', layerId: 'nokta', label: 'P.104', p: { x: ${O.x + 40}, y: ${O.y + 50} }, attrs: {} }); })()`);
    await commitFile(mehmet, P.sheet, snap1, rev);
    await sleep(1500);
    await ui.closeAll();
    await ui.run('file.save');
    await ui.waitFor(`[...document.querySelectorAll('.dialog')].some((d) => d.textContent.includes('başka biri tarafından kaydedildi'))`, 15000);
    await sleep(300);
    await ui.snap('notice-file-conflict');
    await ui.pressTop('.btn', 'Vazgeç');
    await ui.closeAll();
  },
  // ── A file project's revisions (docs/specs/file-revisions.md): its own project for each size, removed after ──
  async (ui) => {
    if (only && !only.some((id) => id.startsWith('revision-'))) return;
    const p = await project(ayse, ORG, { name: 'Ada 1248 pafta', projectType: 'cad', tags: ['pafta'], description: 'Revizyon resimleri için.', storage: 'file' });
    await commitFile(ayse, p, snap1, '0');
    await commitFile(ayse, p, snap2, '1');
    await commitFile(ayse, p, snap1, '2');
    await ayse.run(p, 'project.share', { userId: mehmetMe.user.id, role: 'editor' });
    const file = 'window.kentos.cloud.file.value';
    const cell = async (id) => {
      await ui.hover('.status__save');
      await ui.snap(id);
      await ui.b.move(2, 2);
    };
    try {
      await ui.eval(`import('/src/ui/cloud/CatalogDialog.ts').then((m) => m.openCatalog(window.kentos, ${ids(p)}))`);
      await ui.waitFor(`document.querySelector('.catalog-row[aria-selected="true"]')?.textContent.startsWith(${JSON.stringify(p.name)})`);
      await ui.press('.dialog__foot .btn--primary', 'Aç');
      await ui.waitFor(`${file}?.base.value === '3' && window.kentos.cloud.link.value === 'online' && !document.querySelector('.dialog')`, 30000);
      await ui.closeAll();
      await cell('revision-current');
      // Mehmet saves revision 4 elsewhere: heard, said with who and when, offered; nothing is loaded.
      await commitFile(mehmet, p, snap2, '3');
      await ui.waitFor(`${file}.newer.value?.revision === '4' && !!${file}.newer.value.at`, 15000);
      await cell('revision-newer');
      await ui.eval(`(() => { const u = window.kentos.ui; u.bottomHeight.set(Math.min(260, Math.round(innerHeight * 0.34))); u.bottomTab.set('history'); u.bottomExpanded.set(true); })()`);
      await sleep(300);
      await ui.snap('revision-log');
      await ui.eval(`window.kentos.ui.bottomExpanded.set(false)`);
      await ui.press('.status__save');
      await ui.windowUp(1);
      await ui.snap('revision-newer-question');
      await ui.pressTop('.btn', 'Sonra');
      // An edit here: the cell says both; a click brings the conflict's question.
      await ui.eval(`window.kentos.doc.add({ kind: 'point', layerId: 'nokta', label: 'P.105', p: { x: ${O.x + 50}, y: ${O.y + 12} }, attrs: { Nokta: 'P.105' } })`);
      await ui.waitFor(`document.querySelector('.status__save')?.textContent === 'Yeni revizyon: r4 · kaydedilmedi'`);
      await cell('revision-newer-dirty');
      await ui.press('.status__save');
      await ui.windowUp(1);
      await ui.snap('revision-newer-dirty-question');
      await ui.pressTop('.btn', 'Vazgeç');
      // Kaydet: nothing is uploaded, the question at once; Vazgeç leaves the conflict standing.
      await ui.run('file.save');
      await ui.waitFor(`[...document.querySelectorAll('.dialog')].some((d) => d.textContent.includes('başka biri tarafından kaydedildi'))`, 15000);
      await sleep(200);
      await ui.pressTop('.btn', 'Vazgeç');
      await ui.waitFor(`${file}.state.value === 'conflict'`);
      await cell('revision-conflict');
      // The history: the newest revision and the one the open drawing is based on.
      await ui.run('cloud.history');
      await ui.waitFor(`${historyReady} && !!document.querySelector('.catalog-history__row[data-revision="3"] .catalog-chip')`, 15000);
      // The list and the pane settle first (a late redraw scrolls the pane back to its top).
      await sleep(1200);
      await ui.eval(`document.querySelector('.catalog-history__row[data-revision="3"]').scrollIntoView({ block: 'end' })`);
      await sleep(300);
      await ui.snap('revision-history');
      await ui.closeAll();
      // The newest opened over the edit (dropped on purpose), then another edit: no newer revision is known now.
      await ui.run('cloud.conflicts');
      await ui.windowUp(1);
      await ui.pressTop('.btn--danger', 'Son revizyonu aç');
      await ui.waitFor(`${file}?.base.value === '4' && !window.kentos.doc.dirty.value && !document.querySelector('.dialog')`, 30000);
      await ui.eval(`window.kentos.doc.add({ kind: 'point', layerId: 'nokta', label: 'P.106', p: { x: ${O.x + 55}, y: ${O.y + 12} }, attrs: { Nokta: 'P.106' } })`);
      await ui.run('cloud.openNewest');
      await ui.windowUp(1);
      await ui.snap('revision-unsaved-question');
      await ui.pressTop('.btn', 'Vazgeç');
      // Offline, set for the picture: the cell's words stay; the server cell and the tip say it.
      await ui.eval(`(() => { window.kentos.cloud.link.set('offline'); window.kentos.server.state.set('offline'); })()`);
      await cell('revision-offline');
      await ui.eval(`(() => { window.kentos.cloud.link.set('online'); window.kentos.server.state.set('online'); })()`);
    } finally {
      await ui.closeAll();
      // Left, and removed for good, so the next size and the catalog's pictures do not see it.
      await ui.eval(`(() => { if (window.kentos.cloud.project.value?.projectId === ${JSON.stringify(p.projectId)}) { window.kentos.cloud.detach(); window.kentos.commands.execute('edit.undo'); } })()`);
      await ayse.run(p, 'project.trash').catch(() => {});
      await ayse.run(p, 'project.purge', { confirmName: p.name }).catch(() => {});
    }
  },
  // ── Access taken away while the project is open ──
  async (ui) => {
    await ui.openCatalog();
    await ui.list('Benimle paylaşılanlar');
    await ui.pick(P.his.name);
    await ui.press('.dialog__foot .btn--primary', 'Aç');
    await ui.waitFor(`window.kentos.cloud.project.value?.projectId === ${JSON.stringify(P.his.projectId)} && window.kentos.cloud.link.value === 'online' && !document.querySelector('.dialog')`, 30000);
    await mehmet.run(P.his, 'project.access.revoke', { userId: ayseMe.user.id });
    try {
      await ui.waitFor(`[...document.querySelectorAll('.dialog')].some((d) => d.textContent.includes('Projeye erişiminiz kaldırıldı'))`, 20000);
      await sleep(300);
      await ui.snap('notice-access-lost');
    } finally {
      await ui.closeAll();
      await mehmet.run(P.his, 'project.share', { userId: ayseMe.user.id, role: 'viewer' });
      await ayse.run(P.his, 'project.favorite', { favorite: true }).catch(() => {});
    }
  },
  // ── The status bar's cloud cells and the server cell's account menu (ui/statusbar/cellsPlan.ts) ──
  async (ui) => {
    // A database project shared with her as a viewer: read-only; the actions she lacks say which right.
    await ui.openCatalog();
    await ui.list('Benimle paylaşılanlar');
    await ui.pick(P.his.name);
    await ui.press('.dialog__foot .btn--primary', 'Aç');
    await ui.waitFor(`window.kentos.cloud.project.value?.projectId === ${JSON.stringify(P.his.projectId)} && window.kentos.cloud.link.value === 'online' && !document.querySelector('.dialog')`, 30000);
    await ui.hover('.status__save');
    await ui.snap('status-save-database');
    await ui.hover('.status__server');
    await ui.snap('status-server-tip');
    await ui.press('.status__server');
    await ui.waitFor(`!!document.querySelector('.menu')`);
    await sleep(300);
    await ui.snap('status-account-menu');
    await ui.escape();
    // The cell's other states, set on the open project's save for the picture, then put back.
    const was = await ui.eval(`(() => { const s = window.kentos.cloud.sync.value; return { state: s.state.value, pending: s.pending.value }; })()`);
    for (const [state, pending, id] of [['offline_pending', 3, 'status-save-offline'], ['saving', 0, 'status-save-saving']]) {
      await ui.eval(`(() => { const s = window.kentos.cloud.sync.value; s.pending.set(${pending}); s.state.set(${JSON.stringify(state)}); })()`);
      await ui.hover('.status__save');
      await ui.snap(id);
    }
    await ui.eval(`(() => { const s = window.kentos.cloud.sync.value; s.pending.set(${was.pending}); s.state.set(${JSON.stringify(was.state)}); })()`);
    await ui.b.move(2, 2);
    // A file project: its revision, an upload on its way, and a newer revision offered.
    await ui.eval(`import('/src/ui/cloud/CatalogDialog.ts').then((m) => m.openCatalog(window.kentos, ${ids(P.sheet)}))`);
    await ui.waitFor(`document.querySelector('.catalog-row[aria-selected="true"]')?.textContent.startsWith(${JSON.stringify(P.sheet.name)})`);
    await ui.press('.dialog__foot .btn--primary', 'Aç');
    await ui.waitFor(`window.kentos.cloud.file.value && !document.querySelector('.dialog')`, 30000);
    await ui.closeAll();
    await ui.hover('.status__save');
    await ui.snap('status-save-file');
    const file = `window.kentos.cloud.file.value`;
    const before = await ui.eval(`({ state: ${file}.state.value, progress: ${file}.progress.value })`);
    await ui.eval(`(() => { const f = ${file}; f.progress.set(0.42); f.state.set('uploading'); })()`);
    await ui.b.move(2, 2);
    await sleep(200);
    await ui.snap('status-save-file-uploading');
    await ui.eval(`(() => { const f = ${file}; f.newer.set({ revision: String(Number(f.base.value) + 1), by: 'Mehmet Demir' }); f.state.set('outdated'); })()`);
    await ui.hover('.status__save');
    await ui.snap('status-save-file-outdated');
    await ui.eval(`(() => { const f = ${file}; f.newer.set(null); f.progress.set(${before.progress}); f.state.set(${JSON.stringify(before.state)}); })()`);
    await ui.b.move(2, 2);
  },
  // ── Block definitions in a database project (docs/adr/0144 §5): someone else's rename while one here waits is a
  // conflict that names the block; the server's copy taken, the Bloklar panel shows it with its inserts ──
  async (ui) => {
    if (only && !only.some((id) => id.startsWith('blocks-'))) return;
    await ui.eval(`import('/src/ui/cloud/CatalogDialog.ts').then((m) => m.openCatalog(window.kentos, ${ids(P.survey)}))`);
    await ui.waitFor(`document.querySelector('.catalog-row[aria-selected="true"]')?.textContent.startsWith(${JSON.stringify(P.survey.name)})`);
    await ui.press('.dialog__foot .btn--primary', 'Aç');
    await ui.waitFor(`window.kentos.cloud.project.value?.projectId === ${JSON.stringify(P.survey.projectId)} && window.kentos.cloud.link.value === 'online' && !document.querySelector('.dialog')`, 30000);
    const sync = 'window.kentos.cloud.sync.value';
    const id = await ui.eval(`(() => {
      const d = window.kentos.doc;
      const id = crypto.randomUUID();
      const arm = (k, a, b) => ({ kind: 'line', id: k, layerId: '', a, b, attrs: {} });
      d.addBlock({ id, name: 'Rögar', base: { x: 0, y: 0 }, entities: [arm(1, { x: -1.6, y: 0 }, { x: 1.6, y: 0 }), arm(2, { x: 0, y: -1.6 }, { x: 0, y: 1.6 }), { kind: 'circle', id: 3, layerId: '', c: { x: 0, y: 0 }, r: 1.1, attrs: {} }] });
      for (const x of [8, 34, 58]) d.add({ kind: 'insert', layerId: 'yol', block: id, p: { x: ${O.x} + x, y: ${O.y} - 8 }, scale: 1, rotation: 0, attrs: {} });
      return id;
    })()`);
    await ui.waitFor(`${sync}.state.value === 'saved'`, 15000);
    try {
      const there = (await mehmet.call('GET', `/v1/tenants/${P.survey.tenantId}/projects/${P.survey.projectId}/blocks`)).body.blocks.find((r) => r.block.id === id);
      // His rename first, then one here before his is heard: the conflict names the block.
      await mehmet.run(P.survey, 'project.changes', { features: [], blocks: [{ op: 'update', block: { ...there.block, name: 'Rögar kapağı (Mehmet)' } }] }, { [`block:${id}`]: there.version });
      await ui.eval(`(() => { const d = window.kentos.doc; d.updateBlock({ ...d.block(${JSON.stringify(id)}), name: 'Rögar kapağı' }); window.kentos.commands.execute('file.save'); })()`);
      await ui.waitFor(`${sync}.state.value === 'conflict'`, 15000);
      await ui.run('cloud.conflicts');
      await ui.waitFor(`!!document.querySelector('.cloud-conflicts')`);
      await sleep(300);
      await ui.snap('blocks-conflict');
      await ui.pressTop('.dialog__foot .btn', 'Sunucudakini al');
      await ui.waitFor(`${sync}.state.value === 'saved' && window.kentos.doc.block(${JSON.stringify(id)})?.name === 'Rögar kapağı (Mehmet)'`, 15000);
      await ui.eval(`window.kentos.ui.dockTab.set('blocks')`);
      // No tooltip of an earlier scene over the panel: the pointer and the focus leave the status bar.
      await ui.b.move(300, 170);
      await ui.eval(`(() => { document.activeElement?.blur?.(); window.kentos.view.focus(); })()`);
      await sleep(400);
      await ui.snap('blocks-panel');
    } finally {
      await ui.closeAll();
      await ui
        .eval(`(() => { const d = window.kentos.doc; d.remove([...d.all()].filter((e) => e.kind === 'insert' && e.block === ${JSON.stringify(id)}).map((e) => e.id)); d.removeBlock(${JSON.stringify(id)}); window.kentos.ui.dockTab.set('layers'); })()`)
        .catch(() => {});
      await ui.waitFor(`${sync}.state.value === 'saved'`, 15000).catch(() => {});
    }
  },
];

/** The invitation's page: signed out, then with the wrong account, then accepted by the invited one. */
async function invitationScenes(ui, w) {
  const token = invites[w];
  if (!token) return;
  await ui.run('cloud.signOut');
  await ui.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`);
  await ui.b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0&davet=${token}` });
  await ui.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
  await ui.waitFor(`!!document.querySelector('.dialog--invite') && document.querySelector('.dialog--invite').textContent.includes('Bir projeye davet edildiniz')`, 15000);
  await sleep(300);
  await ui.snap('invitation-signed-out');
  // Signed in as someone else (the inviter herself): refused, with a way to another account.
  await ui.press('.dialog--invite .btn', 'Giriş yap');
  await signIn(ui, 'ayse');
  await ui.waitFor(`!!document.querySelector('.dialog--invite .invite-accept__error')`, 15000);
  await sleep(300);
  await ui.snap('invitation-refused');
  await ui.press('.dialog--invite .btn', 'Başka hesapla giriş yap');
  await signIn(ui, GUEST.login);
  await ui.waitFor(`!!document.querySelector('.dialog--invite .invite-accept__facts')`, 15000);
  await sleep(300);
  await ui.snap('invitation-accepted');
  await ui.closeAll();
}

/** The sign-in window, with the keyboard. */
async function signIn(ui, name) {
  await ui.waitFor(`!!document.querySelector('.dialog--cloud input[name=login]')`);
  await ui.eval(`document.querySelector('.dialog--cloud input[name=login]').focus()`);
  await ui.b.type(name);
  await ui.b.key('Tab');
  await ui.b.type(PASSWORD);
  await ui.b.key('Enter');
  await ui.waitFor(`window.kentos.cloud.auth.value === 'signedIn'`);
  await sleep(400);
}

const written = [];
const failed = [];
try {
  for (const [w, hgt] of sizes) {
    // Turkish, as a user's browser: the date fields read gg.aa.yyyy.
    const b = await launch('about:blank', { width: w, height: hgt, args: ['--lang=tr-TR'] });
    const ui = helpers(b, w);
    try {
      await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
      const ready = 'window.kentos && window.kentos.view.backendKind.value';
      await b.waitFor(ready, 30000);
      await sleep(1200);
      await b.waitFor(ready, 20000);
      await b.waitFor(`window.kentos.server.state.value === 'online'`, 10000);
      await b.eval('document.fonts.ready');
      await ui.run('cloud.signIn');
      await signIn(ui, 'ayse');
      await ui.closeAll();
      for (const [i, scene] of SCENES.entries()) {
        try {
          await scene(ui);
        } catch (e) {
          failed.push(`${w}: sahne ${i + 1}: ${String(e.message ?? e).slice(0, 240)}`);
          await b.shot(`failure-${w}-${i + 1}`, undefined, DIR).catch(() => {});
          await ui.closeAll().catch(() => {});
        }
      }
      try {
        await invitationScenes(ui, w);
      } catch (e) {
        failed.push(`${w}: davet: ${String(e.message ?? e).slice(0, 240)}`);
        await b.shot(`failure-${w}-invite`, undefined, DIR).catch(() => {});
      }
      const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l));
      if (errors.length) console.log(`${w}: konsol: ${errors.slice(0, 5).join(' | ')}`);
    } finally {
      b.close();
    }
  }
} finally {
  await cleanup();
}
console.log(`${written.length} resim: ${DIR}`);
for (const f of failed) console.log(`✗ ${f}`);
process.exit(failed.length ? 1 : 0);
