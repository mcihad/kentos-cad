// Invitations by e-mail in the browser (docs/adr/0035, 0042), a part of the
// cloud end-to-end test (cloud.mjs calls it last, with its browser signed in
// as ayse). Ayşe invites an address from the share dialog (typed in “Kişi
// ekle”, where no member matches, then on the “Davetler” tab) and gets the
// one-time link, which is kept nowhere; the link opened signed out asks for a
// sign-in and keeps the token through it; mehmet's account is refused as
// another address's; “Başka hesapla giriş yap”
// signs in a second account outside the organisation, which accepts it as a
// guest and opens the project; the used link then fails. A second address's
// invitation is replaced by a new one (after a question) and the new one is
// withdrawn: both links fail. The guest reads right in the access list and
// loses the project when Ayşe takes the access away.
import { spawnSync } from 'node:child_process';
import { sleep } from './cdp.mjs';

const GUEST = { login: 'misafir', name: 'Misafir Kişi', email: 'misafir@misafir.example' };
const TOKEN = /^[0-9a-f]{64}$/;

/**
 * @param t the cloud test's browser and helpers: b, check, press, themed, env, url (the app's
 *   address), ready, client, emptyProject, tenantId, ayse (her own API client), openCatalog,
 *   showList, pickExact, server (the kentosd binary), root, ownerUrl (the scratch database's, or null)
 */
