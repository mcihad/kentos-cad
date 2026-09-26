// Cloud end-to-end test (Faz B): the real kentosd against the development
// database, the app in headless Chrome signed in as `ayse`, and `mehmet` as a
// second editor over plain HTTP. Checks sign-in, upload, that a new project
// is its owner's until she shares it (docs/adr/0015) through the share dialog
// (finding him by name, a role change taking effect), autosave, reload
// persistence, another editor's change arriving live, a conflict resolved
// from the dialog, a server restart while an edit waits, and persistent ids
// (ADR 0014 slice 3, docs/adr/0026): an object's id on the server is its
// `uid`; a command whose answer is lost is sent again and written once; an
// undone deletion comes back under the same id above its old versions; the
// same object brought back by two people stays one ("sunucuda zaten var");
// the same v1 file uploaded twice gives two projects with the same ids.
// Then renaming the open
// project from the list, and deletion: by `zeynep` (an admin, under the
// organisation's policy) while the project is open, and from the list after
// a confirmation. Last, `mehmet` in the browser: a project shared with him
// under “Benimle paylaşılanlar”, opened; his role lowered and raised while it
// is open, then his access taken away: the warning, saving stopped, the
// drawing and his edits kept on the device, further saves refused (TODOS.md
// CLOUD-13, CLOUD-21).
// The catalog (docs/adr/0028, CLOUD-02..05): the upload's type and tags;
// every list of “Bulut projeleri” with a server-side search; favourites and
// recents; the project's information edited; a copy made, archived, moved to
// the trash, restored and removed for good after a question; the open
// project archived by an admin (saving stops, the edit stays on the device,
// and goes once it is unarchived and opened again).
// The upload goes through one import (docs/adr/0036, 0038). File projects
// (docs/adr/0031, 0038): the drawing saved as a file project, Kaydet twice
// with its stages said apart, another client's revision heard and offered,
// a conflict answered twice (the newest revision opened; the drawing saved
// as a separate copy), the history's revisions, and downloads whose bytes
// are the ones uploaded. History and checkpoints (docs/adr/0034, 0038): a
// file project's revision and a database project's present state named from
// the history tab, downloaded, restored as new projects that open (the
// source unchanged), a revision restored, a checkpoint removed after a
// question. Last, the other storage mode (docs/adr/0039): “PostGIS'e aktar”
// and “Dosya projesine çevir”, each opening the new project.
//
//   pnpm e2e:cloud     (needs `pnpm db:setup` once; builds kentosd first)
//   KENTOS_E2E_DB=scratch pnpm e2e:cloud
//                      the same against a throwaway database with this build's
//                      migrations and the development accounts
//                      (apps/api/examples/e2e_database.rs), dropped at the end;
//                      kentos_cad is not touched (a migration not applied
//                      there yet can be tried end to end)
//   KENTOS_E2E_SHOTS=dir  where the screenshots go (default scripts/e2e/out)
//   KENTOS_E2E_SERVER=dir the server's build to run against: the directory
//                      holding kentosd and examples/e2e_database (default
//                      this checkout's target/debug; run the script with
//                      node then, since `pnpm e2e:cloud` builds this one)
//
// Projects it creates stay in the development database, named "E2E …"; the
// ones this run deletes are only moved to the trash (`kentosd project
// deleted`), but the one it removes for good.
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import net from 'node:net';
import { readFileSync } from 'node:fs';
import { createServer } from 'vite';
import { fileURLToPath } from 'node:url';
import { OUT, launch, sleep } from './cdp.mjs';

/** The repository root: .env.local and the Cargo target directory. */
const ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
/** The server's build: this checkout's, or another one's (KENTOS_E2E_SERVER). */
const SERVER = process.env.KENTOS_E2E_SERVER ?? `${ROOT}target/debug`;

const env = Object.fromEntries(
  readFileSync(`${ROOT}.env.local`, 'utf8')
    .split('\n')
    .filter((l) => l && !l.startsWith('#') && l.includes('='))
    .map((l) => [l.slice(0, l.indexOf('=')), l.slice(l.indexOf('=') + 1)]),
);
if (!env.KENTOS_DEV_PASSWORD) throw new Error('.env.local içinde KENTOS_DEV_PASSWORD yok: önce `pnpm db:setup` çalıştırın.');

// A throwaway database instead of the development one (KENTOS_E2E_DB=scratch): its name comes back on the
// helper's first line; its address is the development one's with that name. Neither address is printed.
let scratch = null;
let scratchUrl = null;
if (process.env.KENTOS_E2E_DB === 'scratch') {
  scratch = spawn(`${SERVER}/examples/e2e_database`, [], {
    cwd: ROOT,
    env: { ...process.env, KENTOS_DEV_PASSWORD: env.KENTOS_DEV_PASSWORD },
    stdio: ['pipe', 'pipe', 'inherit'],
  });
  const name = await new Promise((resolve, reject) => {
    let out = '';
    scratch.stdout.on('data', (d) => {
      out += d;
      if (out.includes('\n')) resolve(out.slice(0, out.indexOf('\n')).trim());
    });
    scratch.once('exit', (code) => reject(new Error(`geçici veritabanı açılamadı (${code})`)));
  });
  if (!/^kentos_cad_test_[0-9a-z_]+$/.test(name)) throw new Error('geçici veritabanının adı beklenmedik');
  const url = new URL(env.KENTOS_DATABASE_URL);
  url.pathname = `/${name}`;
  scratchUrl = url.toString();
  console.log(`geçici veritabanı: ${name} (kentos_cad'e dokunulmuyor)`);
}
const SHOTS = process.env.KENTOS_E2E_SHOTS ?? OUT;

const freePort = () => new Promise((resolve) => { const s = net.createServer().listen(0, '127.0.0.1', () => { const { port } = s.address(); s.close(() => resolve(port)); }); });
const apiPort = await freePort();
process.env.KENTOS_API_PORT = String(apiPort);
const vite = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await vite.listen();
const url = vite.resolvedUrls.local[0];
const publicUrl = url.replace(/\/$/, '');

let api = null;
async function startApi() {
  // From the repository root: the workspace's target/ and kentosd's .env.local are there.
  api = spawn(`${SERVER}/kentosd`, ['serve'], {
    cwd: ROOT,
    env: { ...process.env, KENTOS_API_PORT: String(apiPort), KENTOS_PUBLIC_URL: publicUrl, KENTOS_LOG: 'warn', ...(scratchUrl ? { KENTOS_DATABASE_URL: scratchUrl } : {}) },
    stdio: ['ignore', 'ignore', 'inherit'],
  });
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(`http://127.0.0.1:${apiPort}/v1/health`)).ok) return;
    } catch {}
    await sleep(100);
  }
  throw new Error('kentosd başlamadı');
}
async function stopApi() {
  if (!api) return;
  const p = api;
  api = null;
  p.kill('SIGINT');
  await new Promise((r) => p.once('exit', r));
}

/** Another member with their own session, straight against the API (mehmet: an editor; zeynep: an admin). */
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
    return { status: res.status, body: res.status === 204 ? null : await res.json() };
  },
  commit(tenantId, projectId, features, expected) {
    return this.call('POST', `/v1/tenants/${tenantId}/projects/${projectId}/commands`, {
      commandName: 'project.changes', version: 1, tenantId, projectId, requestId: `e2e-${crypto.randomUUID()}`,
      idempotencyKey: crypto.randomUUID(), expectedVersions: expected, input: { features },
    });
  },
  /** A catalog or lifecycle command (docs/adr/0028). */
  run(tenantId, projectId, commandName, input = {}) {
    return this.call('POST', `/v1/tenants/${tenantId}/projects/${projectId}/commands`, {
      commandName, version: 1, tenantId, projectId, requestId: `e2e-${crypto.randomUUID()}`, idempotencyKey: crypto.randomUUID(), expectedVersions: {}, input,
    });
  },
});
/** A new project's metadata, as the API takes it (no objects). */
const emptyProject = {
  settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000 },
  origin: { x: 486500, y: 4420200 },
  layers: [{ id: 'cizim', name: 'Çizim', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'ink', lineType: 'continuous', lineWeight: 0.25 }, children: [] }],
  activeLayer: 'cizim',
  styles: { items: [], categories: [] },
};
const mehmet = client();
const zeynep = client();

