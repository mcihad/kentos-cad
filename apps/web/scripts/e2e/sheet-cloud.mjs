// The sheet templates' cloud library end to end (docs/sheet/tasks-web.md C; design §13): the real kentosd
// against a THROWAWAY PostGIS in a docker container on a free port (never the developer's database on 5432; the
// container is stopped and removed at the end, the server's settings live in a temporary directory), the app
// in headless Chrome: Ayşe on two devices, Mehmet, Zeynep and a guest. Checks, failing the run when one fails:
//
// - a template saved on this device is “Bu cihazda” and cannot be shared before it is synced; “Buluta eşitle”
//   from the share window puts it in Ayşe's library, and the window goes on live;
// - Mehmet is found by name among the members of the common organisation and given a role; his role changed;
// - Mehmet sees it under “Benimle paylaşılanlar”, uses it with its questions asked, and @kullanici is his name;
// - Ayşe edits it: Mehmet hears it (the long poll) and his sheet says “Yeni sürüm var”;
// - a conflict between Ayşe's two devices: one offline changes it while the other saves a newer revision; back
//   online, both are kept (“… (bu cihazdaki kopya)”) on both devices and in the cloud; the cached copy and the
//   system templates are used offline;
// - Mehmet's access taken away: the template leaves his list, his sheet stays;
// - the organisation's library (“Örnek Harita Bürosu”): Zeynep, its administrator, publishes her own template into
//   it (“Kuruma yayımla…”; a refusal is said in its window); Mehmet, a plain member, sees it, uses it, cannot edit
//   or delete it (and the server refuses his change); “Kuruma yayımla…” is closed while her own template's change
//   waits here; synced, it goes into the organisation's copy (“Kurumdakini güncelle”) and Mehmet hears its
//   revision 2; a guest of no organisation sees none; Ayşe leaves the organisation and loses it.
//
// Pictures in both sizes (1440×900, 1100×650) and both themes (light first) into
// scripts/e2e/out/shots/sheet-cloud/<n>-<scene>-<light|dark>-<width>.png.
//
//   node apps/web/scripts/e2e/sheet-cloud.mjs      (needs docker and the postgis/postgis:18-3.6 image;
//                                                   builds nothing: target/debug/kentosd must be built)
import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import net from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
const WEB = fileURLToPath(new URL('../..', import.meta.url));
const KENTOSD = process.env.KENTOS_E2E_SERVER ? `${process.env.KENTOS_E2E_SERVER}/kentosd` : `${ROOT}target/debug/kentosd`;
const IMAGE = 'postgis/postgis:18-3.6';
const DIR = join(OUT, 'shots', 'sheet-cloud');
/** The pictures' sizes (sheet-shots.mjs's two). */
const SIZES = [
  [1440, 900],
  [1100, 650],
];
mkdirSync(DIR, { recursive: true });