export async function invitations(t) {
  const { b, check, press, themed, env, ready, client, ayse } = t;
  const password = env.KENTOS_DEV_PASSWORD;

  // The account outside the organisation (a guest to be): made once; a local account's e-mail counts as verified.
  const made = spawnSync(t.server, ['user', 'add', '--login', GUEST.login, '--name', GUEST.name, '--email', GUEST.email, '--password-env', 'KENTOS_DEV_PASSWORD'], {
    cwd: t.root,
    env: { ...process.env, KENTOS_DEV_PASSWORD: password, ...(t.ownerUrl ? { KENTOS_DATABASE_OWNER_URL: t.ownerUrl } : {}) },
    encoding: 'utf8',
  });
  const guest = client();
  const signedGuest = await guest.call('POST', '/v1/auth/login', { login: GUEST.login, password });
  check('an account outside the organisation, with an e-mail, signs in', signedGuest.status === 200 && signedGuest.body.user.email === GUEST.email && !signedGuest.body.memberships.some((m) => m.tenantId === t.tenantId), made.status === 0 ? 'yeni' : 'vardı');

  // A project of Ayşe's to invite to.
  const stamp = new Date().toISOString().slice(0, 19);
  const title = `E2E davet ${stamp}`;
  const created = await ayse.call('POST', `/v1/tenants/${t.tenantId}/projects`, { name: title, ...t.emptyProject });
  const pid = created.body.id;
  const base = `/v1/tenants/${t.tenantId}/projects/${pid}`;

  // ── The share dialog: an address no member matches is offered as an invitation ──
  const shareDialog = '.dialog--share';
  const openShare = async () => {
    await t.openCatalog();
    await t.showList('Projelerim');
    await t.pickExact(title);
    await press('.catalog-details__actions .btn', 'Paylaş');
    await b.waitFor(`!!document.querySelector('${shareDialog} .share-row[data-user]') && !document.querySelector('${shareDialog} input[aria-label="Paylaşılacak kişi"]').disabled`, 8000);
  };
  const invitesShown = `!!document.querySelector('${shareDialog} .invite-list') && !document.querySelector('${shareDialog} .invite-list').textContent.includes('yükleniyor')`;
  await openShare();
  await b.eval(`document.querySelector('${shareDialog} input[aria-label="Paylaşılacak kişi"]').focus()`);
  await b.type(GUEST.email);
  await b.waitFor(`!!document.querySelector('${shareDialog} .share-suggest__invite')`, 8000);
  const offered = await b.eval(`document.querySelector('${shareDialog} .share-suggest').textContent`);
  check('no member matches the address: the list says so and offers an invitation instead', /Kurum dışından biri “Davetler”den e-postayla davet edilir/.test(offered) && offered.includes(`“${GUEST.email}” adresine e-postayla davet gönder`), offered.slice(0, 160));
  await press(`${shareDialog} .share-suggest__invite`, `“${GUEST.email}”`);
  await b.waitFor(`document.querySelector('${shareDialog} .share-tabs [aria-selected="true"]')?.textContent === 'Davetler' && ${invitesShown}`, 5000);
  const form = await b.eval(`(() => { const d = document.querySelector('${shareDialog}'); return { email: d.querySelector('input[aria-label="Davet edilecek e-posta"]').value, roles: [...d.querySelector('select[aria-label="Davetin rolü"]').options].map((o) => o.textContent), wait: d.querySelector('select[aria-label="Davetin geçerlilik süresi"]').selectedOptions[0].textContent }; })()`);
  check('the “Davetler” tab begins with that address; roles up to editor; 14 days unless chosen', form.email === GUEST.email && form.roles.join() === 'Görüntüleyici,Yorumcu,Düzenleyici' && form.wait === '14 gün (varsayılan)', JSON.stringify(form));
  await b.eval(`(() => { const s = document.querySelector('${shareDialog} select[aria-label="Davetin rolü"]'); s.value = 'editor'; s.dispatchEvent(new Event('change')); })()`);
  await press(`${shareDialog} .share-add--invite .btn`, 'Davet et');
  await b.waitFor(`!!document.querySelector('${shareDialog} .invite-link__url') && !!document.querySelector('${shareDialog} .invite-row[data-state="pending"]')`, 10000);
  const link = await b.eval(`document.querySelector('${shareDialog} .invite-link__url').value`);
  const token = new URL(link).searchParams.get('davet') ?? '';
  const shown = await b.eval(`({ warn: document.querySelector('${shareDialog} .invite-link__warn').textContent, row: document.querySelector('${shareDialog} .invite-row[data-state="pending"]').textContent })`);
  check(
    'the invitation’s link is this app’s address with the token, shown once and said so; the list has it waiting',
    link.startsWith(t.url) && TOKEN.test(token) && /yalnız şimdi gösterilir/.test(shown.warn) && shown.row.includes(GUEST.email) && shown.row.includes('Düzenleyici') && shown.row.includes('Bekliyor'),
    shown.row,
  );
  const kept = await b.eval(
    `JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage), window.kentos.log.entries.value.map((e) => e.text)]).includes(${JSON.stringify(token)})`,
  );
  const listed = await ayse.call('GET', `${base}/invitations`);
  check('the token is kept nowhere: not in the log, nor in any storage, nor in the server’s list', !kept && listed.status === 200 && listed.body.invitations[0]?.email === GUEST.email && !JSON.stringify(listed.body).includes(token), `${listed.body.invitations?.length} davet`);
  await themed('cloud-invite');
  // Closed and opened again: the link is gone for good.
  await b.key('Escape');
  await openShare();
  await press(`${shareDialog} .share-tabs .tab`, 'Davetler');
  await b.waitFor(invitesShown, 5000);
  const again = await b.eval(`({ hidden: document.querySelector('${shareDialog} .invite-link').hidden, pending: document.querySelectorAll('${shareDialog} .invite-row[data-state="pending"]').length })`);
  check('opened again, the dialog lists the invitation but never shows its link again', again.hidden && again.pending === 1, JSON.stringify(again));
  await b.key('Escape');

  // ── The link opened signed out: the token leaves the address and waits through a sign-in ──
  const navigate = async (to) => {
    await b.send('Page.navigate', { url: to });
    await sleep(500);
    await b.waitFor(ready, 20000);
  };
  const acceptDialog = '.dialog--invite';
  const signInAs = async (login) => {
    await b.waitFor(`document.querySelector('.dialog--cloud input[name=login]')`, 8000);
    await b.eval(`document.querySelector('.dialog--cloud input[name=login]').focus()`);
    await b.type(login);
    await b.key('Tab');
    await b.type(password);
    await b.key('Enter');
  };
  await b.eval(`window.kentos.cloud.signOut()`);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`, 5000);
  await navigate(link);
  await b.waitFor(`document.querySelector('${acceptDialog} .invite-accept__title')?.textContent === 'Bir projeye davet edildiniz'`, 15000);
  const waiting = await b.eval(`({ search: location.search, kept: sessionStorage.getItem('kentos.invitation') === ${JSON.stringify(token)}, local: JSON.stringify(Object.entries(localStorage)).includes(${JSON.stringify(token)}) })`);
  check('opened signed out: the token leaves the address at once and waits in this tab only (never localStorage); sign-in is asked', !waiting.search.includes('davet') && waiting.kept && !waiting.local, JSON.stringify(waiting));
  await themed('cloud-invite-signin');
  // Mehmet signs in (a member without that e-mail): another address's invitation.
  await press(`${acceptDialog} .dialog__foot .btn`, 'Giriş yap');
  await signInAs('mehmet');
  await b.waitFor(`!!document.querySelector('${acceptDialog} .invite-accept__error')`, 15000);
  const refused = await b.eval(
    `({ search: location.search, kept: sessionStorage.getItem('kentos.invitation') === ${JSON.stringify(token)}, text: document.querySelector('${acceptDialog}').textContent, other: !document.querySelector('${acceptDialog} .dialog__foot .btn--ghost').hidden })`,
  );
  check(
    'through the sign-in, the invitation is tried: another address’s account is refused in the server’s words, the token kept for the right one',
    !refused.search.includes('davet') && refused.kept && /Bu davet başka bir e-posta adresi için/.test(refused.text) && /Giriş yapan: Mehmet Demir/.test(refused.text) && refused.other,
    refused.text.slice(0, 160),
  );
  await themed('cloud-invite-refused');

  // ── “Başka hesapla giriş yap”: the guest-to-be signs in, accepts, and opens it ──
  await press(`${acceptDialog} .dialog__foot .btn`, 'Başka hesapla giriş yap');
  await signInAs(GUEST.login);
  await b.waitFor(`!!document.querySelector('${acceptDialog} .invite-accept__facts')`, 15000);
  const accepted = await b.eval(
    `({ facts: document.querySelector('${acceptDialog} .invite-accept__facts').textContent, title: document.querySelector('${acceptDialog} .invite-accept__title').textContent, kept: sessionStorage.getItem('kentos.invitation') })`,
  );
  check(
    'signed in with the invited address, it is accepted: the project, its workspace, the role and that it is a guest',
    accepted.title.includes(title) && /Örnek Harita Bürosu/.test(accepted.facts) && /Düzenleyici/.test(accepted.facts) && /Misafir olarak/.test(accepted.facts) && accepted.kept === null,
    accepted.facts,
  );
  await themed('cloud-invite-accepted');
  await press(`${acceptDialog} .dialog__foot .btn`, 'Projeyi aç');
  await b.waitFor(`window.kentos.cloud.project.value?.projectId === ${JSON.stringify(pid)} && window.kentos.cloud.link.value === 'online'`, 30000);
  const opened = await b.eval(`({ role: window.kentos.cloud.project.value.role, dialog: !!document.querySelector('${acceptDialog}') })`);
  await b.eval(`(() => { const d = window.kentos.doc; d.add({ kind: 'point', layerId: d.layers.leaves()[0].id, p: { x: 486700, y: 4420300 }, attrs: {} }); })()`);
  await b.waitFor(`window.kentos.cloud.sync.value?.state.value === 'saved' && !window.kentos.doc.dirty.value`, 15000);
  const written = await ayse.call('GET', `${base}/features?limit=10`);
  check('“Projeyi aç” opens it; the guest edits it and the owner sees the edit', opened.role === 'editor' && !opened.dialog && written.body.features?.length === 1, `${written.body.features?.length} nesne`);

  // The used link, opened again: the server's “not found”, the token forgotten.
  await navigate(link);
  await b.waitFor(`!!document.querySelector('${acceptDialog} .invite-accept__error')`, 15000);
  const used = await b.eval(`({ text: document.querySelector('${acceptDialog}').textContent, kept: sessionStorage.getItem('kentos.invitation') })`);
  check('a used link is refused as not found, and forgotten', /Davet bulunamadı: süresi dolmuş, kullanılmış ya da geri alınmış olabilir/.test(used.text) && used.kept === null, used.text.slice(0, 120));
  await b.key('Escape');

  // ── Back to Ayşe: a waiting invitation replaced after a question, then withdrawn; both links fail ──
  await b.eval(`window.kentos.cloud.signOut()`);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedOut'`, 5000);
  await b.eval(`window.kentos.cloud.signIn('ayse', ${JSON.stringify(password)})`);
  await b.waitFor(`window.kentos.cloud.auth.value === 'signedIn'`, 5000);
  const other = 'baska@misafir.example';
  const inviteOther = async () => {
    await b.eval(`(() => { const i = document.querySelector('${shareDialog} input[aria-label="Davet edilecek e-posta"]'); i.value = ${JSON.stringify(other)}; i.dispatchEvent(new Event('input')); })()`);
    await press(`${shareDialog} .share-add--invite .btn`, 'Davet et');
  };
  const newLink = async (before) => {
    await b.waitFor(`document.querySelector('${shareDialog} .invite-link__url')?.value && document.querySelector('${shareDialog} .invite-link__url').value !== ${JSON.stringify(before)}`, 10000);
    return new URL(await b.eval(`document.querySelector('${shareDialog} .invite-link__url').value`)).searchParams.get('davet');
  };
  await openShare();
  await press(`${shareDialog} .share-tabs .tab`, 'Davetler');
  await b.waitFor(invitesShown, 5000);
  await inviteOther();
  const first = await newLink('');
  await inviteOther();
  await b.waitFor(`!!document.querySelector('.dialog[aria-label="Bekleyen davet var"]')`, 5000);
  const asked = await b.eval(`document.querySelector('.dialog[aria-label="Bekleyen davet var"]').textContent`);
  check('inviting a waiting address again asks first: its link would stop working', /bekleyen davet geri alınır; onun bağlantısı artık çalışmaz/.test(asked), asked.slice(0, 140));
  await press('.dialog[aria-label="Bekleyen davet var"] .dialog__foot .btn', 'Yeni davet gönder');
  const second = await newLink(`${t.url}?davet=${first}`);
  await b.waitFor(`document.querySelectorAll('${shareDialog} .invite-row[data-state="revoked"]').length === 1`, 8000);
  const firstFails = await guest.call('POST', '/v1/invitations/accept', { token: first });
  await press(`${shareDialog} .invite-row[data-state="pending"] .btn`, 'Geri al');
  await b.waitFor(`!!document.querySelector('.dialog[aria-label="Daveti geri al"]')`, 5000);
  await b.shot('cloud-invite-revoke');
  await press('.dialog[aria-label="Daveti geri al"] .dialog__foot .btn', 'Daveti geri al');
  await b.waitFor(`document.querySelectorAll('${shareDialog} .invite-row[data-state="revoked"]').length === 2 && !document.querySelector('${shareDialog} .invite-row[data-state="pending"]')`, 8000);
  const secondFails = await guest.call('POST', '/v1/invitations/accept', { token: second });
  const states = (await ayse.call('GET', `${base}/invitations`)).body.invitations.map((i) => `${i.email}:${i.state}`);
  check(
    'a replaced and a withdrawn invitation: both links fail (404), the list says why',
    firstFails.status === 404 && secondFails.status === 404 && states.join() === `${other}:revoked,${other}:revoked,${GUEST.email}:accepted`,
    states.join(' '),
  );

  // ── The guest in the access list; the access taken away ──
  await press(`${shareDialog} .share-tabs .tab`, 'Kişiler');
  const guestRow = `${shareDialog} .share-row[data-user="${signedGuest.body.user.id}"]`;
  await b.waitFor(`!!document.querySelector(${JSON.stringify(guestRow)})`, 5000);
  const row = await b.eval(`(() => { const r = document.querySelector(${JSON.stringify(guestRow)}); return { text: r.textContent, select: !!r.querySelector('select'), title: r.querySelector('.share-role')?.title }; })()`);
  check('the guest reads as one: “Misafir (davetle)”, the role the invitation gave, fixed here (a new invitation changes it)', /Misafir \(davetle\)/.test(row.text) && /Düzenleyici/.test(row.text) && !row.select && /davetle verilir/.test(row.title ?? ''), row.text);
  await themed('cloud-invite-guest');
  await press(`${guestRow} .share-remove`, 'Kaldır');
  await b.waitFor(`!!document.querySelector('.dialog[aria-label="Erişimi kaldır"]')`, 5000);
  await press('.dialog[aria-label="Erişimi kaldır"] .dialog__foot .btn', 'Erişimi kaldır');
  await b.waitFor(`!document.querySelector(${JSON.stringify(guestRow)})`, 8000);
  const lost = await guest.call('GET', base);
  check('the guest’s access taken away: the project is not found for them', lost.status === 404, String(lost.status));
  await b.key('Escape');
}
