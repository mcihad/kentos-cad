// Signing in and signing up (docs/adr/9000 §2). Accounts sign in with their sign-in name and password; an architect,
// surveyor or engineer opens their own account while the platform takes sign-ups, and works once an administrator has
// checked their chamber registration.
import { ApiError } from '../api/client';
import { portal } from '../api/portal';
import type { Profession } from '../api/types';
import { signedIn } from '../app/session';
import { catalog } from '../app/state';
import { h } from '../lib/dom';
import { form, formError, grid, orNull, selectField, textField, values } from '../ui/forms';
import { brandMark, icon } from '../ui/icons';
import { button, notice } from '../ui/kit';

function brand(): HTMLElement {
  return h('div', { class: 'auth-brand' }, brandMark(30), h('span', { class: 'brand-word' }, h('b', null, 'Kent'), 'OS'), h('span', { class: 'brand-product' }, 'Bulut'));
}

export async function loginPage(query: URLSearchParams): Promise<HTMLElement> {
  let open = false;
  try {
    open = (await portal.signupInfo()).open;
  } catch {
    // The page still signs in.
  }
  const f = form(
    [
      textField({ label: 'Giriş adı', name: 'login', autocomplete: 'username', required: true }),
      textField({ label: 'Parola', name: 'password', type: 'password', autocomplete: 'current-password', required: true }),
      formError(),
      button('Oturum aç', { kind: 'primary', type: 'submit' }),
    ],
    async (el) => {
      const v = values(el);
      await portal.login(v.login.trim(), v.password);
      await signedIn(query.get('sonra'));
    },
  );
  return h(
    'div',
    { class: 'auth' },
    h(
      'div',
      { class: 'auth-card' },
      brand(),
      h('h1', null, 'Oturum açın'),
      h('p', { class: 'lead' }, 'Belediyeler, mimarlar, harita ve inşaat mühendisleri dosyalarını burada karşılıklı yürütür.'),
      f,
      h(
        'ul',
        { class: 'auth-roles' },
        h('li', { class: 'auth-role' }, icon('building', 18), h('span', null, h('strong', null, 'Belediyeler'), ' başvuruları inceler, eksikleri bir defada bildirir, projeyi mühendise aktarır.')),
        h('li', { class: 'auth-role' }, icon('badge', 18), h('span', null, h('strong', null, 'Mimar ve mühendisler'), ' başvuru gönderir, aktarılan işi üstlenip teslim eder.')),
        h('li', { class: 'auth-role' }, icon('shield', 18), h('span', null, h('strong', null, 'Sistem yönetimi'), ' kurumları, hesapları ve meslek kayıtlarını yönetir.')),
      ),
      h(
        'div',
        { class: 'auth-foot' },
        open
          ? [h('span', null, 'Mimar, harita ya da inşaat mühendisi misiniz? '), h('a', { href: '#/kayit' }, 'Meslek mensubu olarak kayıt olun')]
          : 'Hesabınızı kurumunuzun yöneticisi ya da sistem yöneticisi açar.',
      ),
    ),
  );
}

