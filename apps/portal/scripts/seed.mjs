// A synthetic demo of the cloud portal (docs/adr/9000 §9), made through the API as its people would make it: the
// platform administrator opens municipalities and sign-ups, professionals sign up and are verified, applications and
// transfers go back and forth with their files and messages. Every name, number and document is made up.
//
//   PORTAL_API=http://127.0.0.1:8797 PORTAL_ADMIN_PASSWORD=… PORTAL_DEMO_PASSWORD=… node scripts/seed.mjs
//
// It expects a fresh database with the platform administrator `sistem` (kentosd platform-admin add). Every demo
// account gets PORTAL_DEMO_PASSWORD.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { deflateSync } from 'node:zlib';

const API = process.env.PORTAL_API ?? 'http://127.0.0.1:8797';
const ADMIN_PASSWORD = process.env.PORTAL_ADMIN_PASSWORD;
const PASSWORD = process.env.PORTAL_DEMO_PASSWORD;
if (!ADMIN_PASSWORD || !PASSWORD) throw new Error('PORTAL_ADMIN_PASSWORD ve PORTAL_DEMO_PASSWORD gerekli.');
const REPO = join(new URL('.', import.meta.url).pathname, '../../..');

/** A signed-in person: their session cookie on every request. */
class Person {
  constructor(name) {
    this.name = name;
    this.cookie = '';
  }

  async req(method, path, body, raw) {
    const headers = { 'x-kentos-client': 'web' };
    if (this.cookie) headers.cookie = this.cookie;
    let payload;
    if (raw) {
      headers['content-type'] = raw.type;
      payload = raw.bytes;
    } else if (body !== undefined) {
      headers['content-type'] = 'application/json';
      payload = JSON.stringify(body);
    }
    const res = await fetch(`${API}${path}`, { method, headers, body: payload });
    const set = res.headers.get('set-cookie');
    if (set) this.cookie = set.split(';')[0];
    const text = await res.text();
    if (!res.ok) throw new Error(`${this.name}: ${method} ${path} → ${res.status} ${text}`);
    return text ? JSON.parse(text) : undefined;
  }

  login(login, password = PASSWORD) {
    return this.req('POST', '/v1/auth/login', { login, password });
  }

  upload(dossier, name, bytes, type, docType, purpose = 'package') {
    const q = new URLSearchParams({ name, purpose, ...(docType ? { docType } : {}) });
    return this.req('POST', `/v1/dossiers/${dossier}/files?${q}`, undefined, { bytes, type });
  }

  async act(dossier, action, extra = {}) {
    const d = await this.req('GET', `/v1/dossiers/${dossier}`);
    return this.req('POST', `/v1/dossiers/${dossier}/actions`, { action, expectedStatus: d.summary.status, note: null, assigneeId: null, attachments: [], ...extra });
  }

  say(dossier, body) {
    return this.req('POST', `/v1/dossiers/${dossier}/messages`, { body, attachments: [] });
  }
}

