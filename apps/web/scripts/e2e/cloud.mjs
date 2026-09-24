// Cloud end-to-end test (Faz B): the real kentosd against the development
// database, the app in headless Chrome signed in as `ayse`, and `mehmet` as a
// second editor over plain HTTP. Checks sign-in, upload, autosave, reload
// persistence, another editor's change arriving live, a conflict resolved
// from the dialog, a server restart while an edit waits, renaming the open
// project from the list, and deletion: by `zeynep` (an admin) while the
// project is open, and from the list after a confirmation.
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

  await mehmet.call('POST', '/v1/auth/login', { login: 'mehmet', password: env.KENTOS_DEV_PASSWORD });
  const listed = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}`);
  check('another member sees it with every object', listed.status === 200 && Number(listed.body.featureCount) === size, listed.body.featureCount);

  // Draw a line with typed coordinates; autosave sends it without Ctrl+S.
  const X = 486900, N = 4420600;
  await b.eval(`window.kentos.view.camera.fit({ minX: ${X - 50}, minY: ${N - 50}, maxX: ${X + 150}, maxY: ${N + 50} }, 20)`);
  // As in the smoke test: keys go to the drawing; Space opens the command line for a typed point.
  const focusCanvas = () => b.eval('window.kentos.view.focus()');
  const cmd = async (t) => {
    await focusCanvas();
    await b.key(' ');
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
  const lineId = await b.eval(`(() => { const k = window.kentos; const e = [...k.doc.all()].at(-1); return k.cloud.sync.value.featureOf(e.id); })()`);
  let got = await mehmet.call('GET', `/v1/tenants/${project.tenantId}/projects/${project.projectId}/features?ids=${lineId}`);
  const line = got.body.features[0];
  check('autosave stores the line exactly', line?.entity.kind === 'line' && line.entity.a.x === X && line.entity.b.x === X + 100 && line.version === '1', JSON.stringify(line?.entity.b));
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
  const reopened = await b.eval(`(() => { const k = window.kentos; return { size: k.doc.size, line: [...k.doc.all()].find((e) => k.cloud.sync.value.featureOf(e.id) === ${JSON.stringify(lineId)}) }; })()`);
  check('after a reload the cloud project opens with the line', reopened.size === size + 1 && reopened.line?.b.x === X + 100, `${reopened.size} nesne`);

  // Mehmet moves the line's end: the change arrives live, with no unsaved mark.
  const moved = { ...line.entity, b: { x: X + 120, y: N } };
  const r1 = await mehmet.commit(project.tenantId, project.projectId, [{ op: 'update', id: lineId, entity: moved }], { [lineId]: '1' });
  check('the other editor commits', r1.status === 200, r1.body.versions?.[lineId]);
  await b.waitFor(`[...window.kentos.doc.all()].some((e) => e.kind === 'line' && e.b.x === ${X + 120})`, 8000);
  check('another editor’s change arrives live', !(await b.eval('window.kentos.doc.dirty.value')));

  // Both change it at once: the browser gets a conflict, then takes the server's copy.
  const localId = await b.eval(`[...window.kentos.doc.all()].find((e) => e.kind === 'line' && e.b.x === ${X + 120}).id`);
  const r2 = await mehmet.commit(project.tenantId, project.projectId, [{ op: 'update', id: lineId, entity: { ...line.entity, b: { x: X + 140, y: N } } }], { [lineId]: '2' });
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
  check('the waiting edit is saved once when the server is back', got.body.features[0].entity.a.x === X - 10 && got.body.features[0].version === '4', got.body.features[0].version);
  await b.waitFor(`window.kentos.cloud.link.value === 'online'`, 40000);
  check('the live channel reconnects', true);

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

  // Renaming the open project from the list (ayse is a project manager: project.edit, but not project.delete).
  await b.eval(`window.kentos.commands.execute('cloud.open')`);
  await b.waitFor(`document.querySelector('.cloud-row')`, 5000);
  await press('.cloud-row', name);
  const del = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.textContent === 'Sil…'); return e && { disabled: e.disabled, title: e.title }; })()`);
  check('a project manager may not delete; the button says why', !!del?.disabled && /project\.delete/.test(del.title), del?.title);
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

  const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l));
  check('no console errors', errors.length === 0, errors.join(' | '));
} catch (e) {
  failures.push(String(e));
  console.error(e);
} finally {
  b.close();
  await vite.close();
  await stopApi();
}
console.log(failures.length ? `\n${failures.length} kontrol başarısız.` : '\nTüm kontroller geçti.');
process.exit(failures.length ? 1 : 0);