const freePort = () => new Promise((resolve) => { const s = net.createServer().listen(0, '127.0.0.1', () => { const { port } = s.address(); s.close(() => resolve(port)); }); });
const secret = () => crypto.randomUUID().replaceAll('-', '');
const failures = [];
const check = (name, ok, detail = '') => {
  console.log(`${ok ? '✓' : '✗'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures.push(name);
};

// ── The throwaway database and the server ───────────────────────────────
const pgPort = await freePort();
const container = `kentos-sheet-web-e2e-${process.pid}`;
const admin = secret();
const tmp = mkdtempSync(join(tmpdir(), 'kentos-sheet-cloud-'));
const envFile = join(tmp, 'env.local');
let api = null;
let vite = null;
const pages = [];

async function cleanUp() {
  for (const p of pages) p.close();
  if (api) {
    api.kill('SIGINT');
    await new Promise((r) => api.once('exit', r));
    api = null;
  }
  await vite?.close();
  spawnSync('docker', ['stop', container], { stdio: 'ignore' });
  rmSync(tmp, { recursive: true, force: true });
}
process.on('SIGINT', () => void cleanUp().then(() => process.exit(130)));

const kentosd = (args, extra = {}) => {
  const r = spawnSync(KENTOSD, args, { cwd: ROOT, env: { ...process.env, KENTOS_ENV_FILE: envFile, ...extra }, encoding: 'utf8' });
  if (r.status !== 0) throw new Error(`kentosd ${args.join(' ')}: ${r.stderr || r.stdout}`);
  return r.stdout;
};

try {
  execFileSync('docker', ['run', '-d', '--rm', '--name', container, '-p', `127.0.0.1:${pgPort}:5432`, '-e', `POSTGRES_PASSWORD=${admin}`, IMAGE], { stdio: 'ignore' });
  console.log(`geçici veritabanı: docker ${container}, 127.0.0.1:${pgPort} (5432'deki veritabanına dokunulmuyor)`);
  // The image starts PostgreSQL twice (its init scripts in between): ready twice in a row is ready.
  for (let ok = 0, i = 0; ok < 2; i++) {
    if (i > 120) throw new Error('geçici veritabanı açılmadı');
    const r = spawnSync('docker', ['exec', container, 'psql', '-U', 'postgres', '-tAc', 'select 1'], { encoding: 'utf8' });
    ok = r.status === 0 && r.stdout.trim() === '1' ? ok + 1 : 0;
    await sleep(ok ? 1500 : 500);
  }
  kentosd(['db-setup', '--admin-url', `postgres://postgres:${admin}@127.0.0.1:${pgPort}/postgres`]);
  kentosd(['migrate']);
  kentosd(['dev-seed']);
  const env = Object.fromEntries(readFileSync(envFile, 'utf8').split('\n').filter((l) => l.includes('=') && !l.startsWith('#')).map((l) => [l.slice(0, l.indexOf('=')), l.slice(l.indexOf('=') + 1)]));
  const password = env.KENTOS_DEV_PASSWORD;
  if (!password || !env.KENTOS_DATABASE_URL?.includes(`:${pgPort}/`)) throw new Error('sunucu ayarları geçici veritabanını göstermiyor');
  // The throwaway database's name, for the steps that change a membership in it (psql in the container).
  const dbName = new URL(env.KENTOS_DATABASE_URL).pathname.slice(1);

  const apiPort = await freePort();
  process.env.KENTOS_API_PORT = String(apiPort);
  vite = await createServer({ root: WEB, configFile: join(WEB, 'vite.config.mjs'), server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
  await vite.listen();
  const url = vite.resolvedUrls.local[0];
  api = spawn(KENTOSD, ['serve'], { cwd: ROOT, env: { ...process.env, KENTOS_ENV_FILE: envFile, KENTOS_API_PORT: String(apiPort), KENTOS_PUBLIC_URL: url.replace(/\/$/, ''), KENTOS_BLOB_DIR: join(tmp, 'blobs'), KENTOS_LOG: 'warn' }, stdio: ['ignore', 'ignore', 'inherit'] });
  for (let i = 0; ; i++) {
    if (i > 100) throw new Error('kentosd başlamadı');
    if (await fetch(`http://127.0.0.1:${apiPort}/v1/health`).then((r) => r.ok, () => false)) break;
    await sleep(100);
  }

  // ── The app, three times ──────────────────────────────────────────────
  const S = `const k = window.kentos; const m = await import('/src/app/sheet/install.ts'); const s = m.sheetsOf(k); const ta = await import('/src/app/sheet/templateActions.ts');`;
  const open = async (who, login) => {
    const b = await launch('about:blank', { width: 1440, height: 900 });
    pages.push(b);
    b.who = who;
    await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
    await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
    await b.waitFor(`window.kentos.server.state.value === 'online'`, 10000);
    await b.eval(`window.kentos.commands.execute('view.theme.light')`);
    await b.eval(`window.kentos.cloud.signIn(${JSON.stringify(login)}, ${JSON.stringify(password)}).then(() => true)`);
    await b.eval(`(async () => { ${S} k.doc.settings.workspace.set('cad'); await s.ensureEngine(); return true; })()`);
    await b.eval('document.fonts.ready.then(() => true)');
    return b;
  };
  const inPage = (b, body) => b.eval(`(async () => { ${S} ${body} })()`);
  /** Waits until an async test in the page is true (cdp's waitFor takes a plain expression; a promise is always true). */
  const until = async (b, body, what, ms = 30000) => {
    const t0 = Date.now();
    while (Date.now() - t0 < ms) {
      if (await inPage(b, body).catch(() => false)) return true;
      await sleep(250);
    }
    throw new Error(`${b.who}: beklenmedi: ${what}`);
  };
  const listOf = `return (await fetch('/v1/me/sheet-templates', { headers: { 'x-kentos-client': 'web' } }).then((x) => x.json())).templates;`;
  /** A scene in both sizes (1440×900, then 1100×650), each light first; the page goes back to 1440×900. */
  const shot = async (b, name) => {
    for (const [w, h] of SIZES) {
      await b.send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 1, mobile: false });
      await sleep(w === SIZES[0][0] ? 300 : 800);
      await b.move(2, 2);
      await sleep(300);
      await b.shot(`${name}-light-${w}`, undefined, DIR);
      await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
      await sleep(500);
      await b.shot(`${name}-dark-${w}`, undefined, DIR);
      await b.eval(`window.kentos.commands.execute('view.theme.light')`);
      await sleep(300);
    }
    await b.send('Emulation.setDeviceMetricsOverride', { width: SIZES[0][0], height: SIZES[0][1], deviceScaleFactor: 1, mobile: false });
    await sleep(500);
  };
  const press = async (b, sel, text = '') => {
    const p = await b.eval(`(() => { const e = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.textContent.trim().startsWith(${JSON.stringify(text)}) && !x.disabled); if (!e) return null; e.scrollIntoView({ block: 'nearest' }); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    if (!p) throw new Error(`${b.who}: bulunamadı ${sel} ${text}`);
    await b.click(...p);
    await sleep(200);
  };
  const gallery = async (b, section) => {
    await inPage(b, `k.commands.execute('sheet.fromTemplate'); return true;`);
    await b.waitFor(`document.querySelector('.tgal .tcard')`, 15000);
    await press(b, `.tgal__src[data-section="${section}"]`);
    await sleep(500);
  };
  const closeAll = (b) => b.eval(`(() => { for (let i = 0; i < 5; i++) { const x = [...document.querySelectorAll('.dialog .dialog__head button[aria-label="Kapat"]')].at(-1); if (!x) break; x.click(); } return true; })()`);
  const cards = (b, section) => inPage(b, `await s[${JSON.stringify(section === 'shared' ? 'shared' : 'device')}].refresh(); return s[${JSON.stringify(section === 'shared' ? 'shared' : 'device')}].cards.value.map((c) => ({ id: c.id, name: c.name, revision: c.revision, badges: c.badges, role: c.role ?? null, description: c.description }));`);
  const synced = (b) => inPage(b, `const lib = s.shelf.library; await lib.sync.run(); return lib.sync.status.value.state;`);

  const ayse = await open('Ayşe 1', 'ayse');

  // (1) A template saved on this device: “Bu cihazda”, and Paylaş says to sync it first.
  const NAME = 'Belediye ifraz paftası (E2E)';
  const made = await inPage(
    ayse,
    `const e = s.engine(); const t = e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi');
     const id = await ta.sheetFromTemplate(s, t, null, undefined, [{ name: 'il', value: 'Sivas' }, { name: 'ilce', value: 'Suşehri' }]);
     const words = { name: ${JSON.stringify(NAME)}, description: 'Kızılırmak mahallesi için', category: 'kadastro', tags: ['e2e'], papers: [{ paper: 'a3', orientation: 'landscape' }], workspaces: ['cad'], projectTypes: [] };
     const saved = await ta.saveAsTemplate(k, s, id, words, null);
     return saved?.meta.id ?? null;`,
  );
  check('a sheet saved as a template on this device', !!made);
  await gallery(ayse, 'mine');
  let mine = await cards(ayse, 'mine');
  check('it is “Bu cihazda” until it is synced', mine.length === 1 && mine[0].badges.join() === 'device', JSON.stringify(mine.map((c) => c.badges)));
  await press(ayse, '.tgal .tcard');
  await shot(ayse, '1-bu-cihazda');
  await press(ayse, '.tgal__actions .btn', 'Paylaş');
  await ayse.waitFor(`document.querySelector('.dialog--share')`, 5000);
  const gate = await ayse.eval(`document.querySelector('.dialog--share .share-template__sync')?.textContent ?? ''`);
  check('Paylaş says to sync it first', gate.includes('Paylaşmak için önce buluta eşitleyin'), gate.slice(0, 80));
  await shot(ayse, '2-once-esitleyin');

  // (2) “Buluta eşitle” from the window: in the library under the cloud's id; the window goes on live.
  await press(ayse, '.dialog--share .share-template__actions .btn', 'Buluta eşitle');
  await ayse.waitFor(`!document.querySelector('.dialog--share input[aria-label="Paylaşılacak kişi"]').disabled`, 20000);
  mine = await cards(ayse, 'mine');
  const cloudId = mine[0]?.id;
  check('“Buluta eşitle” puts it in the cloud under its own id, synced', mine.length === 1 && cloudId !== made && mine[0].badges.join() === 'synced', JSON.stringify(mine));
  const listed = await ayse.eval(`fetch('/v1/me/sheet-templates', { headers: { 'x-kentos-client': 'web' } }).then((r) => r.json())`);
  check('the server lists it as Ayşe’s', listed.templates?.length === 1 && listed.templates[0].id === cloudId && listed.templates[0].role === 'owner');

  // (3) Mehmet found by name (ADR 0024: the members of a common organisation), shared as a viewer, then an editor.
  await ayse.eval(`(() => { const i = document.querySelector('.dialog--share input[aria-label="Paylaşılacak kişi"]'); i.focus(); return true; })()`);
  await ayse.type('meh');
  await ayse.waitFor(`document.querySelector('.dialog--share .share-suggest:not([hidden]) .share-suggest__item')`, 8000);
  const found = await ayse.eval(`[...document.querySelectorAll('.dialog--share .share-suggest__item')].map((li) => li.textContent)`);
  check('finds Mehmet by name among the organisation’s members', found.length === 1 && found[0].includes('Mehmet Demir'), found.join(' | '));
  await shot(ayse, '3-kisi-ara');
  await ayse.key('Enter');
  await press(ayse, '.dialog--share .share-add .btn--primary', 'Paylaş');
  await ayse.waitFor(`document.querySelectorAll('.dialog--share .share-row').length === 2`, 8000);
  const rows = () => ayse.eval(`[...document.querySelectorAll('.dialog--share .share-row')].map((r) => (r.querySelector('.share-name').textContent + ':' + (r.querySelector('select')?.selectedOptions[0]?.textContent ?? 'sahip')))`);
  check('shared with Mehmet as a viewer', (await rows()).join() === 'Ayşe Yılmaz (siz):sahip,Mehmet Demir:Görüntüleyebilir', (await rows()).join());
  await ayse.eval(`(() => { const sel = document.querySelector('.dialog--share .share-row[data-user] select'); sel.value = 'editor'; sel.dispatchEvent(new Event('change')); return true; })()`);
  await ayse.waitFor(`document.querySelector('.dialog--share .share-row[data-user] select')?.selectedOptions[0]?.textContent === 'Düzenleyebilir' && !document.querySelector('.dialog--share .share-row[data-user] select').disabled`, 8000);
  check('his role changed in his row', (await rows()).join().includes('Mehmet Demir:Düzenleyebilir'));
  await shot(ayse, '4-paylasildi');
  await closeAll(ayse);

  // (4) Mehmet: under “Benimle paylaşılanlar”; used with its questions; @kullanici is his name.
  const mehmet = await open('Mehmet', 'mehmet');
  await synced(mehmet);
  let theirs = await cards(mehmet, 'shared');
  check('Mehmet sees it under “Benimle paylaşılanlar”, with his role', theirs.length === 1 && theirs[0].id === cloudId && theirs[0].badges.join() === 'editor,synced', JSON.stringify(theirs));
  await gallery(mehmet, 'shared');
  await press(mehmet, '.tgal .tcard');
  await shot(mehmet, '5-benimle-paylasilanlar');
  await press(mehmet, '.tgal .btn--primary', 'Kullan');
  await mehmet.waitFor(`document.querySelector('.sheet-ask-q')`, 8000);
  const asked = await mehmet.eval(`[...document.querySelectorAll('.sheet-ask-q .sheet-field__label')].map((l) => l.textContent)`);
  check('Kullan asks the template’s questions first', asked.some((l) => l.includes('@ada')), asked.join(' | '));
  await mehmet.eval(`(() => { const f = [...document.querySelectorAll('.sheet-ask-q input')]; const put = (i, v) => { f[i].value = v; }; put(2, 'Kızılırmak'); put(3, '1247'); put(4, 'P-14'); return true; })()`);
  await shot(mehmet, '6-sorular');
  await press(mehmet, '.sheet-ask-q .btn--primary', 'Paftayı oluştur');
  await mehmet.waitFor(`window.kentos && document.querySelector('.shell[data-sheet-mode]')`, 10000);
  const sheet = await inPage(mehmet, `const b = s.book().book; const sh = b.sheets[0]; const vars = Object.fromEntries(sh.variables.map((v) => [v.name, v.value])); const texts = JSON.stringify(s.plan(s.book(), sh.id)); return { vars, origin: sh.origin, user: texts.includes('Mehmet Demir'), findings: s.findings.value.map((f) => f.message) };`);
  check('the sheet has his answers; what he left empty waits for a value', sheet.vars.mahalle === 'Kızılırmak' && sheet.vars.ada === '1247' && sheet.vars.pafta_no === 'P-14' && sheet.vars.il === null, JSON.stringify(sheet.vars));
  check('@kullanici is the signed-in name', sheet.user && !sheet.findings.some((f) => f.includes('@kullanici')), sheet.findings.join(' | ').slice(0, 120));
  await sleep(1500);
  await shot(mehmet, '7-paylasilandan-pafta');

  // (5) Ayşe saves a newer revision; Mehmet hears it and his sheet says “Yeni sürüm var”.
  const save = (b, description) =>
    inPage(
      b,
      `const card = s.device.cards.value.find((c) => c.id === ${JSON.stringify(cloudId)});
       const id = await ta.sheetFromTemplate(s, card.template, null, 'Şablonu düzenle');
       const words = { name: card.name, description: ${JSON.stringify(description)}, category: card.category, tags: card.tags, papers: card.papers, workspaces: card.workspaces, projectTypes: card.projectTypes };
       return !!(await ta.saveAsTemplate(k, s, id, words, card));`,
    );
  await save(ayse, 'Ayşe’nin ikinci sürümü');
  await until(ayse, `const list = await (async () => { ${listOf} })(); return list[0]?.revision === 2;`, 'revizyon 2 bulutta', 20000);
  check('her change goes up as revision 2', true);
  await mehmet.waitFor(`document.querySelector('.sheet-tab__newer')`, 40000);
  theirs = await cards(mehmet, 'shared');
  check('Mehmet hears it without asking (the cloud’s events) and his sheet says “Yeni sürüm var”', theirs[0]?.revision === 2);
  await mehmet.eval(`(() => { document.querySelector('.sheet-insp__tabs [data-tab="page"]')?.click(); return true; })()`);
  await sleep(600);
  await mehmet.eval(`(() => { document.querySelector('.sheet-insp [data-section="template"]')?.scrollIntoView({ block: 'center' }); return true; })()`);
  await sleep(300);
  const pageNote = await mehmet.eval(`document.querySelector('.sheet-insp')?.textContent.includes('Yeni sürüm var')`);
  check('the Sayfa tab says “Yeni sürüm var” too', pageNote);
  await shot(mehmet, '8-yeni-surum-var');

  // (6) A conflict between Ayşe's two devices: the first offline, the second saves first.
  const second = await open('Ayşe 2', 'ayse');
  await synced(second);
  check('Ayşe’s second device downloads her library', (await cards(second, 'mine')).some((c) => c.id === cloudId && c.revision === 2));
  // Everything the offline device will need is loaded before the network goes.
  await ayse.eval(`Promise.all([import('/src/ui/sheet/TemplateGallery.ts'), import('/src/ui/sheet/TemplateQuestions.ts'), import('/src/ui/sheet/ShareTemplateDialog.ts'), import('/src/ui/widgets/confirm.ts')]).then(() => true)`);
  await ayse.send('Network.enable');
  await ayse.send('Network.emulateNetworkConditions', { offline: true, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
  await save(ayse, 'Birinci cihazın çevrimdışı değişikliği');
  await until(ayse, `return s.shelf.library.sync.status.value.state === 'offline';`, 'çevrimdışı', 20000);
  mine = await cards(ayse, 'mine');
  check('offline, the change waits: “Değişti, eşitlenmedi”', mine.find((c) => c.id === cloudId)?.badges.includes('unsynced'), JSON.stringify(mine.map((c) => c.badges)));
  const usedOffline = await inPage(ayse, `const card = s.device.cards.value.find((c) => c.id === ${JSON.stringify(cloudId)}); const sys = s.engine().systemTemplates()[0]; const a = await ta.useTemplate(s, card.template, null, async () => []); const b = await ta.useTemplate(s, sys, null, async () => []); return !!a && !!b;`);
  check('offline, the cached copy and the system templates are used', usedOffline);
  await closeAll(ayse);
  await gallery(ayse, 'mine');
  await press(ayse, '.tgal .tcard');
  await shot(ayse, '9-cevrimdisi');
  await closeAll(ayse);
  await save(second, 'İkinci cihazın değişikliği');
  await until(second, `const list = await (async () => { ${listOf} })(); return list.find((t) => t.id === ${JSON.stringify(cloudId)})?.revision === 3;`, 'revizyon 3 bulutta', 20000);
  await ayse.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
  await ayse.eval(`window.dispatchEvent(new Event('online')), true`);
  await until(ayse, `await s.device.refresh(); return s.device.cards.value.length === 2;`, 'çakışmadan iki şablon', 90000);
  mine = await cards(ayse, 'mine');
  const copy = mine.find((c) => c.name.endsWith('(bu cihazdaki kopya)'));
  const kept = mine.find((c) => c.id === cloudId);
  check('back online, both sides are kept: the cloud’s downloaded, this device’s as “(bu cihazdaki kopya)”', kept?.revision === 3 && kept.description === 'İkinci cihazın değişikliği' && copy?.description === 'Birinci cihazın çevrimdışı değişikliği' && copy.badges.includes('conflict'), JSON.stringify(mine));
  const said = await ayse.eval(`window.kentos.log.entries.value.map((e) => e.text).filter((t) => t.includes('ikisi de kaldı')).length`);
  check('and the user is told', said >= 1);
  const both = await ayse.eval(`fetch('/v1/me/sheet-templates', { headers: { 'x-kentos-client': 'web' } }).then((r) => r.json()).then((r) => r.templates.map((t) => t.name).sort())`);
  check('the cloud has both', both.length === 2 && both.some((n) => n.endsWith('(bu cihazdaki kopya)')), both.join(' | '));
  await until(second, `await s.device.refresh(); return s.device.cards.value.length === 2;`, 'ikinci cihazda iki şablon', 60000);
  check('the other device hears of the copy', true);
  await gallery(ayse, 'mine');
  await ayse.eval(`(() => { [...document.querySelectorAll('.tgal .tcard')].find((c) => c.textContent.includes('bu cihazdaki kopya'))?.click(); return true; })()`);
  await sleep(500);
  await shot(ayse, '10-cakisma');
  await closeAll(ayse);

  // (7) Mehmet's access taken away: the template leaves his list; his sheet stays.
  await inPage(ayse, `await s.cloud().unshare(${JSON.stringify(cloudId)}, (await s.cloud().access(${JSON.stringify(cloudId)})).grants[0].userId); return true;`);
  await until(mehmet, `await s.shared.refresh(); return s.shared.cards.value.length === 0;`, 'paylaşım kalktı', 60000);
  const stays = await inPage(mehmet, `return s.book().book.sheets.length;`);
  check('unshared: it leaves Mehmet’s list, his sheet stays', stays === 1);
  const told = await mehmet.eval(`window.kentos.log.entries.value.some((e) => e.text.includes('artık sizinle paylaşılmıyor'))`);
  check('and he is told', told);

  // ── (8) The organisation's library (design §13 “Kurum şablonları”) ─────────────────────────────────────
  // “Örnek Harita Bürosu” (dev-seed): Zeynep administers it, Ayşe may publish (a project manager), Mehmet is a
  // plain member. A guest of no organisation and one who left see none of it.
  const ORG = 'Örnek Harita Bürosu';
  const ORG_NAME = 'Kurum ifraz paftası (E2E)';
  const orgCards = (b) => inPage(b, `await s.shelf.org.refresh(); return { state: s.shelf.org.state.value, cards: s.shelf.org.cards.value.map((c) => ({ id: c.id, name: c.name, revision: c.revision, badges: c.badges, role: c.role ?? null, org: c.organization?.name ?? null })) };`);
  const zeynep = await open('Zeynep', 'zeynep');
  const zMade = await inPage(
    zeynep,
    `const e = s.engine(); const t = e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi');
     const id = await ta.sheetFromTemplate(s, t, null, undefined, [{ name: 'il', value: 'Sivas' }]);
     const saved = await ta.saveAsTemplate(k, s, id, { name: ${JSON.stringify(ORG_NAME)}, description: 'Büronun ifraz paftası', category: 'kadastro', tags: ['e2e'], papers: [{ paper: 'a3', orientation: 'landscape' }], workspaces: ['cad'], projectTypes: [] }, null);
     return await s.cloud().upload(saved.meta.id, saved.meta.name);`,
  );
  check('the administrator’s own template is in the cloud', !!zMade);
  await gallery(zeynep, 'mine');
  await zeynep.eval(`(() => { [...document.querySelectorAll('.tgal .tcard')].find((c) => c.getAttribute('aria-label') === ${JSON.stringify(ORG_NAME)})?.click(); return true; })()`);
  await sleep(400);
  await press(zeynep, '.tgal__actions .btn', 'Kuruma yayımla');
  await zeynep.waitFor(`document.querySelector('.dialog--publish [data-key="publish.org"]')`, 10000);
  const offered = await zeynep.eval(`document.querySelector('.dialog--publish [data-key="publish.org"]')?.textContent ?? ''`);
  check('“Kuruma yayımla…” offers the organisation she may publish to', offered.includes(ORG), offered);
  await shot(zeynep, '11-kuruma-yayimla');
  // The server refuses (her role in the organisation lowered meanwhile): the window says so in the server's words
  // and stays open; her role back, the same window publishes.
  const zeynepRole = (role) => {
    const r = spawnSync('docker', ['exec', container, 'psql', '-U', 'postgres', '-d', dbName, '-v', 'ON_ERROR_STOP=1', '-tAc', `update kentos.membership set role = '${role}' where tenant_id = (select id from kentos.tenant where slug = 'ornek-buro') and user_id = (select user_id from kentos.local_credential where lower(login) = 'zeynep')`], { encoding: 'utf8' });
    if (r.status !== 0) throw new Error(`rol değişmedi: ${r.stderr}`);
  };
  zeynepRole('editor');
  await press(zeynep, '.dialog--publish .btn', 'Yayımla');
  await zeynep.waitFor(`[...document.querySelectorAll('.dialog--publish .note')].some((n) => n.textContent.includes('Kuruma yayımlanamadı'))`, 20000);
  const refusal = await zeynep.eval(`[...document.querySelectorAll('.dialog--publish .note')].map((n) => n.textContent.trim()).find((t) => t.includes('Kuruma yayımlanamadı')) ?? ''`);
  check('a refusal is said in the publish window, in the server’s words, and the window stays open', /Kuruma yayımlanamadı: \S/.test(refusal), refusal);
  zeynepRole('admin');
  await press(zeynep, '.dialog--publish .btn', 'Yayımla');
  await zeynep.waitFor(`!document.querySelector('.dialog--publish') && document.querySelector('.tgal__src[data-section="org"][aria-selected="true"]')`, 20000);
  let zOrg = await orgCards(zeynep);
  const zCard = zOrg.cards.find((c) => c.name === ORG_NAME);
  check('published: in Kurumum under the organisation, hers (“Yayımladınız”)', zCard?.org === ORG && zCard.role === 'owner' && zCard.badges.join() === 'org,published,synced', JSON.stringify(zCard));
  const zList = await zeynep.eval(`fetch('/v1/me/sheet-templates', { headers: { 'x-kentos-client': 'web' } }).then((r) => r.json())`);
  const orgEntry = zList.organizations?.find((o) => o.name === ORG);
  check('the server lists it in the organisation’s library, from her own', orgEntry?.canPublish === true && orgEntry.templates.some((t) => t.id === zCard?.id && t.publishedFrom === zMade), JSON.stringify(orgEntry?.templates?.map((t) => [t.name, t.role, t.publishedFrom])));
  const zSaid = await zeynep.eval(`window.kentos.log.entries.value.some((e) => e.text.includes('kurumunda yayımlandı'))`);
  check('and she is told', zSaid);
  await sleep(300);
  await shot(zeynep, '12-kurumum-yayimlayan');
  await closeAll(zeynep);

  // A plain member sees it (the cloud tells every member), uses it, and cannot change it.
  await until(mehmet, `await s.shelf.org.refresh(); return s.shelf.org.cards.value.some((c) => c.name === ${JSON.stringify(ORG_NAME)});`, 'Mehmet kurum şablonunu görür', 60000);
  const mOrg = await orgCards(mehmet);
  const mCard = mOrg.cards.find((c) => c.name === ORG_NAME);
  check('a plain member sees it under the organisation, without a role badge', mCard?.org === ORG && mCard.role === 'viewer' && mCard.badges.join() === 'org,synced', JSON.stringify(mCard));
  const used = await inPage(mehmet, `const c = s.shelf.org.cards.value.find((x) => x.name === ${JSON.stringify(ORG_NAME)}); const before = s.book()?.book.sheets.length ?? 0; const ok = await ta.templateAction(k, s, c, 'use', null, async (t) => t.variables.map((v) => ({ name: v.name, value: v.value }))); return ok && s.book().book.sheets.length === before + 1;`);
  check('… uses it', used);
  await gallery(mehmet, 'org');
  await mehmet.eval(`(() => { [...document.querySelectorAll('.tgal .tcard')].find((c) => c.getAttribute('aria-label') === ${JSON.stringify(ORG_NAME)})?.click(); return true; })()`);
  await sleep(400);
  const mButtons = await mehmet.eval(`[...document.querySelectorAll('.tgal__actions .btn')].map((b) => [b.textContent.trim(), b.disabled])`);
  const btn = (label) => mButtons.find((x) => x[0] === label);
  check('… cannot edit or delete it: “Kopyasını düzenle”, Sil closed, no Paylaş', !!btn('Kopyasını düzenle') && !btn('Düzenle') && btn('Sil')?.[1] === true && !btn('Paylaş…'), JSON.stringify(mButtons));
  const refused = await mehmet.eval(`(async () => {
    const list = await fetch('/v1/me/sheet-templates', { headers: { 'x-kentos-client': 'web' } }).then((r) => r.json());
    const org = list.organizations.find((o) => o.name === ${JSON.stringify(ORG)});
    const t = org.templates.find((x) => x.name === ${JSON.stringify(ORG_NAME)});
    const detail = await fetch('/v1/sheet-templates/' + t.id, { headers: { 'x-kentos-client': 'web' } }).then((r) => r.json());
    const envelope = { commandName: 'sheet.template.update', version: 1, tenantId: org.tenantId, projectId: '', requestId: crypto.randomUUID(), idempotencyKey: crypto.randomUUID(), expectedVersions: {}, input: { templateId: t.id, expectedRevision: t.revision, content: detail.content } };
    const r = await fetch('/v1/tenants/' + org.tenantId + '/commands', { method: 'POST', headers: { 'content-type': 'application/json', 'x-kentos-client': 'web' }, body: JSON.stringify(envelope) });
    return r.status;
  })()`);
  check('… and the server refuses his change (403)', refused === 403, String(refused));
  await shot(mehmet, '13-kurumum-uye');
  await closeAll(mehmet);

  // Her own template changed here, not synced yet: “Kuruma yayımla…” is closed until it is (the organisation gets
  // the cloud's latest revision; the desktop's rule); “Şimdi eşitle” sends it. Synced, “Kuruma yayımla…” says the
  // template is published there and puts its content into the organisation's copy (“Kurumdakini güncelle”): its
  // revision 2, which the member hears.
  await inPage(zeynep, `const r = await s.templates.get(${JSON.stringify(zMade)}); const t = r.record.template; await s.templates.save(${JSON.stringify(zMade)}, { ...t, meta: { ...t.meta, description: 'Büronun ifraz paftası, ikinci sürüm' } }, { ...r.record.cloud, changed: true }); s.shelf.libraryChanged(); return true;`);
  await gallery(zeynep, 'mine');
  const pickOwn = async () => {
    await zeynep.eval(`(() => { [...document.querySelectorAll('.tgal .tcard')].find((c) => c.getAttribute('aria-label') === ${JSON.stringify(ORG_NAME)})?.click(); return true; })()`);
    await sleep(400);
  };
  const publishButton = () => zeynep.eval(`(() => { const b = [...document.querySelectorAll('.tgal__actions .btn')].find((x) => x.textContent.trim().startsWith('Kuruma yayımla')); return b ? (b.disabled ? 'kapalı' : 'açık') : 'yok'; })()`);
  await pickOwn();
  const waiting = await publishButton();
  await press(zeynep, '.tgal__actions .btn', 'Şimdi eşitle');
  await until(zeynep, `const r = await s.templates.get(${JSON.stringify(zMade)}); return r.record.cloud.changed === false;`, 'bekleyen değişiklik eşitlenir', 30000);
  await pickOwn();
  const after = await publishButton();
  check('“Kuruma yayımla…” is closed while a change waits here, open once “Şimdi eşitle” sent it', waiting === 'kapalı' && after === 'açık', `${waiting} → ${after}`);
  await press(zeynep, '.tgal__actions .btn', 'Kuruma yayımla');
  await zeynep.waitFor(`[...document.querySelectorAll('.dialog--publish .note')].some((n) => n.textContent.includes('yayımlı'))`, 10000);
  await shot(zeynep, '16-kurumdakini-guncelle');
  await press(zeynep, '.dialog--publish .btn', 'Kurumdakini güncelle');
  await zeynep.waitFor(`!document.querySelector('.dialog--publish')`, 20000);
  await until(mehmet, `await s.shelf.org.refresh(); const c = s.shelf.org.cards.value.find((x) => x.name === ${JSON.stringify(ORG_NAME)}); return c?.revision === 2 && c.description === 'Büronun ifraz paftası, ikinci sürüm';`, 'üye ikinci revizyonu duyar', 60000);
  check('“Kurumdakini güncelle”: the organisation’s copy takes her content as its revision 2, and the member hears it', true);
  await closeAll(zeynep);

  // A guest of no organisation sees nothing of it.
  kentosd(['user', 'add', '--login', 'misafir', '--name', 'Misafir Kişi', '--password-env', 'KENTOS_DEV_PASSWORD'], { KENTOS_DEV_PASSWORD: password });
  const guest = await open('Misafir', 'misafir');
  await synced(guest);
  const gOrg = await orgCards(guest);
  check('a guest sees no organisation’s template, and is told why', gOrg.cards.length === 0 && gOrg.state.state === 'unavailable' && /etkin üyesi değilsiniz/.test(gOrg.state.reason ?? ''), gOrg.state.reason);
  await gallery(guest, 'org');
  await shot(guest, '14-kurumum-misafir');
  await closeAll(guest);

  // One who left: Ayşe had it; her membership ends (the throwaway database), her next sync takes it off her device.
  await until(ayse, `await s.shelf.org.refresh(); return s.shelf.org.cards.value.some((c) => c.name === ${JSON.stringify(ORG_NAME)});`, 'Ayşe kurum şablonunu görür', 60000);
  const left = spawnSync('docker', ['exec', container, 'psql', '-U', 'postgres', '-d', dbName, '-v', 'ON_ERROR_STOP=1', '-tAc', "update kentos.membership set status = 'disabled' where tenant_id = (select id from kentos.tenant where slug = 'ornek-buro') and user_id = (select user_id from kentos.local_credential where lower(login) = 'ayse')"], { encoding: 'utf8' });
  check('Ayşe leaves the organisation', left.status === 0 && /UPDATE 1/.test(left.stdout), (left.stdout || left.stderr).trim());
  await synced(ayse);
  const aOrg = await orgCards(ayse);
  const aTold = await ayse.eval(`window.kentos.log.entries.value.some((e) => e.text.includes(${JSON.stringify(`“${ORG_NAME}” artık “${ORG}” kurumunun şablonları arasında görünmüyor`)}))`);
  check('one who left loses it on her device, and is told', aOrg.cards.length === 0 && aTold, JSON.stringify(aOrg.cards));
  await gallery(ayse, 'org');
  await shot(ayse, '15-kurumdan-ayrilan');
  await closeAll(ayse);

  for (const b of pages) {
    const errors = b.consoleLog.filter((l) => l.startsWith('EXCEPTION'));
    check(`${b.who}: no uncaught error in the page`, errors.length === 0, errors.slice(0, 2).join(' | '));
  }
} catch (e) {
  failures.push(String(e?.stack ?? e));
  console.error(e);
} finally {
  await cleanUp();
  // --rm removes the stopped container a moment later.
  let left = '';
  for (let i = 0; i < 40; i++) {
    left = spawnSync('docker', ['ps', '-a', '--filter', `name=${container}`, '--format', '{{.Names}}'], { encoding: 'utf8' }).stdout.trim();
    if (!left) break;
    await sleep(250);
  }
  console.log(left ? `kap kaldı: ${left}` : 'geçici veritabanı durduruldu ve silindi');
  if (left) failures.push('geçici veritabanının kabı kaldı');
}
console.log(failures.length ? `\n${failures.length} denetim düştü.` : '\nTüm denetimler geçti.');
console.log(`resimler: ${DIR}`);
process.exit(failures.length ? 1 : 0);
