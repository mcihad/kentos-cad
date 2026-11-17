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
// SHOTS_BLOCK=harc,imza runs only those blocks (each block opens its own browser).
const blocks = process.env.SHOTS_BLOCK?.split(',').map((b) => b.trim()).filter(Boolean) ?? [];
const block = (name) => !blocks.length || blocks.includes(name);
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
if (block('giris')) {
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
if (block('sistem')) {
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
if (block('belediye')) {
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
if (block('belediye')) {
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
if (block('mimar')) {
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
if (block('haritaci')) {
  const b = await signIn('can.ozturk', PASSWORD, { width: 1560, height: 1080 });
  await open(b, `#/mesleki/dosya/${T1}`);
  await shot(b, '50-haritaci-teslim-edilen-is');
  await open(b, '#/mesleki/projeler');
  await shot(b, '51-haritaci-projelerim');
  b.close();
}

// ── A professional waiting for the check ─────────────────────────────
if (block('bekleyen')) {
  const b = await signIn('elif.sahin');
  await open(b, '#/mesleki');
  await shot(b, '60-muhendis-dogrulama-bekliyor');
  b.close();
}

// ── Files in the cloud (docs/adr/9000 §12-§15) ───────────────────────
/** A person's cloud projects by name, as the API lists them. */
async function projectsOf(login, view, tenant) {
  const res = await fetch(`${API}/v1/auth/login`, { method: 'POST', headers: { 'x-kentos-client': 'web', 'content-type': 'application/json' }, body: JSON.stringify({ login, password: PASSWORD }) });
  const cookie = res.headers.get('set-cookie').split(';')[0];
  const q = new URLSearchParams({ view, ...(tenant ? { tenant } : {}) });
  const page = await (await fetch(`${API}/v1/me/catalog?${q}`, { headers: { cookie } })).json();
  return page.projects;
}

/** Drops files made in the page onto the drop zone, as a person dragging them from their folder. */
async function drop(b, files) {
  await b.eval(`(() => {
    const dt = new DataTransfer();
    for (const [name, text, type] of ${JSON.stringify(files)}) dt.items.add(new File([Uint8Array.from(atob(text), (c) => c.charCodeAt(0))], name, { type }));
    const zone = document.querySelector('.drop');
    zone.scrollIntoView({ block: 'center' });
    zone.dispatchEvent(new DragEvent('drop', { dataTransfer: dt, bubbles: true, cancelable: true }));
    return true;
  })()`);
}
const b64 = (bytes) => Buffer.from(bytes).toString('base64');

if (block('bulut')) {
  const b = await signIn('ayse.yilmaz', PASSWORD, { width: 1560, height: 1180 });
  await open(b, '#/hesabim');
  await shot(b, '70-hesabim-oturumlar-ve-anahtarlar');
  await b.eval(`document.querySelector('input[name=name]').focus()`);
  await b.type('Belge özetleyici ajan');
  await click(b, 'button', 'Anahtar oluştur');
  await b.waitFor(`!!document.querySelector('.token-box')`, 10000);
  await sleep(300);
  await shot(b, '71-anahtar-bir-kez-gosterilir');
  await closeDialog(b);
  await open(b, `#/mesleki/dosya/${A3}`);
  await shot(b, '72-mimar-belgeler-tarandi');
  b.close();
}
if (block('bulut')) {
  const orgProjects = await projectsOf('mehmet.kaya', 'organization', known.tenant);
  const project = orgProjects.find((p) => p.name.startsWith('Konut')) ?? orgProjects[0];
  const b = await signIn('mehmet.kaya', PASSWORD, { width: 1560, height: 1240 });
  await open(b, `${ORG}/projeler`);
  await shot(b, '73-belediye-projeler');
  await open(b, `#/proje/${project.tenantId}/${project.id}`);
  await b.waitFor(`!!document.querySelector('.preview-image')`, 20000);
  await sleep(500);
  await shot(b, '74-proje-cizim-ve-ekler');
  // A program passed off as a PDF: refused with the server's reason, nothing kept.
  await drop(b, [['ruhsat-eki.pdf', b64([0x4d, 0x5a, 0x90, 0, 3, 0, 0, 0]), 'application/pdf']]);
  await b.waitFor(`!!document.querySelector('.upload.is-failed')`, 20000);
  await sleep(200);
  await shot(b, '75-proje-reddedilen-dosya');
  // A report goes up in parts and joins the list.
  const before = await b.eval(`document.querySelectorAll('.panel .table tbody tr').length`);
  await drop(b, [['kontrol-notlari.txt', b64(Buffer.from('412 ada 7 parsel: temel betonu öncesi kontrol notları.\n', 'utf8')), 'text/plain']]);
  await b.waitFor(`document.querySelectorAll('.panel .table tbody tr').length > ${before}`, 20000);
  await sleep(500);
  await shot(b, '76-proje-ek-eklendi');
  await b.eval(`document.documentElement.dataset.theme = 'dark'; window.scrollTo(0, 0); true`);
  await sleep(300);
  await shot(b, '77-proje-koyu-tema');
  b.close();
}
if (block('bulut')) {
  const mine = await projectsOf('can.ozturk', 'mine');
  const project = mine.find((p) => p.storage === 'file') ?? mine[0];
  const b = await signIn('can.ozturk', PASSWORD, { width: 1560, height: 1080 });
  await open(b, '#/mesleki/projeler');
  await shot(b, '78-haritaci-projelerim');
  await open(b, `#/proje/${project.tenantId}/${project.id}`);
  await b.waitFor(`!!document.querySelector('.preview-image')`, 20000);
  await sleep(500);
  await shot(b, '79-haritaci-dosya-projesi-revizyonlar');
  b.close();
}
if (block('bulut')) {
  const b = await signIn('sistem', ADMIN_PASSWORD);
  await open(b, '#/sistem/kayit');
  await shot(b, '80-sistem-reddedilen-dosyalar');
  await open(b, '#/sistem/ayarlar');
  await shot(b, '81-sistem-bildirim-kanallari');
  b.close();
}
if (block('bulut')) {
  const b = await signIn('ayse.yilmaz', PASSWORD, { width: 1560, height: 1400 });
  await open(b, '#/hesabim');
  await b.eval(`[...document.querySelectorAll('.panel')].find((p) => p.textContent.includes('Bildirimler'))?.scrollIntoView({ block: 'start' }); true`);
  await sleep(300);
  await shot(b, '82-hesabim-bildirimler');
  b.close();
}

// ── An office's shared dossiers (docs/adr/9000 §18) ──────────────────
if (block('buro')) {
  const titled = (list, title) => list.items.find((d) => d.title === title).id;
  const login = async (who) => {
    const res = await fetch(`${API}/v1/auth/login`, { method: 'POST', headers: { 'x-kentos-client': 'web', 'content-type': 'application/json' }, body: JSON.stringify({ login: who, password: PASSWORD }) });
    return res.headers.get('set-cookie').split(';')[0];
  };
  const cookie = await login('emre.tas');
  const me = await (await fetch(`${API}/v1/portal/me`, { headers: { cookie } })).json();
  const office = me.organizations.find((o) => o.tenantSlug === 'yilmaz-mimarlik');
  const list = await (await fetch(`${API}/v1/tenants/${office.tenantId}/dossiers?limit=50`, { headers: { cookie } })).json();
  const A6 = titled(list, '233 ada 4 parsel ticaret binası mimari projesi');
  const OFFICE = '#/kurum/yilmaz-mimarlik';
  let b = await signIn('emre.tas');
  await open(b, OFFICE);
  await shot(b, '83-buro-genel-bakis');
  await open(b, `${OFFICE}/basvurular?liste=tumu`);
  await shot(b, '84-buro-basvurulari');
  b.close();
  b = await signIn('selin.kara', PASSWORD, { width: 1560, height: 1080 });
  await open(b, `${OFFICE}/dosya/${A6}`);
  await shot(b, '85-buro-duzenleyici-dosyada');
  b.close();
  b = await signIn('ayse.yilmaz', PASSWORD, { width: 1560, height: 1080 });
  await open(b, `#/mesleki/dosya/${A3}`);
  await shot(b, '86-mimar-buroyla-paylas');
  b.close();
}

// ── Fees and charges (docs/adr/9003) ─────────────────────────────────
if (block('harc')) {
  const res = await fetch(`${API}/v1/auth/login`, { method: 'POST', headers: { 'x-kentos-client': 'web', 'content-type': 'application/json' }, body: JSON.stringify({ login: 'mehmet.kaya', password: PASSWORD }) });
  const cookie = res.headers.get('set-cookie').split(';')[0];
  const tariffs = await (await fetch(`${API}/v1/tenants/${known.tenant}/fees/schedules`, { headers: { cookie } })).json();
  const draft = tariffs.find((t) => t.state === 'draft').id;
  const A2 = known.byTitle['398 ada 12 parsel mimari proje tadilatı'];
  const panelTo = (title) => `[...document.querySelectorAll('.panel')].find((p) => p.querySelector('h2')?.textContent === ${JSON.stringify(title)})?.scrollIntoView({ block: 'start' }); true`;
  let b = await signIn('mehmet.kaya', PASSWORD, { width: 1560, height: 1100 });
  await open(b, `${ORG}/harclar`);
  await shot(b, '87-belediye-harc-tarifeleri');
  await open(b, `${ORG}/harclar/${draft}`);
  await shot(b, '88-belediye-tarife-taslagi');
  await b.eval(panelTo('Birim tutar tabloları'));
  await sleep(250);
  await shot(b, '89-belediye-tarife-tablolari');
  await b.eval(panelTo('Kalemler'));
  await sleep(600);
  await shot(b, '90-belediye-tarife-kalemleri-ve-deneme');
  await b.eval(`(() => { const p = [...document.querySelectorAll('.panel')].find((p) => p.querySelector('h2')?.textContent === 'Kalemler'); [...p.querySelectorAll('button')].find((x) => x.textContent.trim() === 'Düzenle').click(); return true; })()`);
  await sleep(400);
  await shot(b, '91-belediye-kalem-duzenleme');
  await closeDialog(b);
  b.close();
  b = await signIn('zeynep.demir', PASSWORD, { width: 1560, height: 1100 });
  await open(b, `${ORG}/dosya/${A1}?sekme=harc`);
  await b.waitFor(`!!document.querySelector('.fee-lines')`, 10000);
  await shot(b, '92-belediye-dosyada-harc');
  const pick = (name, value) => `(() => { const el = document.querySelector('[name=${name}]'); el.value = ${JSON.stringify(value)}; el.dispatchEvent(new Event(el.tagName === 'SELECT' ? 'change' : 'input', { bubbles: true })); return true; })()`;
  await click(b, 'button', 'Ek tahakkuk');
  await b.eval(pick('yapi_turu', 'isyeri'));
  await b.eval(pick('insaat_alani', '86,4'));
  await b.eval(pick('bolge', 'ticaret'));
  await sleep(700);
  await b.eval(`[...document.querySelectorAll('.panel')].find((p) => p.querySelector('h2')?.textContent === 'Yeni tahakkuk')?.scrollIntoView({ block: 'start' }); true`);
  await sleep(250);
  await shot(b, '93-belediye-tahakkuk-onizleme');
  b.close();
  b = await signIn('ayse.yilmaz', PASSWORD, { width: 1560, height: 1100 });
  await open(b, `#/mesleki/dosya/${A1}?sekme=harc`);
  await shot(b, '94-mimar-odenecek-harc');
  await open(b, `#/mesleki/dosya/${A2}?sekme=harc`);
  await shot(b, '95-mimar-odenmis-harc');
  b.close();
}

// ── E-signing (docs/adr/9004): the test card; KentOS İmza and its PIN window on a virtual display ──
if (block('imza')) {
  const A1 = known.byTitle['412 ada 7 parsel konut yapı ruhsatı'];
  const b = await signIn('ayse.yilmaz', PASSWORD, { width: 1560, height: 1100 });
  await open(b, `#/mesleki/dosya/${A1}`);
  await b.waitFor(`!!document.querySelector('.sign-mark')`, 10000);
  await shot(b, '96-mimar-e-imzali-belgeler');
  await b.eval(`document.querySelector('.sign-mark').click(); true`);
  await sleep(400);
  await shot(b, '97-mimar-imza-ayrintisi');
  await closeDialog(b);
  await click(b, 'button', 'E-imzala');
  await shot(b, '98-mimar-e-imzala-penceresi');
  await click(b, '.dialog button', 'İmzala');
  await b.waitFor(`document.querySelector('.dialog')?.textContent.includes('PIN’inizi KentOS İmza penceresine girin')`, 30000);
  await shot(b, '99-mimar-pin-bekleniyor');
  await click(b, '.dialog button', 'Vazgeç');
  await open(b, '#/hesabim');
  await b.eval(`[...document.querySelectorAll('.panel')].find((p) => p.querySelector('h2')?.textContent === 'E-imza')?.scrollIntoView({ block: 'start' }); true`);
  await sleep(800);
  await shot(b, '100-hesabim-e-imza');
  b.close();
}

// ── Figures, the calendar, the retention and the archive (docs/adr/9000 §19, §21, §22) ──
if (block('istatistik') || block('grafik')) {
  let b = await signIn('mehmet.kaya', PASSWORD, { width: 1560, height: 1150 });
  await open(b, `${ORG}/istatistik?donem=12ay`);
  await b.waitFor(`!!document.querySelector('.viz-svg')`, 10000);
  await shot(b, '101-belediye-istatistikler');
  await b.eval(`(() => { const hits = [...document.querySelectorAll('.viz-hit')]; hits[hits.length - 1].dispatchEvent(new PointerEvent('pointerenter')); return true; })()`);
  await sleep(250);
  await b.eval(`document.querySelector('.viz').scrollIntoView({ block: 'center' }); true`);
  await sleep(250);
  await shot(b, '102-istatistik-ipucu');
  b.close();
}
if (block('istatistik')) {
  let b = await signIn('sistem', ADMIN_PASSWORD, { width: 1560, height: 1150 });
  await open(b, '#/sistem/ayarlar');
  await b.eval(`[...document.querySelectorAll('.panel')].find((p) => p.querySelector('h2')?.textContent === 'Tatil takvimi')?.scrollIntoView({ block: 'start' }); true`);
  await sleep(600);
  await shot(b, '103-sistem-tatil-takvimi-ve-saklama');
  await click(b, 'button', 'Şimdi çalıştır');
  await sleep(1200);
  await b.eval(`[...document.querySelectorAll('.panel')].find((p) => p.querySelector('h2')?.textContent === 'Kişisel verilerin saklanması')?.scrollIntoView({ block: 'start' }); true`);
  await sleep(300);
  await shot(b, '104-sistem-periyodik-imha');
  b.close();
  b = await signIn('mehmet.kaya', PASSWORD, { width: 1560, height: 900 });
  await open(b, `${ORG}/basvurular?liste=arsiv`);
  await shot(b, '105-belediye-arsiv');
  b.close();
}

console.log(shots.join('\n'));