const failures = [];
const check = (name, ok, detail = '') => {
  console.log(`${ok ? '✓' : '✗'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures.push(name);
};

await startApi();
const b = await launch(url);
{
  const raw = b.shot;
  b.shot = (name, clip) => raw(name, clip, SHOTS);
}
try {
  const ready = 'window.kentos && window.kentos.view.backendKind.value';
  await b.waitFor(ready, 20000);
  await sleep(1200);
  await b.waitFor(ready, 20000);
  await b.waitFor(`window.kentos.server.state.value === 'online'`, 8000);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`, 5000);
  // Scrolled into view first, as a user would: a button below a pane's fold is clicked where it shows.
  const center = (sel, text = '') =>
    b.eval(`(() => { const e = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.textContent.trim().startsWith(${JSON.stringify(text)})); if (!e) return null; e.scrollIntoView({ block: 'nearest' }); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  const press = async (sel, text) => {
    const p = await center(sel, text);
    if (!p) throw new Error(`bulunamadı: ${sel} ${text ?? ''}`);
    await b.click(...p);
    await sleep(150);
  };

  // Sign in through the dialog, with the keyboard.
  await b.eval(`window.kentos.commands.execute('cloud.upload')`);
  await b.waitFor(`document.querySelector('.dialog--cloud input[name=login]')`, 3000);
  await b.type('ayse');
  await b.key('Tab');
  await b.type(env.KENTOS_DEV_PASSWORD);
  await b.shot('cloud-login');
  await b.key('Enter');
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedIn'`, 5000);
  check('signs in with a local account', true, await b.eval('window.kentos.cloud.me.value.user.displayName'));

  // The upload dialog follows by itself; the drawing becomes a cloud project.
  await b.waitFor(`document.querySelector('.dialog--cloud')?.textContent.includes('Proje adı')`, 3000);
  const name = `E2E ${new Date().toISOString().slice(0, 19)}`;
  await b.eval(`(() => { const i = document.querySelector('.dialog--cloud input[aria-label="Proje adı"]'); i.value = ${JSON.stringify(name)}; i.dispatchEvent(new Event('input')); })()`);
  // The catalog's type and tags go with it (docs/adr/0028).
  await b.eval(`(() => { const d = document.querySelector('.dialog--cloud'); const t = d.querySelector('select[aria-label="Proje türü"]'); t.value = 'subdivision'; t.dispatchEvent(new Event('change')); const g = d.querySelector('input[aria-label="Etiketler"]'); g.value = 'E2E, Kadıköy'; g.dispatchEvent(new Event('input')); })()`);
  await b.shot('cloud-upload');
  const size = await b.eval('window.kentos.doc.size');
  await press('.dialog__foot .btn', 'Buluta yükle');
  await b.waitFor(`window.kentos.cloud.project.value && window.kentos.cloud.sync.value.state.value === 'saved'`, 60000);
  const project = await b.eval('window.kentos.cloud.project.value');
  check('uploads the drawing as a cloud project', project.name === name && !(await b.eval('window.kentos.doc.dirty.value')), `${size} nesne`);
  // Through one import (docs/adr/0036): the project's log holds the import and no batch of object commands.
  const importLog = await b.eval(`window.kentos.cloud.api.events(${JSON.stringify(project.tenantId)}, ${JSON.stringify(project.projectId)}, '0').then((p) => p.events.map((e) => e.kind))`);
  check('the upload is one import, not batches of objects', importLog.includes('project.import') && !importLog.includes('project.changes'), importLog.join(', '));
  const uploaded = await b.eval(`window.kentos.cloud.api.details(${JSON.stringify(project.tenantId)}, ${JSON.stringify(project.projectId)})`);
  check('with the type and tags chosen at the upload', uploaded.project.projectType === 'subdivision' && uploaded.project.tags.join() === 'E2E,Kadıköy', `${uploaded.project.projectType} ${uploaded.project.tags}`);
  await b.waitFor(`window.kentos.cloud.link.value === 'online'`, 8000);
  check('the live channel is open', true);

  const mehmetMe = await mehmet.call('POST', '/v1/auth/login', { login: 'mehmet', password: env.KENTOS_DEV_PASSWORD });
  // A new project is its owner's until she shares it: mehmet, an editor of the same organisation, cannot find it yet.
  const hidden = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}`);
  check('a new project is its owner’s until shared', hidden.status === 404, String(hidden.status));
  // She shares it through the dialog: who has access and why, then Mehmet found by name and added as an editor.
  await b.eval(`window.kentos.commands.execute('cloud.share')`);
  await b.waitFor(`document.querySelectorAll('.dialog--share .share-row').length >= 2 && !document.querySelector('.dialog--share input[aria-label="Paylaşılacak kişi"]').disabled`, 8000);
  const people = () =>
    b.eval(
      `[...document.querySelectorAll('.dialog--share .share-row')].map((r) => ({ name: r.querySelector('.share-name').textContent, role: r.querySelector('select')?.selectedOptions[0]?.textContent ?? r.querySelector('.share-role')?.textContent, sub: r.querySelector('.share-sub').textContent }))`,
    );
  const firstList = await people();
  const own = firstList.find((r) => r.name.startsWith('Ayşe'));
  const byPolicy = firstList.find((r) => r.name.startsWith('Zeynep'));
  const storage = await b.eval(`document.querySelector('.dialog--share .share-storage')?.textContent ?? ''`);
  check(
    'the share dialog shows the owner, the admins under the policy and the storage mode',
    own?.role === 'Sahip' && byPolicy?.role === 'Yönetici' && /Kurum politikası/.test(byPolicy.sub) && /PostGIS/.test(storage) && !firstList.some((r) => r.name.startsWith('Mehmet')),
    JSON.stringify(firstList),
  );
  await b.eval(`document.querySelector('.dialog--share input[aria-label="Paylaşılacak kişi"]').focus()`);
  await b.type('mehmet');
  await b.waitFor(`[...document.querySelectorAll('.dialog--share .share-suggest__item')].some((e) => e.textContent.startsWith('Mehmet Demir'))`, 5000);
  await b.shot('cloud-share-find');
  await press('.dialog--share .share-suggest__item', 'Mehmet Demir');
  await press('.dialog--share .btn--primary', 'Paylaş');
  await b.waitFor(`[...document.querySelectorAll('.dialog--share .share-row')].some((r) => r.textContent.includes('Mehmet Demir'))`, 8000);
  const added = (await people()).find((r) => r.name.startsWith('Mehmet'));
  check('the owner shares it with mehmet as an editor, from the dialog', added?.role === 'Düzenleyici' && added.sub === 'Paylaşım', JSON.stringify(added));
  const listed = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}`);
  check('the member it was shared with sees it with every object', listed.status === 200 && Number(listed.body.featureCount) === size && listed.body.access?.role === 'editor', listed.body.featureCount);
  // The dialog in both themes and at the large type size.
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await sleep(200);
  await b.shot('cloud-share-dark');
  await b.eval(`window.kentos.commands.execute('view.theme.light')`);
  await sleep(200);
  await b.shot('cloud-share-light');
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1.08')`);
  await sleep(200);
  await b.shot('cloud-share-large');
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1')`);
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  // A role changed in the dialog takes effect at once: as a viewer his commit is refused; an editor again, it is not.
  const setRole = (role) =>
    b.eval(
      `(() => { const row = [...document.querySelectorAll('.dialog--share .share-row')].find((r) => r.textContent.includes('Mehmet Demir')); const s = row.querySelector('select'); s.value = ${JSON.stringify(role)}; s.dispatchEvent(new Event('change')); })()`,
    );
  const roleNow = () => mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}`).then((r) => r.body.access?.role);
  await setRole('viewer');
  await b.waitFor(`document.querySelector('.dialog--share .cloud-status')?.textContent.includes('Görüntüleyici')`, 8000);
  const asViewer = await roleNow();
  const tried = await mehmet.commit(project.tenantId, project.projectId, [{ op: 'create', id: crypto.randomUUID(), entity: { kind: 'point', id: 1, layerId: 'cizim', attrs: {}, p: { x: 486500, y: 4420200 } } }], {});
  check('a role changed in the dialog takes effect: a viewer’s commit is refused', asViewer === 'viewer' && tried.status === 403, `${asViewer} ${tried.status}`);
  await setRole('editor');
  await b.waitFor(`document.querySelector('.dialog--share .cloud-status')?.textContent.includes('Düzenleyici')`, 8000);
  check('and back to an editor', (await roleNow()) === 'editor');
  await b.key('Escape');
  await b.waitFor(`!document.querySelector('.dialog--share')`, 3000);

  // Draw a line with typed coordinates; autosave sends it without Ctrl+S.
  const X = 486900, N = 4420600;
  await b.eval(`window.kentos.view.camera.fit({ minX: ${X - 50}, minY: ${N - 50}, maxX: ${X + 150}, maxY: ${N + 50} }, 20)`);
  // As in the smoke test: keys go to the drawing; the command line takes a typed point.
  const focusCanvas = () => b.eval('window.kentos.view.focus()');
  const cmd = async (t) => {
    await focusCanvas();
    await b.eval("window.kentos.commands.execute('commandline.focus')");
    await b.type(t);
    await b.key('Enter');
    await sleep(80);
  };
  await focusCanvas();
  await b.key('l');
  await cmd(`${X},${N}`);
  await cmd(`${X + 100},${N}`);
  await b.key('Escape');
  await b.waitFor(`window.kentos.cloud.sync.value.state.value === 'pending' || window.kentos.cloud.sync.value.state.value === 'saving' || window.kentos.cloud.sync.value.state.value === 'saved'`, 2000);
  await b.waitFor(`window.kentos.cloud.sync.value.state.value === 'saved' && !window.kentos.doc.dirty.value`, 8000);
  // The line's persistent id is its id on the server (ADR 0014 slice 3).
  const lineId = await b.eval(`[...window.kentos.doc.all()].at(-1).uid`);
  let got = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}/features?ids=${lineId}`);
  const line = got.body.features[0];
  // A created object's version is its commit's data revision (docs/adr/0026); each change adds one.
  const v0 = Number(line?.version);
  check('autosave stores the line exactly, under its persistent id', line?.entity.kind === 'line' && line.entity.a.x === X && line.entity.b.x === X + 100 && v0 >= 1, `${lineId} v${line?.version}`);
  await b.shot('cloud-saved');

  // Reload: the session cookie survives; the project opens from the list with the line.
  await b.send('Page.reload');
  await sleep(1500);
  await b.waitFor(ready, 20000);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedIn'`, 8000);
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(`document.querySelector('.catalog-row')`, 5000);
  await press('.catalog-row', name);
  await press('.dialog__foot .btn', 'Aç');
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(name)} && window.kentos.cloud.link.value === 'online'`, 30000);
  const reopened = await b.eval(`(() => { const k = window.kentos; return { size: k.doc.size, line: k.doc.byUid(${JSON.stringify(lineId)}) }; })()`);
  check('after a reload the cloud project opens with the line, under the same id', reopened.size === size + 1 && reopened.line?.b.x === X + 100, `${reopened.size} nesne`);

  // Mehmet moves the line's end: the change arrives live, with no unsaved mark.
  const moved = { ...line.entity, b: { x: X + 120, y: N } };
  const r1 = await mehmet.commit(project.tenantId, project.projectId, [{ op: 'update', id: lineId, entity: moved }], { [lineId]: line.version });
  check('the other editor commits', r1.status === 200, r1.body.versions?.[lineId]);
  await b.waitFor(`[...window.kentos.doc.all()].some((e) => e.kind === 'line' && e.b.x === ${X + 120})`, 8000);
  check('another editor’s change arrives live', !(await b.eval('window.kentos.doc.dirty.value')));

  // Both change it at once: the browser gets a conflict, then takes the server's copy.
  let localId = await b.eval(`[...window.kentos.doc.all()].find((e) => e.kind === 'line' && e.b.x === ${X + 120}).id`);
  const r2 = await mehmet.commit(project.tenantId, project.projectId, [{ op: 'update', id: lineId, entity: { ...line.entity, b: { x: X + 140, y: N } } }], { [lineId]: String(v0 + 1) });
  await b.eval(`window.kentos.doc.update(${localId}, { b: { x: ${X + 160}, y: ${N} } })`);
  await b.eval(`window.kentos.commands.execute('file.save')`);
  await b.waitFor(`window.kentos.cloud.sync.value.state.value === 'conflict'`, 8000);
  check('a simultaneous edit is a conflict, not an overwrite', r2.status === 200, await b.eval(`document.querySelector('.status__save')?.textContent`));
  await b.eval(`window.kentos.commands.execute('cloud.conflicts')`);
  await b.waitFor(`document.querySelector('.cloud-conflicts')`, 3000);
  await b.shot('cloud-conflict');
  await press('.dialog__foot .btn', 'Sunucudakini al');
  await b.waitFor(`window.kentos.cloud.sync.value.state.value === 'saved'`, 8000);
  check('taking the server copy shows the other editor’s line', (await b.eval(`window.kentos.doc.get(${localId}).b.x`)) === X + 140);

  // The server goes away while an edit waits; nothing is lost and it is saved once when it is back.
  await stopApi();
  await b.eval(`window.kentos.doc.update(${localId}, { a: { x: ${X - 10}, y: ${N} } })`);
  await b.eval(`window.kentos.commands.execute('file.save')`);
  await b.waitFor(`window.kentos.cloud.sync.value.state.value === 'offline_pending'`, 8000);
  check('with the server down the edit waits on the device', (await b.eval(`document.querySelector('.status__save')?.textContent`)).startsWith('Çevrimdışı'));
  await startApi();
  await b.waitFor(`window.kentos.cloud.sync.value.state.value === 'saved'`, 40000);
  got = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}/features?ids=${lineId}`);
  if (got.status === 401) {
    await mehmet.call('POST', '/v1/auth/login', { login: 'mehmet', password: env.KENTOS_DEV_PASSWORD });
    got = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}/features?ids=${lineId}`);
  }
  check('the waiting edit is saved once when the server is back', got.body.features[0].entity.a.x === X - 10 && got.body.features[0].version === String(v0 + 3), got.body.features[0].version);
  await b.waitFor(`window.kentos.cloud.link.value === 'online'`, 40000);
  check('the live channel reconnects', true);

  // ── Persistent ids (ADR 0014 slice 3, docs/adr/0026) ──
  const lineNow = () => mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}/features?ids=${lineId}`).then((r) => r.body.features[0] ?? null);
  const saved = `window.kentos.cloud.sync.value.state.value === 'saved' && !window.kentos.doc.dirty.value`;
  const save = async () => {
    await b.eval(`window.kentos.commands.execute('file.save')`);
    await b.waitFor(saved, 15000);
  };
  // A lost answer: the commit is made, the reply never arrives. The same command goes again with its key and is written once.
  const cursorBefore = await b.eval('window.kentos.cloud.sync.value.cursor');
  await b.eval(
    `(() => { const real = window.fetch; window.__realFetch = real; window.__lost = null; window.fetch = async (url, init) => { const res = await real(url, init); if (!window.__lost && init?.method === 'POST' && String(url).endsWith('/commands')) { window.__lost = JSON.parse(init.body); throw new TypeError('Failed to fetch'); } return res; }; })()`,
  );
  await b.eval(`window.kentos.doc.update(${localId}, { a: { x: ${X - 5}, y: ${N} } })`);
  await b.eval(`window.kentos.commands.execute('file.save')`);
  await b.waitFor(`!!window.__lost && window.kentos.cloud.sync.value.state.value === 'offline_pending'`, 8000);
  await b.waitFor(saved, 15000);
  await b.eval('window.fetch = window.__realFetch');
  const lost = await b.eval('window.__lost');
  const logged = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}/events?after=${cursorBefore}`);
  const once = logged.body.events.filter((e) => e.requestId === lost.requestId).length;
  const afterLost = await lineNow();
  check('a command whose answer is lost is sent again with its key and written once', once === 1 && afterLost?.version === String(v0 + 4) && afterLost.entity.a.x === X - 5, `${once} kayıt, v${afterLost?.version}`);

  // Deleted, then brought back by undo: the same id, created again above every version it had; an edit based on an older version is refused.
  const vBefore = afterLost.version;
  await b.eval(`(() => { const k = window.kentos; k.selection.clear(); k.doc.remove([${localId}]); })()`);
  await save();
  const whileGone = await lineNow();
  await b.eval('window.kentos.doc.undo()');
  await save();
  const back = await lineNow();
  const uidBack = await b.eval(`window.kentos.doc.get(${localId})?.uid`);
  const stale = await mehmet.commit(project.tenantId, project.projectId, [{ op: 'update', id: lineId, entity: { ...line.entity, b: { x: X + 180, y: N } } }], { [lineId]: vBefore });
  check(
    'an undone deletion comes back under the same id above its old versions; an edit based on an old one is a conflict',
    whileGone === null && uidBack === lineId && Number(back?.version) > Number(vBefore) && back.entity.a.x === X - 5 && stale.status === 409 && stale.body.conflicts?.[0]?.reason === 'changed',
    `v${vBefore} → v${back?.version}, ${stale.status}`,
  );

  // Brought back by two people: she does not hear his first (her live channel is stopped); her undo then finds
  // the object on the server already. One object stays: she keeps hers over his.
  await b.eval(`window.kentos.doc.remove([${localId}])`);
  await save();
  await b.eval('window.kentos.cloud.socket.stop()');
  const his = await mehmet.commit(project.tenantId, project.projectId, [{ op: 'create', id: lineId, entity: { ...line.entity, a: { x: X - 30, y: N } } }], {});
  await b.eval('window.kentos.doc.undo()');
  await b.eval(`window.kentos.commands.execute('file.save')`);
  await b.waitFor(`window.kentos.cloud.sync.value.state.value === 'conflict'`, 8000);
  await b.eval(`window.kentos.commands.execute('cloud.conflicts')`);
  await b.waitFor(`document.querySelector('.cloud-conflicts')`, 3000);
  const reason = await b.eval(`document.querySelector('.cloud-conflicts li .cloud-row__meta')?.textContent`);
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await sleep(200);
  await b.shot('cloud-conflict-exists-dark');
  await b.eval(`window.kentos.commands.execute('view.theme.light')`);
  await sleep(200);
  await b.shot('cloud-conflict-exists-light');
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1.08')`);
  await sleep(200);
  await b.shot('cloud-conflict-exists-large');
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1')`);
  await sleep(100);
  await press('.dialog__foot .btn', 'Benimkini kaydet');
  await b.waitFor(saved, 8000);
  const hers = await lineNow();
  await b.eval('window.kentos.cloud.socket.start()');
  await b.waitFor(`window.kentos.cloud.link.value === 'online'`, 10000);
  await sleep(500);
  const one = await b.eval(`[...window.kentos.doc.all()].filter((e) => e.uid === ${JSON.stringify(lineId)}).length`);
  check(
    'the same object brought back twice stays one: “sunucuda zaten var”, and keeping hers changes it over his',
    reason === 'sunucuda zaten var' && his.status === 200 && Number(hers?.version) === Number(his.body.versions[lineId]) + 1 && hers.entity.a.x === X - 5 && one === 1,
    `${reason} v${his.body.versions?.[lineId]} → v${hers?.version}`,
  );

  // Themes and the large type size, with the save cell showing.
  for (const theme of ['dark', 'light']) {
    await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
    await sleep(200);
    await b.shot(`cloud-status-${theme}`);
  }
  // The "Büyük" type size with a cloud dialog open.
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1.08')`);
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(`document.querySelector('.catalog-row')`, 5000);
  await press('.catalog-row', name);
  await b.waitFor(`!!document.querySelector('.catalog-details__facts') && !document.querySelector('.catalog-details')?.textContent.includes('Hesaplanıyor')`, 5000);
  await sleep(200);
  await b.shot('cloud-projects-large');
  await b.key('Escape');
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1')`);

  // Renaming the open project from the catalog, with its information (ayse owns it: project.edit and project.delete are hers).
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(`document.querySelector('.catalog-row')`, 5000);
  const before = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.textContent === 'Aç'); return e && { disabled: e.disabled, title: e.title, pane: document.querySelector('.catalog-details')?.textContent }; })()`);
  check('with nothing picked the window waits and says why', !!before?.disabled && /seçin/.test(before.title) && /seçin/.test(before.pane), before?.title);
  await press('.catalog-row', name);
  const del = await b.eval(`(() => { const e = [...document.querySelectorAll('.catalog-details__actions .btn')].find((x) => x.textContent === 'Çöpe taşı…'); return e && { disabled: e.disabled, title: e.title }; })()`);
  check('the owner may move her project to the trash (the permission comes with the project)', del && !del.disabled, del?.title);
  await press('.catalog-details__actions .btn', 'Bilgileri düzenle');
  await b.waitFor(`document.querySelector('.dialog[aria-label="Proje bilgileri"]')`, 3000);
  const renamedTo = `${name} (revize)`;
  await b.eval(`(() => { const i = document.querySelector('.dialog[aria-label="Proje bilgileri"] input[aria-label="Proje adı"]'); i.value = ${JSON.stringify(renamedTo)}; i.dispatchEvent(new Event('input')); const d = document.querySelector('.dialog[aria-label="Proje bilgileri"] textarea'); d.value = 'Kadıköy ifrazı; e2e'; d.dispatchEvent(new Event('input')); })()`);
  await b.shot('cloud-project-info');
  await press('.dialog[aria-label="Proje bilgileri"] .dialog__foot .btn', 'Kaydet');
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(renamedTo)} && window.kentos.cloud.sync.value.state.value === 'saved'`, 8000).catch(() => {});
  await b.waitFor(`[...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent === ${JSON.stringify(renamedTo)})`, 5000).catch(() => {});
  const named = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}`);
  const described = await b.eval(`window.kentos.cloud.api.details(${JSON.stringify(project.tenantId)}, ${JSON.stringify(project.projectId)})`);
  check(
    'renaming the open project saves the name for everyone, the description with it, and the list shows it',
    named.body.name === renamedTo && (await b.eval('window.kentos.doc.name.value')) === renamedTo && described.project.description === 'Kadıköy ifrazı; e2e',
    named.body.name,
  );
  await b.key('Escape');

  // ── The catalog (docs/adr/0028): its lists, a search, a favourite, a copy archived, moved to the trash, restored, removed for good ──
  const stamp2 = new Date().toISOString().slice(0, 19);
  const catalogReady = `!!document.querySelector('.catalog-list > *') && !document.querySelector('.catalog-list')?.textContent.includes('yükleniyor')`;
  const detailsReady = `!!document.querySelector('.catalog-details__facts') && !document.querySelector('.catalog-details')?.textContent.includes('Hesaplanıyor')`;
  const openCatalog = async () => {
    await b.eval(`window.kentos.commands.execute('cloud.open')`);
    await b.waitFor(catalogReady, 8000);
  };
  const showList = async (label) => {
    await press('.catalog-nav__item', label);
    await b.waitFor(`document.querySelector('.catalog-nav [aria-selected="true"]')?.textContent === ${JSON.stringify(label)} && ${catalogReady}`, 8000);
  };
  const rowNames = () => b.eval(`[...document.querySelectorAll('.catalog-row__title')].map((e) => e.textContent)`);
  const pick = async (rowName) => {
    await press('.catalog-row', rowName);
    await b.waitFor(`document.querySelector('.catalog-row[aria-selected="true"]')?.textContent.startsWith(${JSON.stringify(rowName)})`, 5000);
  };
  // Mehmet shares a project of his personal space with her, for “Benimle paylaşılanlar”.
  const mehmetSpace = mehmetMe.body.memberships.find((m) => m.tenantKind === 'personal')?.tenantId;
  const hisName = `E2E Mehmet'in ${stamp2}`;
  const hisProject = await mehmet.call('POST', `/v1/tenants/${mehmetSpace}/projects`, { name: hisName, ...emptyProject, projectType: 'gis', tags: ['CBS'] });
  const ayseId = await b.eval('window.kentos.cloud.me.value.user.id');
  const toAyse = await mehmet.call('POST', `/v1/tenants/${mehmetSpace}/projects/${hisProject.body.id}/commands`, {
    commandName: 'project.share', version: 1, tenantId: mehmetSpace, projectId: hisProject.body.id, requestId: 'e2e-paylas',
    idempotencyKey: crypto.randomUUID(), expectedVersions: {}, input: { userId: ayseId, role: 'viewer' },
  });
  check('mehmet shares a project of his personal space with her', hisProject.status === 201 && toAyse.status === 200, `${hisProject.status} ${toAyse.status}`);

  await openCatalog();
  await showList('Projelerim');
  await pick(renamedTo);
  await b.waitFor(detailsReady, 5000);
  const facts = await b.eval(`document.querySelector('.catalog-details__facts').textContent`);
  check('the details say where it is, its type and objects and extent', /Örnek Harita Bürosu/.test(facts) && /nesne/.test(facts) && /Y \d/.test(facts), facts.slice(0, 120));
  await press('.catalog-details__fav', 'Favorilere ekle');
  await b.waitFor(`document.querySelector('.catalog-details__fav')?.getAttribute('aria-pressed') === 'true'`, 5000);
  await showList('Favoriler');
  check('a favourite is listed under “Favoriler”', (await rowNames()).includes(renamedTo));
  await showList('Son kullanılanlar');
  check('the project she opened is her most recent one', (await rowNames())[0] === renamedTo, (await rowNames()).join(' | '));
  await showList('Benimle paylaşılanlar');
  const sharedToHer = await b.eval(
    `(() => { const r = [...document.querySelectorAll('.catalog-row')].find((x) => x.textContent.startsWith(${JSON.stringify(hisName)})); return r && { sub: r.querySelector('.catalog-row__sub').textContent, role: r.querySelector('.catalog-row__role').textContent }; })()`,
  );
  check('“Benimle paylaşılanlar” lists his project with its owner and her role', !!sharedToHer && /Mehmet Demir/.test(sharedToHer.sub) && sharedToHer.role === 'Görüntüleyici', JSON.stringify(sharedToHer));
  await showList('Kurum projeleri');
  check('the organisation’s list has her project', (await rowNames()).includes(renamedTo));

  // A search on the server: every word, in the name, description or tags, Turkish letters folded.
  await showList('Projelerim');
  await b.eval(`document.querySelector('.catalog-search input').focus()`);
  // Her tag, Turkish letters folded, and this run's time (the development database keeps earlier runs' projects).
  await b.type(`KADIKOY ${name.slice(4)}`);
  await b.waitFor(`document.querySelector('.catalog-main__count')?.textContent === '1 proje' && ${catalogReady}`, 8000);
  check('a search finds her project by its tag and name, Turkish letters folded', JSON.stringify(await rowNames()) === JSON.stringify([renamedTo]), (await rowNames()).join(' | '));
  for (const theme of ['dark', 'light']) {
    await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
    await sleep(200);
    await b.shot(`cloud-catalog-search-${theme}`);
  }
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await b.eval(`(() => { const t = document.querySelector('select[aria-label="Proje türü"]'); t.value = 'gis'; t.dispatchEvent(new Event('change')); })()`);
  await b.waitFor(`document.querySelector('.catalog-list')?.textContent.includes('Aramanıza uyan proje yok')`, 8000);
  check('the type filter leaves out what is not of that type', true);
  await b.eval(`(() => { const i = document.querySelector('.catalog-search input'); i.value = ''; i.dispatchEvent(new Event('input')); const t = document.querySelector('select[aria-label="Proje türü"]'); t.value = ''; t.dispatchEvent(new Event('change')); })()`);
  await b.waitFor(`(${catalogReady}) && [...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent === ${JSON.stringify(renamedTo)})`, 8000);

  // Two copies: one archived, one moved to the trash. A copy keeps every object under its persistent id.
  const copy = async (copyName) => {
    await pick(renamedTo);
    await press('.catalog-details__actions .btn', 'Kopyasını oluştur');
    await b.waitFor(`document.querySelector('.dialog[aria-label="Projenin kopyasını oluştur"]')`, 3000);
    await b.eval(`(() => { const i = document.querySelector('.dialog[aria-label="Projenin kopyasını oluştur"] input[aria-label="Kopyanın adı"]'); i.value = ${JSON.stringify(copyName)}; i.dispatchEvent(new Event('input')); })()`);
    if (copyName.endsWith('A')) await b.shot('cloud-catalog-copy');
    await press('.dialog[aria-label="Projenin kopyasını oluştur"] .dialog__foot .btn', 'Kopyasını oluştur');
    await b.waitFor(`!document.querySelector('.dialog[aria-label="Projenin kopyasını oluştur"]') && document.querySelector('.catalog-row[aria-selected="true"]')?.textContent.startsWith(${JSON.stringify(copyName)})`, 30000);
    return b.eval(`document.querySelector('.catalog-row[aria-selected="true"]').dataset.id`);
  };
  const copyA = await copy(`E2E kopya ${stamp2} A`);
  const copyB = await copy(`E2E kopya ${stamp2} B`);
  const ayseClient = client();
  await ayseClient.call('POST', '/v1/auth/login', { login: 'ayse', password: env.KENTOS_DEV_PASSWORD });
  const srcIds = (await ayseClient.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}/features?limit=5000`)).body.features.map((f) => f.id).sort();
  const copyIds = (await ayseClient.call('GET', `/v1/tenants/${project.tenantId}/projects/${copyA}/features?limit=5000`)).body.features;
  check(
    'a copy has every object under its persistent id, at version 1, and is hers alone',
    JSON.stringify(copyIds.map((f) => f.id).sort()) === JSON.stringify(srcIds) && copyIds.every((f) => f.version === '1') && (await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${copyA}`)).status === 404,
    `${copyIds.length} / ${srcIds.length}`,
  );
  await pick(`E2E kopya ${stamp2} A`);
  await press('.catalog-details__actions .btn', 'Arşivle');
  await b.waitFor(`document.querySelector('.dialog[aria-label="Projeyi arşivle"]')`, 3000);
  await b.shot('cloud-catalog-archive-confirm');
  await press('.dialog[aria-label="Projeyi arşivle"] .dialog__foot .btn', 'Arşivle');
  await b.waitFor(`!document.querySelector('.dialog[aria-label="Projeyi arşivle"]') && ${catalogReady} && ![...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent.endsWith(' A'))`, 8000);
  const archivedA = await ayseClient.run(project.tenantId, copyA, 'project.changes', { features: [] });
  check('an archived copy leaves her list and refuses writing (409)', archivedA.status === 409 && archivedA.body.error === 'project_archived', `${archivedA.status}`);
  await pick(`E2E kopya ${stamp2} B`);
  await press('.catalog-details__actions .btn', 'Çöpe taşı');
  await b.waitFor(`document.querySelector('.dialog[aria-label="Çöp kutusuna taşı"]')`, 3000);
  await press('.dialog[aria-label="Çöp kutusuna taşı"] .dialog__foot .btn', 'Çöpe taşı');
  await b.waitFor(`!document.querySelector('.dialog[aria-label="Çöp kutusuna taşı"]') && ${catalogReady} && ![...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent.endsWith(' B'))`, 8000);
  await showList('Arşivlenmişler');
  check('“Arşivlenmişler” lists the archived copy', (await rowNames()).includes(`E2E kopya ${stamp2} A`));
  await showList('Çöp kutusu');
  const inTrash = await b.eval(
    `(() => { const r = [...document.querySelectorAll('.catalog-row')].find((x) => x.textContent.startsWith(${JSON.stringify(`E2E kopya ${stamp2} B`)})); return r && r.querySelector('.catalog-row__side').textContent; })()`,
  );
  check('the trash lists the other with the day it goes for good', !!inTrash && /tarihinde silinir/.test(inTrash), inTrash);

  // Every list in both themes, a project selected; the trash with its restore button; the large type size once.
  const LISTS = [
    ['Son kullanılanlar', 'recent', renamedTo],
    ['Favoriler', 'favorites', renamedTo],
    ['Projelerim', 'mine', renamedTo],
    ['Kurum projeleri', 'organization', renamedTo],
    ['Benimle paylaşılanlar', 'shared', hisName],
    ['Arşivlenmişler', 'archived', `E2E kopya ${stamp2} A`],
    ['Çöp kutusu', 'trash', `E2E kopya ${stamp2} B`],
  ];
  for (const theme of ['dark', 'light']) {
    await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
    for (const [label, id, row] of LISTS) {
      await showList(label);
      await pick(row);
      if (id !== 'trash') await b.waitFor(detailsReady, 5000);
      await sleep(150);
      await b.shot(`cloud-catalog-${id}-${theme}`);
    }
  }
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  const restoreButton = await b.eval(`[...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.classList.contains('btn--primary'))?.textContent`);
  check('in the trash the main action is restoring', restoreButton === 'Geri yükle', restoreButton);
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1.08')`);
  await showList('Projelerim');
  await pick(renamedTo);
  await b.waitFor(detailsReady, 5000);
  await sleep(200);
  await b.shot('cloud-catalog-large');
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1')`);

  // Restored, it is back where it was; moved to the trash again, it is removed for good after a question.
  await showList('Çöp kutusu');
  await pick(`E2E kopya ${stamp2} B`);
  await press('.dialog__foot .btn', 'Geri yükle');
  await b.waitFor(`${catalogReady} && ![...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent.endsWith(' B'))`, 8000);
  await showList('Projelerim');
  check('restored from the trash, it is in her list again', (await rowNames()).includes(`E2E kopya ${stamp2} B`));
  await pick(`E2E kopya ${stamp2} B`);
  await press('.catalog-details__actions .btn', 'Çöpe taşı');
  await b.waitFor(`document.querySelector('.dialog[aria-label="Çöp kutusuna taşı"]')`, 3000);
  await press('.dialog[aria-label="Çöp kutusuna taşı"] .dialog__foot .btn', 'Çöpe taşı');
  await b.waitFor(`!document.querySelector('.dialog[aria-label="Çöp kutusuna taşı"]')`, 8000);
  await showList('Çöp kutusu');
  await pick(`E2E kopya ${stamp2} B`);
  await press('.catalog-details__actions .btn', 'Kalıcı olarak sil');
  await b.waitFor(`document.querySelector('.dialog[aria-label="Kalıcı olarak sil"]')`, 3000);
  const purgeAsk = await b.eval(`(() => { const d = document.querySelector('.dialog[aria-label="Kalıcı olarak sil"]'); return { safe: document.activeElement?.textContent, text: d.textContent }; })()`);
  check('removing for good asks, says it cannot be undone, and the safe answer has the focus', purgeAsk.safe === 'Vazgeç' && /geri alınamaz/.test(purgeAsk.text), JSON.stringify(purgeAsk));
  for (const theme of ['dark', 'light']) {
    await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
    await sleep(200);
    await b.shot(`cloud-catalog-purge-confirm-${theme}`);
  }
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await press('.dialog[aria-label="Kalıcı olarak sil"] .dialog__foot .btn', 'Kalıcı olarak sil');
  await b.waitFor(`!document.querySelector('.dialog[aria-label="Kalıcı olarak sil"]') && ${catalogReady} && ![...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent.endsWith(' B'))`, 8000);
  const purged = await ayseClient.call('GET', `/v1/tenants/${project.tenantId}/projects/${copyB}`);
  check('removed for good: the project is gone (404)', purged.status === 404, String(purged.status));
  await b.key('Escape');

  // zeynep (an admin) archives it while ayse has it open: saving stops like a deletion, the edit stays on the device;
  // unarchived and opened again, the edit goes out.
  const signedZ = await zeynep.call('POST', '/v1/auth/login', { login: 'zeynep', password: env.KENTOS_DEV_PASSWORD });
  if (signedZ.status !== 200) throw new Error('zeynep giriş yapamadı: geliştirme verisini yenileyin (`pnpm kentosd -- dev-seed`).');
  const archivedOpen = await zeynep.run(project.tenantId, project.projectId, 'project.archive');
  await b.waitFor(`window.kentos.cloud.sync.value?.state.value === 'archived'`, 10000).catch(() => {});
  const archivedCell = await b.eval(`document.querySelector('.status__save')?.textContent`);
  await b.eval(`window.kentos.doc.update(${localId}, { a: { x: ${X - 15}, y: ${N} } })`);
  await sleep(700);
  const heldArchived = await b.eval(
    `new Promise((resolve) => { const r = indexedDB.open('kentos.cloud'); r.onsuccess = () => { const q = r.result.transaction('drafts').objectStore('drafts').getAll(); q.onsuccess = () => resolve(q.result.filter((d) => Object.values(d.changes).some((c) => c.entity?.a?.x === ${X - 15})).length); }; })`,
  );
  check(
    'archived by an admin while open: saving stops, the drawing and the edit stay on the device',
    archivedOpen.status === 200 && archivedCell === 'Proje arşivde' && heldArchived === 1 && (await lineNow()).entity.a.x !== X - 15,
    archivedCell,
  );
  for (const theme of ['dark', 'light']) {
    await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
    await sleep(200);
    await b.shot(`cloud-archived-open-${theme}`);
  }
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  const unarchived = await zeynep.run(project.tenantId, project.projectId, 'project.unarchive');
  await openCatalog();
  await showList('Son kullanılanlar');
  await pick(renamedTo);
  await press('.dialog__foot .btn', 'Aç');
  await b.waitFor(`window.kentos.cloud.project.value?.state === 'active' && window.kentos.cloud.link.value === 'online'`, 30000);
  await b.waitFor(saved, 15000).catch(() => {});
  check('unarchived and opened again, the edit kept on the device is saved', unarchived.status === 200 && (await lineNow()).entity.a.x === X - 15);
  localId = await b.eval(`window.kentos.doc.slotOf(${JSON.stringify(lineId)})`);

  // zeynep (an admin) deletes it while ayse has it open: her app stops saving, the drawing stays, edits stay on the device.
  const sizeBefore = await b.eval('window.kentos.doc.size');
  const gone = await zeynep.call('DELETE', `/v1/tenants/${project.tenantId}/projects/${project.projectId}`);
  check('an admin deletes the project', gone.status === 204, String(gone.status));
  await b.waitFor(`window.kentos.cloud.sync.value?.state.value === 'deleted'`, 10000).catch(() => {});
  const cell = await b.eval(`document.querySelector('.status__save')?.textContent`);
  check('the open editor hears it: saving stops and the drawing stays', cell === 'Proje silindi' && (await b.eval('window.kentos.doc.size')) === sizeBefore, cell);
  await b.eval(`window.kentos.doc.update(${localId}, { a: { x: ${X - 20}, y: ${N} } })`);
  await sleep(700);
  const drafted = await b.eval(`new Promise((resolve) => { const r = indexedDB.open('kentos.cloud'); r.onsuccess = () => { const q = r.result.transaction('drafts').objectStore('drafts').getAll(); q.onsuccess = () => resolve(q.result.filter((d) => Object.values(d.changes).some((c) => c.entity?.a?.x === ${X - 20})).length); }; })`);
  check('an edit made after the deletion stays in the device draft', drafted === 1, String(drafted));
  const refused = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}`);
  const remaining = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects`);
  check('the deleted project refuses opening and leaves the list', refused.status === 410 && refused.body.error === 'project_deleted' && !remaining.body.projects.some((p) => p.id === project.projectId), refused.body.message);
  await b.shot('cloud-deleted');

  // The same v1 file opened and uploaded twice: two projects holding the same object ids, the ones the file's
  // content derives (ADR 0014; fixtures/document/v1/identity, checked against an independent Python reference).
  const idDir = new URL('../../../../fixtures/document/v1/identity/', import.meta.url);
  const kcad = readFileSync(new URL('sample.compact.kcad', idDir), 'utf8');
  const expectedIds = JSON.parse(readFileSync(new URL('expected.json', idDir), 'utf8'))
    .cases[0].entities.map((e) => e.uid)
    .sort();
  const uploadFile = (title) =>
    b.eval(`(async () => {
      const k = window.kentos;
      const keep = { picker: k.files.picker, ask: k.files.ask };
      k.files.ask = async () => 'drop';
      k.files.picker = { open: async () => ({ name: 'kimlik.kcad', getFile: async () => new Blob([${JSON.stringify(kcad)}]) }), save: async () => null };
      try {
        if (!(await k.files.open())) return null;
        await k.cloud.upload(${JSON.stringify(project.tenantId)}, ${JSON.stringify(title)});
        const p = k.cloud.project.value;
        const page = await k.cloud.api.features(p.tenantId, p.projectId, null, 5000);
        return { projectId: p.projectId, server: page.features.map((f) => f.id).sort(), drawing: [...k.doc.all()].map((e) => e.uid).sort() };
      } finally {
        k.files.picker = keep.picker;
        k.files.ask = keep.ask;
      }
    })()`);
  const stamp = new Date().toISOString().slice(0, 19);
  const up1 = await uploadFile(`E2E kimlik ${stamp}`);
  const up2 = await uploadFile(`E2E kimlik (yeniden) ${stamp}`);
  const same = (x) => JSON.stringify(x) === JSON.stringify(expectedIds);
  check(
    'the same v1 file uploaded twice: two projects with the same object ids, the ones its content derives',
    !!up1 && !!up2 && up1.projectId !== up2.projectId && same(up1.server) && same(up2.server) && same(up1.drawing) && same(up2.drawing),
    `${up1?.server.length ?? 0} + ${up2?.server.length ?? 0} nesne`,
  );

  // Signed in as zeynep, the browser deletes another project from the list, after asking.
  const spareName = `E2E silinecek ${new Date().toISOString().slice(0, 19)}`;
  const spare = await zeynep.call('POST', `/v1/tenants/${project.tenantId}/projects`, { name: spareName, ...emptyProject });
  await b.eval(`window.kentos.commands.execute('cloud.signOut')`);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`, 5000);
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(`document.querySelector('.dialog--cloud input[name=login]')`, 3000);
  await b.type('zeynep');
  await b.key('Tab');
  await b.type(env.KENTOS_DEV_PASSWORD);
  await b.key('Enter');
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedIn' && !!document.querySelector('.catalog-list > *') && !document.querySelector('.catalog-list')?.textContent.includes('yükleniyor')`, 8000);
  await press('.catalog-nav__item', 'Projelerim');
  await b.waitFor(`[...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent === ${JSON.stringify(spareName)})`, 8000);
  await press('.catalog-row', spareName);
  await press('.catalog-details__actions .btn', 'Çöpe taşı');
  await b.waitFor(`document.querySelector('.dialog[aria-label="Çöp kutusuna taşı"]')`, 3000);
  const asked = await b.eval(`(() => { const d = document.querySelector('.dialog[aria-label="Çöp kutusuna taşı"]'); return { safe: document.activeElement?.textContent, says: d.querySelectorAll('.confirm__details li').length, text: d.textContent }; })()`);
  check('moving to the trash asks first, with the safe button focused and the retention said', asked.safe === 'Vazgeç' && asked.says >= 3 && /\d+ gün/.test(asked.text), JSON.stringify(asked));
  await b.shot('cloud-delete-confirm');
  await press('.dialog[aria-label="Çöp kutusuna taşı"] .dialog__foot .btn', 'Çöpe taşı');
  await b.waitFor(`!document.querySelector('.dialog[aria-label="Çöp kutusuna taşı"]') && ![...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent === ${JSON.stringify(spareName)})`, 8000).catch(() => {});
  const spareGone = await zeynep.call('GET', `/v1/tenants/${project.tenantId}/projects/${spare.body.id}`);
  check('the confirmed move takes it out of the lists for everyone', spare.status === 201 && spareGone.status === 410, `${spare.status} → ${spareGone.status}`);
  await b.key('Escape');

  // ── Mehmet in the browser: shared with him, opened, his role lowered and raised, then his access taken away. ──
  const ayse = client();
  await ayse.call('POST', '/v1/auth/login', { login: 'ayse', password: env.KENTOS_DEV_PASSWORD });
  const sharedName = `E2E paylaşılan ${new Date().toISOString().slice(0, 19)}`;
  const made = await ayse.call('POST', `/v1/tenants/${project.tenantId}/projects`, { name: sharedName, ...emptyProject });
  const sp = { tenantId: made.body.tenantId, projectId: made.body.id };
  const pointId = crypto.randomUUID();
  await ayse.commit(sp.tenantId, sp.projectId, [{ op: 'create', id: pointId, entity: { kind: 'point', id: 1, layerId: 'cizim', attrs: {}, p: { x: 486510, y: 4420210 } } }], {});
  const give = (role) =>
    ayse.call('POST', `/v1/tenants/${sp.tenantId}/projects/${sp.projectId}/commands`, {
      commandName: role ? 'project.share' : 'project.access.revoke',
      version: 1,
      ...sp,
      requestId: `e2e-${crypto.randomUUID()}`,
      idempotencyKey: crypto.randomUUID(),
      expectedVersions: {},
      input: role ? { userId: mehmetMe.body.user.id, role } : { userId: mehmetMe.body.user.id },
    });
  const granted = await give('editor');
  const serverPoint = async () => (await ayse.call('GET', `/v1/tenants/${sp.tenantId}/projects/${sp.projectId}/features?ids=${pointId}`)).body.features[0];
  await b.eval(`window.kentos.commands.execute('cloud.signOut')`);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`, 5000);
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(`document.querySelector('.dialog--cloud input[name=login]')`, 3000);
  await b.type('mehmet');
  await b.key('Tab');
  await b.type(env.KENTOS_DEV_PASSWORD);
  await b.key('Enter');
  // The window grows when its list arrives (it stays centred): the tab is pressed once the list is there.
  const listReady = `!!document.querySelector('.catalog-nav') && !!document.querySelector('.catalog-list > *') && !document.querySelector('.catalog-list')?.textContent.includes('yükleniyor')`;
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedIn' && ${listReady}`, 8000);
  await press('.catalog-nav__item', 'Benimle paylaşılanlar');
  await b.waitFor(`[...document.querySelectorAll('.catalog-row')].some((r) => r.textContent.startsWith(${JSON.stringify(sharedName)}))`, 8000);
  const sharedRow = await b.eval(
    `(() => { const r = [...document.querySelectorAll('.catalog-row')].find((x) => x.textContent.startsWith(${JSON.stringify(sharedName)})); return { sub: r.querySelector('.catalog-row__sub').textContent, role: r.querySelector('.catalog-row__role').textContent }; })()`,
  );
  check('“Benimle paylaşılanlar” lists it with its owner and my role', granted.status === 200 && sharedRow.sub.includes('Ayşe Yılmaz') && sharedRow.role === 'Düzenleyici', JSON.stringify(sharedRow));
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await sleep(200);
  await b.shot('cloud-shared-dark');
  await b.eval(`window.kentos.commands.execute('view.theme.light')`);
  await sleep(200);
  await b.shot('cloud-shared-light');
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await press('.catalog-row', sharedName);
  await press('.dialog__foot .btn', 'Aç');
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(sharedName)} && window.kentos.cloud.link.value === 'online'`, 30000);
  const localPoint = await b.eval(`window.kentos.doc.slotOf(${JSON.stringify(pointId)})`);
  await b.eval(`window.kentos.doc.update(${localPoint}, { p: { x: 486520, y: 4420210 } })`);
  await b.eval(`window.kentos.commands.execute('file.save')`);
  await b.waitFor(`window.kentos.cloud.sync.value.state.value === 'saved' && !window.kentos.doc.dirty.value`, 8000);
  check('the recipient opens it and his edit is saved', (await serverPoint())?.entity.p.x === 486520);

  // Lowered to a viewer while it is open: saving stops, his next edit stays on the device.
  await give('viewer');
  await b.waitFor(`window.kentos.cloud.project.value?.canWrite === false && window.kentos.cloud.sync.value.state.value === 'readonly'`, 10000);
  await b.eval(`window.kentos.doc.update(${localPoint}, { p: { x: 486530, y: 4420210 } })`);
  await sleep(800);
  check('a lowered role reaches the open editor: the edit waits on the device', (await serverPoint())?.entity.p.x === 486520 && (await b.eval(`window.kentos.cloud.sync.value.pending.value`)) === 1);
  // An editor again: the held edit goes out.
  await give('editor');
  await b.waitFor(`window.kentos.cloud.project.value?.canWrite === true && window.kentos.cloud.sync.value.state.value === 'saved'`, 10000);
  check('raised again, the held edit is saved', (await serverPoint())?.entity.p.x === 486530);

  // Taken away while it is open: a clear warning, saving stops, the drawing and his edits stay here.
  const sizeShared = await b.eval('window.kentos.doc.size');
  await give(null);
  await b.waitFor(`window.kentos.cloud.sync.value?.state.value === 'revoked'`, 10000).catch(() => {});
  await b.waitFor(`document.querySelector('.dialog[aria-label="Projeye erişiminiz kaldırıldı"]')`, 5000).catch(() => {});
  const notice = await b.eval(`document.querySelector('.dialog[aria-label="Projeye erişiminiz kaldırıldı"]')?.textContent ?? ''`);
  const lostCell = await b.eval(`document.querySelector('.status__save')?.textContent`);
  check(
    'revoking shows the warning in the recipient’s open editor; saving stops and the drawing stays',
    /artık erişemiyorsunuz/.test(notice) && /Yerel kopya kaydet/.test(notice) && lostCell === 'Erişim kaldırıldı' && (await b.eval('window.kentos.doc.size')) === sizeShared,
    lostCell,
  );
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await sleep(200);
  await b.shot('cloud-revoked-dark');
  await b.eval(`window.kentos.commands.execute('view.theme.light')`);
  await sleep(200);
  await b.shot('cloud-revoked-light');
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await press('.dialog[aria-label="Projeye erişiminiz kaldırıldı"] .dialog__foot .btn', 'Tamam');
  await b.eval(`window.kentos.doc.update(${localPoint}, { p: { x: 486540, y: 4420210 } })`);
  await sleep(800);
  const kept = await b.eval(
    `new Promise((resolve) => { const r = indexedDB.open('kentos.cloud'); r.onsuccess = () => { const q = r.result.transaction('drafts').objectStore('drafts').getAll(); q.onsuccess = () => resolve(q.result.filter((d) => Object.values(d.changes).some((c) => c.entity?.p?.x === 486540)).length); }; })`,
  );
  const direct = await b.eval(
    `window.kentos.cloud.api.command({ commandName: 'project.changes', version: 1, tenantId: ${JSON.stringify(sp.tenantId)}, projectId: ${JSON.stringify(sp.projectId)}, requestId: 'e2e-sonra', idempotencyKey: crypto.randomUUID(), expectedVersions: {}, input: { features: [{ op: 'create', id: crypto.randomUUID(), entity: { kind: 'point', id: 1, layerId: 'cizim', attrs: {}, p: { x: 1, y: 2 } } }] } }).then(() => 'kaydedildi', (e) => e.code)`,
  );
  check(
    'after it, edits stay in the device draft and the server refuses further saves',
    kept === 1 && direct === 'not_found' && (await serverPoint())?.entity.p.x === 486530 && (await b.eval(`window.kentos.commands.isEnabled('cloud.share')`)) === false,
    `${kept} ${direct}`,
  );
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(listReady, 5000);
  await press('.catalog-nav__item', 'Benimle paylaşılanlar');
  await b.waitFor(`!document.querySelector('.catalog-list')?.textContent.includes('yükleniyor')`, 8000);
  const stillListed = await b.eval(`[...document.querySelectorAll('.catalog-row')].some((r) => r.textContent.startsWith(${JSON.stringify(sharedName)}))`);
  const onShared = await b.eval(`document.querySelector('.catalog-nav [aria-selected="true"]')?.textContent`);
  check('taken away, it leaves “Benimle paylaşılanlar”', onShared === 'Benimle paylaşılanlar' && !stillListed, onShared);
  await b.key('Escape');

  // ── File projects (docs/adr/0031, 0038) ──
  const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
  /** A client's raw request: a file's bytes down, or up. */
  const raw = async (who, method, path, body) => {
    const res = await fetch(`http://127.0.0.1:${apiPort}${path}`, {
      method,
      headers: { 'x-kentos-client': 'web', cookie: who.cookie, ...(body ? { 'content-type': 'application/octet-stream' } : {}) },
      body,
    });
    return { status: res.status, bytes: new Uint8Array(await res.arrayBuffer()) };
  };
  /** Another client saves a revision: the bytes of revision `from`, committed on `base`. */
  const commitAs = async (who, fp, from, base) => {
    const bytes = (await raw(who, 'GET', `/v1/tenants/${fp.tenantId}/projects/${fp.projectId}/files/${from}`)).bytes;
    const up = await who.call('POST', `/v1/tenants/${fp.tenantId}/projects/${fp.projectId}/uploads`, { size: bytes.length, sha256: sha(bytes) });
    const put = await raw(who, 'PUT', `/v1/tenants/${fp.tenantId}/projects/${fp.projectId}/uploads/${up.body.id}`, bytes);
    return who.call('POST', `/v1/tenants/${fp.tenantId}/projects/${fp.projectId}/commands`, {
      commandName: 'project.file.commit', version: 1, ...fp, requestId: `e2e-${crypto.randomUUID()}`, idempotencyKey: crypto.randomUUID(), expectedVersions: { '@file': base }, input: { uploadId: up.body.id },
    }).then((r) => ({ ...r, put: put.status }));
  };
  const themed = async (shot) => {
    for (const theme of ['dark', 'light']) {
      await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
      await sleep(250);
      await b.shot(`${shot}-${theme}`);
    }
    await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1.08')`);
    await sleep(250);
    await b.shot(`${shot}-large`);
    await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1')`);
    await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  };
  // Ayşe again, in the browser: the drawing on screen saved to the cloud as a file project.
  await b.eval(`window.kentos.commands.execute('cloud.signOut')`);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`, 5000);
  await b.eval(`window.kentos.commands.execute('cloud.uploadFile')`);
  await b.waitFor(`document.querySelector('.dialog--cloud input[name=login]')`, 3000);
  await b.type('ayse');
  await b.key('Tab');
  await b.type(env.KENTOS_DEV_PASSWORD);
  await b.key('Enter');
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedIn' && !!document.querySelector('.cloud-storage')`, 8000);
  const fileName = `E2E dosya ${new Date().toISOString().slice(0, 19)}`;
  await b.eval(`(() => { const i = document.querySelector('.dialog--cloud input[aria-label="Proje adı"]'); i.value = ${JSON.stringify(fileName)}; i.dispatchEvent(new Event('input')); })()`);
  const uploadForm = await b.eval(`({ file: document.querySelector('.cloud-storage input[value=file]').checked, button: [...document.querySelectorAll('.dialog__foot .btn')].map((x) => x.textContent) })`);
  check('“Buluta dosya olarak kaydet” offers the storage mode, file chosen', uploadForm.file && uploadForm.button.includes('Buluta dosya olarak kaydet'), JSON.stringify(uploadForm));
  await themed('cloud-file-upload');
  await press('.dialog__foot .btn', 'Buluta dosya olarak kaydet');
  await b.waitFor(`window.kentos.cloud.file.value?.base.value === '1' && window.kentos.cloud.link.value === 'online'`, 60000);
  const fp = await b.eval(`(({ tenantId, projectId, storage }) => ({ tenantId, projectId, storage }))(window.kentos.cloud.project.value)`);
  const fbase = `/v1/tenants/${fp.tenantId}/projects/${fp.projectId}`;
  const ayseFiles = await ayse.call('GET', `${fbase}/files`);
  check('the drawing is a file project whose revision 1 holds it', fp.storage === 'file' && ayseFiles.body.current === '1' && !(await b.eval('window.kentos.doc.dirty.value')), JSON.stringify(ayseFiles.body.revisions?.map((r) => r.revision)));

  // Kaydet twice (Ctrl+S): the stages said apart, “saved” only once the server committed (SYNC-04).
  await b.eval(`(() => { window.__fileStates = []; window.kentos.cloud.file.value.state.subscribe((s) => window.__fileStates.push(s)); })()`);
  const addPoint = (x) => b.eval(`(() => { const d = window.kentos.doc; const layer = d.layers.leaves().find((l) => !d.layers.isLocked(l.id))?.id; d.add({ kind: 'point', layerId: layer, p: { x: ${x}, y: 4420260 }, attrs: {} }); })()`);
  await addPoint(486600);
  await b.key('s', { ctrl: true });
  await b.waitFor(`window.kentos.cloud.file.value.base.value === '2' && window.kentos.cloud.file.value.state.value === 'saved'`, 30000);
  await addPoint(486601);
  await b.key('s', { ctrl: true });
  await b.waitFor(`window.kentos.cloud.file.value.base.value === '3' && window.kentos.cloud.file.value.state.value === 'saved'`, 30000);
  const states = await b.eval('window.__fileStates');
  const stages = states.filter((x, i) => x !== states[i - 1]);
  check('Kaydet says its stages apart and “saved” comes last', JSON.stringify(stages.slice(0, 5)) === JSON.stringify(['pending', 'encoding', 'uploading', 'verifying', 'saved']), stages.join(' → '));
  const saved3 = await b.eval('window.kentos.cloud.file.value.lastSaved.value');
  const down3 = await raw(ayse, 'GET', `${fbase}/files/3`);
  check('a revision downloads as the bytes that were uploaded', down3.status === 200 && sha(down3.bytes) === saved3.sha256 && down3.bytes.length === saved3.size, `${down3.bytes.length} bayt`);
  await themed('cloud-file-saved');

  // Mehmet saves a revision from elsewhere: heard, said, offered; nothing is reloaded by itself.
  await ayse.call('POST', `${fbase}/commands`, { commandName: 'project.share', version: 1, ...fp, requestId: `e2e-${crypto.randomUUID()}`, idempotencyKey: crypto.randomUUID(), expectedVersions: {}, input: { userId: mehmetMe.body.user.id, role: 'editor' } });
  const r4 = await commitAs(mehmet, fp, '3', '3');
  await b.waitFor(`window.kentos.cloud.file.value.newer.value?.revision === '4'`, 8000);
  const outdated = await b.eval(`({ cell: document.querySelector('.status__save')?.textContent, base: window.kentos.cloud.file.value.base.value })`);
  check('another client’s revision is said and offered, not loaded', r4.status === 200 && r4.put === 200 && outdated.base === '3' && /Yeni revizyon: r4/.test(outdated.cell), JSON.stringify(outdated));
  // An edit saved over it: refused (SYNC-06); the question offers a copy, a local file or the newest revision.
  await addPoint(486602);
  await b.key('s', { ctrl: true });
  await b.waitFor(`!!document.querySelector('.dialog[aria-label="Dosya başka biri tarafından kaydedildi"]')`, 15000);
  const asked4 = await b.eval(`(() => { const d = document.querySelector('.dialog[aria-label="Dosya başka biri tarafından kaydedildi"]'); return { answers: [...d.querySelectorAll('.dialog__foot .btn')].map((x) => x.textContent), text: d.textContent }; })()`);
  const kept4 = await ayse.call('GET', `${fbase}/files`);
  check(
    'a Kaydet over another’s revision is refused and asks; nothing is written',
    kept4.body.current === '4' && asked4.answers.join('|') === 'Son revizyonu aç|Vazgeç|Yerel dosyaya kaydet|Ayrı kopya olarak kaydet' && /revizyon 4/.test(asked4.text),
    asked4.answers.join(' | '),
  );
  await themed('cloud-file-conflict');
  await press('.dialog[aria-label="Dosya başka biri tarafından kaydedildi"] .dialog__foot .btn', 'Son revizyonu aç');
  await b.waitFor(`window.kentos.cloud.file.value?.base.value === '4' && !window.kentos.doc.dirty.value`, 30000);
  const r4size = await b.eval('window.kentos.doc.size');
  const r4objects = (await ayse.call('GET', `${fbase}/files`)).body.revisions.find((r) => r.revision === '4')?.objects;
  check('“Son revizyonu aç” opens revision 4 and drops the edit', String(r4size) === r4objects && (await b.eval(`window.kentos.cloud.file.value.state.value`)) === 'saved', `${r4size} nesne`);
  // Once more, answered with a separate copy: the drawing, the edit included, becomes a new file project.
  await addPoint(486603);
  const r5 = await commitAs(mehmet, fp, '4', '4');
  await b.waitFor(`window.kentos.cloud.file.value.newer.value?.revision === '5'`, 8000);
  await b.key('s', { ctrl: true });
  await b.waitFor(`!!document.querySelector('.dialog[aria-label="Dosya başka biri tarafından kaydedildi"]')`, 15000);
  await press('.dialog[aria-label="Dosya başka biri tarafından kaydedildi"] .dialog__foot .btn', 'Ayrı kopya olarak kaydet');
  await b.waitFor(`!!document.querySelector('.cloud-storage input[value=file]:checked')`, 5000);
  await press('.dialog__foot .btn', 'Buluta dosya olarak kaydet');
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(`${fileName} (kopya)`)} && window.kentos.cloud.file.value?.base.value === '1'`, 60000);
  const fileCopy = await b.eval(`({ id: window.kentos.cloud.project.value.projectId, tenantId: window.kentos.cloud.project.value.tenantId, size: window.kentos.doc.size, dirty: window.kentos.doc.dirty.value })`);
  const original = await ayse.call('GET', `${fbase}/files`);
  check('“Ayrı kopya olarak kaydet” makes the drawing, its edit included, a new file project; the original keeps its revisions', r5.status === 200 && fileCopy.size === r4size + 1 && !fileCopy.dirty && original.body.current === '5', `${fileCopy.size} nesne`);

  // ── History and checkpoints (docs/adr/0034, 0038) ──
  // Downloads are written to a stand-in for the save dialog; `window.__disk.bytes` is the file.
  const resetDisk = () =>
    b.eval(`(() => { const disk = (window.__disk = { bytes: null }); window.kentos.files.picker = { save: async (name) => ({ name, getFile: async () => new Blob([disk.bytes ?? new Uint8Array()]), createWritable: async () => { const parts = []; return { write: async (d) => parts.push(d), close: async () => { disk.bytes = new Uint8Array(await new Blob(parts).arrayBuffer()); } }; } }), open: async () => null }; })()`);
  const diskSha = () => b.eval(`crypto.subtle.digest('SHA-256', window.__disk.bytes).then((d) => [...new Uint8Array(d)].map((x) => x.toString(16).padStart(2, '0')).join(''))`);
  const diskObjects = () => b.eval(`window.kentos.files.kcad().then((c) => c.decode(window.__disk.bytes)).then((d) => d.columns.kinds.length)`);
  const setField = (dialog, label, value) =>
    b.eval(`(() => { const i = document.querySelector(${JSON.stringify(`${dialog} [aria-label="${label}"]`)}); i.value = ${JSON.stringify(value)}; i.dispatchEvent(new Event('input')); })()`);
  const pickExact = async (title) => {
    await b.waitFor(`[...document.querySelectorAll('.catalog-row__title')].some((e) => e.textContent === ${JSON.stringify(title)})`, 8000);
    await b.eval(`[...document.querySelectorAll('.catalog-row')].find((r) => r.querySelector('.catalog-row__title')?.textContent === ${JSON.stringify(title)}).click()`);
    await b.waitFor(`document.querySelector('.catalog-row[aria-selected="true"] .catalog-row__title')?.textContent === ${JSON.stringify(title)}`, 5000);
  };
  const historyShown = `!!document.querySelector('.catalog-history__tools') && !document.querySelector('.catalog-details')?.textContent.includes('Geçmiş yükleniyor')`;
  /** The catalog on “Projelerim”, the project titled `title` selected, its history tab showing. */
  const historyOf = async (title) => {
    await openCatalog();
    await showList('Projelerim');
    await pickExact(title);
    await press('.catalog-details__tab', 'Geçmiş');
    await b.waitFor(historyShown, 8000);
  };
  const stamp3 = new Date().toISOString().slice(11, 19);
  const cpDialog = '.dialog[aria-label="Kontrol noktası oluştur"]';
  const rsDialog = '.dialog[aria-label="Yeni proje olarak geri yükle"]';
  await resetDisk();

  // The open file project (the copy): its one revision, named as a checkpoint from the history tab.
  const copyTitle = `${fileName} (kopya)`;
  const copyBase = `/v1/tenants/${fileCopy.tenantId}/projects/${fileCopy.id}`;
  await historyOf(copyTitle);
  const copyHistory = await b.eval(`({ revisions: [...document.querySelectorAll('.catalog-history__row[data-revision]')].map((r) => r.dataset.revision), empty: document.querySelector('.catalog-details').textContent.includes('Henüz kontrol noktası yok') })`);
  check('the open file project’s history: its one revision, no checkpoint yet', copyHistory.revisions.join() === '1' && copyHistory.empty, JSON.stringify(copyHistory));
  await press('.catalog-history__tools .btn', 'Kontrol noktası oluştur');
  await b.waitFor(`!!document.querySelector(${JSON.stringify(cpDialog)})`, 3000);
  const cpName = `Teslim ${stamp3}`;
  await setField(cpDialog, 'Kontrol noktasının adı', cpName);
  await setField(cpDialog, 'Not', 'Belediyeye teslim; e2e');
  const cpForm = await b.eval(`(() => { const s = document.querySelector(${JSON.stringify(`${cpDialog} select`)}); return { revision: s?.value, options: s?.options.length ?? 0 }; })()`);
  check('naming a file project’s checkpoint offers its revisions, the newest chosen', cpForm.revision === '1' && cpForm.options === 1, JSON.stringify(cpForm));
  await themed('cloud-checkpoint-create');
  await press(`${cpDialog} .dialog__foot .btn`, 'Kontrol noktası oluştur');
  const listedPoint = (n) => `[...document.querySelectorAll('.catalog-history__row[data-checkpoint] .catalog-history__title')].some((e) => e.textContent.startsWith(${JSON.stringify(n)}))`;
  await b.waitFor(`!document.querySelector(${JSON.stringify(cpDialog)}) && ${listedPoint(cpName)}`, 15000);
  const cps = await ayse.call('GET', `${copyBase}/checkpoints`);
  const cp = cps.body.checkpoints?.[0];
  check(
    'the checkpoint names revision 1 with its note, and the history lists it',
    cps.status === 200 && cps.body.checkpoints.length === 1 && cp.name === cpName && cp.kind === 'revision' && cp.revision === '1' && cp.note === 'Belediyeye teslim; e2e',
    JSON.stringify(cp && { name: cp.name, kind: cp.kind, revision: cp.revision }),
  );
  await themed('cloud-history');
  // Downloaded from its row: the bytes of the revision it names.
  await press('.catalog-history__row[data-checkpoint] .btn', 'İndir');
  await b.waitFor(`!!window.__disk.bytes`, 15000);
  check('a checkpoint downloads as the bytes it holds', (await diskSha()) === cp.sha256, `${cp.size} bayt`);
  // Restored as a new project: a file project's point is a file project, opened as a copy is; the source stays.
  await press('.catalog-history__row[data-checkpoint] .btn', 'Yeni proje olarak geri yükle');
  await b.waitFor(`!!document.querySelector(${JSON.stringify(rsDialog)})`, 3000);
  const restoredTitle = `${fileName} (geri yüklenen)`;
  await setField(rsDialog, 'Yeni projenin adı', restoredTitle);
  await themed('cloud-restore');
  await press(`${rsDialog} .dialog__foot .btn`, 'Yeni proje olarak geri yükle');
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(restoredTitle)} && window.kentos.cloud.file.value?.base.value === '1' && !window.kentos.doc.dirty.value`, 60000);
  const restoredFile = await b.eval(`({ id: window.kentos.cloud.project.value.projectId, storage: window.kentos.cloud.project.value.storage, size: window.kentos.doc.size })`);
  const copyKept = await ayse.call('GET', `${copyBase}/files`);
  const copyPoints = await ayse.call('GET', `${copyBase}/checkpoints`);
  check(
    'the checkpoint restored as a new file project and opened; the source keeps its revision and checkpoint',
    restoredFile.storage === 'file' && restoredFile.id !== fileCopy.id && restoredFile.size === fileCopy.size && copyKept.body.current === '1' && copyPoints.body.checkpoints.length === 1,
    `${restoredFile.size} nesne`,
  );

  // A database project (the v1 file uploaded above): its checkpoint is a snapshot of its present state.
  const dbTitle = `E2E kimlik ${stamp}`;
  const dbBase = `/v1/tenants/${project.tenantId}/projects/${up1.projectId}`;
  await historyOf(dbTitle);
  const dbHistory = await b.eval(`({ revisions: document.querySelectorAll('.catalog-history__row[data-revision]').length, says: document.querySelector('.catalog-details').textContent.includes('revizyon dosyaları yoktur') })`);
  check('a database project’s history has no revision files, and says so', dbHistory.revisions === 0 && dbHistory.says, JSON.stringify(dbHistory));
  await press('.catalog-history__tools .btn', 'Kontrol noktası oluştur');
  await b.waitFor(`!!document.querySelector(${JSON.stringify(cpDialog)})`, 3000);
  const dbPoint = `Ada ${stamp3}`;
  await setField(cpDialog, 'Kontrol noktasının adı', dbPoint);
  const noRevision = await b.eval(`!document.querySelector(${JSON.stringify(`${cpDialog} select`)})`);
  await press(`${cpDialog} .dialog__foot .btn`, 'Kontrol noktası oluştur');
  await b.waitFor(`!document.querySelector(${JSON.stringify(cpDialog)}) && ${listedPoint(dbPoint)}`, 15000);
  const dbCps = await ayse.call('GET', `${dbBase}/checkpoints`);
  const dbCp = dbCps.body.checkpoints?.find((c) => c.name === dbPoint);
  check('a database project’s checkpoint: its present state as one snapshot, no revision asked', noRevision && dbCp?.kind === 'snapshot' && dbCp.objects === String(expectedIds.length), JSON.stringify(dbCp && { kind: dbCp.kind, objects: dbCp.objects }));
  await themed('cloud-history-database');
  // “.kcad olarak indir” on a database project: one snapshot of the moment, checked against its SHA-256, readable.
  await resetDisk();
  await press('.catalog-details__actions .btn', '.kcad olarak indir');
  await b.waitFor(`!!window.__disk.bytes`, 15000);
  check('“.kcad olarak indir” writes a database project as one readable .kcad', (await diskObjects()) === expectedIds.length, `${expectedIds.length} nesne`);
  // Restored under its default name: a database project, every object under its persistent id; the source stays.
  await press('.catalog-history__row[data-checkpoint] .btn', 'Yeni proje olarak geri yükle');
  await b.waitFor(`!!document.querySelector(${JSON.stringify(rsDialog)})`, 3000);
  await press(`${rsDialog} .dialog__foot .btn`, 'Yeni proje olarak geri yükle');
  const dbRestored = `${dbTitle} (${dbPoint})`;
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(dbRestored)} && window.kentos.cloud.sync.value?.state.value === 'saved'`, 60000);
  const fromPoint = await b.eval(`({ storage: window.kentos.cloud.project.value.storage, uids: [...window.kentos.doc.all()].map((e) => e.uid).sort() })`);
  const sourceIds = (await ayse.call('GET', `${dbBase}/features?limit=5000`)).body.features.map((f) => f.id).sort();
  check('a database checkpoint restored as a new database project with the same ids; the source unchanged', fromPoint.storage === 'database' && same(fromPoint.uids) && same(sourceIds), `${fromPoint.uids.length} nesne`);
  // Removed after a question, by its maker.
  await historyOf(dbTitle);
  await b.waitFor(listedPoint(dbPoint), 8000);
  await press('.catalog-history__row[data-checkpoint] .btn', 'Sil');
  await b.waitFor(`!!document.querySelector('.dialog[aria-label="Kontrol noktasını sil"]')`, 3000);
  await press('.dialog[aria-label="Kontrol noktasını sil"] .dialog__foot .btn', 'Sil');
  await b.waitFor(`!${listedPoint(dbPoint)} && ${historyShown}`, 8000);
  const dbLeft = await ayse.call('GET', `${dbBase}/checkpoints`);
  check('its maker removes the checkpoint after a question', !dbLeft.body.checkpoints.some((c) => c.name === dbPoint), String(dbLeft.body.checkpoints.length));
  await b.key('Escape');

  // The history of a file project that is not open: its revisions, newest first, each downloadable and restorable;
  // “.kcad olarak indir” gives its newest revision's bytes. The catalog must say how it is kept (docs/adr/0039).
  await resetDisk();
  await historyOf(fileName);
  await b.waitFor(`document.querySelectorAll('.catalog-history__row[data-revision]').length === 5`, 8000);
  const revRows = await b.eval(`[...document.querySelectorAll('.catalog-history__row[data-revision]')].map((r) => r.dataset.revision)`);
  check('the history lists the revisions, newest first', revRows.join() === '5,4,3,2,1', revRows.join());
  await themed('cloud-file-history');
  await press('.catalog-details__actions .btn', '.kcad olarak indir');
  await b.waitFor(`!!window.__disk.bytes`, 15000);
  const newest = await raw(ayse, 'GET', `${fbase}/files/5`);
  check('“.kcad olarak indir” writes the newest revision’s bytes', (await diskSha()) === sha(newest.bytes), `${newest.bytes.length} bayt`);
  // Any revision restored as a new project: revision 2, a file project, opened.
  await press('.catalog-history__row[data-revision="2"] .btn', 'Yeni proje olarak geri yükle');
  await b.waitFor(`!!document.querySelector(${JSON.stringify(rsDialog)})`, 3000);
  await press(`${rsDialog} .dialog__foot .btn`, 'Yeni proje olarak geri yükle');
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(`${fileName} (r2)`)} && window.kentos.cloud.file.value?.base.value === '1'`, 60000);
  const r2objects = (await ayse.call('GET', `${fbase}/files`)).body.revisions.find((r) => r.revision === '2')?.objects;
  check('a revision restored as a new file project and opened', String(await b.eval('window.kentos.doc.size')) === r2objects, `${r2objects} nesne`);

  // ── Another storage mode (docs/adr/0039): a new project from this one, opened; the source unchanged ──
  // “PostGIS'e aktar” on the file project: its newest revision, object by object.
  const r5objects = (await ayse.call('GET', `${fbase}/files`)).body.revisions.find((r) => r.revision === '5')?.objects;
  await openCatalog();
  await showList('Projelerim');
  await pickExact(fileName);
  // Its details: the objects of its newest revision (the server counts no rows of a file project).
  await b.waitFor(detailsReady, 8000);
  const fileFacts = await b.eval(`document.querySelector('.catalog-details__facts').textContent`);
  check('a file project’s details count its newest revision’s objects', fileFacts.includes(`${r5objects} nesne (revizyon 5)`) && fileFacts.includes('Dosya projesinde hesaplanmaz'), fileFacts.slice(0, 200));
  await press('.catalog-details__actions .btn', "PostGIS'e aktar");
  const toDb = '.dialog[aria-label="PostGIS\'e aktar"]';
  await b.waitFor(`!!document.querySelector(${JSON.stringify(toDb)})`, 3000);
  await themed('cloud-convert');
  await press(`${toDb} .dialog__foot .btn`, "PostGIS'e aktar");
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(`${fileName} (PostGIS)`)} && window.kentos.cloud.sync.value?.state.value === 'saved'`, 60000);
  const asDb = await b.eval(`({ storage: window.kentos.cloud.project.value.storage, size: window.kentos.doc.size })`);
  check('“PostGIS\'e aktar” makes the newest revision a database project and opens it', asDb.storage === 'database' && String(asDb.size) === r5objects, `${asDb.size} nesne`);
  // “Dosya projesine çevir” on a database project: its present state as revision 1 of a new file project.
  await openCatalog();
  await showList('Projelerim');
  await pickExact(`E2E kimlik (yeniden) ${stamp}`);
  await press('.catalog-details__actions .btn', 'Dosya projesine çevir');
  const toFile = '.dialog[aria-label="Dosya projesine çevir"]';
  await b.waitFor(`!!document.querySelector(${JSON.stringify(toFile)})`, 3000);
  await press(`${toFile} .dialog__foot .btn`, 'Dosya projesine çevir');
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(`E2E kimlik (yeniden) ${stamp} (dosya)`)} && window.kentos.cloud.file.value?.base.value === '1'`, 60000);
  const asFile = await b.eval(`({ storage: window.kentos.cloud.project.value.storage, uids: [...window.kentos.doc.all()].map((e) => e.uid).sort() })`);
  check('“Dosya projesine çevir” makes a database project a file project and opens it', asFile.storage === 'file' && same(asFile.uids), `${asFile.uids.length} nesne`);

  const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l));
  check('no console errors', errors.length === 0, errors.join(' | '));
} catch (e) {
  failures.push(String(e));
  console.error(e);
  // What the page showed when it stopped, for whoever reads the failure.
  await b.shot('cloud-failure').catch(() => {});
  console.error((await b.eval(`document.querySelector('.dialog')?.innerText ?? ''`).catch(() => '')).slice(0, 2000));
} finally {
  b.close();
  await vite.close();
  await stopApi();
  if (scratch && scratch.exitCode === null) {
    scratch.stdin.end();
    await new Promise((r) => scratch.once('exit', r));
  }
}
console.log(failures.length ? `\n${failures.length} kontrol başarısız.` : '\nTüm kontroller geçti.');
process.exit(failures.length ? 1 : 0);
