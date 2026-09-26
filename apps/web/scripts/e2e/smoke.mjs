// End-to-end smoke test: boots the app on its own Vite server, drives it in
// headless Chrome with real mouse/keyboard events and checks the document
// through the dev-only `window.kentos` handle.
//
//   pnpm e2e            (CHROME_BIN overrides the browser binary)
import http from 'node:http';
import { readFileSync } from 'node:fs';
import { createServer } from 'vite';
import { launch, sleep, WEBGPU_ARGS } from './cdp.mjs';

// A stand-in for the KentOS API (apps/api) so the test needs no Rust build: the
// dev server's /v1 forwarding (vite.config.mjs) reaches it through KENTOS_API_PORT.
const api = http.createServer((req, res) => {
  if (req.url !== '/v1/health') return res.writeHead(404).end();
  res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify({ status: 'ok', service: 'kentos-api', version: '0.0.0-e2e', contracts: 1 }));
});
await new Promise((r) => api.listen(0, '127.0.0.1', r));
process.env.KENTOS_API_PORT = String(api.address().port);

// No file watching or hot reload: a file saved while the test runs must not reload the page under it.
const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
const b = await launch(url, { args: WEBGPU_ARGS });
const failures = [];
const check = (name, ok, detail = '') => {
  console.log(`${ok ? '✓' : '✗'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures.push(name);
};

try {
  const ready = 'window.kentos && window.kentos.view.backendKind.value';
  await b.waitFor(ready, 20000);
  await sleep(1200); // first-load dependency optimisation can reload once
  await b.waitFor(ready, 20000);
  check('app boots with a GPU backend', true, await b.eval('window.kentos.view.backendKind.value'));

  // The status bar says whether the API answers; the drawing never waits for it.
  await b.waitFor(`window.kentos.server.state.value !== 'checking'`, 8000).catch(() => {});
  const serverText = () => b.eval(`document.querySelector('.status__server')?.textContent ?? ''`);
  check('status bar shows the API as connected', (await b.eval('window.kentos.server.state.value')) === 'online' && (await serverText()) === 'Sunucu: bağlı', await serverText());
  await new Promise((r) => (api.closeAllConnections(), api.close(r)));
  await b.eval(`window.kentos.commands.execute('server.check')`);
  await b.waitFor(`window.kentos.server.state.value === 'offline'`, 8000).catch(() => {});
  check('without the API it shows “Sunucu: yok” and says why', (await serverText()) === 'Sunucu: yok' && (await b.eval('window.kentos.server.detail.value')) === 'API çalışmıyor.', await b.eval('window.kentos.server.detail.value'));

  const base = await b.eval('window.kentos.doc.size');
  const toScreen = (x, y) =>
    b.eval(`(() => { const k = window.kentos; const s = k.view.camera.worldToScreen({x:${x}, y:${y}}); const r = k.view.clientRect(); return [Math.round(s.x + r.left), Math.round(s.y + r.top)]; })()`);
  // Work east of the sample sheet frame, where the drawing is empty.
  const E = 487060;
  const N = 4420100;
  await b.eval(`window.kentos.view.camera.fit({ minX: ${E - 20}, minY: ${N - 80}, maxX: ${E + 140}, maxY: ${N + 80} }, 20)`);
  await sleep(200);
  const at = async (dx, dy) => toScreen(E + dx, N + dy);
  const added = async () => (await b.eval('window.kentos.doc.size')) - base;
  await b.click(...(await at(120, -70)));

  // Lines + quick trim
  await b.key('l');
  await b.click(...(await at(0, 0)));
  await b.click(...(await at(100, 0)));
  await b.key('Enter');
  await b.click(...(await at(30, -20)));
  await b.click(...(await at(30, 20)));
  await b.key('Enter');
  await b.click(...(await at(70, -20)));
  await b.click(...(await at(70, 20)));
  await b.key('Escape');
  check('line tool draws three lines (no stale snap)', (await added()) === 3, `+${await added()}`);
  await b.key('t', { shift: true });
  await b.move(...(await at(50, 0)));
  await b.click(...(await at(50, 0)));
  await b.key('Escape');
  check('trim splits the crossed line in two', (await added()) === 4, `+${await added()}`);

  // Spline, dimension, text
  await b.key('s');
  for (const [dx, dy] of [[0, 40], [30, 60], [60, 35], [90, 55]]) await b.click(...(await at(dx, dy)));
  await b.key('Enter');
  await b.key('d');
  await b.click(...(await at(0, -40)));
  await b.click(...(await at(90, -40)));
  await b.click(...(await at(45, -32)));
  await b.key('Escape');
  // Text: click, type straight into the field that opens there, Enter.
  await b.key('t');
  await b.click(...(await at(0, -60)));
  const fieldOpen = await b.eval(`!document.querySelector('.inline-text').hidden && document.activeElement === document.querySelector('.inline-text__input')`);
  await b.type('Deneme');
  await b.key('Enter');
  check('text tool opens a focused field where you click', fieldOpen);
  await b.key('Escape');
  const kinds = await b.eval(`[...window.kentos.doc.all()].slice(-3).map(e => e.kind).join(',')`);
  check('spline, dimension and text are created', kinds === 'spline,dimension,text', kinds);

  // Rectangle + hatch inside it
  await b.key('r');
  await b.click(...(await at(110, -60)));
  await b.click(...(await at(135, -20)));
  await b.key('Escape');
  await b.key('h');
  await b.move(...(await at(120, -40)));
  await b.click(...(await at(120, -40)));
  await b.key('Escape');
  check('hatch fills the rectangle', (await b.eval(`[...window.kentos.doc.all()].at(-1).kind`)) === 'hatch');

  // Double-click text → inline editor
  const tid = await b.eval(`[...window.kentos.doc.all()].find(e => e.kind === 'text' && e.text === 'Deneme').id`);
  const tp = await b.eval(`(() => { const t = window.kentos.doc.get(${tid}); return [t.p.x + t.height, t.p.y + t.height * 0.4]; })()`);
  const [tx, ty] = await toScreen(tp[0], tp[1]);
  await b.click(tx, ty);
  await b.click(tx, ty, { clickCount: 2 });
  check('double click opens the inline text editor', await b.eval(`!document.querySelector('.inline-text').hidden`));
  await b.type('Düzenlendi');
  await b.key('Enter');
  const edited = await b.eval(`window.kentos.doc.get(${tid}).text`);
  check('inline edit commits the new text', edited === 'Düzenlendi', edited);

  // Editing tools on exact, typed geometry (a second work area further east).
  const X = E + 200;
  const focusCanvas = () => b.eval('window.kentos.view.focus()');
  const key = async (k, o) => (await focusCanvas(), await b.key(k, o));
  // The command line gets the focus without moving the mouse (Space is Enter now, docs/adr/0018).
  const cmd = async (text) => (await b.eval("window.kentos.commands.execute('commandline.focus')"), await b.type(text), await b.key('Enter'));
  const newest = () => b.eval('[...window.kentos.doc.all()].at(-1)');
  await b.eval(`window.kentos.view.camera.fit({ minX: ${X - 20}, minY: ${N - 80}, maxX: ${X + 140}, maxY: ${N + 80} }, 20)`);
  await key('Escape');

  // Polyline with a tangent arc segment: Y switches to arcs, D back to lines.
  await key('p');
  await cmd(`${X},${N}`);
  await cmd(`${X + 40},${N}`);
  await cmd('Y');
  await cmd(`${X + 40},${N + 30}`);
  await cmd('D');
  await cmd(`${X},${N + 30}`);
  await key('Enter');
  const arcPath = await newest();
  check('polyline arc mode draws a tangent half circle', arcPath.kind === 'polyline' && Math.abs(arcPath.bulges[1] - 1) < 1e-9, JSON.stringify(arcPath.bulges));
  await key('Escape');

  // Explode it, then join the pieces back.
  await b.eval(`window.kentos.selection.set([${arcPath.id}])`);
  await key('x');
  await sleep(50);
  const pieces = await b.eval(`[...window.kentos.selection.ids.value].map((id) => window.kentos.doc.get(id).kind).join(',')`);
  await key('j');
  await sleep(50);
  const rejoined = await b.eval(`window.kentos.doc.get([...window.kentos.selection.ids.value][0])`);
  check('explode and join round-trip a polyline with an arc', pieces === 'line,arc,line' && rejoined.pts.length === 4 && Math.abs(rejoined.bulges[1] - 1) < 1e-9, pieces);
  await b.eval('window.kentos.selection.clear()');

  // Chamfer a rectangle corner (imar köşe kesmesi), then break a line and divide the rest.
  await key('r');
  await cmd(`${X + 70},${N - 60}`);
  await cmd(`${X + 120},${N - 20}`);
  await key('Escape');
  const rect = await newest();
  // Two neighbouring edges, then a typed distance.
  await key('p', { shift: true });
  await b.click(...(await toScreen(X + 95, N - 60)));
  await b.click(...(await toScreen(X + 120, N - 40)));
  await cmd('5');
  await key('Escape');
  const cut = await b.eval(`window.kentos.doc.get(${rect.id}).pts.length`);
  check('chamfer cuts a polygon corner', cut === 5, `${cut} köşe`);
  await key('l');
  await cmd(`${X},${N - 60}`);
  await cmd(`${X + 60},${N - 60}`);
  await key('Escape');
  const brokenLine = await newest();
  await key('b');
  await b.click(...(await toScreen(X + 30, N - 60)));
  await key('Enter');
  await key('Escape');
  // The first half is the line itself (same slot and persistent id, ADR 0014); the second is a new object.
  const halves = await b.eval(`[...window.kentos.doc.all()].slice(-2).map((e) => ({ kind: e.kind, id: e.id, uid: e.uid }))`);
  check(
    'break at one point splits a line in two: the line keeps its id, the other half is new',
    halves.map((h) => h.kind).join(',') === 'line,line' && halves[0].id === brokenLine.id && halves[0].uid === brokenLine.uid && !!halves[1].uid && halves[1].uid !== brokenLine.uid,
    JSON.stringify(halves),
  );

  // Copy / paste back at the original coordinates: a new object with a persistent id of its own.
  await b.eval(`window.kentos.selection.set([${rect.id}])`);
  await key('c', { ctrl: true });
  const beforePaste = await b.eval('window.kentos.doc.size');
  await key('v', { ctrl: true, shift: true });
  const pastedCopy = await newest();
  check('copy and paste-in-place duplicate the selection as a new object', (await b.eval('window.kentos.doc.size')) === beforePaste + 1 && /^[0-9a-f]{8}-[0-9a-f]{4}-7/.test(pastedCopy.uid) && pastedCopy.uid !== rect.uid, pastedCopy.uid);
  await b.eval('window.kentos.selection.clear()');

  // Toolbox: every tool visible without scrolling; a group title folds its tools.
  const box = await b.eval(`(() => { const body = document.querySelector('.toolbox__body'); return { scroll: body.scrollHeight > body.clientHeight, tools: document.querySelectorAll('.toolbox__tool').length }; })()`);
  check('toolbox shows every tool without scrolling', !box.scroll && box.tools === (await b.eval('window.kentos.tools.list().length')), JSON.stringify(box));
  const titleAt = await b.eval(`(() => { const r = document.querySelector('.toolbox__title').getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  await b.click(...titleAt);
  const folded = await b.eval(`document.querySelector('.toolbox__grid').hidden`);
  await b.click(...titleAt);
  check('a toolbox group folds and opens from its title', folded && !(await b.eval(`document.querySelector('.toolbox__grid').hidden`)));

  // Mouse only: the command bar's "Yay" button, then a corner rounded by pulling the mouse.
  const chip = async (label) => {
    const at = await b.eval(`(() => { const el = [...document.querySelectorAll('.cmdbar__opt')].find((x) => x.textContent.startsWith(${JSON.stringify(label)})); if (!el) return null; const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    if (at) await b.click(...at);
    return !!at;
  };
  // Kept below the command bar, which floats over the top of the drawing.
  await key('p');
  await b.click(...(await toScreen(X, N + 40)));
  await b.click(...(await toScreen(X + 30, N + 40)));
  const yay = await chip('Yay');
  await b.move(...(await toScreen(X + 30, N + 25)));
  await b.click(...(await toScreen(X + 30, N + 25)));
  await key('Enter');
  const bulged = await newest();
  check('command bar option buttons work with the mouse', yay && bulged.kind === 'polyline' && (bulged.bulges ?? []).some((x) => Math.abs(x) > 0.5), JSON.stringify(bulged.bulges));
  await key('Escape');
  await key('r');
  await cmd(`${X + 70},${N + 20}`);
  await cmd(`${X + 120},${N + 60}`);
  await key('Escape');
  const box2 = await newest();
  await key('f', { shift: true });
  await b.move(...(await toScreen(X + 120, N + 20)));
  await b.click(...(await toScreen(X + 120, N + 20)));
  await b.move(...(await toScreen(X + 120, N + 28)));
  await b.click(...(await toScreen(X + 120, N + 28)));
  await key('Escape');
  const rounded = await b.eval(`window.kentos.doc.get(${box2.id})`);
  check('fillet: click a corner, pull the mouse, click', rounded.pts.length === 5 && (rounded.bulges ?? []).some((x) => Math.abs(x) > 0.1), `${rounded.pts.length} köşe`);

  // Right button held during a command: menu → one-shot midpoint snap, used by the next click.
  const pressRight = async (x, y, ms) => {
    await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none' });
    await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'right', clickCount: 1 });
    await sleep(ms);
    await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'right', clickCount: 1 });
    await sleep(60);
  };
  const menuRow = (text) => b.eval(`(() => { const row = [...document.querySelectorAll('.menu .menu__item')].find((r) => r.textContent.includes(${JSON.stringify(text)})); if (!row) return null; const r = row.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  await key('l');
  await cmd(`${X + 130},${N - 70}`);
  await cmd(`${X + 130},${N - 30}`);
  await key('Escape');
  await key('l');
  await pressRight(...(await toScreen(X + 60, N + 60)), 450);
  const snapSub = await menuRow('Tek seferlik kenet');
  if (snapSub) {
    await b.move(...snapSub);
    await sleep(250);
    const mid = await menuRow('Orta nokta');
    if (mid) await b.click(...mid);
  }
  await b.move(...(await toScreen(X + 131, N - 49)));
  await b.click(...(await toScreen(X + 131, N - 49)));
  await b.click(...(await toScreen(X + 100, N - 49)));
  const snapped = await newest();
  check('held right button → one-shot midpoint snap', !!snapSub && Math.abs(snapped.a.x - X - 130) < 1e-9 && Math.abs(snapped.a.y - N + 50) < 1e-9, `a=(${(snapped.a.x - X).toFixed(3)}, ${(snapped.a.y - N).toFixed(3)})`);
  await key('Escape');

  // Typing a distance while the mouse is on the drawing opens the field beside the cursor.
  await key('l');
  await b.click(...(await toScreen(X, N - 75)));
  await b.move(...(await toScreen(X + 20, N - 75)));
  await key('1');
  await b.key('2');
  await b.key('.');
  await b.key('5');
  const field = await b.eval(`(() => { const el = document.querySelector('.cursor-input'); return el.hidden ? null : el.querySelector('input').value; })()`);
  await b.key('Enter');
  const typedLine = await newest();
  check('cursor input: typed 12.5 draws 12.5 m towards the mouse', field === '12.5' && Math.abs(Math.hypot(typedLine.b.x - typedLine.a.x, typedLine.b.y - typedLine.a.y) - 12.5) < 1e-9, `alan=${field}`);
  await key('Escape');
  await key('Escape');

  // Grip menu: right click a vertex grip of a selected rectangle deletes that vertex.
  await b.eval(`window.kentos.selection.set([${box2.id}])`);
  await sleep(60);
  const gripBefore = (await b.eval(`window.kentos.doc.get(${box2.id})`)).pts.length;
  await pressRight(...(await toScreen(X + 70, N + 60)), 30);
  const delRow = await menuRow('Köşeyi sil');
  if (delRow) await b.click(...delRow);
  check('grip menu deletes a vertex', (await b.eval(`window.kentos.doc.get(${box2.id})`)).pts.length === gripBefore - 1);
  await b.eval('window.kentos.selection.clear()');

  // Object tracking: rest on a corner, then the point locks exactly level with it.
  await key('l');
  await b.move(...(await toScreen(X + 130, N - 30)));
  await sleep(480);
  const acquired = await b.eval('window.kentos.view.trackPoints.length');
  await b.move(...(await toScreen(X + 100, N - 29.6)));
  await sleep(40);
  await b.click(...(await toScreen(X + 100, N - 29.6)));
  await b.click(...(await toScreen(X + 100, N + 10)));
  const tracked = await newest();
  check('object tracking: resting acquires a point, the next pick is level with it', acquired === 1 && tracked.a.y === N - 30, `${acquired} nokta, y=${(tracked.a.y - N).toFixed(9)}`);
  await key('Escape');

  // Shapes: rotated rectangle (edge, then width), regular polygon, arc continuing from a line.
  const ringArea = (pts) => Math.abs(pts.reduce((acc, p, i) => { const q = pts[(i + 1) % pts.length]; return acc + p.x * q.y - q.x * p.y; }, 0) / 2);
  await key('r', { alt: true });
  await cmd(`${X + 20},${N - 110}`);
  await cmd('@20<45');
  await b.move(...(await toScreen(X + 10, N - 90)));
  await cmd('5');
  const rot = await newest();
  check('rotated rectangle: 20 m edge at 45°, 5 m wide', rot.kind === 'polygon' && Math.abs(ringArea(rot.pts) - 100) < 1e-6, `alan ${ringArea(rot.pts).toFixed(4)}`);
  await key('Escape');
  await key('g', { shift: true });
  await cmd('8');
  await cmd(`${X + 70},${N - 110}`);
  await cmd(`${X + 78},${N - 110}`);
  const oct = await newest();
  check('regular polygon: 8 corners on the circle', oct.pts.length === 8 && oct.pts.every((q) => Math.abs(Math.hypot(q.x - X - 70, q.y - N + 110) - 8) < 1e-9));
  await key('Escape');
  await key('l');
  await cmd(`${X + 100},${N - 110}`);
  await cmd(`${X + 120},${N - 110}`);
  await key('Escape');
  await key('a');
  await chip('Devam');
  await cmd(`${X + 130},${N - 100}`);
  const cont = await newest();
  check('arc continues tangent to the last line', cont.kind === 'arc' && Math.abs(cont.c.x - X - 120) < 1e-9 && Math.abs(cont.r - 10) < 1e-9, `c=(${(cont.c.x - X).toFixed(6)}, ${(cont.c.y - N).toFixed(6)})`);
  await key('Escape');

  // Ellipse (exact crossing snap) and a construction line trimmed into a ray.
  await b.eval(`window.kentos.view.camera.fit({ minX: ${X - 10}, minY: ${N + 85}, maxX: ${X + 90}, maxY: ${N + 150} }, 20)`);
  await sleep(100);
  await key('l', { shift: true });
  await cmd(`${X + 20},${N + 110}`);
  await cmd(`${X + 60},${N + 110}`);
  await cmd('10');
  const el = await newest();
  check('ellipse from an axis and the other half-axis', el.kind === 'ellipse' && Math.abs(Math.hypot(el.major.x, el.major.y) - 20) < 1e-9 && Math.abs(el.ratio - 0.5) < 1e-12);
  await key('Escape');
  await key('x', { shift: true });
  await chip('Yatay');
  await cmd(`${X},${N + 115}`);
  await key('Escape');
  const xl = await newest();
  const cross = 40 - 20 * Math.sqrt(1 - 0.25);
  await key('l');
  await b.move(...(await toScreen(X + cross + 0.15, N + 115.1)));
  await b.click(...(await toScreen(X + cross + 0.15, N + 115.1)));
  await b.click(...(await toScreen(X + cross, N + 140)));
  const onCross = await newest();
  check('snap to xline × ellipse crossing is exact', Math.abs(onCross.a.x - X - cross) < 1e-9 && onCross.a.y === N + 115, `x=${(onCross.a.x - X).toFixed(12)}`);
  await key('Escape');
  await key('t', { shift: true });
  await b.move(...(await toScreen(X + 5, N + 115)));
  await b.click(...(await toScreen(X + 5, N + 115)));
  await key('Escape');
  const rays = await b.eval(`[...window.kentos.doc.all()].filter((e) => e.kind === 'ray' && e.p.y === ${N + 115}).map((e) => e.dir.x)`);
  // What a trim leaves is the object itself (ADR 0014): the xline's slot and persistent id, now a ray.
  const trimmedXl = await b.eval(`window.kentos.doc.get(${xl.id})`);
  check('trimming an xline on one side leaves a ray, the same object', trimmedXl?.kind === 'ray' && trimmedXl.uid === xl.uid && rays.includes(1), JSON.stringify({ rays, kind: trimmedXl?.kind }));

  // Option letters: in a running command a plain letter that is an option triggers it (S: side count).
  await key('g', { shift: true });
  await b.key('s');
  await b.key('7');
  await b.key('Enter');
  await b.click(...(await toScreen(X + 75, N + 95)));
  await b.click(...(await toScreen(X + 82, N + 95)));
  const hept = await newest();
  check('option letter beats the tool shortcut (S → kenar sayısı)', hept.kind === 'polygon' && hept.pts.length === 7, `${hept.pts?.length}`);
  await key('Escape');

  // Point calculator (Netcad's koordinat hesap makinası) inside a running line: yan nokta.
  await key('l');
  await cmd(`${X},${N + 100}`);
  await cmd(`${X},${N + 140}`);
  await key('Escape');
  await key('l');
  await cmd(`${X + 60},${N + 100}`);
  await cmd('YAN');
  await b.move(...(await toScreen(X, N + 100)));
  await b.click(...(await toScreen(X, N + 100)));
  await b.move(...(await toScreen(X, N + 140)));
  await b.click(...(await toScreen(X, N + 140)));
  await cmd('30,5');
  const side = await newest();
  check('point calculator: yan nokta 30/5 feeds the line', Math.abs(side.b.x - X - 5) < 1e-9 && Math.abs(side.b.y - N - 130) < 1e-9, `(${(side.b.x - X).toFixed(9)}, ${(side.b.y - N).toFixed(9)})`);
  await key('Escape');

  await b.eval(`window.kentos.view.camera.fit({ minX: ${X - 20}, minY: ${N - 80}, maxX: ${X + 140}, maxY: ${N + 80} }, 20)`);
  await sleep(100);

  // Dragging a panel splitter must never show an empty (black) viewport frame.
  const frames = [];
  b.on('Page.screencastFrame', (p) => {
    frames.push(p.data);
    b.send('Page.screencastFrameAck', { sessionId: p.sessionId });
  });
  const vr = await b.eval('(() => { const r = window.kentos.view.clientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; })()');
  const [spx, spy] = await b.eval(`(() => { const r = document.querySelector('.shell__right .splitter').getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  await b.send('Page.startScreencast', { format: 'png', everyNthFrame: 1 });
  await sleep(200);
  await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: spx, y: spy, button: 'left', clickCount: 1 });
  for (let i = 1; i <= 25; i++) {
    await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: spx - i * 4, y: spy, button: 'left', buttons: 1 });
    await sleep(16);
  }
  await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: spx - 100, y: spy, button: 'left', clickCount: 1 });
  await sleep(200);
  await b.send('Page.stopScreencast');
  let blackFrames = 0;
  for (const data of frames) {
    const share = await b.eval(`(async () => {
      const img = new Image(); img.src = 'data:image/png;base64,${data}'; await img.decode();
      const c = document.createElement('canvas'); c.width = img.width; c.height = img.height;
      const g = c.getContext('2d'); g.drawImage(img, 0, 0);
      const k = img.width / innerWidth;
      const d = g.getImageData(Math.round((${vr.x} + 120) * k), Math.round((${vr.y} + 20) * k), Math.round((${vr.w} - 400) * k), Math.round((${vr.h} - 40) * k)).data;
      let black = 0;
      for (let i = 0; i < d.length; i += 4) if (d[i] + d[i + 1] + d[i + 2] < 20) black++;
      return black / (d.length / 4);
    })()`);
    if (share > 0.5) blackFrames++;
  }
  check('resizing a panel never flashes a black viewport', frames.length > 5 && blackFrames === 0, `${blackFrames}/${frames.length} kare siyah`);
  await b.eval('window.kentos.ui.dockWidth.set(312)');

  // Layers panel: edits write the object counts into the rows in place instead of rebuilding the tree
  // (CLAUDE.md §6.3). The layer's and its group's counts follow add, undo and redo, and the rows stay the
  // same elements; the eye button hides the layer in the same row without taking the keyboard focus.
  await b.eval(`window.kentos.ui.dockTab.set('layers')`);
  const layerRows = `['kaldirim', 'g-ulasim'].map((id) => document.querySelector('.panel--layers .tree__row[data-id="' + id + '"]'))`;
  const layerCounts = () => b.eval(`${layerRows}.map((r) => Number(r?.querySelector('.tree__count')?.textContent))`);
  await b.eval(`window.__layerRows = ${layerRows}`);
  const [c0, g0] = await layerCounts();
  await b.eval(`(() => { const k = window.kentos; k.doc.transact('Ekle', () => { for (let i = 0; i < 3; i++) k.doc.add({ kind: 'line', layerId: 'kaldirim', a: { x: ${E} + i, y: ${N} - 200 }, b: { x: ${E} + i, y: ${N} - 190 }, attrs: {} }); }); })()`);
  const countsAdded = await layerCounts();
  await b.eval(`window.kentos.commands.execute('edit.undo')`);
  const countsUndone = await layerCounts();
  await b.eval(`window.kentos.commands.execute('edit.redo')`);
  const countsRedone = await layerCounts();
  const sameRows = () => b.eval(`window.__layerRows.every((r, i) => !!r && r === ${layerRows}[i])`);
  check(
    'Layers panel: counts follow add, undo and redo in the same row elements',
    `${countsAdded}` === `${c0 + 3},${g0 + 3}` && `${countsUndone}` === `${c0},${g0}` && `${countsRedone}` === `${c0 + 3},${g0 + 3}` && (await sameRows()),
    `${c0},${g0} → ${countsAdded} → ${countsUndone} → ${countsRedone}`,
  );
  await b.eval(`window.kentos.commands.execute('edit.undo')`);
  const eyeAt = await b.eval(`(() => { const r = window.__layerRows[0].querySelector('.ibtn--row').getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  await b.eval('window.__focusBefore = document.activeElement');
  await b.click(...eyeAt);
  const eyeHidden = await b.eval(`!window.kentos.doc.layers.get('kaldirim').visible && window.__layerRows[0].hasAttribute('data-hidden')`);
  const focusKept = await b.eval('document.activeElement === window.__focusBefore');
  await b.click(...eyeAt);
  check(
    'Layers panel: the eye hides the layer in the same row and leaves the focus',
    eyeHidden && focusKept && (await b.eval(`window.kentos.doc.layers.get('kaldirim').visible && !window.__layerRows[0].hasAttribute('data-hidden')`)) && (await sameRows()) && `${await layerCounts()}` === `${c0},${g0}`,
  );
  await b.eval('delete window.__layerRows; delete window.__focusBefore');
  // A long layer tree builds only the rows near its scroll window (TreeView, CLAUDE.md §6.3): with a
  // 300-layer group the last layer has no row; End scrolls to it, builds it with its count and selects it.
  const virtualTree = await b.eval(`(async () => {
    const k = window.kentos, layers = k.doc.layers;
    const saved = structuredClone(layers.tree), active = layers.active.value;
    const g = layers.add({ name: 'Uzun grup', type: 'group' }, null);
    const ids = Array.from({ length: 300 }, (_, i) => layers.add({ name: 'Uzun ' + i }, g.id).id);
    k.doc.addMany([0, 1].map((i) => ({ kind: 'point', layerId: ids[299], p: { x: ${E} + i, y: ${N} - 300 }, attrs: {} })));
    await new Promise((ok) => setTimeout(ok, 100));
    const tree = document.querySelector('.panel--layers .tree');
    const rows = () => tree.querySelectorAll('.tree__row:not(.tree__probe)').length;
    const rowOf = (id) => tree.querySelector('.tree__row[data-id="' + id + '"]');
    const out = { built: rows(), lastBefore: !!rowOf(ids[299]) };
    tree.focus();
    tree.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true, cancelable: true }));
    await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
    const last = rowOf(ids[299]);
    const r = last?.getBoundingClientRect(), t = tree.getBoundingClientRect();
    Object.assign(out, { selected: last?.getAttribute('aria-selected'), inView: !!r && r.top >= t.top - 1 && r.bottom <= t.bottom + 1, count: last?.querySelector('.tree__count')?.textContent, place: last?.getAttribute('aria-posinset') + '/' + last?.getAttribute('aria-setsize'), after: rows() });
    k.commands.execute('edit.undo');
    layers.reset(saved, active);
    await new Promise((ok) => setTimeout(ok, 50));
    return out;
  })()`);
  check(
    'Layers panel: a long tree builds only the rows near its window; End scrolls to the last and builds it',
    virtualTree.built < 80 && !virtualTree.lastBefore && virtualTree.selected === 'true' && virtualTree.inView && virtualTree.count === '2' && virtualTree.place === '300/300' && virtualTree.after < 130,
    JSON.stringify(virtualTree),
  );

  // Area operations (Alan işlemleri) on fresh squares east of everything else.
  const AX = E + 400;
  await b.eval(`window.kentos.view.camera.fit({ minX: ${AX - 10}, minY: ${N - 60}, maxX: ${AX + 140}, maxY: ${N + 60} }, 20)`);
  await sleep(100);
  const addGeom = (geom) => b.eval(`(() => { const k = window.kentos; let id; k.doc.transact('t', () => { id = k.doc.add({ ...${JSON.stringify(geom)}, layerId: k.doc.layers.active.value, attrs: {} }).id; }); return id; })()`);
  const sq = (x, y, s) => [{ x: AX + x, y: N + y }, { x: AX + x + s, y: N + y }, { x: AX + x + s, y: N + y + s }, { x: AX + x, y: N + y + s }];
  const netOf = (id) => b.eval(`window.kentos.doc.get(${id}) && (() => { const e = window.kentos.doc.get(${id}); const a = (p) => { let s = 0; for (let i = 0, j = p.length - 1; i < p.length; j = i++) s += (p[j].x - p[0].x) * (p[i].y - p[0].y) - (p[i].x - p[0].x) * (p[j].y - p[0].y); return Math.abs(s / 2); }; return a(e.pts) - (e.holes || []).reduce((t, h) => t + a(h.pts), 0); })()`);
  const selected = () => b.eval('[...window.kentos.selection.ids.value]');
  const sqA = await addGeom({ kind: 'polygon', pts: sq(0, 0, 20) });
  const sqB = await addGeom({ kind: 'polygon', pts: sq(10, 10, 20) });
  await b.eval(`window.kentos.selection.set([${sqA}, ${sqB}])`);
  await key('b', { alt: true });
  await sleep(150);
  const united = await selected();
  check('Alt+B unites two squares into one area', united.length === 1 && Math.abs((await netOf(united[0])) - 700) < 1e-6);
  await b.eval(`window.kentos.commands.execute('edit.undo')`);
  const island = await addGeom({ kind: 'polygon', pts: sq(6, 6, 4) });
  await b.eval(`window.kentos.selection.set([${sqA}])`);
  await key('c', { alt: true });
  await b.click(...(await toScreen(AX + 8, N + 8)));
  await key('Enter');
  await sleep(150);
  const [holedId] = await selected();
  const holedE = await b.eval(`window.kentos.doc.get(${holedId})`);
  check('Alt+C with an inner area leaves a hole (adalı alan)', holedE?.holes?.length === 1 && Math.abs((await netOf(holedId)) - 384) < 1e-6 && !!(await b.eval(`window.kentos.doc.get(${island})`)));
  await key('Escape');
  await key('Escape');
  await key('h');
  await b.click(...(await toScreen(AX + 2, N + 2)));
  await sleep(100);
  const hatchE = await b.eval('[...window.kentos.doc.all()].at(-1)');
  check('hatching a holed area leaves its island empty', hatchE.kind === 'hatch' && hatchE.holes?.length === 1);
  await key('Escape');
  for (const [x1, y1, x2, y2] of [[60, -2, 90, -2], [88, -4, 88, 26], [90, 24, 60, 24], [62, 26, 62, -4]]) await addGeom({ kind: 'line', a: { x: AX + x1, y: N + y1 }, b: { x: AX + x2, y: N + y2 } });
  await key('b', { shift: true });
  await b.click(...(await toScreen(AX + 70, N + 10)));
  await sleep(150);
  const [faceId] = await selected();
  check('Shift+B: a click inside crossing lines makes the enclosed area', Math.abs((await netOf(faceId)) - 26 * 26) < 1e-6);
  await key('Escape');
  await b.eval(`window.kentos.doc.remove([${faceId}])`);
  await key('h');
  await key('b');
  await b.click(...(await toScreen(AX + 70, N + 10)));
  await sleep(100);
  const byLines = await b.eval('[...window.kentos.doc.all()].at(-1)');
  const hatchRingArea = (r) => Math.abs(r.reduce((s, p, i) => s + (p.x - r[0].x) * (r[(i + 1) % r.length].y - r[0].y) - (r[(i + 1) % r.length].x - r[0].x) * (p.y - r[0].y), 0)) / 2;
  check('hatch by lines (B) fills the region the crossing lines close', byLines.kind === 'hatch' && Math.abs(hatchRingArea(byLines.ring) - 26 * 26) < 1e-6);
  await key('b');
  await key('Escape');

  // Paralel çizgi (Y): typed distances and axis; Dik çık (O) on its first leg.
  await key('y');
  await key('s');
  await cmd('3');
  await key('a');
  await cmd('4');
  await cmd(`${AX},${N - 40}`);
  await cmd(`${AX + 20},${N - 40}`);
  await cmd(`${AX + 20},${N - 20}`);
  await key('Enter');
  await sleep(100);
  const [pl, pr] = await b.eval('[...window.kentos.doc.all()].slice(-3)');
  const at0 = (p, x, y) => Math.abs(p.x - (AX + x)) < 1e-9 && Math.abs(p.y - (N + y)) < 1e-9;
  check('Paralel çizgi: sides at 3 m and 4 m, mitred at the turn', at0(pl.pts[1], 17, -37) && at0(pr.pts[1], 24, -44), JSON.stringify([pl.pts[1], pr.pts[1]]));
  await key('Escape');
  await key('o');
  await b.click(...(await toScreen(AX + 2, N - 40)));
  await cmd('6');
  await cmd('-5');
  const perp = await newest();
  check('Dik çık: 6 m from the clicked end, 5 m to the left', perp.kind === 'line' && at0(perp.a, 6, -40) && at0(perp.b, 6, -35), JSON.stringify(perp));
  await key('Escape');

  // Dimension styles: linear ΔX locked with X and a typed offset; C picks "Çap" on any keyboard.
  await key('d');
  await key('d');
  await cmd(`${AX},${N - 40}`);
  await cmd(`${AX + 20},${N - 30}`);
  await key('x');
  await cmd('-6');
  const lin = await newest();
  check('linear dimension ΔX (X), typed offset', lin.kind === 'dimension' && lin.style === 'linear' && lin.angle === 90 && lin.offset === -6);
  await key('c');
  check('C selects “Çap (Ç)” instead of the circle tool', (await b.eval('window.kentos.tools.prompt.value')).includes('çapı ölçülecek'));
  await key('h');
  await key('Escape');

  // Kutupsal dizi (typed centre and count) and Uzat-kısalt (typed total length).
  const unit = await addGeom({ kind: 'line', a: { x: AX + 110, y: N - 40 }, b: { x: AX + 115, y: N - 40 } });
  await b.eval(`window.kentos.selection.set([${unit}])`);
  const beforeArray = await b.eval('window.kentos.doc.size');
  await b.eval(`window.kentos.commands.execute('tool.arrayPolar')`);
  await cmd(`${AX + 100},${N - 40}`);
  await key('n');
  await cmd('3');
  await key('Enter');
  check('kutupsal dizi: 3 adet, 2 yeni kopya', (await b.eval('window.kentos.doc.size')) === beforeArray + 2);
  await key('Escape');
  await b.eval(`window.kentos.view.camera.fit({ minX: ${AX + 95}, minY: ${N - 60}, maxX: ${AX + 125}, maxY: ${N - 20} }, 20)`);
  await sleep(100);
  await key('u', { shift: true });
  await b.click(...(await toScreen(AX + 114, N - 40)));
  await cmd('12');
  const longer = await b.eval(`window.kentos.doc.get(${unit})`);
  check('uzat-kısalt: typed total length at the clicked end', Math.abs(longer.b.x - (AX + 122)) < 1e-9 && longer.a.x === AX + 110);
  await key('Escape');

  // Buda with a chosen boundary (S): only that object cuts.
  const hLine = await addGeom({ kind: 'line', a: { x: AX + 100, y: N - 50 }, b: { x: AX + 120, y: N - 50 } });
  await addGeom({ kind: 'line', a: { x: AX + 105, y: N - 55 }, b: { x: AX + 105, y: N - 45 } });
  await addGeom({ kind: 'line', a: { x: AX + 110, y: N - 55 }, b: { x: AX + 110, y: N - 45 } });
  await key('t', { shift: true });
  await key('s');
  await b.click(...(await toScreen(AX + 110, N - 47)));
  await key('Enter');
  await b.click(...(await toScreen(AX + 116, N - 50)));
  const trimmedH = await b.eval(`window.kentos.doc.get(${hLine}) ?? [...window.kentos.doc.all()].filter(e => e.kind === 'line' && e.a.y === ${N - 50} && e.b.y === ${N - 50})[0]`);
  check('buda with a chosen boundary cuts only there', Math.abs(Math.max(trimmedH.a.x, trimmedH.b.x) - (AX + 110)) < 1e-9);
  await key('t');
  await key('Escape');

  // Çoklu çizgi yay seçenekleri: Uzunluk, then an arc by radius.
  await key('p');
  await cmd(`${AX + 100},${N - 70}`);
  await cmd(`${AX + 110},${N - 70}`);
  await key('u');
  await cmd('5');
  await key('y');
  await key('r');
  await cmd('5');
  await cmd(`${AX + 120},${N - 65}`);
  await key('Enter');
  const pw = await newest();
  check('çoklu çizgi: Uzunluk and a Yarıçap arc', pw.kind === 'polyline' && pw.pts[2].x === AX + 115 && Math.abs(pw.bulges[2] - Math.tan(Math.PI / 8)) < 1e-9);
  await key('Escape');

  // Daire TTT (incircle of a 12-9-15 triangle) and Döndür with Referans.
  await b.eval(`window.kentos.view.camera.fit({ minX: ${AX + 95}, minY: ${N - 95}, maxX: ${AX + 125}, maxY: ${N - 70} }, 20)`);
  await sleep(100);
  for (const [x1, y1, x2, y2] of [[100, -90, 112, -90], [112, -90, 100, -81], [100, -81, 100, -90]]) await addGeom({ kind: 'line', a: { x: AX + x1, y: N + y1 }, b: { x: AX + x2, y: N + y2 } });
  await key('c');
  await cmd('TTT');
  await b.click(...(await toScreen(AX + 106, N - 90)));
  await b.click(...(await toScreen(AX + 106, N - 85.5)));
  await b.click(...(await toScreen(AX + 100, N - 85.5)));
  const inc = await newest();
  check('daire TTT: the incircle, r = 3', inc.kind === 'circle' && Math.abs(inc.r - 3) < 1e-9 && Math.abs(inc.c.x - (AX + 103)) < 1e-9);
  await key('Escape');
  const slanted = await addGeom({ kind: 'line', a: { x: AX + 115, y: N - 90 }, b: { x: AX + 118, y: N - 86 } });
  await b.eval(`window.kentos.selection.set([${slanted}])`);
  await key('r', { shift: true });
  await cmd(`${AX + 115},${N - 90}`);
  await key('r');
  await cmd(`${AX + 115},${N - 90}`);
  await cmd(`${AX + 118},${N - 86}`);
  await cmd('90');
  const turned = await b.eval(`window.kentos.doc.get(${slanted})`);
  check('döndür Referans: the line turns onto north', Math.abs(turned.b.x - (AX + 115)) < 1e-9 && Math.abs(turned.b.y - (N - 85)) < 1e-9);

  // Move, copy, mirror, array and paste: the geometry store moves its own copies and only the
  // new geometry comes back (packed, not JSON). Typed points, exact coordinates, every other
  // field kept, one undo step each.
  const MX = AX + 200;
  await b.eval(`window.kentos.view.camera.fit({ minX: ${MX - 10}, minY: ${N - 30}, maxX: ${MX + 140}, maxY: ${N + 80} }, 20)`);
  await sleep(100);
  const parcel = await b.eval(`(() => { const k = window.kentos; let id; k.doc.transact('t', () => { id = k.doc.add({ kind: 'polygon', layerId: k.doc.layers.active.value, attrs: { Ada: '104', Parsel: '7' }, label: '7', color: '#aa3322', pts: [{ x: ${MX}, y: ${N} }, { x: ${MX + 10.25}, y: ${N} }, { x: ${MX + 10.25}, y: ${N + 8.5} }, { x: ${MX}, y: ${N + 8.5} }], bulges: [0, 0.25, 0, 0] }).id; }); return id; })()`);
  const bow = await addGeom({ kind: 'arc', c: { x: MX + 5, y: N + 20 }, r: 3, a0: 0.5, a1: 2 });
  const both = () => b.eval(`[${parcel}, ${bow}].map((id) => window.kentos.doc.get(id))`);
  const start = await both();
  await b.eval(`window.kentos.selection.set([${parcel}, ${bow}])`);
  await key('m', { shift: true });
  await cmd(`${MX},${N}`);
  await cmd('@12.5,-7.25');
  const [mp, ma] = await both();
  check(
    'move: a typed displacement lands exactly, every other field kept',
    mp.pts.every((p, i) => p.x === start[0].pts[i].x + 12.5 && p.y === start[0].pts[i].y - 7.25) && JSON.stringify(mp.bulges) === '[0,0.25,0,0]' && mp.label === '7' && mp.color === '#aa3322' && mp.attrs.Parsel === '7' && ma.c.x === start[1].c.x + 12.5 && ma.r === 3 && Math.abs(ma.a0 - 0.5) < 1e-9 && Math.abs(ma.a1 - 2) < 1e-9,
    JSON.stringify([mp.pts[2], ma]),
  );
  await key('z', { ctrl: true });
  const undoneMove = JSON.stringify(await both()) === JSON.stringify(start);
  await key('y', { ctrl: true });
  check('move: one undo step takes both back, redo moves them again', undoneMove && JSON.stringify(await both()) === JSON.stringify([mp, ma]));
  const beforeCopy = await b.eval('window.kentos.doc.size');
  await b.eval(`window.kentos.selection.set([${parcel}])`);
  await key('c', { shift: true });
  await cmd(`${MX},${N}`);
  await cmd('@0,20');
  await cmd('@0,40');
  await key('Escape');
  const copies = await b.eval('[...window.kentos.doc.all()].slice(-2)');
  const ownAttrs = await b.eval(`[...window.kentos.doc.all()].slice(-2).every((e) => e.attrs !== window.kentos.doc.get(${parcel}).attrs)`);
  check(
    'copy: copies at typed offsets with the parcel’s fields and their own attributes',
    (await b.eval('window.kentos.doc.size')) === beforeCopy + 2 && copies.every((c, k) => c.id !== parcel && c.kind === 'polygon' && c.pts.every((p, i) => p.x === mp.pts[i].x && p.y === mp.pts[i].y + 20 * (k + 1)) && c.label === '7' && c.color === '#aa3322' && c.attrs.Ada === '104') && ownAttrs,
  );
  await b.eval(`window.kentos.selection.set([${bow}])`);
  await key('i', { shift: true });
  await cmd(`${MX + 60},${N}`);
  await cmd(`${MX + 60},${N + 10}`);
  const mirrored = await newest();
  check('mirror: the arc lands across the axis, still counter-clockwise', mirrored.kind === 'arc' && mirrored.id !== bow && Math.abs(mirrored.c.x - (2 * (MX + 60) - ma.c.x)) < 1e-9 && mirrored.c.y === ma.c.y && Math.abs(mirrored.a0 - (Math.PI - 2)) < 1e-8 && Math.abs(mirrored.a1 - (Math.PI - 0.5)) < 1e-8, JSON.stringify(mirrored));
  const beforeGrid = await b.eval('window.kentos.doc.size');
  await b.eval(`window.kentos.selection.set([${parcel}])`);
  await key('a', { shift: true });
  await cmd('2,3');
  await cmd('15,12');
  const grid = await b.eval('[...window.kentos.doc.all()].slice(-5)');
  const cells = [[15, 0], [30, 0], [0, 12], [15, 12], [30, 12]];
  check('array 2 × 3: five copies on the grid', (await b.eval('window.kentos.doc.size')) === beforeGrid + 5 && grid.every((c, k) => c.pts.every((p, i) => p.x === mp.pts[i].x + cells[k][0] && p.y === mp.pts[i].y + cells[k][1])));
  await key('z', { ctrl: true });
  check('array: one undo step takes all five back', (await b.eval('window.kentos.doc.size')) === beforeGrid);
  await b.eval(`window.kentos.selection.set([${parcel}, ${bow}])`);
  await key('c', { ctrl: true });
  const pasteBase = await b.eval('window.kentos.clipboard.get().base');
  const beforePasteTool = await b.eval('window.kentos.doc.size');
  await key('v', { ctrl: true });
  await b.move(...(await toScreen(MX + 90, N + 50)));
  await cmd(`${MX + 100},${N + 50}`);
  const pasted = await b.eval('[...window.kentos.selection.ids.value].map((id) => window.kentos.doc.get(id))');
  const [pdx, pdy] = [MX + 100 - pasteBase.x, N + 50 - pasteBase.y];
  check(
    'paste: the copies land by the base point, fields kept',
    (await b.eval('window.kentos.doc.size')) === beforePasteTool + 2 && pasted.length === 2 && pasted[0].pts.every((p, i) => p.x === mp.pts[i].x + pdx && p.y === mp.pts[i].y + pdy) && pasted[0].attrs.Parsel === '7' && pasted[0].label === '7' && pasted[1].kind === 'arc' && pasted[1].c.x === ma.c.x + pdx && pasted[1].r === 3,
    `${pdx}, ${pdy}`,
  );
  await key('z', { ctrl: true });
  check('paste: one undo step', (await b.eval('window.kentos.doc.size')) === beforePasteTool);
  await b.eval('window.kentos.selection.clear()');

  // A letter that is neither an option nor a shortcut starts the command line, as in AutoCAD and
  // on the desktop (docs/adr/0021): K is free, so “KA” and Enter typed on the drawing start Kapalı alan.
  await key('k');
  const lineText = await b.eval(`document.activeElement?.closest('.cmdline') ? document.activeElement.value : null`);
  await b.key('a');
  await b.key('Enter');
  const started = await b.eval('window.kentos.tools.activeId.value');
  check('a free letter on the drawing starts the command line (KA → Kapalı alan)', lineText === 'k' && started === 'polygon', `${lineText} → ${started}`);
  await key('Escape');

  // Drawing engines: WebGL2 by default; WebGPU switched live from the status
  // bar must draw the same scene. Pixels are read straight after a frame.
  check('WebGL2 is the default engine', (await b.eval('window.kentos.view.backendKind.value')) === 'webgl2');
  const inked = () =>
    b.eval(`(() => {
      const v = window.kentos.view; v.glQueued = true; v.frame();
      const c = document.querySelector('.viewport__gl');
      const o = document.createElement('canvas'); o.width = c.width; o.height = c.height;
      const g = o.getContext('2d'); g.drawImage(c, 0, 0);
      const d = g.getImageData(0, 0, o.width, o.height).data;
      const bg = v.palette.background.map((c) => c * 255);
      let n = 0;
      for (let i = 0; i < d.length; i += 4) if (Math.abs(d[i] - bg[0]) + Math.abs(d[i + 1] - bg[1]) + Math.abs(d[i + 2] - bg[2]) > 30) n++;
      return n;
    })()`);
  // The sample sheet, not everything: the symbol catalogue below it grows with the library.
  await b.eval(`window.kentos.view.camera.fit(window.kentos.doc.homeView)`);
  // The faint full-screen grid is all antialiasing; engines differ there only by sampling.
  const gridWasOn = await b.eval('window.kentos.settings.grid.value');
  await b.eval('window.kentos.settings.grid.set(false)');
  await sleep(100);

  // Style engine: a layer renderer (hatch, centroid text from an expression)
  // and a system library symbol on one object; the engine comparison below
  // then runs on this styled scene.
  const plainInk = await inked();
  await b.eval(`(() => {
    const L = window.kentos.doc.layers;
    const txt = { type: 'marker', layers: [{ id: 't', type: 'text', text: { expr: "'P' || Parsel" }, size: 2.5, color: '#FFFFFF', weight: 600 }] };
    L.setStyle('parsel', { renderer: { type: 'categorized', expr: 'Nitelik', categories: [
      { value: 'Arsa', label: 'Arsa', symbols: { fill: { type: 'fill', layers: [
        { id: 'h', type: 'hatchFill', angle: 45, spacing: 1.5, width: 0.1, color: '#C9A227' },
        { id: 'o', type: 'simpleLine', color: '#E0E0E0', width: 0.2 },
        { id: 'c', type: 'centroidMarker', marker: txt } ] } } } ],
      other: { fill: { ref: 'temel.alan.capraz' } } } });
  })()`);
  const axis = await b.eval(`window.kentos.doc.byLayer('yol-ekseni').find((e) => e.kind === 'line' || e.kind === 'polyline').id`);
  await b.eval(`window.kentos.doc.update(${axis}, { symbol: 'temel.cizgi.oklu' })`);
  await sleep(300);
  const styledInk = await inked();
  check('style: renderer and symbol add ink (hatches, arrows, text)', styledInk > 1.3 * plainInk, `${styledInk} / ${plainInk} px`);
  check('style: text markers are drawn into the atlas', (await b.eval('window.kentos.view.atlas.entries.size')) > 0);
  await key('z', { ctrl: true });
  check('style: an object symbol is one undo step', (await b.eval(`window.kentos.doc.get(${axis}).symbol ?? null`)) === null);
  await b.eval(`window.kentos.doc.update(${axis}, { symbol: 'temel.cizgi.oklu' })`);
  await sleep(100);

  const glInk = await inked();
  const gpuReady = await b.eval('(async () => !!(await navigator.gpu?.requestAdapter()))()');
  if (!gpuReady) console.log('– WebGPU denetimleri atlandı: bu tarayıcıda WebGPU bağdaştırıcısı yok.');
  else {
    await b.click(...(await b.eval(`(() => { const r = document.querySelector('.status__renderer').getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; })()`)));
    await sleep(150);
    const gpuItem = await b.eval(`(() => { const t = [...document.querySelectorAll('.menu [role^=menuitem]')].find((e) => e.textContent.includes('WebGPU')); if (!t) return null; const r = t.getBoundingClientRect(); return [r.left + 20, r.top + r.height / 2]; })()`);
    if (gpuItem) await b.click(...gpuItem);
    await b.waitFor(`window.kentos.view.backendKind.value === 'webgpu'`, 10000).catch(() => {});
    check('status bar switches to WebGPU live', (await b.eval(`window.kentos.view.backendKind.value + '|' + document.querySelector('.status__renderer').textContent`)) === 'webgpu|WebGPU');
    const gpuInk = await inked();
    check('WebGPU draws the same scene as WebGL2', gpuInk > 0.85 * glInk && gpuInk < 1.15 * glInk, `${gpuInk} / ${glInk} px`);
    await b.eval(`window.kentos.commands.execute('view.renderer.webgl2')`);
    await b.waitFor(`window.kentos.view.backendKind.value === 'webgl2'`, 10000).catch(() => {});
    check('switching back to WebGL2 keeps one canvas', (await b.eval(`window.kentos.view.backendKind.value + '|' + document.querySelectorAll('.viewport__gl').length`)) === 'webgl2|1');
  }
  await b.eval(`window.kentos.settings.grid.set(${gridWasOn})`);
  await b.eval(`(() => { const k = window.kentos; k.doc.layers.setStyle('parsel', { renderer: undefined }); k.doc.update(${axis}, { symbol: undefined }); })()`);

  // Style windows: the manager lists the library, the designer saves an edited
  // copy of a system symbol, the layer style window classifies and applies.
  {
    const center = (sel, text = '') =>
      b.eval(`(() => { const e = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.textContent.trim().includes(${JSON.stringify(text)})); if (!e) return null; e.scrollIntoView({ block: 'nearest' }); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    const press = async (sel, text) => {
      const p = await center(sel, text);
      if (!p) throw new Error(`bulunamadı: ${sel} ${text ?? ''}`);
      await b.click(...p);
      await sleep(250);
    };
    await b.eval(`window.kentos.commands.execute('style.manager')`);
    await sleep(500);
    await press('.tree__row', 'Temel');
    await press('.tree__row', 'Alanlar');
    await press('.scard', 'Tarama 45');
    check('style manager: tree, pictures and details', (await b.eval(`document.querySelectorAll('.dialog--styles .scard canvas').length`)) >= 5 && !!(await center('.smgr__actions .btn', 'Kopyasını düzenle')));
    await press('.smgr__actions .btn', 'Kopyasını düzenle');
    await sleep(600);
    await b.eval(`(() => { const lab = [...document.querySelectorAll('.sdf__label')].find((l) => l.textContent === 'Açı'); const inp = lab.parentElement.querySelector('input'); inp.focus(); inp.select(); })()`);
    await b.type('135');
    await press('.dialog--sdesign .btn--primary', 'Kaydet');
    const copy = await b.eval(`window.kentos.styles.library.items('user').find((i) => i.name.startsWith('Tarama 45'))`);
    check('symbol designer saves an edited copy in the user library', copy?.symbol.layers[0].angle === 135, JSON.stringify(copy?.symbol.layers[0]));
    await b.key('Escape');
    await sleep(200);
    await b.key('Escape');
    await sleep(200);
    await b.eval(`window.kentos.doc.layers.setActive('parsel')`);
    await b.eval(`window.kentos.commands.execute('style.layerStyle')`);
    await sleep(500);
    await press('.seg__opt', 'Kategorili');
    await press('.lsty__panel .btn', 'Değerlerden sınıfla');
    await press('.dialog--lstyle .btn--primary', 'Tamam');
    const r = await b.eval(`window.kentos.doc.layers.get('parsel').style.renderer`);
    check('layer style window classifies by value and applies', r?.type === 'categorized' && r.categories.length > 1, `${r?.type} ${r?.categories?.length}`);
    await b.eval(`(() => { const k = window.kentos; k.doc.layers.setStyle('parsel', { renderer: undefined }); k.styles.library.remove(${JSON.stringify(copy?.id ?? '')}); k.doc.layers.setActive('taslak'); })()`);

    // SVG editor: draw a rectangle by dragging, save it as a drawing of the user's library.
    await b.eval(`window.kentos.commands.execute('style.svgEditor')`);
    await sleep(600);
    const docPt = (x, y) => b.eval(`(() => { const svg = document.querySelector('.svge__svg'); const m = svg.firstElementChild.getCTM(); const r = svg.getBoundingClientRect(); return [Math.round(r.left + m.e + ${x} * m.a), Math.round(r.top + m.f + ${y} * m.d)]; })()`);
    await b.eval(`document.querySelector('.svge__stage').focus()`);
    await b.key('r');
    const [ax, ay] = await docPt(20, 20);
    const [bx, by] = await docPt(80, 60);
    await b.drag(ax, ay, bx, by);
    await press('.dialog--svge .btn--primary', 'Kaydet');
    const drawn = await b.eval(`window.kentos.styles.library.items('user').find((i) => i.kind === 'asset' && i.name === 'Yeni çizim')`);
    check('SVG editor draws a rectangle and saves it as a library drawing', !!drawn && /<rect[^>]*width="60"[^>]*height="40"/.test(drawn.data), drawn?.data?.slice(0, 120));
    await b.key('Escape');
    await sleep(200);
    if (drawn) await b.eval(`window.kentos.styles.library.remove(${JSON.stringify(drawn?.id ?? '')})`);
    // Closes the editor the way a user does: Esc (selection, then the window), without saving when asked.
    const closeEditor = async () => {
      for (let i = 0; i < 5 && (await b.eval(`!!document.querySelector('.dialog--svge')`)); i++) {
        const leave = await center('.dialog--confirm .btn', 'Kaydetmeden kapat');
        if (leave) await b.click(...leave);
        else {
          await b.eval(`document.querySelector('.svge__stage')?.focus()`);
          await b.key('Escape');
        }
        await sleep(150);
      }
    };
    await closeEditor();

    // SVG editor files: paste markup (group transform, CSS class), export and read back, trace a bitmap, edit the source.
    await b.eval(`window.kentos.commands.execute('style.svgEditor')`);
    await sleep(600);
    await b.eval(`window.__blobs = []; { const o = URL.createObjectURL; URL.createObjectURL = (x) => { window.__blobs.push(x); return o.call(URL, x); }; }`);
    const svgShapes = () => b.eval(`[...document.querySelectorAll('.svge__svg [data-id]')].map((e) => ({ tag: e.tagName, x: e.getAttribute('x'), w: e.getAttribute('width'), fill: e.getAttribute('fill'), d: e.getAttribute('d'), rule: e.getAttribute('fill-rule') }))`);
    const pasteSvg = (text) => b.eval(`(() => { const dt = new DataTransfer(); dt.setData('text/plain', ${JSON.stringify(text)}); const st = document.querySelector('.svge__stage'); st.focus(); st.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true })); })()`);
    await pasteSvg(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 50 50"><style>.k { fill: #E0457B }</style><g transform="translate(10 5) scale(2)"><rect class="k" width="5" height="5"/><circle cx="10" cy="10" r="2"/></g></svg>`);
    await sleep(200);
    let sh = await svgShapes();
    check('SVG editor pastes markup with a group transform and a CSS class', sh.length === 2 && sh[0].x === '20' && sh[0].w === '20' && sh[0].fill === '#E0457B', JSON.stringify(sh));
    await press('.svge__pbar .btn', 'Dışa aktar');
    await press('.dialog--svgfile .seg__opt', 'SVG (düz renk)');
    await press('.dialog--svgfile .btn--primary', 'İndir');
    const exported = await b.eval(`window.__blobs.at(-1)?.text() ?? ''`);
    await pasteSvg(exported);
    await sleep(200);
    sh = await svgShapes();
    check('SVG editor exports a plain SVG that reads back the same', !/currentColor|param\(/.test(exported) && sh.length === 4 && sh[2].x === '20' && sh[2].fill === '#E0457B', `${sh.length} şekil`);
    await b.eval(`(async () => {
      const c = document.createElement('canvas'); c.width = 100; c.height = 100; const g = c.getContext('2d');
      g.fillStyle = '#fff'; g.fillRect(0, 0, 100, 100); g.fillStyle = '#000'; g.fillRect(20, 20, 60, 60); g.fillStyle = '#fff'; g.fillRect(40, 40, 20, 20);
      const blob = await new Promise((ok) => c.toBlob(ok, 'image/png'));
      const dt = new DataTransfer(); dt.items.add(new File([blob], 'kare.png', { type: 'image/png' }));
      const st = document.querySelector('.svge__stage'); const r = st.getBoundingClientRect();
      st.dispatchEvent(new DragEvent('drop', { dataTransfer: dt, bubbles: true, cancelable: true, clientX: r.left + 100, clientY: r.top + 100 }));
    })()`);
    await sleep(300);
    await press('.menu__item', 'Bitmap izle');
    await sleep(500);
    const traceStats = await b.eval(`document.querySelector('.svgt__stats')?.textContent ?? ''`);
    await press('.dialog--svgtrace .btn--primary', 'Çizime ekle');
    const traced = (await svgShapes()).at(-1);
    check('SVG editor traces a bitmap into a path with its hole', /^1 parça, 1 delik, 8 düğüm/.test(traceStats) && traced?.tag === 'path' && (traced.d.match(/M/g) ?? []).length === 2 && traced.rule === 'evenodd', `${traceStats} ${traced?.d?.slice(0, 40)}`);
    await press('.svge__pbar .btn', 'Kaynak');
    await b.eval(`(() => { const t = document.querySelector('.svgs__text'); t.value = t.value.replace(/(<rect[^>]*?)width="20"/, '$1width="30"'); t.dispatchEvent(new Event('input')); })()`);
    await press('.svgs__head .btn', 'Uygula');
    check('SVG editor applies an edited source as one step', (await svgShapes())[0].w === '30');
    await closeEditor();
    check('SVG editor closes without saving when asked', !(await b.eval(`!!document.querySelector('.dialog--svge')`)));

    // SVG editor editing: a union of two overlapping rectangles from the Yol menu, fillets on corner nodes
    // (dragged and typed), align left in the Hizala tab, a polar array from the Dizi tab as one undo step.
    await b.eval(`window.kentos.commands.execute('style.svgEditor')`);
    await sleep(600);
    await b.eval(`document.querySelector('.svge__stage').focus()`);
    const byLabel = async (label) => {
      const p = await center(`[aria-label="${label}"]`);
      if (!p) throw new Error(`bulunamadı: ${label}`);
      await b.click(...p);
      await sleep(150);
    };
    const pathD = async () => (await svgShapes()).find((x) => x.tag === 'path')?.d ?? '';
    const nodeCount = (d) => (d.match(/[MLC]/g) ?? []).length;
    await b.key('r');
    await b.drag(...(await docPt(10, 10)), ...(await docPt(50, 50)));
    await b.key('r');
    await b.drag(...(await docPt(30, 30)), ...(await docPt(70, 70)));
    await b.key('a', { ctrl: true });
    await press('.svge__pbar .btn', 'Yol');
    await press('.menu__item', 'Birleşim');
    sh = await svgShapes();
    const union = await pathD();
    check('SVG editor unites two overlapping rectangles into one eight-node path', sh.length === 1 && sh[0].tag === 'path' && nodeCount(union) === 8 && /M?10 10/.test(union) && /70 70/.test(union), union);
    check('SVG editor gives the keys back to the canvas after a menu choice', await b.eval(`document.activeElement === document.querySelector('.svge__stage')`));
    await b.eval(`document.querySelector('.svge__stage').focus()`);
    await b.key('a');
    await byLabel('Köşe yuvarla: köşeye basıp çekin');
    await b.drag(...(await docPt(10, 10)), ...(await docPt(16, 10)));
    const filleted = await pathD();
    check('SVG editor rounds a corner node by dragging along its side', nodeCount(filleted) === 9 && /C/.test(filleted) && !/M10 10|L10 10/.test(filleted), filleted);
    await byLabel('Köşe yuvarla: köşeye basıp çekin');
    await b.click(...(await docPt(70, 70)));
    await b.eval(`(() => { const i = document.querySelector('[aria-label="Yarıçap ya da pah boyu"]'); i.focus(); i.select(); })()`);
    await b.type('5');
    await byLabel('Seçili köşeleri bu yarıçapla yuvarla');
    const typed = await pathD();
    check('SVG editor rounds a chosen corner by a typed radius', nodeCount(typed) === 10 && /65 70/.test(typed) && /70 65/.test(typed), typed);
    await b.eval(`document.querySelector('.svge__stage').focus()`);
    await b.key('Escape');
    await b.key('Escape');
    await b.key('r');
    await b.drag(...(await docPt(40, 75)), ...(await docPt(60, 90)));
    await b.key('a', { ctrl: true });
    await press('.svgp__tab', 'Hizala');
    await byLabel('Sol kenarlar');
    const aligned = (await svgShapes()).find((x) => x.tag === 'rect');
    check('SVG editor aligns left edges to the selection', aligned?.x === '10', JSON.stringify(aligned));
    await b.click(...(await docPt(20, 82)));
    await press('.svgp__tab', 'Dizi');
    await press('.seg__opt', 'Dairesel');
    await press('.svgp__foot .btn', 'Uygula');
    const arrayed = await b.eval(`[...document.querySelectorAll('.svge__svg > g:first-child [data-id]')].map((e) => e.getAttribute('transform') ?? '')`);
    const turns = arrayed.filter((t) => /^rotate\((60|120|180|240|300) 50 50\)$|^rotate\(-?\d+(\.\d+)? /.test(t));
    check('SVG editor makes a polar array round the canvas centre', arrayed.length === 7 && turns.length === 5, JSON.stringify(arrayed));
    await b.eval(`document.querySelector('.svge__stage').focus()`);
    await b.key('z', { ctrl: true });
    check('SVG editor takes the array back in one undo step', (await svgShapes()).length === 2);
    // The editor in both themes and at the "Büyük" size, a path's nodes on show; the preview ink follows the theme.
    const theme0 = await b.eval('window.kentos.ui.theme.value');
    await b.click(...(await docPt(12, 30)));
    await b.key('a');
    const rectFill = () => b.eval(`document.querySelector('.svge__svg > g:first-child rect[data-id]')?.getAttribute('fill') ?? ''`);
    await b.eval(`window.kentos.commands.execute('view.theme.dark')`);
    await sleep(200);
    const inkDark = await rectFill();
    await b.shot('smoke-svg-edit-dark');
    await b.eval(`window.kentos.commands.execute('view.theme.light')`);
    await sleep(200);
    const inkLight = await rectFill();
    await b.shot('smoke-svg-edit-light');
    await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1.08')`);
    await sleep(200);
    await b.shot('smoke-svg-edit-large');
    await b.eval(`document.documentElement.style.setProperty('--ui-scale', '1'); window.kentos.commands.execute(${JSON.stringify(`view.theme.${theme0}`)})`);
    check('SVG editor preview ink follows the theme', !!inkDark && !!inkLight && inkDark !== inkLight, `${inkDark} → ${inkLight}`);
    await b.eval(`document.querySelector('.svge__stage').focus()`);
    await b.key('Escape');
    await b.key('Escape');
    await closeEditor();

    // Clicks on the SVG canvas. The editor counts double clicks itself (the canvas captures the pointer and
    // re-renders on a press, so the browser fires no dblclick): a double click ends a polyline, opens a path's
    // nodes, turns a node smooth and back, adds a node on a segment. A still click on one of two chosen shapes
    // narrows the choice and moves nothing (no snap to the grid), with no undo step.
    await b.eval(`window.kentos.commands.execute('style.svgEditor')`);
    await sleep(600);
    const mouse = (type, [x, y], buttons = 0) => b.send('Input.dispatchMouseEvent', { type, x, y, button: type === 'mouseMoved' && !buttons ? 'none' : 'left', buttons, clickCount: 1 });
    const stroke = async (from, to) => {
      await mouse('mouseMoved', from);
      await mouse('mousePressed', from, 1);
      await mouse('mouseMoved', to, 1);
      await mouse('mouseReleased', to);
      await sleep(60);
    };
    const dblAt = async (x, y) => {
      const p = await docPt(x, y);
      await b.click(...p);
      await b.click(...p, { clickCount: 2 });
      await sleep(250);
    };
    const stage = () => b.eval(`document.querySelector('.svge__stage').focus()`);
    await stage();
    await b.key('l');
    await b.click(...(await docPt(20, 30)));
    await b.click(...(await docPt(50, 60)));
    await dblAt(80, 30);
    const polyline = await pathD();
    await stage();
    await b.key('v');
    await b.key('Escape');
    await dblAt(35, 45);
    const nodesShown = await b.eval(`document.querySelectorAll('[data-node$=",node"]').length`);
    await dblAt(50, 60);
    const smooth = await pathD();
    await dblAt(50, 60);
    const corner = await pathD();
    await dblAt(65, 45);
    const inserted = await pathD();
    check(
      'SVG editor double clicks end a polyline, open its nodes, turn a node smooth and back and add a node on a segment',
      polyline === 'M20 30L50 60L80 30' && nodesShown === 3 && /^M20 30C.+ 50 60C.+ 80 30$/.test(smooth) && corner === polyline && inserted === 'M20 30L50 60L65 45L80 30',
      JSON.stringify({ polyline, nodesShown, smooth, corner, inserted }),
    );
    await stage();
    await b.key('Escape');
    await b.key('Escape');
    await b.key('b');
    await stroke(await docPt(20, 75), await docPt(30, 70));
    await stroke(await docPt(45, 80), await docPt(45, 80));
    await stroke(await docPt(70, 85), await docPt(70, 85));
    await b.key('Enter');
    await b.key('v');
    await b.key('a', { ctrl: true });
    const selectedRows = () => b.eval(`document.querySelectorAll('.svge__row[aria-selected="true"]').length`);
    const bothChosen = await selectedRows();
    const drawnBefore = JSON.stringify(await svgShapes());
    await stroke(await docPt(45, 80), await docPt(45, 80));
    const narrowed = await selectedRows();
    const unmoved = JSON.stringify(await svgShapes()) === drawnBefore;
    await stage();
    await b.key('z', { ctrl: true });
    const leftAfterUndo = (await svgShapes()).length;
    check('SVG editor narrows a choice by a still click on a pen path, moving nothing and with no undo step', bothChosen === 2 && narrowed === 1 && unmoved && leftAfterUndo === 1, `${bothChosen} → ${narrowed}, ${unmoved ? 'yerinde' : 'kaydı'}, geri almadan sonra ${leftAfterUndo} şekil`);
    await closeEditor();

    // Questions ask in a window of their own over the editor (ui/widgets/confirm.ts, DESIGN.md §7.9.1). An editor
    // with nothing changed closes at once; with a change, × asks: Esc, Vazgeç and a second × go back to the work,
    // "Kaydetmeden kapat" closes, "Kaydet ve kapat" saves first. Tab stays in the question.
    const isOpen = (sel) => b.eval(`!!document.querySelector(${JSON.stringify(sel)})`);
    const question = '.dialog--confirm[aria-label="Kaydedilmemiş değişiklikler"]';
    const closeX = async (sel) => {
      const p = await center(`${sel} .dialog__head .ibtn`);
      if (!p) throw new Error(`bulunamadı: ${sel} ×`);
      await b.click(...p);
      await sleep(250);
      return p;
    };
    await b.eval(`window.kentos.commands.execute('style.svgEditor')`);
    await sleep(500);
    await closeX('.dialog--svge');
    check('an untouched SVG drawing closes without a question', !(await isOpen('.dialog--svge')) && !(await isOpen('.dialog--confirm')));
    await b.eval(`window.kentos.commands.execute('style.svgEditor')`);
    await sleep(500);
    await b.eval(`document.querySelector('.svge__stage').focus()`);
    await b.key('r');
    await b.drag(...(await docPt(20, 20)), ...(await docPt(60, 50)));
    const svgX = await closeX('.dialog--svge');
    const asked = await b.eval(`(() => { const q = document.querySelector(${JSON.stringify(question)}); return q && { role: q.getAttribute('role'), focus: document.activeElement?.textContent, buttons: [...q.querySelectorAll('.dialog__foot .btn')].map((x) => x.textContent) }; })()`);
    check(
      'closing with a change asks in a window of its own, with Kaydet ve kapat focused',
      asked?.role === 'alertdialog' && asked.focus === 'Kaydet ve kapat' && JSON.stringify(asked.buttons) === '["Kaydetmeden kapat","Vazgeç","Kaydet ve kapat"]',
      JSON.stringify(asked),
    );
    await b.shot('confirm-unsaved');
    const inQuestion = [];
    for (let i = 0; i < 4; i++) {
      await b.key('Tab');
      inQuestion.push(await b.eval(`!!document.activeElement?.closest('.dialog--confirm')`));
    }
    await b.key('Escape');
    await sleep(200);
    const svgShapeCount = () => b.eval(`document.querySelectorAll('.svge__svg [data-id]').length`);
    check('Tab stays in the question; Esc goes back to the drawing', inQuestion.every(Boolean) && (await isOpen('.dialog--svge')) && !(await isOpen('.dialog--confirm')) && (await svgShapeCount()) === 1, JSON.stringify(inQuestion));
    await b.click(...svgX);
    await sleep(250);
    await press(`${question} .btn`, 'Vazgeç');
    await b.click(...svgX);
    await sleep(250);
    // The second click lands on the question's backdrop: Vazgeç, never a close without saving.
    await b.click(...svgX);
    await sleep(250);
    check('Vazgeç and a second × keep the editor and its drawing', (await isOpen('.dialog--svge')) && !(await isOpen('.dialog--confirm')) && (await svgShapeCount()) === 1);
    await b.click(...svgX);
    await sleep(250);
    await press(`${question} .btn`, 'Kaydetmeden kapat');
    check('Kaydetmeden kapat closes the editor and saves nothing', !(await isOpen('.dialog--svge')) && !(await b.eval(`window.kentos.styles.library.items('user').some((i) => i.kind === 'asset' && i.name === 'Yeni çizim')`)));
    // The model designer: untouched closes; with an input, Kaydet ve kapat saves and closes.
    await b.eval(`window.kentos.commands.execute('processing.newModel')`);
    await sleep(300);
    await closeX('.dialog--designer');
    check('an untouched new model closes without a question', !(await isOpen('.dialog--designer')) && !(await isOpen('.dialog--confirm')));
    await b.eval(`window.kentos.commands.execute('processing.newModel')`);
    await sleep(300);
    await press('.mpalette__input', 'Nesneler');
    await closeX('.dialog--designer');
    await press(`${question} .btn`, 'Kaydet ve kapat');
    const kept = await b.eval(`window.kentos.processing.models.value.find((m) => m.label === 'Yeni model' && m.inputs.length === 1 && !m.steps.length)`);
    check('Kaydet ve kapat saves the model, then closes', !!kept && !(await isOpen('.dialog--designer')) && !(await isOpen('.dialog--confirm')), JSON.stringify(kept?.inputs));
    if (kept) await b.eval(`window.kentos.processing.removeModel(${JSON.stringify(kept.id)})`);
    // The symbol designer: a new symbol closes untouched; renamed, its Vazgeç asks.
    await b.eval(`window.kentos.commands.execute('style.manager')`);
    await sleep(400);
    await press('.dialog--styles .btn', 'Yeni sembol');
    await press('.menu__item', '');
    await sleep(300);
    await closeX('.dialog--sdesign');
    const plainClose = !(await isOpen('.dialog--sdesign')) && !(await isOpen('.dialog--confirm'));
    await press('.dialog--styles .btn', 'Yeni sembol');
    await press('.menu__item', '');
    await sleep(300);
    await b.eval(`(() => { const i = document.querySelector('.dialog--sdesign input[aria-label="Sembol adı"]'); i.focus(); i.select(); })()`);
    await b.type('Adı değişen sembol');
    await press('.dialog--sdesign .dialog__foot .btn', 'Vazgeç');
    const renamedAsks = await isOpen(question);
    await press(`${question} .btn`, 'Kaydetmeden kapat');
    check('the symbol designer closes untouched and asks after a rename', plainClose && renamedAsks && !(await isOpen('.dialog--sdesign')) && (await isOpen('.dialog--styles')));
    // Deleting from the library asks in the same window: Vazgeç has the focus, Esc keeps the symbol, Sil removes it.
    const doomed = await b.eval(`(() => { const lib = window.kentos.styles.library; return lib.copy(lib.items('system').find((i) => i.kind === 'symbol').id, 'user', { name: 'Silinecek deneme' }).id; })()`);
    await b.eval(`(() => { const f = document.querySelector('.dialog--styles input[type="search"], .dialog--styles .field'); f.focus(); })()`);
    await b.type('Silinecek');
    await sleep(400);
    await press('.scard', 'Silinecek deneme');
    await press('.smgr__actions .btn', 'Sil');
    const del = await b.eval(`(() => { const q = document.querySelector('.dialog--confirm'); return q && { title: q.getAttribute('aria-label'), focus: document.activeElement?.textContent, danger: q.querySelector('.btn--danger')?.textContent }; })()`);
    await b.key('Escape');
    await sleep(200);
    const stays = await b.eval(`!!window.kentos.styles.library.get(${JSON.stringify(doomed)})`);
    await press('.smgr__actions .btn', 'Sil');
    await press('.dialog--confirm .btn', 'Sil');
    const removed = !(await b.eval(`!!window.kentos.styles.library.get(${JSON.stringify(doomed)})`));
    check('deleting from the library asks in a window: Vazgeç focused, Esc keeps, Sil removes', del?.title === 'Kitaplıktan sil' && del.focus === 'Vazgeç' && del.danger === 'Sil' && stays && removed, JSON.stringify(del));
    await closeX('.dialog--styles');
  }

  // İşlem araçları: open from the İşlemler menu, run from the dialog, one undo step
  {
    const center = (sel, text = '') =>
      b.eval(`(() => { const e = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.textContent.trim().startsWith(${JSON.stringify(text)})); if (!e) return null; const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    const press = async (sel, text) => {
      const p = await center(sel, text);
      if (!p) throw new Error(`bulunamadı: ${sel} ${text ?? ''}`);
      await b.click(...p);
      await sleep(150);
    };
    await b.eval(`(() => { const k = window.kentos; k.selection.set(k.doc.byLayer('parsel').filter((e) => e.kind === 'polygon').slice(0, 4).map((e) => e.id)); })()`);
    await press('.menubar__item', 'İşlemler');
    await press('.menu__item', 'Nokta işlemleri');
    await press('.menu__item', 'Köşe noktalarını numarala');
    check('processing: dialog opens from the İşlemler menu', !!(await center('.dialog--ptool')));
    const count = await b.eval(`document.querySelector('.pfield__count')?.textContent ?? ''`);
    check('processing: input shows what it will read', /^4 kapalı alan; seçili nesneler/.test(count), count);
    await b.eval(`(() => { const i = document.querySelector('[data-param="prefix"] input'); i.focus(); i.select(); })()`);
    await b.type('K');
    const preview = await b.eval(`document.querySelector('.ptool__preview-value')?.textContent ?? ''`);
    check('processing: preview follows the typed prefix', preview.startsWith('K00001, K00002'), preview);
    const size0 = await b.eval('window.kentos.doc.size');
    await press('.ptool__run');
    await b.waitFor(`document.querySelector('.ptool__status')?.dataset.kind === 'ok'`, 5000).catch(() => {});
    const made = await b.eval(`(() => { const k = window.kentos; const l = k.doc.layers.leaves().find((x) => x.name === 'Köşe noktaları'); return l ? k.doc.byLayer(l.id).map((e) => e.label) : []; })()`);
    check('processing: corners numbered into a new layer', made.length > 4 && made[0] === 'K00001' && (await b.eval('window.kentos.doc.size')) === size0 + made.length, `${made.length} nokta, ${made[0]}`);
    await b.shot('smoke-processing');
    await press('.dialog__foot .btn', 'Kapat');
    await b.eval(`window.kentos.commands.execute('processing.history')`);
    await sleep(120);
    check('processing: history lists the run', (await b.eval(`document.querySelectorAll('.phist__row').length`)) === 1 && (await b.eval('window.kentos.ui.dockTab.value')) === 'processing');
    await b.eval(`window.kentos.commands.execute('edit.undo')`);
    check('processing: the run is one undo step', (await b.eval('window.kentos.doc.size')) === size0);
    // İfadeyle seç: a condition typed in the dialog selects what it matches
    await b.eval(`window.kentos.commands.execute('processing.run.selection.byExpression')`);
    await sleep(150);
    await press('[data-param="input"] .seg__opt', 'Tümü');
    await b.eval(`(() => { const i = document.querySelector('[data-param="condition"] input'); i.focus(); i.select(); })()`);
    await b.type("Nitelik = 'Arsa' ve $alan > 450");
    const expected = await b.eval(`(() => { const k = window.kentos; return [...k.doc.all()].filter((e) => k.doc.layers.isVisible(e.layerId) && e.attrs.Nitelik === 'Arsa' && e.kind === 'polygon' && k.doc.get(e.id) && (() => { let a = 0; const p = e.pts; for (let i = 0; i < p.length; i++) { const q = p[(i + 1) % p.length]; a += p[i].x * q.y - q.x * p[i].y; } return Math.abs(a / 2) > 450; })()).length; })()`);
    const preview2 = await b.eval(`document.querySelector('[data-param="condition"] .pfield__preview')?.textContent ?? ''`);
    check('processing: expression preview counts the matches', preview2.startsWith(`${expected} / `), preview2);
    await press('.ptool__run');
    await b.waitFor(`document.querySelector('.ptool__status')?.dataset.kind === 'ok'`, 5000).catch(() => {});
    check('processing: İfadeyle seç selects the matches', expected > 0 && (await b.eval('window.kentos.selection.size')) === expected, String(expected));
    await press('.dialog__foot .btn', 'Kapat');
    // The same tool in the Web Worker gives the same result as in the page
    const both = await b.eval(`(async () => {
      const k = window.kentos;
      const p = k.processing;
      const tool = p.registry.get('attributes.calculate');
      const values = { input: { scope: 'all', kinds: ['polygon'] }, field: 'Deneme', value: "metin($alan, 3) || '/' || $sıra", where: '', empty: 'keep', label: false };
      const page = await p.runner.run(tool, values, { target: 'client' });
      const a = [...k.doc.all()].filter((e) => e.attrs.Deneme).map((e) => e.attrs.Deneme);
      k.doc.undo();
      const bg = await p.runner.run(tool, values, { target: 'worker' });
      const w = [...k.doc.all()].filter((e) => e.attrs.Deneme).map((e) => e.attrs.Deneme);
      k.doc.undo();
      return { page: page.status, bg: bg.status, target: bg.record && bg.record.target, same: a.length > 0 && JSON.stringify(a) === JSON.stringify(w), n: a.length };
    })()`);
    check('processing: the Web Worker gives the same result as the page', both.page === 'ok' && both.bg === 'ok' && both.target === 'worker' && both.same, JSON.stringify(both));
    // Models: the built-in one runs as one undo step; the designer builds and saves a new one
    await b.eval(`(() => { const k = window.kentos; k.selection.set(k.doc.byLayer('parsel').filter((e) => e.kind === 'polygon').slice(0, 3).map((e) => e.id)); k.commands.execute('processing.model.builtin.parcelSheet'); })()`);
    await sleep(200);
    const m0 = await b.eval('window.kentos.doc.size');
    await press('.ptool__run');
    await b.waitFor(`document.querySelector('.ptool__status')?.dataset.kind === 'ok'`, 8000).catch(() => {});
    const m1 = await b.eval('window.kentos.doc.size');
    const record = await b.eval('window.kentos.processing.runner.history.value[0]');
    check('processing: the built-in model runs its three steps', m1 > m0 && record.toolId === 'model:builtin.parcelSheet' && /^3 adım çalıştı/.test(record.summary), record.summary);
    await press('.dialog__foot .btn', 'Kapat');
    await b.eval(`window.kentos.commands.execute('edit.undo')`);
    check('processing: one undo takes the whole model back', (await b.eval('window.kentos.doc.size')) === m0);
    await b.eval(`window.kentos.commands.execute('processing.newModel')`);
    await sleep(250);
    await press('.mpalette__input', 'Nesneler');
    await press('.mpalette__tool', 'Kenar uzunluklarını yaz');
    const wired = await b.eval(`document.querySelectorAll('.medge').length`);
    await press('.dialog__foot .btn', 'Kaydet');
    const saved = await b.eval(`window.kentos.processing.models.value.find((m) => m.label === 'Yeni model')`);
    check('processing: the designer chains a tool to the input and saves the model', wired === 1 && !!saved && saved.steps[0].values.input?.kind === 'input', JSON.stringify(saved?.steps?.[0]?.values));
    await press('.dialog__foot .btn', 'Kapat');
    await b.eval(`window.kentos.ui.dockTab.set('layers'); window.kentos.ui.processingTab.set('tools'); window.kentos.selection.clear()`);
  }

  // Hesap menu (ui/calc, crates/shared/geometry-core/src/survey): a connected traverse typed into its table
  // from measurements made from designed points, added to the drawing as one undo step; a resection at a
  // known point gives that point back. Values are typed with the keyboard into the fields.
  {
    const center = (sel) => b.eval(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    const typeAt = async (sel, text) => {
      const p = await center(sel);
      if (!p) throw new Error(`bulunamadı: ${sel}`);
      await b.click(...p);
      await b.eval(`document.activeElement.select?.()`);
      await b.type(text);
      await sleep(40);
    };
    const helpers = `const k = window.kentos; const pt = (n) => [...k.doc.all()].find((e) => e.kind === 'point' && e.label === n).p;
      const semt = (a, b) => { let t = Math.atan2(b.x - a.x, b.y - a.y); if (t < 0) t += 2 * Math.PI; return t; };
      const g = (r) => { let v = (r * 200) / Math.PI; v %= 400; if (v < 0) v += 400; return v; };`;
    const plan = await b.eval(`(() => { ${helpers}
      const A = pt('P.101'), A0 = pt('P.105'), E = pt('P.102'), E0 = pt('P.103');
      const X1 = { x: A.x + 0.35 * (E.x - A.x) + 12.3, y: A.y + 0.35 * (E.y - A.y) - 8.7 };
      const X2 = { x: A.x + 0.7 * (E.x - A.x) - 6.1, y: A.y + 0.7 * (E.y - A.y) + 9.4 };
      const st = [A, X1, X2, E], ang = [], dist = [];
      for (let i = 0; i < 3; i++) ang.push(g(semt(st[i], st[i + 1]) - semt(st[i], i ? st[i - 1] : A0)).toFixed(5));
      ang.push(g(semt(E, E0) - semt(E, X2)).toFixed(5));
      for (let i = 0; i < 3; i++) dist.push(Math.hypot(st[i + 1].x - st[i].x, st[i + 1].y - st[i].y).toFixed(4));
      return { X1, X2, ang, dist };
    })()`);
    await b.eval(`window.kentos.commands.execute('calc.traverse')`);
    await b.waitFor(`!!document.querySelector('.dialog--calc [data-key="start"]')`, 10000);
    await b.eval(`[...document.querySelectorAll('.dialog--calc .seg button')].find((x) => x.textContent === 'Bağlı')?.click()`);
    for (const [key, name] of [['start', 'P.101'], ['back', 'P.105'], ['end', 'P.102'], ['fore', 'P.103']]) await typeAt(`.dialog--calc [data-key="${key}"]`, name);
    const cells = [['0', 'angle', plan.ang[0]], ['0', 'distance', plan.dist[0]], ['1', 'name', 'H1'], ['1', 'angle', plan.ang[1]], ['1', 'distance', plan.dist[1]], ['2', 'name', 'H2'], ['2', 'angle', plan.ang[2]], ['2', 'distance', plan.dist[2]], ['3', 'angle', plan.ang[3]]];
    for (const [row, key, v] of cells) await typeAt(`.dialog--calc input[data-row="${row}"][data-key="${key}"]`, v);
    await sleep(200);
    const shown = await b.eval(`document.querySelector('.dialog--calc .io-summary').innerText`);
    check('Hesap: a connected traverse shows its misclosures', /Açı kapanma hatası/.test(shown) && /fs = 0\.\d mm/.test(shown), shown.split('\n')[0]);
    await b.shot('calc-traverse');
    const n0 = await b.eval('window.kentos.doc.size');
    const addAt = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog--calc .btn--primary')].find((x) => x.textContent === 'Çizime ekle'); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    await b.click(...addAt);
    await sleep(200);
    const got = await b.eval(`[...window.kentos.doc.all()].filter((e) => e.kind === 'point' && (e.label === 'H1' || e.label === 'H2')).map((e) => ({ n: e.label, layer: e.layerId, p: e.p }))`);
    const near = (p, q) => Math.hypot(p.x - q.x, p.y - q.y) < 1e-4;
    check(
      'Hesap: the traverse points go to Poligon noktaları within 0.1 mm of the designed ones, one undo step',
      got.length === 2 && got.every((g) => g.layer === 'poligon') && near(got[0].p, plan.X1) && near(got[1].p, plan.X2) && (await b.eval('window.kentos.doc.size')) === n0 + 2,
      JSON.stringify(got),
    );
    await b.key('z', { ctrl: true });
    await sleep(100);
    check('Hesap: undo takes the traverse points back', (await b.eval('window.kentos.doc.size')) === n0);
    // Resection at P.107 towards three sample points seen left to right.
    const rs = await b.eval(`(() => { ${helpers} const P = pt('P.107');
      const o = ['P.101', 'P.104', 'P.112', 'P.109'].map((n) => ({ n, t: semt(P, pt(n)) })).sort((a, b) => a.t - b.t).slice(0, 3);
      return { names: o.map((x) => x.n), alpha: g(o[1].t - o[0].t).toFixed(6), beta: g(o[2].t - o[1].t).toFixed(6), P }; })()`);
    await b.eval(`window.kentos.commands.execute('calc.resection')`);
    await b.waitFor(`!!document.querySelector('.dialog--calc [data-key="c"]')`, 10000);
    for (const [key, name] of [['a', rs.names[0]], ['b', rs.names[1]], ['c', rs.names[2]]]) await typeAt(`.dialog--calc [data-key="${key}"]`, name);
    await typeAt('.dialog--calc input[aria-label^="α"]', rs.alpha);
    await typeAt('.dialog--calc input[aria-label^="β"]', rs.beta);
    await sleep(200);
    const cellsOut = await b.eval(`[...document.querySelectorAll('.dialog--calc .calc-section tbody td')].map((c) => c.textContent)`);
    check('Hesap: a resection at a known point gives it back', cellsOut[1] === rs.P.x.toFixed(3) && cellsOut[2] === rs.P.y.toFixed(3), JSON.stringify({ cellsOut, want: rs.P }));
    await b.key('Escape');
    await sleep(100);
  }

  // File exchange (src/io, crates/shared/formats): an in-memory picker hands files to the importers and takes the
  // exported bytes; the Rust formats module runs in its own worker. Coordinates must arrive exactly.
  const ioCenter = (sel, text = '') =>
    b.eval(`(() => { const e = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.textContent.trim().startsWith(${JSON.stringify(text)})); if (!e) return null; const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  const ioPress = async (sel, text) => {
    const p = await ioCenter(sel, text);
    if (!p) throw new Error(`bulunamadı: ${sel} ${text ?? ''}`);
    await b.click(...p);
    await sleep(200);
  };
  const ioPicker = (name, text) =>
    b.eval(`(() => {
      const k = window.kentos;
      const io = (window.__io ??= { original: k.files.picker });
      io.written = null;
      k.files.picker = {
        open: async () => ({ name: ${JSON.stringify(name)}, getFile: async () => new Blob([${JSON.stringify(text)}]) }),
        save: async (n) => ({ name: n, getFile: async () => new Blob([]), createWritable: async () => { const parts = []; return { write: async (d) => { parts.push(d); }, close: async () => { io.written = { name: n, parts }; } }; } }),
      };
    })()`);
  const ioWritten = () => b.eval(`(() => { const w = window.__io.written; if (!w) return null; const bytes = w.parts.map((p) => (typeof p === 'string' ? new TextEncoder().encode(p) : p)); const all = new Uint8Array(bytes.reduce((s, p) => s + p.length, 0)); let at = 0; for (const p of bytes) { all.set(p, at); at += p.length; } return { name: w.name, text: new TextDecoder().decode(all) }; })()`);

  // Coordinate lists (Netcad NCN): preview with the detected columns, the coordinate system question,
  // one undo step, exact values; the export writes the same text back.
  {
    await ioPicker('deneme.ncn', '# deneme\r\n1001 487061.123 4420101.456 105.2\r\n1002 487071.5 4420111.25 106.75\r\nbozuk satır\r\n');
    await b.eval('window.kentos.selection.clear()');
    const n0 = await b.eval('window.kentos.doc.size');
    await b.eval(`window.kentos.commands.execute('file.import.ncn')`);
    await b.waitFor(`document.querySelector('.dialog--io .io-table tbody tr')`, 20000).catch(() => {});
    const preview = await b.eval(`({ cols: [...document.querySelectorAll('.dialog--io thead select')].map((s) => s.value), rows: document.querySelectorAll('.dialog--io tbody tr').length, bad: document.querySelectorAll('.dialog--io tbody tr[data-error]').length })`);
    check('coordinate import: the preview reads the NCN as Ad Y X Z and marks the bad line', JSON.stringify(preview.cols) === '["name","y","x","z"]' && preview.rows === 3 && preview.bad === 1, JSON.stringify(preview));
    await b.shot('io-coords-import');
    // Another system than the project's blocks the import: nothing is reprojected silently.
    const blocked = await b.eval(`(() => {
      const s = document.querySelector('.dialog--io select[aria-label="Bu koordinatlar hangi sistemde?"]');
      const primary = document.querySelector('.dialog--io .dialog__foot .btn--primary');
      const keep = s.value;
      s.value = '2322';
      s.dispatchEvent(new Event('change'));
      const r = { disabled: primary.disabled, warned: !!document.querySelector('.dialog--io .note--warn') };
      s.value = keep;
      s.dispatchEvent(new Event('change'));
      return { ...r, after: primary.disabled };
    })()`);
    check('coordinate import: another coordinate system blocks it with a warning', blocked.disabled && blocked.warned && !blocked.after, JSON.stringify(blocked));
    await ioPress('.dialog--io .dialog__foot .btn--primary', 'İçe aktar');
    await b.waitFor(`!document.querySelector('.dialog--io')`, 10000).catch(() => {});
    const got = await b.eval(`[...window.kentos.doc.all()].filter((e) => e.kind === 'point' && (e.label === '1001' || e.label === '1002')).map((e) => [e.label, e.p.x, e.p.y, e.z, window.kentos.doc.layers.get(e.layerId)?.name, e.attrs.Ad])`);
    check('coordinate import adds the points exactly, on a new layer named after the file', JSON.stringify(got) === JSON.stringify([['1001', 487061.123, 4420101.456, 105.2, 'deneme', '1001'], ['1002', 487071.5, 4420111.25, 106.75, 'deneme', '1002']]), JSON.stringify(got));
    await b.eval(`window.kentos.commands.execute('edit.undo')`);
    const undone = await b.eval('window.kentos.doc.size');
    await b.eval(`window.kentos.commands.execute('edit.redo')`);
    check('coordinate import is one undo step', undone === n0 && (await b.eval('window.kentos.doc.size')) === n0 + 2, `${n0} → ${undone}`);
    await b.eval(`(() => { const k = window.kentos; k.selection.set([...k.doc.all()].filter((e) => e.kind === 'point' && (e.label === '1001' || e.label === '1002')).map((e) => e.id)); k.commands.execute('file.export.ncn'); })()`);
    await b.waitFor(`document.querySelector('.dialog--io .io-summary')`, 10000).catch(() => {});
    await ioPress('.dialog--io .dialog__foot .btn--primary', 'Dışa aktar');
    await b.waitFor(`window.__io.written`, 10000).catch(() => {});
    const out = await ioWritten();
    check('coordinate export writes the selected points back exactly (NCN)', out?.text === '1001 487061.123 4420101.456 105.2\r\n1002 487071.5 4420111.25 106.75\r\n' && /\.ncn$/.test(out.name), JSON.stringify(out));
    await b.eval(`(() => { const k = window.kentos; k.files.picker = window.__io.original; k.selection.clear(); })()`);
  }

  // DXF (fixtures/formats/v1/blocks.dxf, Windows-1254 bytes as they are): the file's layers become new layers in a
  // group named after it, a layer the user leaves out stays out, nested inserts arrive exploded with exact
  // coordinates, and the whole import is one undo step.
  {
    const raw = readFileSync(new URL('../../../../fixtures/formats/v1/blocks.dxf', import.meta.url)).toString('base64');
    await b.eval(`(() => {
      const k = window.kentos;
      const bytes = Uint8Array.from(atob(${JSON.stringify(raw)}), (c) => c.charCodeAt(0));
      window.__io ??= { original: k.files.picker };
      k.files.picker = { open: async () => ({ name: 'kapi.dxf', getFile: async () => new Blob([bytes]) }), save: async () => null };
      k.selection.clear();
    })()`);
    const n0 = await b.eval('window.kentos.doc.size');
    await b.eval(`window.kentos.commands.execute('file.import.dxf')`);
    await b.waitFor(`document.querySelector('.dialog--io .io-table tbody tr')`, 20000).catch(() => {});
    const rows = await b.eval(`[...document.querySelectorAll('.dialog--io tbody tr')].map((r) => r.children[1].textContent.trim())`);
    const box = await b.eval(`(() => { const row = [...document.querySelectorAll('.dialog--io tbody tr')].find((r) => r.children[1].textContent.trim() === 'DIZI'); const e = row?.querySelector('input'); if (!e) return null; const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    if (box) await b.click(...box);
    await sleep(150);
    await b.shot('io-dxf-import');
    const summary = await b.eval(`document.querySelector('.dialog--io .io-summary')?.textContent ?? ''`);
    check('DXF import: the layers table lists the file\'s layers and the report names what was left out', JSON.stringify(rows) === '["0","DETAY","DIZI","KAPILAR"]' && /8 nesne alınacak/.test(summary) && /kendini içeriyor/.test(summary), `${JSON.stringify(rows)} ${summary.slice(0, 160)}`);
    await ioPress('.dialog--io .dialog__foot .btn--primary', 'İçe aktar');
    await b.waitFor(`!document.querySelector('.dialog--io')`, 10000).catch(() => {});
    const got = await b.eval(`(() => {
      const k = window.kentos;
      const group = k.doc.layers.tree.find((n) => n.name === 'kapi.dxf');
      const on = (name) => [...k.doc.all()].filter((e) => k.doc.layers.get(e.layerId)?.name === name);
      const circle = on('KAPILAR').find((e) => e.kind === 'circle');
      const point = on('KAPILAR').find((e) => e.kind === 'point');
      return { layers: group ? group.children.map((c) => c.name) : null, circle: circle && [circle.c.x, circle.c.y, circle.r], point: point && [point.p.x, point.p.y, point.z], dizi: on('DIZI').length, added: k.doc.size - ${n0} };
    })()`);
    check(
      'DXF import: new layers in a group named after the file, blocks exploded exactly, a left-out layer stays out',
      JSON.stringify(got) === JSON.stringify({ layers: ['0', 'DETAY', 'KAPILAR'], circle: [1000, 2010, 1], point: [996, 2003, 101.5], dizi: 0, added: 8 }),
      JSON.stringify(got),
    );
    // The geometry store (picking, snapping) hears of the import through its one change event: the circle is picked at once.
    const picked = await b.eval(`(() => {
      const k = window.kentos;
      const circle = [...k.doc.all()].find((e) => e.kind === 'circle' && e.c.x === 1000 && e.c.y === 2010);
      k.view.camera.fit({ minX: 999, minY: 2009, maxX: 1003, maxY: 2011 });
      const hit = k.view.pick(k.view.camera.worldToScreen({ x: 1001, y: 2010 }));
      return !!circle && hit?.id === circle.id;
    })()`);
    check('DXF import: an imported object can be picked at once', picked === true, String(picked));
    await b.eval(`window.kentos.commands.execute('edit.undo')`);
    const undone = await b.eval('window.kentos.doc.size');
    await b.eval(`window.kentos.commands.execute('edit.redo')`);
    check('DXF import is one undo step', undone === n0 && (await b.eval('window.kentos.doc.size')) === n0 + 8, `${n0} → ${undone}`);
    await b.eval(`(() => { const k = window.kentos; k.files.picker = window.__io.original; k.selection.clear(); })()`);
  }

  // DXF export (crates/shared/formats dxf/writer): the selected objects go out through the in-memory picker as an
  // AutoCAD 2007 DXF; imported back, the same objects return onto the same layer with the same coordinates, bit for
  // bit, and with their labels and attributes (KentOS data in the file). Exporting is not saving: the drawing stays as it was.
  {
    await ioPicker('kullanilmaz.dxf', '');
    const made = await b.eval(`(() => {
      const k = window.kentos;
      const layer = k.doc.layers.add({ name: 'DXF deneme', style: { color: '#7FB2E5', lineType: 'dashed', lineWeight: 0.35 } });
      const P = (x, y) => ({ x: ${E + 900.123} + x, y: ${N + 50.456} + y });
      const objects = [
        { kind: 'polygon', pts: [P(0, 0), P(20, 0), P(20, 20), P(0, 20)], bulges: [0, 0.2, 0, 0], holes: [{ pts: [P(2, 2), P(4, 2), P(4, 4)] }], label: '101', attrs: { Ada: '12', Parsel: '101' } },
        { kind: 'arc', c: P(30, 5), r: 3.25, a0: 0.1, a1: 2.2, attrs: {} },
        { kind: 'spline', pts: [P(0, 30), P(5, 35), P(10, 30), P(15, 36)], closed: false, attrs: {} },
        { kind: 'text', p: P(0, 40), text: 'Çınar ağacı', height: 2, rotation: 15, attrs: {} },
        { kind: 'point', p: P(1 / 3, 0.1 + 0.2), z: 105.25, label: 'P7', attrs: { Ad: 'P7' } },
        { kind: 'line', a: P(0, -5), b: P(20, -5), color: '#FF0000', attrs: {} },
        { kind: 'dimension', a: P(0, -10), b: P(20, -12), offset: 2, height: 1.5, attrs: {} },
        { kind: 'dimension', a: P(40, 0), b: P(35, 6), c: P(35, 0), offset: 4.5, height: 1.5, style: 'angular', attrs: {} },
      ];
      const ids = [];
      k.doc.transact('DXF deneme', () => { for (const o of objects) ids.push(k.doc.add({ ...o, layerId: layer.id }).id); });
      k.selection.set(ids);
      return { ids, layerId: layer.id, dirty: k.doc.dirty.value, size: k.doc.size };
    })()`);
    await b.eval(`window.kentos.commands.execute('file.export.dxf')`);
    await b.waitFor(`document.querySelector('.dialog--io .io-summary')`, 10000).catch(() => {});
    const rows = await b.eval(`[...document.querySelectorAll('.dialog--io tbody tr')].map((r) => [r.children[1].textContent.trim(), r.children[2].textContent.trim()])`);
    const summary = await b.eval(`document.querySelector('.dialog--io .io-summary')?.textContent ?? ''`);
    check(
      'DXF export: the window lists the selection by layer and says what DXF changes',
      JSON.stringify(rows) === '[["DXF deneme","8"]]' && /8 nesne 1 katmanla yazılacak/.test(summary) && /2 ölçü DXF ölçüsü olarak yazılır/.test(summary) && /adalı alan/.test(summary) && /KentOS verisi/.test(summary),
      `${JSON.stringify(rows)} ${summary.slice(0, 200)}`,
    );
    await b.shot('io-dxf-export');
    await ioPress('.dialog--io .dialog__foot .btn--primary', 'Dışa aktar');
    await b.waitFor(`window.__io.written`, 10000).catch(() => {});
    const dxf = await ioWritten();
    const kept = await b.eval(`({ dirty: window.kentos.doc.dirty.value, size: window.kentos.doc.size, open: !!document.querySelector('.dialog--io') })`);
    check(
      'DXF export writes an AutoCAD 2007 DXF, with the dimensions as DXF dimensions, and leaves the drawing as it was',
      !!dxf && /\.dxf$/.test(dxf.name) && dxf.text.includes('$ACADVER\r\n  1\r\nAC1021\r\n') && dxf.text.endsWith('  0\r\nEOF\r\n') && (dxf.text.match(/\r\n  0\r\nDIMENSION\r\n/g) ?? []).length === 2 && kept.dirty === made.dirty && kept.size === made.size && !kept.open,
      `${dxf?.name} ${dxf?.text.length} ${JSON.stringify(kept)}`,
    );
    await b.eval(`(() => {
      const k = window.kentos;
      const parts = window.__io.written.parts;
      k.selection.clear();
      k.files.picker = { ...k.files.picker, open: async () => ({ name: 'geri.dxf', getFile: async () => new Blob(parts) }) };
    })()`);
    const lastId = await b.eval(`Math.max(...[...window.kentos.doc.all()].map((e) => e.id))`);
    await b.eval(`window.kentos.commands.execute('file.import.dxf')`);
    await b.waitFor(`document.querySelector('.dialog--io .io-table tbody tr')`, 20000).catch(() => {});
    const where = await b.eval(`[...document.querySelectorAll('.dialog--io tbody tr')].map((r) => [r.children[1].textContent.trim(), r.children[3].textContent.trim()])`);
    check('DXF export → import: the file\'s layer goes back to the drawing\'s layer of the same name', where.length === 1 && where[0][0] === 'DXF deneme' && /katmanına eklenir/.test(where[0][1]), JSON.stringify(where));
    await ioPress('.dialog--io .dialog__foot .btn--primary', 'İçe aktar');
    await b.waitFor(`!document.querySelector('.dialog--io')`, 10000).catch(() => {});
    await b.shot('io-dxf-roundtrip');
    const back = await b.eval(`(() => {
      const k = window.kentos;
      // Key order, absent fields and ids aside (imported objects are new ones, ADR 0014), JSON of every field:
      // numbers print exactly, so equal text is equal bits.
      const canon = (v) => (Array.isArray(v) ? v.map(canon) : v && typeof v === 'object' ? Object.fromEntries(Object.keys(v).filter((key) => key !== 'id' && key !== 'uid' && v[key] !== undefined).sort().map((key) => [key, canon(v[key])])) : v);
      const mine = ${JSON.stringify(made.ids)}.map((id) => JSON.stringify(canon(k.doc.get(id))));
      const got = [...k.doc.all()].filter((e) => e.id > ${lastId}).map((e) => JSON.stringify(canon(e)));
      return { same: JSON.stringify(mine) === JSON.stringify(got), mine, got };
    })()`);
    check('DXF export → import: the same objects come back (dimensions as dimensions), coordinates bit for bit, with labels and attributes', back.same, back.same ? '' : `${back.mine.join(' ')} ≠ ${back.got.join(' ')}`.slice(0, 600));
    await b.eval(`window.kentos.commands.execute('edit.undo')`);
    await b.eval(`window.kentos.commands.execute('edit.undo')`);
    const cleared = await b.eval('window.kentos.doc.size');
    check('DXF export → import: the import and the test objects each undo in one step', cleared === made.size - made.ids.length, `${made.size} → ${cleared}`);
    await b.eval(`(() => { const k = window.kentos; k.files.picker = window.__io.original; k.selection.clear(); })()`);
  }

  // Local .kcad files: Ctrl+S writes (dirty clears only after the write), Ctrl+O asks about unsaved changes and reopens it.
  // Headless Chrome has no native file dialogs, so an in-memory picker stands in for them.
  {
    await b.eval(`(() => {
      const k = window.kentos;
      const disk = (window.__disk = {});
      const file = (name, fail) => ({
        name,
        getFile: async () => new Blob([disk[name] ?? '']),
        createWritable: async () => { let s = ''; return { write: async (d) => { s += d; }, close: async () => { if (fail) throw new Error('disk dolu'); disk[name] = s; } }; },
      });
      window.__files = { original: k.files.picker, file };
      k.files.picker = { save: async (n) => file(n), open: async () => file(Object.keys(disk)[0]) };
      k.files.handle = null;
      k.selection.clear();
    })()`);
    // What the file holds: persistent ids are not written in v1; opening derives them (ADR 0014, checked at the end).
    const saved = await b.eval(`JSON.stringify([...window.kentos.doc.all()].map(({ uid, ...e }) => e))`);
    await b.key('s', { ctrl: true });
    await b.waitFor(`!window.kentos.files.busy.value && Object.keys(window.__disk).length === 1`, 5000).catch(() => {});
    const written = await b.eval(`(() => { const [name, text] = Object.entries(window.__disk)[0] ?? []; const f = text ? JSON.parse(text) : {}; return { name, format: f.format, n: f.entities?.length, dirty: window.kentos.doc.dirty.value }; })()`);
    const size = await b.eval('window.kentos.doc.size');
    check('Ctrl+S writes a .kcad file and the drawing turns clean', /\.kcad$/.test(written.name ?? '') && written.format === 'kentos.document' && written.n === size && !written.dirty, JSON.stringify(written));
    await b.eval(`(() => { const k = window.kentos; k.doc.remove([[...k.doc.all()].at(-1).id]); })()`);
    check('an edit after saving marks the drawing unsaved', await b.eval('window.kentos.doc.dirty.value'));
    await b.key('o', { ctrl: true });
    await b.waitFor(`[...document.querySelectorAll('.dialog__foot .btn')].some((x) => x.textContent === 'Kaydetmeden devam et')`, 3000).catch(() => {});
    const drop = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.textContent === 'Kaydetmeden devam et'); if (!e) return null; const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    check('Ctrl+O asks before dropping unsaved changes', !!drop);
    if (drop) await b.click(...drop);
    await b.waitFor(`!window.kentos.files.busy.value && !window.kentos.doc.dirty.value`, 5000).catch(() => {});
    const reopened = await b.eval(`JSON.stringify([...window.kentos.doc.all()].map(({ uid, ...e }) => e))`);
    check('the reopened file holds the saved drawing, with no undo history', reopened === saved && !(await b.eval('window.kentos.doc.canUndo.value')), `${await b.eval('window.kentos.doc.size')} nesne`);
    await b.eval(`(() => { const k = window.kentos; k.doc.remove([[...k.doc.all()].at(-1).id]); k.files.picker = { save: async (n) => window.__files.file(n, true), open: async () => null }; k.commands.execute('file.saveAs'); })()`);
    await b.waitFor(`!window.kentos.files.busy.value`, 3000).catch(() => {});
    const failed = await b.eval(`window.kentos.log.entries.value.at(-1)?.text ?? ''`);
    check('a failed write keeps the drawing unsaved', await b.eval('window.kentos.doc.dirty.value'), failed);
    // The unsaved edit stays: the undo/redo round trip below takes it back and forth.
    await b.eval(`(() => { const k = window.kentos; k.files.picker = window.__files.original; k.files.handle = null; k.selection.clear(); })()`);
  }

  // Undo / redo round trip
  const before = await b.eval('window.kentos.doc.size');
  await b.eval(`window.kentos.commands.execute('edit.undo'); window.kentos.commands.execute('edit.redo')`);
  check('undo/redo keeps the document intact', (await b.eval('window.kentos.doc.size')) === before);

  await b.shot('smoke-final');

  // Dosya → Yeni proje (Ctrl+Alt+N): name, system and scale in the dialog; unsaved changes are asked about over it.
  {
    const center = (sel, text = '') =>
      b.eval(`(() => { const e = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.textContent.trim().startsWith(${JSON.stringify(text)})); if (!e) return null; e.scrollIntoView({ block: 'nearest' }); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    const press = async (sel, text) => {
      const p = await center(sel, text);
      if (!p) throw new Error(`bulunamadı: ${sel} ${text ?? ''}`);
      await b.click(...p);
      await sleep(150);
    };
    check('the drawing has unsaved changes before Yeni proje', await b.eval('window.kentos.doc.dirty.value'));
    await key('n', { ctrl: true, alt: true });
    await b.waitFor(`document.querySelector('.dialog--newproj')`, 3000).catch(() => {});
    // The name field opens focused with its text selected: typing replaces it.
    await b.type('Ada 200');
    await press('.dialog--newproj .crs-search__input');
    await b.type('5254');
    await press('.dialog--newproj .seg__opt', '1:500');
    // Work mode: an announced mode (Yakında) cannot be chosen; CAD can.
    await press('.dialog--newproj .wspick__card[data-mode="plan3d"]');
    const soonChosen = await b.eval(`document.querySelector('.wspick__card[data-mode="plan3d"]').getAttribute('aria-checked')`);
    await press('.dialog--newproj .wspick__card[data-mode="cad"]');
    await b.shot('newproject-dialog');
    await press('.dialog__foot .btn', 'Oluştur');
    await b.waitFor(`[...document.querySelectorAll('.dialog__foot .btn')].some((x) => x.textContent === 'Kaydetmeden devam et')`, 3000).catch(() => {});
    // Vazgeç in the question goes back to the dialog, with nothing changed.
    const question = '.dialog[aria-label="Kaydedilmemiş değişiklikler"] .dialog__foot .btn';
    await press(question, 'Vazgeç');
    const stayed = await b.eval(`!!document.querySelector('.dialog--newproj') && window.kentos.doc.size > 0 && window.kentos.doc.dirty.value`);
    await press('.dialog--newproj .dialog__foot .btn', 'Oluştur');
    await b.waitFor(`[...document.querySelectorAll('.dialog__foot .btn')].some((x) => x.textContent === 'Kaydetmeden devam et')`, 3000).catch(() => {});
    check('Yeni proje asks about unsaved changes, and Vazgeç returns to its dialog', stayed && !!(await center(question, 'Kaydetmeden devam et')));
    await press(question, 'Kaydetmeden devam et');
    await b.waitFor(`!document.querySelector('.dialog--newproj') && window.kentos.doc.size === 0`, 5000).catch(() => {});
    const np = await b.eval(`(() => { const k = window.kentos; const leaves = k.doc.layers.leaves().map((l) => l.id); return { size: k.doc.size, name: k.doc.name.value, srid: k.doc.crs.value.srid, scale: k.doc.settings.plotScale.value, dirty: k.doc.dirty.value, undo: k.doc.canUndo.value, file: k.files.handle, parcel: leaves.includes('parsel') && leaves.includes('kot'), active: k.doc.layers.active.value, origin: k.doc.origin, mode: k.doc.settings.workspace.value }; })()`);
    check(
      'Yeni proje opens an empty drawing with the chosen name, system, scale and work mode (not an announced one)',
      np.size === 0 && np.name === 'Ada 200' && np.srid === 5254 && np.scale === 500 && np.mode === 'cad' && soonChosen === 'false' && !np.dirty && !np.undo && np.file === null && np.parcel && np.active === 'taslak',
      JSON.stringify(np),
    );
    // A typed line lands exactly; the first Ctrl+S asks where to write, under the project's name.
    await key('l');
    await cmd(`${np.origin.x + 10},${np.origin.y + 10}`);
    await cmd(`${np.origin.x + 60},${np.origin.y + 10}`);
    await key('Escape');
    const drawn = await b.eval('[...window.kentos.doc.all()]');
    await b.eval(`(() => { window.__asked = null; window.kentos.files.picker = { save: async (n) => { window.__asked = n; return window.__files.file(n); }, open: async () => null }; })()`);
    await b.key('s', { ctrl: true });
    await b.waitFor(`!window.kentos.files.busy.value && window.__asked !== null`, 5000).catch(() => {});
    const asked = await b.eval('window.__asked');
    check('a line in the new project, and the first save asks for “Ada 200.kcad”', drawn.length === 1 && drawn[0].b.x === np.origin.x + 60 && asked === 'Ada 200.kcad' && !(await b.eval('window.kentos.doc.dirty.value')), `${asked}`);
    // The saved file heads the recent files; the start screen lists it and opens it again with one click.
    await b.waitFor(`window.kentos.files.recent.list.value[0]?.name === 'Ada 200.kcad'`, 3000).catch(() => {});
    await b.eval(`window.kentos.commands.execute('file.start')`);
    await b.waitFor(`!!document.querySelector('.start .recent')`, 10000).catch(() => {});
    const listed = await b.eval(`[...document.querySelectorAll('.start .recent__name')].map((e) => e.textContent)`);
    // Another (clean) drawing's name meanwhile: opening from the list asks nothing and brings the file's back.
    await b.eval(`(() => { const k = window.kentos; k.doc.name.set('Başka'); k.doc.markSaved(k.doc.revision); })()`);
    await b.eval(`document.querySelector('.start .recent__open').click()`);
    await b.waitFor(`!document.querySelector('.start') && !window.kentos.files.busy.value`, 5000).catch(() => {});
    const reopened = await b.eval(`({ name: window.kentos.doc.name.value, size: window.kentos.doc.size })`);
    check('the start screen lists the saved file first and opens it again', listed[0] === 'Ada 200' && reopened.name === 'Ada 200' && reopened.size === 1, `${JSON.stringify(listed)} ${JSON.stringify(reopened)}`);
    await b.eval(`(() => { window.kentos.files.picker = window.__files.original; window.kentos.files.handle = null; })()`);
    await b.shot('newproject-drawn');

    // CAD mode: no map, coordinate or processing menus and no parcel tool in the toolbox; the parcel command still runs by name.
    const ui = () =>
      b.eval(`({ menus: [...document.querySelectorAll('.menubar__item')].map((m) => m.dataset.menu), parcel: !!document.querySelector('.toolbox [data-tool="parcel"]'), line: !!document.querySelector('.toolbox [data-tool="line"]'), status: document.querySelector('.status__mode').textContent, mode: window.kentos.doc.settings.workspace.value, dirty: window.kentos.doc.dirty.value })`);
    const cadUi = await ui();
    await cmd('PARSEL');
    const byName = await b.eval('window.kentos.tools.activeId.value');
    await key('Escape');
    check(
      'CAD mode hides the map, coordinate and processing menus and the parcel tool, whose command still runs by name',
      !cadUi.menus.includes('map') && !cadUi.menus.includes('crs') && !cadUi.menus.includes('processing') && cadUi.menus.includes('draw') && !cadUi.parcel && cadUi.line && cadUi.status === 'CAD' && byName === 'parcel',
      JSON.stringify({ ...cadUi, byName }),
    );
    // Back to Hibrit from the status bar's mode cell; announced modes are listed with Yakında.
    await press('.status__mode');
    const modeMenu = await b.eval(`[...document.querySelectorAll('.menu__item')].map((e) => e.textContent)`);
    await b.shot('workspace-menu');
    await press('.menu__item', 'Hibrit');
    const hybridUi = await ui();
    check(
      'the status bar switches the work mode back to Hibrit (the project is unsaved); 3D Plan and Afet Analizi say Yakında',
      hybridUi.mode === 'hybrid' && hybridUi.menus.includes('map') && hybridUi.parcel && hybridUi.status === 'Hibrit' && hybridUi.dirty && modeMenu.some((t) => t.includes('3D Plan') && t.includes('Yakında')) && modeMenu.some((t) => t.includes('Afet Analizi') && t.includes('Yakında')),
      JSON.stringify({ ...hybridUi, modeMenu }),
    );
  }

  // Şerit (Uygulama ayarları → Görünüm → Arayüz düzeni): built from the same menu model and tool catalog as the
  // classic shell, switched live. Panels shrink to the window, a selection brings its own tab, the tabs holding the
  // running tool carry a dot, and folded (Ctrl+F1) a tab opens over the drawing until a command runs.
  {
    const at = (sel) =>
      b.eval(`(() => { const e = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.offsetParent); if (!e) return null; e.scrollIntoView({ block: 'nearest' }); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    const click = async (sel) => {
      const p = await at(sel);
      if (!p) throw new Error(`bulunamadı: ${sel}`);
      await b.click(...p);
      await sleep(120);
    };
    const tab = (id) => click(`.ribbon__tab[data-tab="${id}"]`);
    const viewportH = () => b.eval(`window.kentos.view.clientRect().height`);
    const classicH = await viewportH();

    await key(',', { ctrl: true });
    await b.waitFor(`document.querySelector('.layout-card[data-shell="ribbon"]')`, 3000).catch(() => {});
    await click('.layout-card[data-shell="ribbon"]');
    await b.shot('ribbon-settings');
    const save = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.textContent === 'Kaydet'); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    await b.click(...save);
    await b.waitFor(`!!document.querySelector('.ribbon .ribbon__strip .rpanel')`, 15000).catch(() => {});
    const swapped = await b.eval(`({ ribbon: !!document.querySelector('.ribbon'), menubar: !!document.querySelector('.menubar'), toolbar: !!document.querySelector('.toolbar'), toolbox: document.querySelector('.toolbox').hidden, pref: window.kentos.prefs.shell.value })`);
    check('Şerit chosen in Uygulama ayarları replaces the menu bar, toolbar and toolbox at once', swapped.ribbon && !swapped.menubar && !swapped.toolbar && swapped.toolbox && swapped.pref === 'ribbon', JSON.stringify(swapped));

    // Every tool of the catalog and every menu command has a button; the tabs are derived, nothing lists tools twice.
    const tabIds = await b.eval(`[...document.querySelectorAll('.ribbon__tab')].filter((t) => !t.hidden).map((t) => t.dataset.tab)`);
    const seen = new Set();
    for (const id of tabIds) {
      await tab(id);
      // Buttons, the choices of split buttons and the panels' ▾ lists.
      for (const c of await b.eval(`[...document.querySelectorAll('.ribbon__strip [data-command]')].map((e) => e.dataset.command)`)) seen.add(c);
      for (const c of await b.eval(`[...document.querySelectorAll('.ribbon__strip [data-commands]')].flatMap((e) => e.dataset.commands.split(' '))`)) seen.add(c);
    }
    const tools = await b.eval(`window.kentos.tools.list().map((t) => 'tool.' + t.id)`);
    const missing = tools.filter((id) => !seen.has(id));
    const unknown = await b.eval(`${JSON.stringify([...seen])}.filter((id) => !window.kentos.commands.get(id))`);
    check('every tool of the catalog is on the ribbon (a button, a split button or a panel’s ▾), and every button a registered command', missing.length === 0 && unknown.length === 0, [...missing, ...unknown].join(', '));

    // A tool button runs its tool (filled amber); Giriş, which also offers it, carries the running-tool dot.
    await tab('draw');
    await click('.ribbon__strip [data-command="tool.circle"]');
    const running = await b.eval(`({ tool: window.kentos.tools.activeId.value, pressed: document.querySelector('.ribbon__strip [data-command="tool.circle"]').getAttribute('aria-pressed'), dot: document.querySelector('.ribbon__tab[data-tab="home"]').hasAttribute('data-active-tool') })`);
    check('Daire on the ribbon runs the circle tool, marks its button and puts a dot on Giriş', running.tool === 'circle' && running.pressed === 'true' && running.dot, JSON.stringify(running));
    await key('Escape');

    // Daire ▾ lists its methods: 3 nokta starts the tool with that method, and the button remembers it.
    const menuItem = (text) => b.eval(`(() => { const e = [...document.querySelectorAll('.menu__item')].find((x) => x.textContent.includes(${JSON.stringify(text)})); if (!e) return null; const r = e.getBoundingClientRect(); return [Math.round(r.left + 30), Math.round(r.top + r.height / 2)]; })()`);
    await tab('home');
    await click('.ribbon__strip [data-split="circle"] .rsplit__arrow');
    const listed = await b.eval(`[...document.querySelectorAll('.menu__item')].map((e) => e.textContent)`);
    await b.shot('ribbon-split');
    await b.click(...(await menuItem('3 nokta')));
    await sleep(150);
    const method = await b.eval(`({ tool: window.kentos.tools.activeId.value, prompt: window.kentos.tools.prompt.value })`);
    await key('Escape');
    await click('.ribbon__strip [data-split="circle"] .rsplit__main');
    const again = await b.eval(`window.kentos.tools.prompt.value`);
    await key('Escape');
    check(
      'Daire ▾ lists its five methods; 3 nokta starts the circle by three points, and the button starts it so again',
      listed.length === 5 && method.tool === 'circle' && /ilk noktayı/.test(method.prompt) && /ilk noktayı/.test(again),
      `${listed.length} · ${method.prompt} · ${again}`,
    );
    // A seldom used tool waits under its panel's ▾.
    await tab('draw');
    await click('.ribbon__strip .rpanel[data-panel="Eğri"] .rpanel__more');
    await b.click(...(await menuItem('Halka')));
    await sleep(150);
    const donut = await b.eval(`window.kentos.tools.activeId.value`);
    await key('Escape');
    check('Halka is under the Eğri panel’s ▾ and runs from there', donut === 'donut', donut);

    // Narrow window: panels step down (labels, then icons) instead of being cut off.
    const fits = async () => {
      const out = [];
      for (const id of tabIds) {
        await tab(id);
        out.push(await b.eval(`(() => { const s = document.querySelector('.ribbon__strip'); const levels = [...s.querySelectorAll('.rpanel')].map((p) => Number(p.dataset.level)); return { id: '${id}', over: s.scrollWidth > s.clientWidth + 1, shrunk: levels.some((l) => l > 0) }; })()`));
      }
      return out;
    };
    const wide = await fits();
    await b.send('Emulation.setDeviceMetricsOverride', { width: 1100, height: 900, deviceScaleFactor: 1, mobile: false });
    await sleep(300);
    const narrow = await fits();
    await tab('home');
    await b.shot('ribbon-1100');
    await b.send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 900, deviceScaleFactor: 1, mobile: false });
    await sleep(300);
    check(
      'at 1100 px every tab fits by shrinking its panels; at 1600 px Değiştir keeps its labels',
      narrow.every((t) => !t.over) && narrow.some((t) => t.shrunk) && wide.every((t) => !t.over) && !wide.find((t) => t.id === 'modify').shrunk,
      JSON.stringify(narrow.filter((t) => t.over || t.shrunk).map((t) => t.id)),
    );

    // A selection brings the contextual Seçim tab with its count and summary; clearing it takes the tab away.
    await tab('home');
    const line = await b.eval(`[...window.kentos.doc.all()].find((e) => e.kind === 'line').id`);
    await b.eval(`window.kentos.selection.set([${line}])`);
    await sleep(150);
    await tab('selection');
    const summary = await b.eval(`({ tab: document.querySelector('.ribbon__tab[data-tab="selection"]').textContent, count: document.querySelector('.rsel__count').textContent, kinds: document.querySelector('.rsel__kinds').textContent })`);
    await b.shot('ribbon-selection');
    await key('Escape');
    await sleep(150);
    const after = await b.eval(`({ hidden: document.querySelector('.ribbon__tab[data-tab="selection"]').hidden, current: document.querySelector('.ribbon__tab[aria-selected="true"]').dataset.tab })`);
    check('a selection shows the Seçim tab (count, kinds); Esc clears it and returns to Giriş', summary.tab === 'Seçim1' && summary.count === '1' && summary.kinds === '1 çizgi' && after.hidden && after.current === 'home', JSON.stringify({ summary, after }));

    // Folded with Ctrl+F1: the drawing grows; a tab opens over it and a command closes it again.
    await key('F1', { ctrl: true });
    await sleep(200);
    const foldedH = await viewportH();
    await tab('view');
    const peek = await b.eval(`getComputedStyle(document.querySelector('.ribbon__strip')).position`);
    await click('.ribbon__strip [data-command="view.zoomExtents"]');
    const closed = await b.eval(`getComputedStyle(document.querySelector('.ribbon__strip')).display`);
    await key('F1', { ctrl: true });
    await sleep(200);
    check('Ctrl+F1 folds the ribbon to its tabs; a tab opens it over the drawing until a command runs', foldedH > classicH && peek === 'absolute' && closed === 'none' && (await viewportH()) < foldedH, `${classicH} → ${foldedH}, ${peek}, ${closed}`);

    // Komut ara (Alt+Q): a typed name, Enter runs it.
    await key('q', { alt: true });
    const focused = await b.eval(`document.activeElement === document.querySelector('.rsearch__input')`);
    await b.type('elips');
    await sleep(120);
    const found = await b.eval(`[...document.querySelectorAll('.rsearch__title')].map((e) => e.textContent)`);
    await b.key('Enter');
    await sleep(80);
    check('Alt+Q searches commands by name and Enter runs the first', focused && found[0] === 'Elips' && (await b.eval('window.kentos.tools.activeId.value')) === 'ellipse', JSON.stringify(found));
    await key('Escape');

    // Key tips: Alt tapped alone shows letters on the tabs; a tab's letters open it and show the controls'; those run the control.
    const tipOver = (sel) =>
      b.eval(`(() => { const r = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [...document.querySelectorAll('.keytips__tip')].find((t) => { const q = t.getBoundingClientRect(); return Math.abs(q.left + q.width / 2 - (r.left + r.width / 2)) < 3 && q.top > r.top - 2 && q.top < r.bottom + 2; })?.textContent ?? ''; })()`);
    await key('Alt');
    await sleep(120);
    const modifyTip = await tipOver('.ribbon__tab[data-tab="modify"]');
    for (const ch of modifyTip) await b.key(ch.toLowerCase());
    await sleep(250);
    const moveTip = await tipOver('.ribbon__strip [data-command="tool.move"]');
    for (const ch of moveTip) await b.key(ch.toLowerCase());
    await sleep(120);
    check(
      'Alt shows key tips: the tab letters open Değiştir, the button letters start Taşı',
      !!modifyTip && !!moveTip && (await b.eval('window.kentos.tools.activeId.value')) === 'move' && (await b.eval(`!document.querySelector('.keytips')`)),
      `${modifyTip} ${moveTip}`,
    );
    await key('Escape');
    await b.key('F6');
    await sleep(100);
    const f6 = await b.eval(`document.querySelectorAll('.keytips__tip').length`);
    await b.key('Escape');
    check('F6 shows the key tips too; Esc takes them away', f6 > 3 && (await b.eval(`!document.querySelector('.keytips')`)), String(f6));

    // Quick access: right button on a ribbon command adds it to the bar, remembered with the layout.
    await tab('view');
    const zoom = await at('.ribbon__strip [data-command="view.zoomExtents"]');
    await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: zoom[0], y: zoom[1], button: 'none' });
    await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: zoom[0], y: zoom[1], button: 'right', clickCount: 1 });
    await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: zoom[0], y: zoom[1], button: 'right', clickCount: 1 });
    await sleep(120);
    await click('.menu .menu__item');
    check('a right click adds a ribbon command to the quick access bar', await b.eval(`!!document.querySelector('.ribbon__qat [data-command="view.zoomExtents"]') && window.kentos.ui.ribbonQuickAccess.value.includes('view.zoomExtents')`));
    await b.shot('ribbon-dark');
    await b.eval(`window.kentos.commands.execute('view.theme.light')`);
    await sleep(150);
    await b.shot('ribbon-light');
    await b.eval(`window.kentos.commands.execute('view.theme.dark')`);

    // Back to the classic shell from the ribbon's own Şerit arayüzü button: menus and toolbox return.
    await click('.ribbon__strip [data-command="view.ribbon"]');
    await b.waitFor(`!!document.querySelector('.menubar')`, 3000).catch(() => {});
    const back = await b.eval(`({ ribbon: !!document.querySelector('.ribbon'), menubar: !!document.querySelector('.menubar'), toolbox: !document.querySelector('.toolbox').hidden })`);
    check('the Şerit arayüzü button returns to menus and toolbox', !back.ribbon && back.menubar && back.toolbox && (await viewportH()) === classicH, JSON.stringify(back));
  }

  // The KentOS mark opens the application menu (loaded on first use): files on the left, formats and the cloud on
  // the right; Esc closes it, a row runs its command.
  {
    const at = (sel) => b.eval(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    const loadedBefore = await b.eval(`performance.getEntriesByType('resource').some((r) => r.name.includes('AppMenu'))`);
    await b.click(...(await at('.brand')));
    await b.waitFor(`!!document.querySelector('.appmenu')`, 5000).catch(() => {});
    const opened = await b.eval(`({ expanded: document.querySelector('.brand').getAttribute('aria-expanded'), nav: [...document.querySelectorAll('.appmenu__item .appmenu__label')].map((e) => e.textContent), pane: document.querySelector('.appmenu__pane').dataset.pane })`);
    await b.move(...(await at('.appmenu__item[data-pane="import"]')));
    await sleep(200);
    const importRows = await b.eval(`[...document.querySelectorAll('.appmenu__pane [data-command]')].map((e) => e.dataset.command)`);
    await b.move(...(await at('.appmenu__item[data-pane="cloud"]')));
    await sleep(200);
    const cloudPane = await b.eval(`document.querySelector('.appmenu__pane').dataset.pane`);
    await b.shot('appmenu-cloud');
    await b.key('Escape');
    await sleep(150);
    const closed = await b.eval(`!document.querySelector('.appmenu') && document.querySelector('.brand').getAttribute('aria-expanded') === 'false'`);
    await b.click(...(await at('.brand')));
    await b.waitFor(`!!document.querySelector('.appmenu')`, 5000).catch(() => {});
    await b.click(...(await at('.appmenu__item[data-command="file.settings"]')));
    await sleep(300);
    const ran = await b.eval(`({ menu: !!document.querySelector('.appmenu'), dialog: !!document.querySelector('.dialog') })`);
    await b.key('Escape');
    await sleep(200);
    check(
      'the KentOS mark opens the application menu (loaded then): Yeni to Proje ayarları, İçe aktar lists the formats, Bulut its pane; Esc closes; a row runs its command',
      !loadedBefore &&
        opened.expanded === 'true' &&
        opened.nav[0] === 'Yeni' &&
        opened.nav.includes('Bulut') &&
        opened.pane === 'overview' &&
        importRows.includes('file.import.dxf') &&
        importRows.includes('file.import.ncn') &&
        cloudPane === 'cloud' &&
        closed &&
        !ran.menu &&
        ran.dialog,
      JSON.stringify({ loadedBefore, opened, importRows, cloudPane, closed, ran }),
    );
  }

  // Proje ayarları → Çizim yazı tipi (a project setting), Çizim kalitesi (anti-aliasing made again live) and Tam ekran.
  {
    const at = (sel) => b.eval(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; e.scrollIntoView({ block: 'nearest' }); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    const press = async (sel) => {
      const p = await at(sel);
      if (!p) throw new Error(`bulunamadı: ${sel}`);
      await b.click(...p);
      await sleep(150);
    };
    const saveDialog = async () => {
      const p = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.textContent === 'Kaydet'); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
      await b.click(...p);
      await sleep(300);
    };
    await b.eval(`window.kentos.commands.execute('file.settings')`);
    await b.waitFor(`!!document.querySelector('.font-pick--drawing')`, 3000).catch(() => {});
    await press('.font-pick--drawing [data-font="arimo"]');
    await b.shot('drawing-font-settings');
    await saveDialog();
    await b.waitFor(`document.fonts.check('500 12px Arimo')`, 5000).catch(() => {});
    await sleep(300);
    const font = await b.eval(`({ setting: window.kentos.doc.settings.drawingFont.value, css: getComputedStyle(document.documentElement).getPropertyValue('--font-drawing').trim(), loaded: document.fonts.check('500 12px Arimo'), dirty: window.kentos.doc.dirty.value, file: window.kentos.doc.settings.toJSON().drawingFont })`);
    await b.shot('drawing-font-arimo');
    check(
      'Proje ayarları sets the drawing typeface (Arimo): the project is unsaved, the file will carry it, the bundled face is loaded',
      font.setting === 'arimo' && font.css.startsWith('Arimo') && font.loaded && font.dirty && font.file === 'arimo',
      JSON.stringify(font),
    );
    await b.eval(`window.kentos.doc.settings.drawingFont.set('barlow')`);

    // Kenar yumuşatma (TODOS.md AA-01/02, SET-03/05): the counts come from the context, a preset only fills
    // values, a change applies live on the same canvas and backend, and a count the device lacks is kept as
    // asked while the nearest one it has draws, with the reason in the window.
    const gpu = () =>
      b.eval(`({ kind: window.kentos.view.backendKind.value, samples: window.kentos.view.samples, counts: [...window.kentos.view.sampleCounts], same: document.querySelector('canvas.viewport__gl') === window.__smokeCanvas, canvases: document.querySelectorAll('canvas.viewport__gl').length, msaa: (({ requested, effective, reason }) => ({ requested, effective, reason }))(window.kentos.settingsStore.resolved('graphics.msaa')) })`);
    const segment = (label, text) =>
      b.eval(`(() => { const e = [...document.querySelectorAll('[aria-label="${label}"] .seg__opt')].find((x) => x.textContent === ${JSON.stringify(text)}); e.scrollIntoView({ block: 'nearest' }); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    for (const kind of ['webgl2', 'webgpu']) {
      await b.eval(`window.kentos.commands.execute('view.renderer.${kind}')`);
      await b.waitFor(`window.kentos.view.backendKind.value === '${kind}' && document.querySelectorAll('canvas.viewport__gl').length === 1`, 8000).catch(() => {});
      await b.eval(`window.__smokeCanvas = document.querySelector('canvas.viewport__gl')`);
      await b.eval(`window.kentos.commands.execute('tools.options', 'engine')`);
      await b.waitFor(`!!document.querySelector('[aria-label="Grafik hazır ayarı"]')`, 3000).catch(() => {});
      await b.click(...(await segment('Grafik hazır ayarı', 'Hızlı')));
      await sleep(150);
      await saveDialog();
      await b.waitFor(`window.kentos.view.samples === 1`, 8000).catch(() => {});
      const fast = await gpu();
      // 8× asked for: WebGPU validates 1 and 4 only; SwiftShader's WebGL2 reports its own list.
      await b.eval(`window.kentos.commands.execute('tools.options', 'engine')`);
      await b.waitFor(`!!document.querySelector('[aria-label="Kenar yumuşatma (MSAA)"]')`, 3000).catch(() => {});
      await b.click(...(await segment('Kenar yumuşatma (MSAA)', '8×')));
      await b.eval(`document.querySelector('[aria-label="Tam çözünürlük (HiDPI)"]').click()`);
      await sleep(200);
      await b.eval(`document.querySelector('[aria-label="Kenar yumuşatma (MSAA)"]').scrollIntoView({ block: 'start' })`);
      await b.shot(`settings-engine-${kind}-dark`);
      const shown = await b.eval(`[...document.querySelectorAll('.settings__content .note')].map((n) => n.textContent).join(' | ')`);
      await saveDialog();
      await sleep(300);
      const eight = await gpu();
      const expected = Math.max(...eight.counts.filter((c) => c <= 8));
      check(
        `${kind}: kenar yumuşatma changes live on the same canvas; 8× asked for draws with the device's ${expected}× and the window says so`,
        fast.samples === 1 &&
          fast.same &&
          fast.canvases === 1 &&
          eight.same &&
          eight.canvases === 1 &&
          eight.samples === expected &&
          eight.msaa.requested === 8 &&
          eight.msaa.effective === expected &&
          (expected === 8 ? !eight.msaa.reason : eight.msaa.reason === 'device_unsupported' && shown.includes('İstenen 8×')) &&
          (kind !== 'webgpu' || JSON.stringify(eight.counts) === '[1,4]'),
        JSON.stringify({ fast, eight, shown: shown.slice(0, 160) }),
      );
    }
    // A count whose targets cannot be made (forced past the context's limit on WebGL2) falls back to the last
    // working one and says so; the drawing keeps being drawn (TODOS.md AA-02).
    await b.eval(`window.kentos.commands.execute('view.renderer.webgl2')`);
    await b.waitFor(`window.kentos.view.backendKind.value === 'webgl2'`, 8000).catch(() => {});
    await b.eval(`window.kentos.prefs.msaa.set(4)`);
    await sleep(300);
    const failed = await b.eval(`(async () => {
      const k = window.kentos;
      const backend = k.view.backend;
      const before = backend.samples;
      const counts = backend.sampleCounts;
      backend.sampleCounts = [...counts, 64];
      backend.setSamples(64);
      k.view.requestRender();
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const warned = k.log.entries.value.some((e) => e.level === 'warn' && e.text.includes('64× bu aygıtta kurulamadı'));
      const result = { before, after: backend.samples, warned, drawing: !!backend.draw, reason: k.settingsStore.resolved('graphics.msaa').reason ?? null };
      // The forced count was the test's, not the device's.
      backend.sampleCounts = counts;
      k.log.clear();
      return result;
    })()`);
    check('a sample count the device refuses falls back to the last working one and is reported', failed.after === failed.before && failed.warned && failed.drawing, JSON.stringify(failed));
    await b.eval(`window.kentos.commands.execute('tools.options', 'engine')`);
    await b.waitFor(`!!document.querySelector('[aria-label="Grafik hazır ayarı"]')`, 3000).catch(() => {});
    await b.click(...(await segment('Grafik hazır ayarı', 'Kaliteli')));
    await sleep(150);
    await saveDialog();
    await b.waitFor(`window.kentos.view.samples === 4`, 8000).catch(() => {});
    check('the Kaliteli preset brings 4× back', (await b.eval('window.kentos.view.samples')) === 4, String(await b.eval('window.kentos.view.samples')));
    // The window in both themes (engine and settings file sections), for the design review.
    for (const theme of ['light', 'dark']) {
      await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
      for (const section of ['engine', 'file']) {
        await b.eval(`window.kentos.commands.execute('tools.options', '${section}')`);
        await b.waitFor(`!!document.querySelector('.dialog--settings')`, 3000).catch(() => {});
        await b.eval(`document.querySelector('[aria-label="Grafik hazır ayarı"]')?.scrollIntoView({ block: 'start' })`);
        await sleep(200);
        await b.shot(`settings-${section}-${theme}`);
        await b.key('Escape');
        await sleep(150);
      }
    }

    // Tam ekran from the menu bar's button, and back.
    await press('.menubar__icon[data-command="view.fullscreen"]');
    await sleep(400);
    const full = await b.eval('!!document.fullscreenElement');
    await press('.menubar__icon[data-command="view.fullscreen"]');
    await sleep(400);
    const back = await b.eval('!!document.fullscreenElement');
    check('the Tam ekran button fills the screen and leaves it', full && !back, JSON.stringify({ full, back }));
  }

  // Uygulama ayarları → Görünüm: accent colour and typeface, applied on Kaydet; the typefaces come with the app.
  {
    const at = (sel) => b.eval(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; e.scrollIntoView({ block: 'nearest' }); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    const press = async (sel) => {
      const p = await at(sel);
      if (!p) throw new Error(`bulunamadı: ${sel}`);
      await b.click(...p);
      await sleep(150);
    };
    const before = await b.eval(`({ accent: document.documentElement.dataset.accent, font: getComputedStyle(document.body).fontFamily.split(',')[0].replaceAll('"', ''), jakarta: document.fonts.check('600 13px "Plus Jakarta Sans"', 'ğşıİ') })`);
    await b.eval(`window.kentos.commands.execute('tools.options', 'appearance')`);
    await b.waitFor(`!!document.querySelector('.accent-pick')`, 3000).catch(() => {});
    await press('.accent-pick__opt[data-accent="bordeaux"]');
    await press('.font-pick__card[data-font="inter"]');
    await b.shot('appearance-settings');
    const save = await b.eval(`(() => { const e = [...document.querySelectorAll('.dialog__foot .btn')].find((x) => x.textContent === 'Kaydet'); const r = e.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    await b.click(...save);
    await b.waitFor(`document.fonts.check('13px "Inter"')`, 5000).catch(() => {});
    // Preferences are written to the typed settings a moment after they change (docs/adr/0023).
    await b.waitFor(`JSON.parse(localStorage.getItem('kentos.settings.v1') ?? '{}').user?.['appearance.uiFont'] === 'inter'`, 3000).catch(() => {});
    const after = await b.eval(`({
      accent: document.documentElement.dataset.accent,
      fill: getComputedStyle(document.documentElement).getPropertyValue('--c-accent').trim(),
      font: getComputedStyle(document.body).fontFamily.split(',')[0].replaceAll('"', ''),
      inter: document.fonts.check('13px "Inter"'),
      stored: Object.fromEntries(Object.entries(JSON.parse(localStorage.getItem('kentos.settings.v1')).user).map(([k, v]) => [k.split('.')[1], v])),
      foreign: performance.getEntriesByType('resource').map((r) => r.name).filter((n) => !n.startsWith(location.origin) && !n.startsWith('data:') && !n.startsWith('blob:')),
    })`);
    check(
      'Lacivert and Plus Jakarta Sans by default; Bordo and Inter apply on Kaydet and are remembered; nothing is fetched from another host',
      before.accent === 'navy' && before.font === 'Plus Jakarta Sans' && before.jakarta && after.accent === 'bordeaux' && after.fill === '#c24a63' && after.font === 'Inter' && after.inter && after.stored.accent === 'bordeaux' && after.stored.uiFont === 'inter' && after.foreign.length === 0,
      JSON.stringify({ before, after: { ...after, stored: { accent: after.stored?.accent, uiFont: after.stored?.uiFont } } }),
    );
    await b.eval(`(async () => { const a = await import('/src/app/appearance.ts'); window.kentos.prefs.accent.set('navy'); window.kentos.prefs.uiFont.set('jakarta'); a.applyAccent('navy'); await a.applyUiFont('jakarta'); window.kentos.view.refreshPalette(); })()`);
  }

  // Opening a .kcad: the objects get the persistent ids its content derives (ADR 0014), worked out by the Rust
  // contracts in the formats worker; the independent Python reference wrote the same (fixtures/document/v1/identity).
  {
    const dir = new URL('../../../../fixtures/document/v1/identity/', import.meta.url);
    const text = readFileSync(new URL('sample.compact.kcad', dir), 'utf8');
    const expected = JSON.parse(readFileSync(new URL('expected.json', dir), 'utf8')).cases[0].entities;
    const opened = await b.eval(`(async () => {
      const k = window.kentos;
      const keep = { picker: k.files.picker, ask: k.files.ask };
      k.files.ask = async () => 'drop';
      k.files.picker = { open: async () => ({ name: 'kimlik.kcad', getFile: async () => new Blob([${JSON.stringify(text)}]) }), save: async () => null };
      try {
        const ok = await k.files.open();
        return { ok, ids: [...k.doc.all()].map((e) => ({ id: e.id, uid: e.uid })) };
      } finally {
        k.files.picker = keep.picker;
        k.files.ask = keep.ask;
      }
    })()`);
    check('opening a v1 drawing gives its objects the ids its content derives', opened.ok && JSON.stringify(opened.ids) === JSON.stringify(expected), JSON.stringify(opened.ids.slice(0, 2)));
  }

  const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l));
  check('no console errors', errors.length === 0, errors.join(' | '));
} catch (e) {
  failures.push(String(e));
  console.error(e);
} finally {
  b.close();
  await server.close();
  api.close();
}
console.log(failures.length ? `\n${failures.length} kontrol başarısız.` : '\nTüm kontroller geçti.');
process.exit(failures.length ? 1 : 0);
