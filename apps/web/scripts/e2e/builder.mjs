// The expression builder with the keyboard and the mouse (DESIGN.md §7.16, docs/adr/0100 §5), over
// İfadeyle seç: the ε button opens it on the field's text; typing a name opens the completion list,
// Enter takes an entry (a function with the cursor between its parentheses), Esc closes the list and not
// the window; Ctrl+Space lists everything; a double click on the tree puts an entry in, around the
// selection for a function; a field's values go in as the language writes them; the preview steps
// through the objects; Vazgeç leaves the field as it was and Tamam writes the text back.
// Exits 1 when a check fails.
//
//   node scripts/e2e/builder.mjs     (from apps/web)
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
const caret = () => b.eval(`document.querySelector('.dialog--exprb .xed__input').selectionStart`);
const field = () => b.eval(`document.querySelector('.pfield__expr').value`);
const listOpen = () => b.eval(`!!document.querySelector('.xed__list:not([hidden])')`);
const builderOpen = () => b.eval(`!!document.querySelector('.dialog--exprb')`);
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
const openBuilder = async () => {
  await click('.exprb-open');
  await b.waitFor(`document.querySelector('.dialog--exprb .xed__input')`, 8000);
  await sleep(250);
};
const setText = async (s) => {
  await b.eval(`(() => { const t = document.querySelector('.dialog--exprb .xed__input'); t.focus(); t.select(); })()`);
  await b.type(s);
  await sleep(150);
};

try {
  await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
  await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
  await sleep(800);
  await b.eval(`window.kentos.commands.execute('processing.run.selection.byExpression')`);
  await b.waitFor(`document.querySelector('.exprb-open')`, 8000);
  const before = await field();

  // Opening: the field's text, the cursor at its end.
  await openBuilder();
  check('opens on the field’s text', (await text()) === before, await text());
  check('the cursor at the end', (await caret()) === before.length, await caret());

  // Typing a name: the list, Enter takes the function with the cursor inside.
  await setText('yuv');
  check('typing a name opens the list', await listOpen(), await listOpen());
  await b.key('Enter');
  check('Enter takes yuvarla()', (await text()) === 'yuvarla()', await text());
  check('the cursor between the parentheses', (await caret()) === 8, await caret());
  check('the signature shows', await b.eval(`document.querySelector('.xed__sigcode')?.textContent === 'yuvarla(sayı, basamak)'`), null);

  // Esc closes the list, not the window.
  await b.type('$al');
  await sleep(120);
  check('$ opens the variables', await listOpen(), await listOpen());
  await b.key('Escape');
  await sleep(150);
  check('Esc closes the list', !(await listOpen()), await listOpen());
  check('… and not the window', await builderOpen(), await builderOpen());

  // Ctrl+Space where nothing is being written: everything.
  await setText('Ada + ');
  await b.key(' ', { ctrl: true });
  await sleep(150);
  const count = await b.eval(`document.querySelectorAll('.xed__list:not([hidden]) .xed__item').length`);
  check('Ctrl+Space lists everything', count > 40, count);
  await b.key('Escape');

  // The tree: a double click on a function wraps the selection.
  await setText('$alan');
  await b.eval(`document.querySelector('.dialog--exprb .xed__input').setSelectionRange(0, 5)`);
  await b.eval(`document.querySelector('.xtree__search').focus()`);
  await b.type('mutlak');
  await sleep(200);
  // The search took the focus; the editor keeps its selection for the insert.
  await dblclick('.xtree__row.xtree__item');
  check('a function wraps the selection', (await text()) === 'mutlak($alan)', await text());

  // A field's values go in quoted.
  await b.eval(`(() => { const s = document.querySelector('.xtree__search'); s.value = ''; s.dispatchEvent(new Event('input')); })()`);
  await sleep(150);
  await setText("Ada = ");
  await click('.xtree__row.xtree__item');
  await click('.xhelp__vbtns .btn');
  await b.waitFor(`document.querySelector('.xhelp__value')`, 4000);
  const value = await b.eval(`document.querySelector('.xhelp__value').textContent`);
  await dblclick('.xhelp__value');
  const got = await text();
  check('a value goes in quoted', got.startsWith('Ada = ') && got.includes(`'${value.replace(/'/g, "''")}'`), got);

  // The preview steps through the objects.
  await setText('$sıra');
  const first = await b.eval(`document.querySelector('.exprb__pvalue').textContent`);
  await click('.exprb__step .ibtn:last-of-type');
  const second = await b.eval(`document.querySelector('.exprb__pvalue').textContent`);
  check('the preview follows the object', first === '1' && second === '2', [first, second]);

  // Vazgeç leaves the field; Tamam writes the text back.
  await click('.dialog--exprb .dialog__foot .btn:not(.btn--primary)');
  check('Vazgeç closes', !(await builderOpen()), await builderOpen());
  check('… and leaves the field', (await field()) === before, await field());
  await openBuilder();
  await setText("Nitelik = 'Arsa'");
  await b.key('Enter', { ctrl: true });
  await sleep(200);
  check('Ctrl+Enter is Tamam', !(await builderOpen()), await builderOpen());
  check('… and writes the text back', (await field()) === "Nitelik = 'Arsa'", await field());

  const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l) && !l.includes('/v1/'));
  check('no console errors', errors.length === 0, errors.slice(0, 3));
} catch (e) {
  failed++;
  console.log(`✗ ${String(e.message ?? e)}`);
} finally {
  b.close();
  await server.close();
}
console.log(failed ? `\n${failed} denetim düştü.` : '\nİfade oluşturucu denetimleri geçti.');
process.exit(failed ? 1 : 0);
