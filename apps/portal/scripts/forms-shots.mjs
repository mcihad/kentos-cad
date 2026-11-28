// The forms' acceptance in the cloud portal (docs/adr/9194): each of the three accounts signs in to the cloud server
// through its sign-in page with the keyboard, goes to Projeler and opens the project the desktop acceptance made
// (apps/desktop/src/shell/cloud/live_forms.rs) from the list, as a person would. Mehmet, its Veri girişi, reads team A's
// records with Durum and Bolge only and is told which fields his role does not read; Elif, its Görüntüleyici, and Ayşe,
// its owner, read every record and field, the note never written (NULL) apart from the empty one. What each page says
// is checked, then pictured; Mehmet's way in is pictured step by step. Run by apps/desktop/scripts/forms-roles-live.sh:
//
//   PORTAL_URL=http://127.0.0.1:5181/ PORTAL_DEMO_PASSWORD=… node scripts/forms-shots.mjs
//
// The project's ids come from .run/cloud-forms/project.json (FORMS_PROJECT); the pictures go to
// .run/gui-evidence/kullanim-9194/ (never in Git). Chrome: $CHROME_BIN, else the one in /Applications.
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { launch, sleep } from './cdp.mjs';

const ROOT = join(new URL('.', import.meta.url).pathname, '../../..');
const PORTAL = process.env.PORTAL_URL ?? 'http://127.0.0.1:5180/';
const PASSWORD = process.env.PORTAL_DEMO_PASSWORD;
if (!PASSWORD) throw new Error('PORTAL_DEMO_PASSWORD gerekli (apps/desktop/scripts/forms-roles-live.sh).');
const { tenant, project } = JSON.parse(readFileSync(process.env.FORMS_PROJECT ?? join(ROOT, '.run/cloud-forms/project.json'), 'utf8'));
const OUT = join(ROOT, '.run/gui-evidence/kullanim-9194');
mkdirSync(OUT, { recursive: true });
const MAC_CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
if (!process.env.CHROME_BIN && existsSync(MAC_CHROME)) process.env.CHROME_BIN = MAC_CHROME;

// A Turkish browser, tall enough for the drawing and the records together.
const TR = ['--lang=tr-TR', '--accept-lang=tr-TR,tr'];
const SIZE = { width: 1440, height: 1560 };

/** A screen's worth from the top: the sign-in page, the first page, the list. */
const SCREEN = { x: 0, y: 0, width: 1440, height: 900 };

async function picture(b, name, clip = null) {
  const file = await b.shot(name, clip, OUT);
  console.log(`resim: ${file}`);
}

/** Signs in through the sign-in page with the keyboard; with `steps`, the filled page and the first one are pictured. */
async function signIn(login, steps) {
  const b = await launch(`${PORTAL}#/giris`, { ...SIZE, args: TR });
  await b.waitFor(`!!document.querySelector('input[name=login]')`, 20000);
  await b.eval(`document.querySelector('input[name=login]').focus()`);
  await b.type(login);
  await b.eval(`document.querySelector('input[name=password]').focus()`);
  await b.type(PASSWORD);
  const masked = await b.eval(`document.querySelector('input[name=password]').type === 'password'`);
  check(login, 'parola alanı maskeli', masked, masked);
  if (steps) {
    // The card stands in the middle of the tall page: a screen's worth around it.
    const middle = await b.eval(`(() => { const r = document.querySelector('.auth-card').getBoundingClientRect(); return r.top + r.height / 2; })()`);
    const y = Math.max(0, Math.min(SIZE.height - SCREEN.height, Math.round(middle - SCREEN.height / 2)));
    await picture(b, steps.signIn, { ...SCREEN, y });
  }
  await b.key('Enter');
  await b.waitFor(`!!document.querySelector('.shell .content > .page') && !document.querySelector('.content > .loading')`, 20000);
  await sleep(500);
  if (steps) await picture(b, steps.first, SCREEN);
  return b;
}

