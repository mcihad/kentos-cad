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
//
//   pnpm e2e:cloud     (needs `pnpm db:setup` once; builds kentosd first)
//
// Projects it creates stay in the development database, named "E2E …"; the
// ones this run deletes are only marked deleted (`kentosd project deleted`).
import { spawn } from 'node:child_process';
import net from 'node:net';
import { readFileSync } from 'node:fs';
import { createServer } from 'vite';
import { fileURLToPath } from 'node:url';
import { launch, sleep } from './cdp.mjs';

/** The repository root: .env.local and the Cargo target directory. */
const ROOT = fileURLToPath(new URL('../../../../', import.meta.url));

const env = Object.fromEntries(
  readFileSync(`${ROOT}.env.local`, 'utf8')
    .split('\n')
    .filter((l) => l && !l.startsWith('#') && l.includes('='))
    .map((l) => [l.slice(0, l.indexOf('=')), l.slice(l.indexOf('=') + 1)]),
);
if (!env.KENTOS_DEV_PASSWORD) throw new Error('.env.local içinde KENTOS_DEV_PASSWORD yok: önce `pnpm db:setup` çalıştırın.');

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
  api = spawn('./target/debug/kentosd', ['serve'], { cwd: ROOT, env: { ...process.env, KENTOS_API_PORT: String(apiPort), KENTOS_PUBLIC_URL: publicUrl, KENTOS_LOG: 'warn' }, stdio: ['ignore', 'ignore', 'inherit'] });
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
});
const mehmet = client();
const zeynep = client();

