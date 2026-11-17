// Screenshots of the cloud portal (docs/adr/9000 §9), each person signed in through the sign-in page with the keyboard
// as they would: the platform administrator, a municipality's owner, its project manager and editor, an architect, a
// surveyor and a professional waiting for the check. Run after scripts/seed.mjs on a fresh database:
//
//   PORTAL_URL=http://127.0.0.1:5180/ PORTAL_ADMIN_PASSWORD=… PORTAL_DEMO_PASSWORD=… node scripts/shots.mjs
//
// The pictures go to .run/gui-evidence/portal/ (CLAUDE.md: GUI evidence, never in Git).
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { launch, sleep } from '../../web/scripts/e2e/cdp.mjs';

const PORTAL = process.env.PORTAL_URL ?? 'http://127.0.0.1:5180/';
const API = process.env.PORTAL_API ?? 'http://127.0.0.1:8797';
const PASSWORD = process.env.PORTAL_DEMO_PASSWORD;
const ADMIN_PASSWORD = process.env.PORTAL_ADMIN_PASSWORD;
const OUT = join(new URL('.', import.meta.url).pathname, '../../../.run/gui-evidence/portal');
mkdirSync(OUT, { recursive: true });
const only = process.argv.slice(2);
const shots = [];

// A Turkish browser: dates and numbers in the browser's own controls as a Turkish user sees them.
const TR = ['--lang=tr-TR', '--accept-lang=tr-TR,tr'];

async function signIn(login, password = PASSWORD, size = { width: 1560, height: 1000 }) {
  const b = await launch(`${PORTAL}#/giris`, { ...size, args: TR });
  await b.waitFor(`!!document.querySelector('input[name=login]')`, 20000);
  await b.eval(`document.querySelector('input[name=login]').focus()`);
  await b.type(login);
  await b.eval(`document.querySelector('input[name=password]').focus()`);
  await b.type(password);
  await b.key('Enter');
  await b.waitFor(`!!document.querySelector('.shell .content > .page')`, 20000);
  return b;
}

/** Opens `hash` and waits for its page (not the loading line, not the page before). */
async function open(b, hash) {
  // The same address again draws the page again (the app listens to hashchange only).
  await b.eval(
    `window.__before = document.querySelector('.content > *'); if (location.hash === ${JSON.stringify(hash)}) window.dispatchEvent(new HashChangeEvent('hashchange')); else location.hash = ${JSON.stringify(hash)}; true`,
  );
  await b.waitFor(`(() => { const el = document.querySelector('.content > *'); return el && el !== window.__before && !el.classList.contains('loading'); })()`, 20000);
  await sleep(250);
}

async function shot(b, name) {
  if (only.length && !only.some((o) => name.includes(o))) return;
  shots.push(await b.shot(name, null, OUT));
}

async function click(b, selector, text) {
  const found = await b.eval(`(() => { const el = [...document.querySelectorAll(${JSON.stringify(selector)})].find((e) => e.textContent.trim() === ${JSON.stringify(text)}); if (el) el.click(); return !!el; })()`);
  if (!found) throw new Error(`“${text}” bulunamadı (${selector})`);
  await sleep(350);
}

async function closeDialog(b) {
  await b.key('Escape');
  await sleep(150);
}

/** The demo's dossiers by title, as the API lists them to `login`. */
async function ids(login, password = PASSWORD) {
  const res = await fetch(`${API}/v1/auth/login`, { method: 'POST', headers: { 'x-kentos-client': 'web', 'content-type': 'application/json' }, body: JSON.stringify({ login, password }) });
  const cookie = res.headers.get('set-cookie').split(';')[0];
  const me = await (await fetch(`${API}/v1/portal/me`, { headers: { cookie } })).json();
  const tenant = me.organizations.find((o) => o.tenantSlug === 'ornek-belediyesi');
  const list = await (await fetch(`${API}/v1/tenants/${tenant.tenantId}/dossiers?limit=100`, { headers: { cookie } })).json();
  const byTitle = Object.fromEntries(list.items.map((d) => [d.title, d.id]));
  return { tenant: tenant.tenantId, byTitle };
}

const known = await ids('mehmet.kaya');
const A1 = known.byTitle['412 ada 7 parsel konut yapı ruhsatı'];
const A3 = known.byTitle['105 ada 2 parsel yapı kullanma izni'];
const T1 = known.byTitle['501 ada 1–4 parseller zemin aplikasyonu'];
const T2 = known.byTitle['Belediye hizmet binası cephe düzenlemesi'];
const ORG = '#/kurum/ornek-belediyesi';

// ── Signing in ───────────────────────────────────────────────────────
{
  const b = await launch(`${PORTAL}#/giris`, { width: 1560, height: 1000, args: TR });
  await b.waitFor(`!!document.querySelector('input[name=login]')`, 20000);
  await sleep(300);
  await shot(b, '01-giris');
  await b.eval(`location.hash = '#/kayit'`);
  await b.waitFor(`!!document.querySelector('select[name=profession]')`, 20000);
  await b.eval(`(() => { const s = document.querySelector('select[name=profession]'); s.value = 'surveyor'; s.dispatchEvent(new Event('change', { bubbles: true })); })()`);
  await sleep(200);
  await shot(b, '02-kayit');
  b.close();
}

