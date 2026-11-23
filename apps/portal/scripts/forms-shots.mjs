// The forms' acceptance in the cloud portal (docs/adr/9194): the page of the project the desktop acceptance made
// (apps/desktop/src/shell/cloud/live_forms.rs) as each of its three accounts reads it, each signed in through the
// sign-in page with the keyboard. Mehmet, its Veri girişi, reads team A's records with Durum and Bolge only and is told
// which fields his role does not read; Elif, its Görüntüleyici, and Ayşe, its owner, read every record and field, the
// note never written (NULL) apart from the empty one. What each page says is checked, then pictured. Run by
// apps/desktop/scripts/forms-roles-live.sh:
//
//   PORTAL_URL=http://127.0.0.1:5181/ PORTAL_DEMO_PASSWORD=… node scripts/forms-shots.mjs
//
// The project's ids come from .run/cloud-forms/project.json (FORMS_PROJECT); the pictures go to
// .run/gui-evidence/kullanim-9194/ (never in Git). Chrome: $CHROME_BIN, else the one in /Applications.
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { launch, sleep } from '../../web/scripts/e2e/cdp.mjs';

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

async function signIn(login) {
  const b = await launch(`${PORTAL}#/giris`, { ...SIZE, args: TR });
  await b.waitFor(`!!document.querySelector('input[name=login]')`, 20000);
  await b.eval(`document.querySelector('input[name=login]').focus()`);
  await b.type(login);
  await b.eval(`document.querySelector('input[name=password]').focus()`);
  await b.type(PASSWORD);
  await b.key('Enter');
  await b.waitFor(`!!document.querySelector('.shell .content > .page')`, 20000);
  return b;
}

/** The project's page, its records and its drawing loaded. */
async function projectPage(b) {
  await b.eval(`location.hash = ${JSON.stringify(`#/proje/${tenant}/${project}`)}; true`);
  await b.waitFor(`!!document.querySelector('.records-layer') && !document.querySelector('.preview-frame.is-loading')`, 30000);
  await sleep(400);
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

for (const [login, picture, role, expect] of [
  [
    'mehmet',
    '14-portal-veri-girisi',
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
    '15-portal-goruntuleyici',
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
    '16-portal-sahip',
    'Sahip',
    ({ records: r, drawing, svg }) => [
      ['dört kayıt', ['D-1', 'D-2', 'D-3', 'D-4'].every((d) => r.includes(d))],
      ['notlar ve hiç yazılmamış not (NULL)', r.includes(NOTE) && r.includes('NULL')],
      ['gizlenen alan yok', !r.includes('okumuyor')],
      ['çizim ve SVG düğmesi', drawing.includes('4 nesne çizildi') && svg],
    ],
  ],
]) {
  const b = await signIn(login);
  try {
    const page = await projectPage(b);
    check(login, `erişimi “${role}”`, page.access.startsWith(role), page.access);
    for (const [what, ok] of expect(page)) check(login, what, ok, page);
    const file = await b.shot(picture, null, OUT);
    console.log(`resim: ${file}`);
  } finally {
    b.close();
  }
}