const failures = [];
const check = (name, ok, detail = '') => {
  console.log(`${ok ? '✓' : '✗'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures.push(name);
};

await startApi();
const b = await launch(url);
try {
  const ready = 'window.kentos && window.kentos.view.backendKind.value';
  await b.waitFor(ready, 20000);
  await sleep(1200);
  await b.waitFor(ready, 20000);
  await b.waitFor(`window.kentos.server.state.value === 'online'`, 8000);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`, 5000);
  const center = (sel, text = '') =>
    b.eval(`(() => { const e = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.textContent.trim().startsWith(${JSON.stringify(text)})); if (!e) return null; const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
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
  const size = await b.eval('window.kentos.doc.size');
  await press('.dialog__foot .btn', 'Buluta yükle');
  await b.waitFor(`window.kentos.cloud.project.value && window.kentos.cloud.sync.value.state.value === 'saved'`, 60000);
  const project = await b.eval('window.kentos.cloud.project.value');
  check('uploads the drawing as a cloud project', project.name === name && !(await b.eval('window.kentos.doc.dirty.value')), `${size} nesne`);
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
  await b.waitFor(`document.querySelector('.cloud-row')`, 5000);
  await press('.cloud-row', name);
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
  const localId = await b.eval(`[...window.kentos.doc.all()].find((e) => e.kind === 'line' && e.b.x === ${X + 120}).id`);
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
  await b.waitFor(`document.querySelector('.cloud-row')`, 5000);
  await sleep(200);
  await b.shot('cloud-projects-large');
  await b.key('Escape');
  await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1')`);

  // Renaming the open project from the list (ayse owns it: project.edit and project.delete are hers).
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(`document.querySelector('.cloud-row')`, 5000);
  const before = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.textContent === 'Sil…'); return e && { disabled: e.disabled, title: e.title }; })()`);
  check('with nothing picked the buttons wait and say why', !!before?.disabled && /seçin/.test(before.title), before?.title);
  await press('.cloud-row', name);
  const del = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.textContent === 'Sil…'); return e && { disabled: e.disabled, title: e.title }; })()`);
  check('the owner may delete her project (the permission comes with the project)', del && !del.disabled, del?.title);
  await press('.dialog__foot .btn', 'Yeniden adlandır');
  await b.waitFor(`document.querySelector('.dialog[aria-label="Bulut projesini yeniden adlandır"]')`, 3000);
  const renamedTo = `${name} (revize)`;
  // The field opens with the old name selected: typing replaces it.
  await b.type(renamedTo);
  await b.key('Enter');
  await b.waitFor(`window.kentos.cloud.project.value?.name === ${JSON.stringify(renamedTo)} && window.kentos.cloud.sync.value.state.value === 'saved'`, 8000).catch(() => {});
  await b.waitFor(`[...document.querySelectorAll('.cloud-row__name')].some((e) => e.textContent === ${JSON.stringify(renamedTo)})`, 5000).catch(() => {});
  const named = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}`);
  check('renaming the open project saves the name for everyone and the list shows it', named.body.name === renamedTo && (await b.eval('window.kentos.doc.name.value')) === renamedTo, named.body.name);
  await b.key('Escape');

  // zeynep (an admin) deletes it while ayse has it open: her app stops saving, the drawing stays, edits stay on the device.
  const signedZ = await zeynep.call('POST', '/v1/auth/login', { login: 'zeynep', password: env.KENTOS_DEV_PASSWORD });
  if (signedZ.status !== 200) throw new Error('zeynep giriş yapamadı: geliştirme verisini yenileyin (`pnpm kentosd -- dev-seed`).');
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
  const spare = await zeynep.call('POST', `/v1/tenants/${project.tenantId}/projects`, {
    name: spareName,
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000 },
    origin: { x: 486500, y: 4420200 },
    layers: [{ id: 'cizim', name: 'Çizim', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'ink', lineType: 'continuous', lineWeight: 0.25 }, children: [] }],
    activeLayer: 'cizim',
    styles: { items: [], categories: [] },
  });
  await b.eval(`window.kentos.commands.execute('cloud.signOut')`);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`, 5000);
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(`document.querySelector('.dialog--cloud input[name=login]')`, 3000);
  await b.type('zeynep');
  await b.key('Tab');
  await b.type(env.KENTOS_DEV_PASSWORD);
  await b.key('Enter');
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedIn' && !!document.querySelector('.cloud-row')`, 8000);
  await press('.cloud-row', spareName);
  await press('.dialog__foot .btn', 'Sil');
  await b.waitFor(`document.querySelector('.dialog[aria-label="Bulut projesini sil"]')`, 3000);
  const asked = await b.eval(`(() => { const d = document.querySelector('.dialog[aria-label="Bulut projesini sil"]'); return { safe: document.activeElement?.textContent, says: d.querySelectorAll('.cloud-consequences li').length }; })()`);
  check('deleting asks first, with the safe button focused', asked.safe === 'Vazgeç' && asked.says >= 3, JSON.stringify(asked));
  await b.shot('cloud-delete-confirm');
  await press('.dialog[aria-label="Bulut projesini sil"] .dialog__foot .btn', 'Projeyi sil');
  await b.waitFor(`!document.querySelector('.dialog[aria-label="Bulut projesini sil"]') && ![...document.querySelectorAll('.cloud-row__name')].some((e) => e.textContent === ${JSON.stringify(spareName)})`, 8000).catch(() => {});
  const spareGone = await zeynep.call('GET', `/v1/tenants/${project.tenantId}/projects/${spare.body.id}`);
  check('the confirmed delete removes it from the list for everyone', spare.status === 201 && spareGone.status === 410, `${spare.status} → ${spareGone.status}`);
  await b.key('Escape');

  // ── Mehmet in the browser: shared with him, opened, his role lowered and raised, then his access taken away. ──
  const ayse = client();
  await ayse.call('POST', '/v1/auth/login', { login: 'ayse', password: env.KENTOS_DEV_PASSWORD });
  const sharedName = `E2E paylaşılan ${new Date().toISOString().slice(0, 19)}`;
  const made = await ayse.call('POST', `/v1/tenants/${project.tenantId}/projects`, {
    name: sharedName,
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000 },
    origin: { x: 486500, y: 4420200 },
    layers: [{ id: 'cizim', name: 'Çizim', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'ink', lineType: 'continuous', lineWeight: 0.25 }, children: [] }],
    activeLayer: 'cizim',
    styles: { items: [], categories: [] },
  });
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
  const listReady = `!!document.querySelector('.cloud-tabs') && !!document.querySelector('.cloud-list > *') && !document.querySelector('.cloud-list')?.textContent.includes('yükleniyor')`;
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedIn' && ${listReady}`, 8000);
  await press('.cloud-tabs .tab', 'Benimle paylaşılanlar');
  await b.waitFor(`[...document.querySelectorAll('.cloud-row--shared')].some((r) => r.textContent.startsWith(${JSON.stringify(sharedName)}))`, 8000);
  const sharedRow = await b.eval(
    `(() => { const r = [...document.querySelectorAll('.cloud-row--shared')].find((x) => x.textContent.startsWith(${JSON.stringify(sharedName)})); return { sub: r.querySelector('.cloud-row__sub').textContent, role: r.querySelector('.cloud-row__role').textContent }; })()`,
  );
  check('“Benimle paylaşılanlar” lists it with its owner and my role', granted.status === 200 && sharedRow.sub.includes('Ayşe Yılmaz') && sharedRow.role === 'Düzenleyici', JSON.stringify(sharedRow));
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await sleep(200);
  await b.shot('cloud-shared-dark');
  await b.eval(`window.kentos.commands.execute('view.theme.light')`);
  await sleep(200);
  await b.shot('cloud-shared-light');
  await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
  await press('.cloud-row--shared', sharedName);
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
  await press('.cloud-tabs .tab', 'Benimle paylaşılanlar');
  await b.waitFor(`!document.querySelector('.cloud-list')?.textContent.includes('yükleniyor')`, 8000);
  const stillListed = await b.eval(`[...document.querySelectorAll('.cloud-row--shared')].some((r) => r.textContent.startsWith(${JSON.stringify(sharedName)}))`);
  const onShared = await b.eval(`document.querySelector('.cloud-tabs [aria-selected="true"]')?.textContent`);
  check('taken away, it leaves “Benimle paylaşılanlar”', onShared === 'Benimle paylaşılanlar' && !stillListed, onShared);
  await b.key('Escape');

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
}
console.log(failures.length ? `\n${failures.length} kontrol başarısız.` : '\nTüm kontroller geçti.');
process.exit(failures.length ? 1 : 0);