// ── The platform administrator ───────────────────────────────────────
{
  const b = await signIn('sistem', ADMIN_PASSWORD);
  await open(b, '#/sistem');
  await shot(b, '10-sistem-genel-bakis');
  await open(b, '#/sistem/kurumlar');
  await shot(b, '11-sistem-kurumlar');
  await click(b, 'button', 'Yeni kurum');
  await b.eval(`(() => { const n = document.querySelector('.dialog input[name=name]'); n.value = 'Pınarlı Belediyesi'; n.dispatchEvent(new Event('input', { bubbles: true })); })()`);
  await sleep(150);
  await shot(b, '12-sistem-yeni-kurum');
  await closeDialog(b);
  await open(b, `#/sistem/kurumlar/${known.tenant}`);
  await shot(b, '13-sistem-kurum-ayrintisi');
  await open(b, '#/sistem/meslek-mensuplari');
  await shot(b, '14-sistem-meslek-mensuplari');
  await click(b, 'button', 'İncele');
  await shot(b, '15-sistem-kayit-dogrulama');
  await closeDialog(b);
  await open(b, '#/sistem/hesaplar');
  await shot(b, '16-sistem-hesaplar');
  await open(b, '#/sistem/kayit');
  await shot(b, '17-sistem-denetim-kaydi');
  await open(b, '#/sistem/ayarlar');
  await shot(b, '18-sistem-ayarlar');
  b.close();
}

// ── The municipality: its owner, its project manager, its editor ─────
{
  const b = await signIn('mehmet.kaya');
  await open(b, ORG);
  await shot(b, '20-belediye-genel-bakis');
  await open(b, `${ORG}/basvurular?liste=tumu`);
  await shot(b, '21-belediye-gelen-basvurular');
  await open(b, `${ORG}/aktarimlar?liste=tumu`);
  await shot(b, '22-belediye-proje-aktarimlari');
  await open(b, `${ORG}/dosya/${T1}`);
  await shot(b, '23-belediye-teslim-onay-bekliyor');
  await open(b, `${ORG}/aktarimlar/yeni`);
  await b.waitFor(`document.querySelectorAll('.pick').length > 0`, 10000);
  await shot(b, '24-belediye-yeni-aktarim');
  await open(b, `${ORG}/meslek-mensuplari`);
  await shot(b, '25-belediye-meslek-mensuplari');
  await open(b, `${ORG}/uyeler`);
  await shot(b, '26-belediye-uyeler');
  await open(b, `${ORG}/ayarlar`);
  await shot(b, '27-belediye-ayarlar');
  b.close();
}
{
  const b = await signIn('ali.celik', PASSWORD, { width: 1560, height: 1180 });
  await open(b, `${ORG}/dosya/${A1}`);
  await shot(b, '30-belediye-basvuru-paketler');
  await open(b, `${ORG}/dosya/${A1}?sekme=yazisma`);
  await shot(b, '31-belediye-basvuru-yazisma');
  await open(b, `${ORG}/dosya/${A1}?sekme=gecmis`);
  await shot(b, '32-belediye-basvuru-gecmis');
  await open(b, `${ORG}/dosya/${A1}`);
  await click(b, 'button', 'Düzeltme iste');
  await b.eval(`document.querySelector('.dialog textarea[name=note]').value = '1. Otopark hesabı imar durumundaki orana göre yeniden yapılmalı.\\n2. Çatı arası kullanım alanı kat alanına katılmalı.'`);
  await shot(b, '33-belediye-duzeltme-iste');
  await closeDialog(b);
  await click(b, 'button', 'Onayla');
  await shot(b, '34-belediye-onay-penceresi');
  await closeDialog(b);
  await b.eval(`document.documentElement.dataset.theme = 'dark'`);
  await sleep(200);
  await shot(b, '35-belediye-basvuru-koyu-tema');
  b.close();
}

// ── The architect ────────────────────────────────────────────────────
{
  const b = await signIn('ayse.yilmaz', PASSWORD, { width: 1560, height: 1080 });
  await open(b, '#/mesleki');
  await shot(b, '40-mimar-genel-bakis');
  await open(b, '#/mesleki/basvurular?liste=tumu');
  await shot(b, '41-mimar-basvurularim');
  await open(b, '#/mesleki/basvurular/yeni');
  await shot(b, '42-mimar-yeni-basvuru');
  await open(b, `#/mesleki/dosya/${A3}`);
  await shot(b, '43-mimar-duzeltme-istendi');
  await open(b, `#/mesleki/dosya/${A1}`);
  await shot(b, '44-mimar-basvuru-belediyede');
  await open(b, '#/mesleki/isler?liste=tumu');
  await shot(b, '45-mimar-aktarilan-isler');
  await open(b, `#/mesleki/dosya/${T2}`);
  await shot(b, '46-mimar-is-ustlenme');
  await open(b, '#/mesleki/profil');
  await shot(b, '47-mimar-profil');
  await open(b, '#/bildirimler');
  await shot(b, '48-mimar-bildirimler');
  await b.eval(`document.querySelector('.bell').click()`);
  await b.waitFor(`!!document.querySelector('.notif-menu .notif')`, 10000);
  await sleep(200);
  await shot(b, '49-mimar-bildirim-kutusu');
  b.close();
}

// ── The surveyor ─────────────────────────────────────────────────────
{
  const b = await signIn('can.ozturk', PASSWORD, { width: 1560, height: 1080 });
  await open(b, `#/mesleki/dosya/${T1}`);
  await shot(b, '50-haritaci-teslim-edilen-is');
  await open(b, '#/mesleki/projeler');
  await shot(b, '51-haritaci-projelerim');
  b.close();
}

// ── A professional waiting for the check ─────────────────────────────
{
  const b = await signIn('elif.sahin');
  await open(b, '#/mesleki');
  await shot(b, '60-muhendis-dogrulama-bekliyor');
  b.close();
}

console.log(shots.join('\n'));