/** A small one-page PDF that says what the document is (ASCII only: the demo's documents are placeholders). */
function pdf(title, lines = []) {
  const esc = (s) => s.replace(/[\\()]/g, (c) => `\\${c}`);
  const text = [`BT /F1 20 Tf 72 760 Td (${esc(title)}) Tj ET`, ...lines.map((l, i) => `BT /F1 11 Tf 72 ${725 - i * 18} Td (${esc(l)}) Tj ET`), 'BT /F1 9 Tf 72 60 Td (KentOS Bulut - ornek belge, gercek degildir) Tj ET'].join('\n');
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>',
    `<< /Length ${Buffer.byteLength(text)} >>\nstream\n${text}\nendstream`,
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
  ];
  let out = '%PDF-1.4\n';
  const offsets = [];
  objects.forEach((o, i) => {
    offsets.push(Buffer.byteLength(out));
    out += `${i + 1} 0 obj\n${o}\nendobj\n`;
  });
  const xref = Buffer.byteLength(out);
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n${offsets.map((o) => `${String(o).padStart(10, '0')} 00000 n \n`).join('')}`;
  out += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(out, 'latin1');
}

const PDF = 'application/pdf';
const KCAD = 'application/vnd.kentos.kcad';
const kcad = (name) => readFileSync(join(REPO, 'samples', name));
const daysFromNow = (n) => new Date(Date.now() + n * 86_400_000).toISOString().slice(0, 10);

const admin = new Person('sistem');
await admin.login('sistem', ADMIN_PASSWORD);
// Notifications also go out by e-mail and SMS (docs/adr/9000 §17): what the server does with them is its own setting.
await admin.req('PUT', '/v1/platform/settings', { emailNotifications: true, smsNotifications: true });
await admin.req('PUT', '/v1/platform/settings', { signupOpen: true });

// ── Organisations ────────────────────────────────────────────────────
const owner = (login, displayName, email) => ({ userId: null, login, password: PASSWORD, displayName, email });
const ornek = await admin.req('POST', '/v1/platform/tenants', {
  slug: 'ornek-belediyesi', name: 'Örnek Belediyesi', category: 'municipality', province: 'Örnek', district: 'Merkez', seats: 25,
  owner: owner('mehmet.kaya', 'Mehmet Kaya', 'mehmet.kaya@ornek-belediyesi.test'),
});
const yesilvadi = await admin.req('POST', '/v1/platform/tenants', {
  slug: 'yesilvadi-belediyesi', name: 'Yeşilvadi Belediyesi', category: 'municipality', province: 'Örnek', district: 'Yeşilvadi', seats: 10,
  owner: owner('hakan.yildiz', 'Hakan Yıldız', 'hakan.yildiz@yesilvadi.test'),
});
const kuzeytepe = await admin.req('POST', '/v1/platform/tenants', {
  slug: 'kuzeytepe-belediyesi', name: 'Kuzeytepe Belediyesi', category: 'municipality', province: 'Örnek', district: 'Kuzeytepe', seats: 8,
  owner: owner('sevgi.ates', 'Sevgi Ateş', 'sevgi.ates@kuzeytepe.test'),
});
await admin.req('POST', '/v1/platform/tenants', {
  slug: 'ornek-mimarlik', name: 'Örnek Mimarlık Bürosu', category: 'office', province: 'Örnek', district: 'Merkez', seats: 5,
  owner: owner('deniz.koc', 'Deniz Koç', 'deniz.koc@ornek-mimarlik.test'),
});
await admin.req('PATCH', `/v1/platform/tenants/${kuzeytepe.id}`, { name: 'Kuzeytepe Belediyesi', category: 'municipality', province: 'Örnek', district: 'Kuzeytepe', seats: 8, status: 'suspended' });

const mehmet = new Person('mehmet');
await mehmet.login('mehmet.kaya');
for (const [login, displayName, role] of [
  ['zeynep.demir', 'Zeynep Demir', 'editor'],
  ['ali.celik', 'Ali Çelik', 'project_manager'],
  ['selin.arslan', 'Selin Arslan', 'viewer'],
  ['murat.kilic', 'Murat Kılıç', 'editor'],
]) {
  await mehmet.req('POST', `/v1/tenants/${ornek.id}/members`, { login, password: PASSWORD, displayName, email: `${login}@ornek-belediyesi.test`, role });
}
await mehmet.req('PUT', `/v1/tenants/${ornek.id}/portal`, {
  acceptsApplications: true, applicationKinds: [], contactEmail: 'imar@ornek-belediyesi.test', contactPhone: '0 (300) 555 01 00',
  address: 'Cumhuriyet Mahallesi, Belediye Caddesi No: 1, Merkez',
});
const hakan = new Person('hakan');
await hakan.login('hakan.yildiz');
await hakan.req('PUT', `/v1/tenants/${yesilvadi.id}/portal`, {
  acceptsApplications: true, applicationKinds: ['building_permit', 'occupancy_permit', 'application_sketch', 'plan_amendment'],
  contactEmail: 'imar@yesilvadi.test', contactPhone: '0 (300) 555 02 00', address: 'Yeşilvadi Meydanı No: 3',
});

// ── Professionals sign up; the administrator checks them ─────────────
const signup = async (login, displayName, profession, chamberNo, officeName) => {
  const p = new Person(login);
  await p.req('POST', '/v1/portal/signup', {
    login, password: PASSWORD, displayName, email: `${login}@ornek-posta.test`, profession, chamberNo, officeName, phone: null, province: 'Örnek',
  });
  return p;
};
const ayse = await signup('ayse.yilmaz', 'Ayşe Yılmaz', 'architect', '41234', 'Yılmaz Mimarlık');
const can = await signup('can.ozturk', 'Can Öztürk', 'surveyor', '7781', 'Öztürk Harita Mühendislik');
await signup('elif.sahin', 'Elif Şahin', 'civil_engineer', '52310', 'Şahin Proje');
await signup('burak.aydin', 'Burak Aydın', 'electrical_engineer', '30877', null);
await signup('deniz.aksoy', 'Deniz Aksoy', 'urban_planner', '1290', null);
const pros = await admin.req('GET', '/v1/platform/professionals');
const idOf = (name) => pros.find((p) => p.displayName === name).userId;
await admin.req('POST', `/v1/platform/professionals/${idOf('Ayşe Yılmaz')}/review`, { status: 'verified', note: 'Mimarlar Odası sicil kaydı denetlendi.' });
await admin.req('POST', `/v1/platform/professionals/${idOf('Can Öztürk')}/review`, { status: 'verified', note: 'HKMO sicil kaydı denetlendi.' });
await admin.req('POST', `/v1/platform/professionals/${idOf('Deniz Aksoy')}/review`, {
  status: 'rejected', note: 'Oda sicil numarası Şehir Plancıları Odası kayıtlarıyla eşleşmedi; doğru numarayla yeniden başvurun.',
});
await ayse.login('ayse.yilmaz');
await can.login('can.ozturk');
// The architect wants SMS too (a made-up number of no operator).
await ayse.req('PUT', '/v1/me/notification-settings', { email: true, sms: true, phone: '0500 000 00 01' });
// Her own office (docs/adr/9000 §18): an editor who prepares the packages, a project manager who sends them.
const yilmaz = await admin.req('POST', '/v1/platform/tenants', {
  slug: 'yilmaz-mimarlik', name: 'Yılmaz Mimarlık', category: 'office', province: 'Örnek', district: 'Merkez', seats: 4,
  owner: { userId: idOf('Ayşe Yılmaz'), login: null, password: null, displayName: null, email: null },
});
await ayse.req('POST', `/v1/tenants/${yilmaz.id}/members`, { login: 'selin.kara', password: PASSWORD, displayName: 'Selin Kara', email: 'selin.kara@yilmaz-mimarlik.test', role: 'editor' });
await ayse.req('POST', `/v1/tenants/${yilmaz.id}/members`, { login: 'emre.tas', password: PASSWORD, displayName: 'Emre Taş', email: 'emre.tas@yilmaz-mimarlik.test', role: 'project_manager' });
const selin = new Person('selin');
await selin.login('selin.kara');
const emre = new Person('emre');
await emre.login('emre.tas');
const zeynep = new Person('zeynep');
await zeynep.login('zeynep.demir');
const ali = new Person('ali');
await ali.login('ali.celik');

// ── Applications ─────────────────────────────────────────────────────
const site = (neighbourhood, block, parcel, address = null) => ({ province: 'Örnek', district: 'Merkez', neighbourhood, block, parcel, address });

// A building permit: deficiencies said at once, corrected, sent again; waits for the decision.
const a1 = (await ayse.req('POST', '/v1/me/dossiers', {
  tenantId: ornek.id, kind: 'building_permit', title: '412 ada 7 parsel konut yapı ruhsatı',
  description: 'Bodrum, zemin ve 3 normal kattan oluşan 8 bağımsız bölümlü konut. Proje imar durum belgesine göre hazırlanmıştır.',
  site: site('Cumhuriyet Mahallesi', '412', '7', 'Lale Sokak No: 12'), ownerName: 'Hasan Er',
})).id;
await ayse.upload(a1, 'Konut-412-7.kcad', kcad('Konut100m2.kcad'), KCAD, 'kentos_project');
await ayse.upload(a1, 'mimari-proje.pdf', pdf('Mimari proje', ['412 ada 7 parsel', 'Vaziyet plani, kat planlari, kesit ve gorunusler']), PDF, 'architectural');
await ayse.upload(a1, 'statik-proje.pdf', pdf('Statik proje', ['Betonarme tasiyici sistem']), PDF, 'structural');
await ayse.upload(a1, 'tapu.pdf', pdf('Tapu kaydi', ['412 ada 7 parsel, Cumhuriyet Mahallesi']), PDF, 'title_deed');
await ayse.upload(a1, 'roperli-kroki.pdf', pdf('Roperli kroki'), PDF, 'site_sketch');
await ayse.act(a1, 'send', { note: 'Yapı ruhsatı başvurumuzu projeleri ve belgeleriyle sunarız.' });
await zeynep.act(a1, 'start_review');
await zeynep.say(a1, 'Merhaba, başvurunuzu inceliyoruz. Tesisat projelerini pakette göremedik.');
await ayse.say(a1, 'Tesisat projelerini makine ve elektrik mühendislerimizden aldık, bugün ekliyoruz.');
await zeynep.act(a1, 'request_changes', {
  note: '1. Elektrik ve mekanik tesisat projeleri eksik.\n2. Statik hesap raporu pakette yok.\n3. Röperli krokide komşu parsellere olan mesafeler okunmuyor; ölçüleri yazın.',
});
await ayse.upload(a1, 'elektrik-projesi.pdf', pdf('Elektrik projesi'), PDF, 'electrical');
await ayse.upload(a1, 'mekanik-tesisat-projesi.pdf', pdf('Mekanik tesisat projesi'), PDF, 'mechanical');
await ayse.upload(a1, 'statik-hesap-raporu.pdf', pdf('Statik hesap raporu'), PDF, 'calculations');
await ayse.upload(a1, 'roperli-kroki-duzeltilmis.pdf', pdf('Roperli kroki (duzeltilmis)', ['Komsu parsel mesafeleri eklendi']), PDF, 'site_sketch');
await ayse.act(a1, 'send', { note: 'Eksikleri tamamladık; röperli krokide komşu parsel mesafelerini ekledik.' });
// The municipality opens the submitted project in its own workspace.
const a1d = await ali.req('GET', `/v1/dossiers/${a1}`);
const a1kcad = a1d.rounds[0].files.find((f) => f.isProject);
await ali.req('POST', `/v1/dossiers/${a1}/files/${a1kcad.id}/import`, { tenantId: ornek.id });

// An architectural project, approved.
const a2 = (await ayse.req('POST', '/v1/me/dossiers', {
  tenantId: ornek.id, kind: 'architectural_project', title: '398 ada 12 parsel mimari proje tadilatı',
  description: 'Zemin katta dükkânın büroya dönüştürülmesi; taşıyıcı sisteme dokunulmuyor.', site: site('Yenişehir Mahallesi', '398', '12'), ownerName: 'Fatma Kara',
})).id;
await ayse.upload(a2, 'Tadilat-398-12.kcad', kcad('konut-3-1.kcad'), KCAD, 'kentos_project');
await ayse.upload(a2, 'tadilat-mimari-proje.pdf', pdf('Tadilat mimari projesi'), PDF, 'architectural');
await ayse.act(a2, 'send');
await ali.act(a2, 'approve', { note: 'Proje imar durumuna ve yönetmeliğe uygun bulundu.' });

// An occupancy permit: a document asked for.
const a3 = (await ayse.req('POST', '/v1/me/dossiers', {
  tenantId: ornek.id, kind: 'occupancy_permit', title: '105 ada 2 parsel yapı kullanma izni', description: 'Yapı tamamlandı; iskân başvurusu.',
  site: site('Bahçelievler Mahallesi', '105', '2'), ownerName: 'Kemal Uçar',
})).id;
await ayse.upload(a3, 'yapi-kullanma-basvurusu.pdf', pdf('Yapi kullanma izni basvurusu'), PDF, 'other');
await ayse.act(a3, 'send');
await zeynep.act(a3, 'request_changes', { note: 'Yapı denetim kuruluşunun iş bitirme tutanağı eklenmeli.' });

// A draft to another municipality.
await ayse.req('POST', '/v1/me/dossiers', {
  tenantId: yesilvadi.id, kind: 'plan_amendment', title: '220 ada plan değişikliği teklifi', description: 'Ticaret alanının konut + ticaret alanına dönüştürülmesi teklifi.',
  site: { province: 'Örnek', district: 'Yeşilvadi', neighbourhood: 'Pınarbaşı Mahallesi', block: '220', parcel: null, address: null }, ownerName: null,
});

// A surveyor's application sketch, just arrived; and a site sketch, rejected.
const a4 = (await can.req('POST', '/v1/me/dossiers', {
  tenantId: ornek.id, kind: 'application_sketch', title: '415 ada 3 parsel aplikasyon krokisi', description: 'Parsel köşe noktalarının aplikasyonu yapıldı.',
  site: site('Cumhuriyet Mahallesi', '415', '3'), ownerName: 'Selim Doğan',
})).id;
await can.upload(a4, 'aplikasyon-krokisi.pdf', pdf('Aplikasyon krokisi', ['415 ada 3 parsel']), PDF, 'application_sketch');
await can.act(a4, 'send');
const a5 = (await can.req('POST', '/v1/me/dossiers', {
  tenantId: ornek.id, kind: 'site_sketch', title: '322 ada 5 parsel röperli kroki', description: '', site: site('Yenişehir Mahallesi', '322', '5'), ownerName: 'Naci Ekinci',
})).id;
await can.upload(a5, 'roperli-kroki-322-5.pdf', pdf('Roperli kroki 322/5'), PDF, 'site_sketch');
await can.act(a5, 'send');
await ali.act(a5, 'reject', { note: 'Kroki parsel köşe koordinatlarını göstermiyor; yeniden hazırlayıp yeni başvuru yapın.' });

// An application of Ayşe's office: Selin prepares the package, Emre (the office's project manager) sends it.
const a6 = (await ayse.req('POST', '/v1/me/dossiers', {
  tenantId: ornek.id, kind: 'architectural_project', title: '233 ada 4 parsel ticaret binası mimari projesi',
  description: 'Zemin ve iki normal katlı ticaret binası; otopark bodrum katta.', site: site('Yeni Mahalle', '233', '4', 'Çarşı Caddesi No: 18'),
  ownerName: 'Kaan Yıldız', officeId: yilmaz.id,
})).id;
await selin.upload(a6, 'mimari-proje.pdf', pdf('Mimari proje', ['233 ada 4 parsel', 'Ticaret binasi']), PDF, 'architectural');
await selin.upload(a6, 'tapu.pdf', pdf('Tapu'), PDF, 'title_deed');
await emre.act(a6, 'send', { note: 'Büromuz adına gönderilmiştir; sorumlu mimar Ayşe Yılmaz.' });
await zeynep.act(a6, 'start_review');

// ── Transfers ────────────────────────────────────────────────────────
// A survey: the municipality's project and documents sent, taken on, opened as a project, delivered; waits for approval.
const t1 = (await mehmet.req('POST', `/v1/tenants/${ornek.id}/dossiers`, {
  professionalId: idOf('Can Öztürk'), kind: 'survey', title: '501 ada 1–4 parseller zemin aplikasyonu',
  description: 'İmar uygulaması sonrası 501 adadaki dört parselin köşe noktalarının zemine aplikasyonu ve tutanağı.',
  site: site('Sanayi Mahallesi', '501', '1–4'), dueOn: daysFromNow(14),
})).id;
const projects = await mehmet.req('GET', `/v1/me/catalog?view=organization&tenant=${ornek.id}`);
await mehmet.req('POST', `/v1/dossiers/${t1}/files/from-project`, { tenantId: ornek.id, projectId: projects.projects[0].id, docType: 'kentos_project' });
await mehmet.upload(t1, 'parselasyon-plani.pdf', pdf('Parselasyon plani', ['501 ada']), PDF, 'other');
await mehmet.act(t1, 'send', { note: 'Aplikasyonu iki hafta içinde tamamlamanızı rica ederiz.' });
await can.act(t1, 'accept', { note: 'Önümüzdeki hafta sahaya çıkıyoruz.' });
const t1d = await can.req('GET', `/v1/dossiers/${t1}`);
const t1kcad = t1d.rounds[0].files.find((f) => f.isProject);
const canProject = await can.req('POST', `/v1/dossiers/${t1}/files/${t1kcad.id}/import`, { tenantId: null });
await can.say(t1, 'Sınır taşlarının ikisi yerinde değildi; yeniden çaktık, tutanağa işledik.');
await can.upload(t1, 'aplikasyon-tutanagi.pdf', pdf('Aplikasyon tutanagi', ['501 ada 1-4 parseller']), PDF, 'application_sketch');
await can.req('POST', `/v1/dossiers/${t1}/files/from-project`, { tenantId: canProject.tenantId, projectId: canProject.projectId, docType: 'kentos_project' });
await can.act(t1, 'deliver', { note: 'Aplikasyon yapıldı; tutanak ve güncel proje ektedir.' });

// A revision sent to the architect, waiting for her.
const t2 = (await ali.req('POST', `/v1/tenants/${ornek.id}/dossiers`, {
  professionalId: idOf('Ayşe Yılmaz'), kind: 'project_revision', title: 'Belediye hizmet binası cephe düzenlemesi',
  description: 'Ana cephede pencere oranlarının ve giriş saçağının düzeltilmesi.', site: site('Cumhuriyet Mahallesi', '1', '1', 'Belediye Caddesi No: 1'), dueOn: daysFromNow(10),
})).id;
await ali.upload(t2, 'cephe-notlari.pdf', pdf('Cephe duzeltme notlari'), PDF, 'other');
await ali.act(t2, 'send');

// A technical review, completed.
const t3 = (await ali.req('POST', `/v1/tenants/${ornek.id}/dossiers`, {
  professionalId: idOf('Can Öztürk'), kind: 'technical_review', title: 'Kuzey yolu genişletme güzergâhı görüşü',
  description: 'Önerilen güzergâhın mülkiyet sınırlarına etkisi.', site: site('Kuzey Mahallesi', '610', null), dueOn: daysFromNow(5),
})).id;
await ali.upload(t3, 'guzergah.pdf', pdf('Guzergah onerisi'), PDF, 'other');
await ali.act(t3, 'send');
await can.act(t3, 'accept');
await can.upload(t3, 'teknik-gorus.pdf', pdf('Teknik gorus'), PDF, 'other');
await can.act(t3, 'deliver');
await mehmet.act(t3, 'complete', { note: 'Görüş alındı, teşekkür ederiz.' });

// ── Files in the cloud (docs/adr/9000 §12-§15) ───────────────────────
/** Sends `bytes` as `name` through /v1/uploads in small parts, as a phone on a weak connection would. */
async function sendParts(person, name, bytes, part = 64 * 1024) {
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  let up = await person.req('POST', '/v1/uploads', { name, size: bytes.length, sha256 });
  while (!up.received) {
    const from = up.receivedBytes;
    up = await person.req('PUT', `/v1/uploads/${up.id}?offset=${from}`, undefined, { bytes: bytes.subarray(from, from + part), type: 'application/octet-stream' });
  }
  return up;
}

/** A small PNG drawn here: a made-up site sketch (parcels, a building, a north arrow). */
function sketch(width = 480, height = 320) {
  const px = Buffer.alloc((width * 3 + 1) * height, 255);
  const set = (x, y, [r, g, b]) => {
    if (x < 0 || y < 0 || x >= width || y >= height) return;
    const i = y * (width * 3 + 1) + 1 + x * 3;
    px[i] = r;
    px[i + 1] = g;
    px[i + 2] = b;
  };
  const line = (x0, y0, x1, y1, c) => {
    const n = Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0));
    for (let k = 0; k <= n; k++) set(Math.round(x0 + ((x1 - x0) * k) / n), Math.round(y0 + ((y1 - y0) * k) / n), c);
  };
  for (let y = 0; y < height; y++) px[y * (width * 3 + 1)] = 0;
  const ink = [30, 40, 51];
  const blue = [37, 99, 235];
  const green = [46, 125, 79];
  for (const [x0, y0, x1, y1] of [[40, 40, 220, 40], [220, 40, 220, 280], [220, 280, 40, 280], [40, 280, 40, 40], [220, 40, 440, 60], [440, 60, 430, 280], [430, 280, 220, 280]]) line(x0, y0, x1, y1, ink);
  for (let y = 110; y <= 200; y++) for (let x = 80; x <= 180; x++) if (x === 80 || x === 180 || y === 110 || y === 200) set(x, y, blue);
  for (let x = 250; x <= 400; x += 2) line(x, 240, x, 250, green);
  line(460, 300, 460, 250, ink);
  line(460, 250, 452, 266, ink);
  line(460, 250, 468, 266, ink);
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, 'latin1'), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body) >>> 0);
    return Buffer.concat([len, body, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(px)), chunk('IEND', Buffer.alloc(0))]);
}

function crc32(buf) {
  let c = ~0;
  for (const b of buf) {
    c ^= b;
    for (let k = 0; k < 8; k++) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1;
  }
  return ~c;
}

// The municipality keeps the reports beside the project it opened from the first application.
const orgProject = projects.projects[0];
for (const [name, bytes, note] of [
  ['zemin-etudu-raporu.pdf', pdf('Zemin etudu raporu', ['412 ada 7 parsel', 'Sondaj 1-3']), 'Zemin etüdü, üç sondaj'],
  ['saha-krokisi.png', sketch(), 'Saha krokisi, 14 Kasım keşfi'],
  ['yapi-denetim-tutanagi.pdf', pdf('Yapi denetim tutanagi', ['Temel betonu oncesi kontrol']), null],
]) {
  const up = await sendParts(mehmet, name, bytes);
  await mehmet.req('POST', `/v1/tenants/${ornek.id}/projects/${orgProject.id}/attachments`, { uploadId: up.id, note });
}
// The surveyor's own project gets the site photo-sketch of the job.
const canSketch = await sendParts(can, 'aplikasyon-krokisi.png', sketch(560, 360));
await can.req('POST', `/v1/tenants/${canProject.tenantId}/projects/${canProject.projectId}/attachments`, { uploadId: canSketch.id, note: '501 ada köşe noktaları' });
// A dossier document sent in parts from a phone.
const phoneDoc = await sendParts(ayse, 'statik-hesap-ozeti.pdf', pdf('Statik hesap ozeti', ['412 ada 7 parsel']));
await ayse.req('POST', `/v1/dossiers/${a3}/files/from-upload`, { uploadId: phoneDoc.id, purpose: 'package', docType: 'calculations' });

// Programs signed in as the architect: her phone and her office's desktop; and a read-only token for an agent.
const program = async (client, label) => {
  const res = await fetch(`${API}/v1/auth/token`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ login: 'ayse.yilmaz', password: PASSWORD, client, label }) });
  if (!res.ok) throw new Error(`token: ${res.status} ${await res.text()}`);
  return (await res.json()).token;
};
const phone = await program('mobile', 'Ayşe’nin telefonu');
await program('desktop', 'Ofis bilgisayarı');
await fetch(`${API}/v1/me/notifications`, { headers: { authorization: `Bearer ${phone}` } });
await ayse.req('POST', '/v1/me/tokens', { name: 'Rapor ajanı', scope: 'read', expiresInDays: 30 });

// Files the scan refuses: a program passed off as a PDF, and the antivirus test file (put together here, in two
// halves, so that this script is not itself taken for one).
for (const [name, bytes] of [
  ['ruhsat-eki.pdf', Buffer.concat([Buffer.from('MZ'), Buffer.alloc(64)])],
  ['deneme.txt', Buffer.from(['X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-', 'ANTIVIRUS-TEST-FILE!$H+H*'].join(''), 'latin1')],
]) {
  try {
    await sendParts(ayse, name, bytes);
    throw new Error(`${name} alınmamalıydı`);
  } catch (e) {
    if (!String(e.message).includes('422')) throw e;
  }
}

console.log(JSON.stringify({ ornek: ornek.id, yesilvadi: yesilvadi.id, a1, a2, a3, a4, a5, a6, t1, t2, t3, orgProject: orgProject.id }));
