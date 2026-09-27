// The expression's flow with the mouse and the keyboard (docs/adr/0101), over İfadeyle seç: Akış shows
// the text as nodes with their values; a value written in the inspector changes the text; an input's
// connection is dragged off and back on; a double click in the tree puts a node down; Delete removes
// the selected node and Ctrl+Z brings it back; an operator button adds a node into the selected node's
// empty input; Tamam writes the flow's text back.
// Exits 1 when a check fails.
//
//   node scripts/e2e/flow.mjs     (from apps/web)
import { createServer } from 'vite';
import { launch, sleep } from './cdp.mjs';

const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
const b = await launch('about:blank', { width: 1440, height: 900 });
let failed = 0;
const check = (name, ok, got) => {
  if (!ok) failed++;
  console.log(`${ok ? '✓' : '✗'} ${name}${ok ? '' : `: ${JSON.stringify(got)}`}`);
};
const text = () => b.eval(`document.querySelector('.dialog--exprb .xed__input').value`);
const field = () => b.eval(`document.querySelector('.pfield__expr').value`);
const nodes = () => b.eval(`[...document.querySelectorAll('.xfn')].map((n) => n.dataset.id)`);
const centre = (sel) =>
  b.eval(`(() => { const el = document.querySelector(${JSON.stringify(sel)}); if (!el) return null; const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
const click = async (sel, clickCount = 1) => {
  const at = await centre(sel);
  if (!at) throw new Error(`yok: ${sel}`);
  await b.click(...at, { clickCount });
  await sleep(200);
};
const dblclick = async (sel) => {
  const at = await centre(sel);
  if (!at) throw new Error(`yok: ${sel}`);
  await b.click(...at);
  await b.click(...at, { clickCount: 2 });
  await sleep(250);
};
const drag = async (from, to) => {
  const a = Array.isArray(from) ? from : await centre(from);
  const z = Array.isArray(to) ? to : await centre(to);
  if (!a || !z) throw new Error(`yok: ${JSON.stringify([from, to])}`);
  await b.drag(...a, ...z);
  await sleep(250);
};

try {
  await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
  await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
  await sleep(800);
  await b.eval(`window.kentos.commands.execute('processing.run.selection.byExpression')`);
  await b.waitFor(`document.querySelector('.exprb-open')`, 8000);
  await click('.exprb-open');
  await b.waitFor(`document.querySelector('.dialog--exprb .xed__input')`, 8000);
  await sleep(250);
  await b.eval(`(() => { const t = document.querySelector('.dialog--exprb .xed__input'); t.focus(); t.select(); })()`);
  await b.type('yuvarla($alan, 2)');
  await sleep(150);

  // Akış: the same text as nodes, with their values.
  await click('.exprb__tab:last-child');
  await b.waitFor(`document.querySelector('.xfn[data-id="0"]')`, 4000);
  check('Akış draws the text’s nodes', JSON.stringify(await nodes()) === JSON.stringify(['r', '0', '0.0', '0.1']), await nodes());
  const result = await b.eval(`document.querySelector('.xfn[data-id="r"] .xfn__value').textContent`);
  const preview = await b.eval(`document.querySelector('.exprb__pvalue').textContent`);
  check('the result shows the preview', result === preview && result !== '—', [result, preview]);
  check('a field’s node shows its value', await b.eval(`/^= /.test(document.querySelector('.xfn[data-id="0.0"] .xfn__value').textContent)`), null);

  // A value written in the inspector.
  await click('.xfn[data-id="0.1"] .xfn__title');
  await b.waitFor(`document.querySelector('.xfi:not([hidden]) .xfi__input')`, 3000);
  await b.eval(`(() => { const i = document.querySelector('.xfi__input'); i.focus(); i.select(); })()`);
  await b.type('3');
  await b.key('Enter');
  await sleep(200);
  check('the inspector writes the value', (await text()) === 'yuvarla($alan, 3)', await text());

  // An input's connection dragged off (apart), then back on.
  const pin = await centre('.xfn[data-id="0"] .xfn__port[data-port="1"] .xfn__pin');
  await drag(pin, [pin[0] - 60, pin[1] + 160]);
  check('dragged off, the input empties', (await text()) === 'yuvarla($alan)', await text());
  check('… and the value stands apart', (await nodes()).includes('1'), await nodes());
  await drag('.xfn[data-id="1"] .xfn__out', '.xfn[data-id="0"] .xfn__port[data-port="1"]');
  check('dragged back on', (await text()) === 'yuvarla($alan, 3)', await text());

  // The tree: a double click puts a node down; Delete removes it, Ctrl+Z brings it back.
  await click('.xflow');
  await dblclick('.xtree__row.xtree__item');
  const added = await nodes();
  check('a double click in the tree puts a node down', added.includes('1'), added);
  await b.eval(`document.querySelector('.xflow').focus()`);
  await b.key('Delete');
  await sleep(200);
  check('Delete removes the selected node', !(await nodes()).includes('1'), await nodes());
  await b.key('z', { ctrl: true });
  await sleep(200);
  check('Ctrl+Z brings it back', (await nodes()).includes('1'), await nodes());

  // An operator button: into the selected node's empty input.
  await drag('.xfn[data-id="0"] .xfn__port[data-port="0"] .xfn__pin', [pin[0] - 80, pin[1] + 260]);
  await click('.xfn[data-id="0"] .xfn__title');
  await click('.exprb__op');
  check('an operator goes into the empty input', /^yuvarla\(\? = \?/.test(await text()), await text());
  check('the empty input is the error', await b.eval(`document.querySelector('.exprb__status--error') !== null`), null);

  // Metin shows the same text; Tamam writes it back.
  await click('.exprb__tab:first-child');
  await b.eval(`(() => { const t = document.querySelector('.dialog--exprb .xed__input'); t.focus(); t.select(); })()`);
  await b.type('$alan * 2');
  await click('.exprb__tab:last-child');
  check('the text’s change is the flow’s', JSON.stringify(await nodes()).includes('"0.1"'), await nodes());
  await click('.dialog--exprb .dialog__foot .btn--primary');
  await sleep(200);
  check('Tamam writes the text back', (await field()) === '$alan * 2', await field());

  const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l) && !l.includes('/v1/'));
  check('no console errors', errors.length === 0, errors.slice(0, 3));
} catch (e) {
  failed++;
  console.log(`✗ ${String(e.message ?? e)}`);
} finally {
  b.close();
  await server.close();
}
console.log(failed ? `\n${failed} denetim düştü.` : '\nAkış denetimleri geçti.');
process.exit(failed ? 1 : 0);