export async function signupPage(): Promise<HTMLElement> {
  let open = false;
  // The form's ticket (docs/adr/9000 §20): sent back with the sign-up; a new one when the server asks for it.
  let ticket: string | null = null;
  try {
    const info = await portal.signupInfo();
    open = info.open;
    ticket = info.ticket ?? null;
  } catch {
    // Said below as closed.
  }
  const professions = catalog.value?.professions ?? [];
  if (!open) {
    return h(
      'div',
      { class: 'auth' },
      h(
        'div',
        { class: 'auth-card' },
        brand(),
        h('h1', null, 'Kayıt şu an kapalı'),
        notice('info', 'Platform şu anda yeni meslek mensubu kaydı almıyor.', 'Hesap için sistem yöneticisine başvurun.'),
        h('div', { class: 'auth-foot' }, h('a', { href: '#/giris' }, 'Oturum açma sayfasına dönün')),
      ),
    );
  }
  const chamberHint = h('span', null, 'Mesleğinizi seçin; odanız ona göre yazılır.');
  const professionSelect = selectField({
    label: 'Meslek',
    name: 'profession',
    required: true,
    placeholder: 'Seçin…',
    options: professions.map((p) => ({ value: p.code, label: p.label })),
    hint: chamberHint,
  });
  professionSelect.querySelector('select')?.addEventListener('change', (e) => {
    const p = professions.find((x) => x.code === (e.target as HTMLSelectElement).value);
    chamberHint.textContent = p ? `Oda: ${p.chamber}` : 'Mesleğinizi seçin; odanız ona göre yazılır.';
  });
  const f = form(
    [
      h('div', { class: 'form-section' }, h('h3', null, 'Mesleki bilgiler'), grid(
        professionSelect,
        textField({ label: 'Oda sicil numarası', name: 'chamberNo', required: true, hint: 'Sistem yöneticisi odanın kayıtlarıyla karşılaştırır.' }),
        textField({ label: 'Büro ya da şirket adı', name: 'officeName' }),
        textField({ label: 'İl', name: 'province' }),
      )),
      h('div', { class: 'form-section' }, h('h3', null, 'Hesap'), grid(
        textField({ label: 'Ad soyad', name: 'displayName', required: true, autocomplete: 'name' }),
        textField({ label: 'E-posta', name: 'email', type: 'email', required: true, autocomplete: 'email' }),
        textField({ label: 'Telefon', name: 'phone', autocomplete: 'tel' }),
        textField({ label: 'Giriş adı', name: 'login', required: true, autocomplete: 'username', hint: 'Küçük harf, rakam, . _ - @' }),
        textField({ label: 'Parola', name: 'password', type: 'password', required: true, autocomplete: 'new-password', hint: 'En az 10 karakter.' }),
        textField({ label: 'Parola (yeniden)', name: 'password2', type: 'password', required: true, autocomplete: 'new-password' }),
      )),
      notice('info', 'Kaydınız doğrulanana kadar başvuru gönderemezsiniz.', 'Doğrulama sonucunu bildirimlerinizde görürsünüz.'),
      // A field no person sees or fills: a machine that fills every field gives itself away.
      h('div', { class: 'hp-field', 'aria-hidden': 'true' }, h('label', null, 'Web sitesi ', h('input', { name: 'website', tabindex: -1, autocomplete: 'off' }))),
      formError(),
      h('div', { class: 'form-actions' }, button('Vazgeç', { kind: 'ghost', href: '#/giris' }), button('Kayıt ol', { kind: 'primary', type: 'submit' })),
    ],
    async (el) => {
      const v = values(el);
      if (v.password !== v.password2) throw new ApiError(400, 'invalid', 'İki parola aynı değil.', 'password2');
      try {
      await portal.signUp({
        login: v.login.trim(),
        password: v.password,
        displayName: v.displayName,
        email: v.email,
        profession: v.profession as Profession,
        chamberNo: v.chamberNo,
        officeName: orNull(v.officeName),
        phone: orNull(v.phone),
        province: orNull(v.province),
        ticket,
        website: orNull(v.website),
      });
      } catch (e) {
        // A used or old ticket: the next try takes a new one.
        if (e instanceof ApiError && e.path === 'ticket') ticket = (await portal.signupInfo().catch(() => null))?.ticket ?? ticket;
        throw e;
      }
      await signedIn('#/mesleki');
    },
  );
  return h(
    'div',
    { class: 'auth' },
    h('div', { class: 'auth-card wide' }, brand(), h('h1', null, 'Meslek mensubu kaydı'), h('p', { class: 'lead' }, 'Belediyelere başvuru göndermek ve size aktarılan işleri almak için hesabınızı açın.'), f, h('div', { class: 'auth-foot' }, 'Hesabınız var mı? ', h('a', { href: '#/giris' }, 'Oturum açın'))),
  );
}