/** Projeler in the side bar, then the project's row: its page, its records and its drawing loaded. */
async function projectPage(b, login, steps) {
  // The page drawn anew, not the one before it (the first page may hold a table of its own).
  await b.eval(`window.__before = document.querySelector('.content > *'); true`);
  const found = await b.eval(`(() => { const a = [...document.querySelectorAll('.shell a')].find((e) => e.textContent.trim() === 'Projeler'); if (a) a.click(); return !!a; })()`);
  check(login, 'yan çubukta Projeler', found, found);
  await b.waitFor(
    `(() => { const el = document.querySelector('.content > *'); return !!el && el !== window.__before && !el.classList.contains('loading') && !!el.querySelector('.table, .table-empty'); })()`,
    20000,
  );
  await sleep(300);
  const row = await b.eval(`(() => { const r = [...document.querySelectorAll('.content tr.row-link')].find((r) => r.textContent.includes('Saha denetimleri')); return r ? r.innerText : ''; })()`);
  check(login, 'proje listede, rolüyle', row.includes(ROLE_OF[login]), row || (await b.eval(`document.querySelector('.content').innerText`)));
  if (steps) await picture(b, steps.list, SCREEN);
  await b.eval(`[...document.querySelectorAll('.content tr.row-link')].find((r) => r.textContent.includes('Saha denetimleri')).click(); true`);
  await b.waitFor(`!!document.querySelector('.records-layer') && !document.querySelector('.preview-frame.is-loading')`, 30000);
  await sleep(400);
  const at = await b.eval(`location.hash`);
  check(login, 'satırdan projenin sayfası', at === `#/proje/${tenant}/${project}`, at);
  const text = (selector) => b.eval(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); return el ? el.innerText : ''; })()`);
  return {
    records: await text('.records'),
    access: await text('.access'),
    drawing: await text('.preview'),
    svg: await b.eval(`!!document.querySelector('a[href*="preview.svg"]')`),
  };
}

function check(login, what, ok, seen) {
  if (!ok) throw new Error(`${login}: ${what} — sayfada: ${JSON.stringify(seen)}`);
  console.log(`${login}: ${what}`);
}

const NOTE = 'Teknik ekip notu';
const WITHHELD = 'Rolünüz bu alanları okumuyor, gösterilmiyor: Not, Ekip';

// Each account's role in the project, as the list says it.
const ROLE_OF = { mehmet: 'Veri girişi', elif: 'Görüntüleyici', ayse: 'Sahip' };

// Mehmet's way in, step by step.
const WAY_IN = { signIn: '14-portal-giris', first: '15-portal-acilis', list: '16-portal-projeler' };

for (const [login, page, role, expect] of [
  [
    'mehmet',
    '17-portal-veri-girisi',
    'Veri girişi',
    ({ records: r, drawing, svg }) => [
      ['yalnız A ekibinin iki kaydı', r.includes('D-1') && r.includes('D-3') && !r.includes('D-2') && !r.includes('D-4')],
      ['notun değeri yok', !r.includes(NOTE) && !r.includes('Kuzey ekibine')],
      ['okumadığı alanlar adıyla', r.includes(WITHHELD)],
      ['kapsamın kayıtları daralttığı', r.includes('Formun kapsamı bu katmanın yalnız bazı kayıtlarını size açıyor')],
      ['çizim nedeniyle kapalı, SVG düğmesi yok', drawing.includes('Veri girişi projenin bütün çizimini görmez') && !svg],
    ],
  ],
  [
    'elif',
    '18-portal-goruntuleyici',
    'Görüntüleyici',
    ({ records: r, drawing, svg }) => [
      ['dört kayıt', ['D-1', 'D-2', 'D-3', 'D-4'].every((d) => r.includes(d))],
      ['notlar ve hiç yazılmamış not (NULL)', r.includes(NOTE) && r.includes('Kuzey ekibine') && r.includes('NULL')],
      ['gizlenen alan yok', !r.includes('okumuyor')],
      ['çizim ve SVG düğmesi', drawing.includes('4 nesne çizildi') && svg],
    ],
  ],
  [
    'ayse',
    '19-portal-sahip',
    'Sahip',
    ({ records: r, drawing, svg }) => [
      ['dört kayıt', ['D-1', 'D-2', 'D-3', 'D-4'].every((d) => r.includes(d))],
      ['notlar ve hiç yazılmamış not (NULL)', r.includes(NOTE) && r.includes('NULL')],
      ['gizlenen alan yok', !r.includes('okumuyor')],
      ['çizim ve SVG düğmesi', drawing.includes('4 nesne çizildi') && svg],
    ],
  ],
]) {
  const steps = login === 'mehmet' ? WAY_IN : null;
  const b = await signIn(login, steps);
  try {
    const seen = await projectPage(b, login, steps);
    check(login, `erişimi “${role}”`, seen.access.startsWith(role), seen.access);
    for (const [what, ok] of expect(seen)) check(login, what, ok, seen);
    await picture(b, page);
  } finally {
    b.close();
  }
}
