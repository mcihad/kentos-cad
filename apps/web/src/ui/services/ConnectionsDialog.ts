import '../../styles/services.css';
import type { AppContext } from '../../app/context';
import type { AuthKind } from '../../contracts/generated/AuthKind';
import type { ConnectionSecret } from '../../contracts/generated/ConnectionSecret';
import type { ServiceConnection } from '../../contracts/generated/ServiceConnection';
import { getWire, serviceFetch, type Wire } from '../../io/services/fetch';
import { loadServices } from '../../io/services/module';
import { AUTH_KINDS, AUTH_KIND_LABELS, connectionsProblem, originOf } from '../../model/serviceRules';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const CONNECTIONS_TITLE = 'Bağlantılar';

/**
 * Bağlantılar (docs/adr/0208 §12; the desktop's `services/connections.rs`): the project's connections on the left, the
 * one chosen on the right: its name, its origin (where its proof goes), how it proves itself and its secret values.
 * Kaydet writes the connections into the project's settings (not an undo step, as the other settings) and the secrets
 * into this browser's storage (app/connectionSecrets.ts): a secret never goes into the drawing. Dene asks a service
 * that uses the connection with the values as typed and says what it answered.
 */

/** A connection as the window edits it: its definition and its secrets as typed (the desktop's `Row`). */
export interface ConnectionRow {
  from: string | null;
  id: string;
  name: string;
  origin: string;
  auth: AuthKind;
  pairs: [string, string][];
  user: string;
  password: string;
  token: string;
  clientId: string;
  clientSecret: string;
  tokenUrl: string;
  scope: string;
}

export function rowOf(c: ServiceConnection, s: ConnectionSecret | null): ConnectionRow {
  return {
    from: c.id,
    id: c.id,
    name: c.name,
    origin: c.origin,
    auth: c.auth,
    pairs: (c.names ?? []).map((n, i) => [n, s?.values?.[i] ?? ''] as [string, string]),
    user: s?.user ?? '',
    password: s?.password ?? '',
    token: s?.token ?? '',
    clientId: s?.clientId ?? '',
    clientSecret: s?.clientSecret ?? '',
    tokenUrl: c.tokenUrl ?? '',
    scope: c.scope ?? '',
  };
}

/** The connection a row defines (no secret). */
export function connectionOfRow(r: ConnectionRow): ServiceConnection {
  const named = r.auth === 'query' || r.auth === 'header';
  const some = (t: string) => (t.trim() ? t.trim() : undefined);
  const c: ServiceConnection = { id: r.id, name: r.name.trim(), origin: originOf(r.origin) ?? r.origin.trim(), auth: r.auth };
  if (named) c.names = r.pairs.map(([n]) => n.trim());
  if (r.auth === 'arcgis' || r.auth === 'oauth2') {
    const t = some(r.tokenUrl);
    if (t) c.tokenUrl = t;
  }
  if (r.auth === 'oauth2') {
    const s = some(r.scope);
    if (s) c.scope = s;
  }
  return c;
}

/** Its secrets as typed (none for a connection that proves nothing). */
export function secretOfRow(r: ConnectionRow): ConnectionSecret | null {
  const some = (t: string) => (t ? t : undefined);
  switch (r.auth) {
    case 'none':
      return null;
    case 'query':
    case 'header':
      return { id: r.id, values: r.pairs.map(([, v]) => v) };
    case 'basic':
    case 'arcgis':
      return { id: r.id, ...(some(r.user) && { user: r.user }), ...(some(r.password) && { password: r.password }) };
    case 'bearer':
    case 'google':
      return { id: r.id, ...(some(r.token.trim()) && { token: r.token.trim() }) };
    case 'oauth2':
      return { id: r.id, ...(some(r.clientId.trim()) && { clientId: r.clientId.trim() }), ...(some(r.clientSecret) && { clientSecret: r.clientSecret }) };
  }
}

/** A new connection's id: `baglanti`, then `baglanti-2` … as the others leave free. */
const freeId = (rows: readonly ConnectionRow[]) => {
  for (let n = 1; ; n++) {
    const id = n === 1 ? 'baglanti' : `baglanti-${n}`;
    if (!rows.some((r) => r.id === id)) return id;
  }
};

/** The layers of the drawing whose service or source uses connection `id`, by name. */
const usersOf = (ctx: AppContext, id: string): string[] =>
  ctx.doc.layers
    .leaves()
    .filter((l) => l.service?.connection === id || l.feed?.connection === id)
    .map((l) => l.name);

const sizeText = (n: number) => (n >= 1024 * 1024 ? `${(n / (1024 * 1024)).toFixed(1)} MB` : n >= 1024 ? `${Math.floor(n / 1024)} KB` : `${n} bayt`);

export interface ConnectionsOptions {
  /** The connection chosen first. */
  focus?: string;
  /** A new connection added first (Yeni bağlantı… from a service's window). */
  add?: boolean;
  /** Told when the window closes: the id of the connection added last, when Kaydet kept it. */
  done?: (added: string | null) => void;
}

/** Opens Bağlantılar over the window that asked. */
export function openConnections(ctx: AppContext, opts: ConnectionsOptions = {}): void {
  const before = ctx.doc.settings.connections.value.map((c) => rowOf(c, ctx.secrets.get(c.origin, c.id)));
  const rows: ConnectionRow[] = before.map((r) => ({ ...r, pairs: r.pairs.map(([a, b]) => [a, b] as [string, string]) }));
  let chosen = opts.focus ? Math.max(0, rows.findIndex((r) => r.id === opts.focus)) : rows.length ? 0 : -1;
  let added: string | null = null;
  let saved = false;
  let show = false;
  let said: { kind: 'ok' | 'error' | 'info'; text: string } | null = null;
  let trying = false;

  const list = h('div', { class: 'conn-list', role: 'listbox', 'aria-label': 'Bağlantılar' });
  const form = h('div', { class: 'conn-form' });
  const summary = h('div', { class: 'io-summary' });
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet') as HTMLButtonElement;
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç') as HTMLButtonElement;
  const add = h('button', { class: 'btn', type: 'button' }, icon('plus', 14), 'Yeni bağlantı') as HTMLButtonElement;
  const remove = h('button', { class: 'btn', type: 'button' }, icon('erase', 14), 'Sil') as HTMLButtonElement;

  const changed = () => JSON.stringify(rows) !== JSON.stringify(before);

  function renderList(): void {
    replaceChildren(
      list,
      ...(rows.length
        ? rows.map((r, i) => {
            const host = r.origin.replace(/^[a-z]+:\/\//i, '');
            const item = h(
              'button',
              { class: `conn-item${i === chosen ? ' is-selected' : ''}`, type: 'button', role: 'option', 'aria-selected': String(i === chosen) },
              h('span', { class: 'conn-item__name' }, r.name),
              h('span', { class: 'conn-item__meta' }, host),
              h('span', { class: 'conn-item__meta' }, AUTH_KIND_LABELS[r.auth]),
            );
            item.addEventListener('click', () => {
              chosen = i;
              said = null;
              render();
            });
            return item;
          })
        : [h('p', { class: 'conn-empty' }, 'Bağlantı yok. Yeni bağlantı ekleyin.')]),
    );
    remove.disabled = chosen < 0;
  }

  const input = (value: string, on: (v: string) => void, opts: { placeholder?: string; secret?: boolean; label: string }) => {
    const el = h('input', { class: 'field', type: opts.secret && !show ? 'password' : 'text', value, placeholder: opts.placeholder ?? '', 'aria-label': opts.label, spellcheck: 'false', autocomplete: 'off' }) as HTMLInputElement;
    el.addEventListener('input', () => {
      on(el.value);
      if (opts.label === 'Ad' || opts.label === 'Köken') renderList();
      check();
    });
    return el;
  };
  const row = (label: string, control: HTMLElement) => h('div', { class: 'conn-row' }, h('span', { class: 'conn-row__label' }, label), control);

  function renderForm(): void {
    const r = rows[chosen];
    if (!r) {
      replaceChildren(form, h('p', { class: 'conn-empty' }, 'Soldan bir bağlantı seçin.'));
      return;
    }
    const kinds = h('select', { class: 'field', 'aria-label': 'Doğrulama' }, AUTH_KINDS.map((k) => h('option', { value: k, selected: k === r.auth }, AUTH_KIND_LABELS[k]))) as HTMLSelectElement;
    kinds.addEventListener('change', () => {
      r.auth = kinds.value as AuthKind;
      if ((r.auth === 'query' || r.auth === 'header') && !r.pairs.length) r.pairs.push([r.auth === 'query' ? 'apikey' : 'X-API-Key', '']);
      render();
    });
    const parts: HTMLElement[] = [
      row('Ad', input(r.name, (v) => (r.name = v), { placeholder: 'Bağlantının adı', label: 'Ad' })),
      row('Köken', input(r.origin, (v) => (r.origin = v), { placeholder: 'https://makine[:kapı]', label: 'Köken' })),
      h('p', { class: 'io-field__hint' }, 'Kanıt yalnız bu kökene (şema, makine ve kapı) giden isteklere eklenir.'),
      row('Doğrulama', kinds),
    ];
    switch (r.auth) {
      case 'none':
        parts.push(h('p', { class: 'conn-empty' }, 'Servis kimlik istemiyor.'));
        break;
      case 'query':
      case 'header': {
        r.pairs.forEach((p, i) => {
          const del = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Kaldır', disabled: r.pairs.length < 2 }, icon('close', 12)) as HTMLButtonElement;
          del.addEventListener('click', () => {
            r.pairs.splice(i, 1);
            render();
          });
          parts.push(
            h(
              'div',
              { class: 'conn-pair' },
              input(p[0], (v) => (p[0] = v), { placeholder: r.auth === 'query' ? 'Parametre' : 'Başlık', label: 'Ad' + (i + 1) }),
              input(p[1], (v) => (p[1] = v), { placeholder: 'Değer', secret: true, label: 'Değer' }),
              del,
            ),
          );
        });
        const more = h('button', { class: 'btn btn--small', type: 'button' }, r.auth === 'query' ? 'Parametre ekle' : 'Başlık ekle');
        more.addEventListener('click', () => {
          r.pairs.push(['', '']);
          render();
        });
        parts.push(more);
        break;
      }
      case 'basic':
      case 'arcgis':
        parts.push(row('Kullanıcı adı', input(r.user, (v) => (r.user = v), { label: 'Kullanıcı adı' })));
        parts.push(row('Parola', input(r.password, (v) => (r.password = v), { secret: true, label: 'Parola' })));
        if (r.auth === 'arcgis')
          parts.push(row('Belirteç adresi', input(r.tokenUrl, (v) => (r.tokenUrl = v), { placeholder: '…/arcgis/tokens/generateToken (boşsa sunucunun)', label: 'Belirteç adresi' })));
        break;
      case 'bearer':
        parts.push(row('Belirteç', input(r.token, (v) => (r.token = v), { secret: true, label: 'Belirteç' })));
        break;
      case 'google':
        parts.push(row('API anahtarı', input(r.token, (v) => (r.token = v), { placeholder: 'AIza…', secret: true, label: 'API anahtarı' })));
        parts.push(h('p', { class: 'io-field__hint' }, 'Anahtar sizin Google Cloud hesabınızındır (Map Tiles API açık, faturalandırma tanımlı); KentOS anahtar vermez.'));
        break;
      case 'oauth2':
        parts.push(row('İstemci kimliği', input(r.clientId, (v) => (r.clientId = v), { label: 'İstemci kimliği' })));
        parts.push(row('İstemci sırrı', input(r.clientSecret, (v) => (r.clientSecret = v), { secret: true, label: 'İstemci sırrı' })));
        parts.push(row('Belirteç adresi', input(r.tokenUrl, (v) => (r.tokenUrl = v), { placeholder: 'https://…/token', label: 'Belirteç adresi' })));
        parts.push(row('Kapsam', input(r.scope, (v) => (r.scope = v), { placeholder: 'isteğe bağlı', label: 'Kapsam' })));
        break;
    }
    const showBox = h('input', { type: 'checkbox', checked: show }) as HTMLInputElement;
    showBox.addEventListener('change', () => {
      show = showBox.checked;
      renderForm();
    });
    const used = usersOf(ctx, r.from ?? r.id);
    const tryBtn = h('button', { class: 'btn', type: 'button', disabled: trying }, icon('serviceConnections', 14), 'Dene') as HTMLButtonElement;
    tryBtn.addEventListener('click', () => void tryConnection());
    parts.push(
      h('label', { class: 'io-check' }, showBox, 'Değerleri göster'),
      h('p', { class: 'io-field__hint' }, used.length ? `Kullanan katmanlar: ${used.join(', ')}` : 'Bu bağlantıyı kullanan katman yok.'),
      h('div', null, tryBtn),
    );
    replaceChildren(form, ...parts);
  }

  function check(): void {
    const p = connectionsProblem(rows.map(connectionOfRow));
    replaceChildren(
      summary,
      p
        ? summaryLine('error', p)
        : said
          ? summaryLine(said.kind, said.text)
          : summaryLine('info', 'Gizli değerler çizime yazılmaz; yalnız bu tarayıcıda saklanır. Çizim başka cihazda açılınca orada yeniden girilir.'),
    );
    save.disabled = p !== null || !changed();
  }

  function render(): void {
    renderList();
    renderForm();
    check();
  }

  /** Dene: a service of the drawing that uses the chosen connection asked with its values as typed. */
  async function tryConnection(): Promise<void> {
    const r = rows[chosen];
    if (!r) return;
    const conn = connectionOfRow(r);
    const secret = secretOfRow(r);
    const service = ctx.doc.layers.leaves().find((l) => l.service?.connection === (r.from ?? r.id))?.service;
    if (!service) {
      said = { kind: 'info', text: "Bu bağlantıyı kullanan servis yok: Harita servisi penceresinde bu bağlantıyla Bağlan'a basarak deneyin." };
      check();
      return;
    }
    trying = true;
    said = { kind: 'info', text: 'Servise soruluyor…' };
    render();
    try {
      const m = await loadServices();
      const probe = m.probe(JSON.stringify(service));
      const wire: Wire = probe ? (JSON.parse(probe) as Wire) : getWire(service.url);
      const a = await serviceFetch(m, wire, { connection: conn, secret }, { proxy: ctx.cloud.auth.value === 'signedIn', referer: service.url });
      said =
        a.status >= 200 && a.status < 300
          ? { kind: 'ok', text: `Bağlantı çalışıyor: servis ${a.status} dedi (${sizeText(a.bytes.length)}).` }
          : { kind: 'error', text: a.status === 401 || a.status === 403 ? `Sunucu erişimi reddetti (${a.status}): değerleri denetleyin.` : `Sunucu ${a.status} dedi.` };
    } catch (e) {
      said = { kind: 'error', text: e instanceof Error ? e.message : String(e) };
    }
    trying = false;
    render();
  }

  const addRow = () => {
    const id = freeId(rows);
    rows.push({ from: null, id, name: 'Yeni bağlantı', origin: 'https://', auth: 'query', pairs: [['apikey', '']], user: '', password: '', token: '', clientId: '', clientSecret: '', tokenUrl: '', scope: '' });
    chosen = rows.length - 1;
    added = id;
    said = null;
  };
  add.addEventListener('click', () => {
    addRow();
    render();
  });
  remove.addEventListener('click', () => {
    const r = rows[chosen];
    if (!r) return;
    const used = usersOf(ctx, r.from ?? r.id);
    if (used.length) {
      said = { kind: 'error', text: `Bu bağlantıyı şu katmanlar kullanıyor: ${used.join(', ')}. Önce onları silin ya da başka bağlantıya geçirin.` };
      check();
      return;
    }
    rows.splice(chosen, 1);
    chosen = Math.min(chosen, rows.length - 1);
    said = null;
    render();
  });
  save.addEventListener('click', () => {
    const list = rows.map(connectionOfRow);
    const p = connectionsProblem(list);
    if (p) return;
    // The ones gone or moved: their secrets forgotten at their old place.
    for (const b of before) if (!rows.some((r) => r.from === b.id && connectionOfRow(r).origin === b.origin)) ctx.secrets.remove(b.origin, b.id);
    for (const r of rows) {
      const c = connectionOfRow(r);
      const s = secretOfRow(r);
      const why = s ? ctx.secrets.put(c.origin, s) : ctx.secrets.remove(c.origin, c.id);
      if (why) ctx.log.warn(why);
    }
    ctx.doc.settings.assign({ connections: list });
    ctx.log.success(`Bağlantılar kaydedildi (${list.length}); gizli değerler bu tarayıcıda saklandı.`);
    saved = true;
    dialog.close();
  });
  cancel.addEventListener('click', () => dialog.close());

  const dialog = new Dialog({
    title: CONNECTIONS_TITLE,
    width: 860,
    className: 'dialog--connections',
    content: [h('div', { class: 'conn-panes' }, h('div', { class: 'conn-left' }, list, h('div', { class: 'conn-actions' }, add, remove)), form), summary],
    footer: [cancel, save],
    onClose: () => opts.done?.(saved && added && rows.some((r) => r.id === added) ? added : null),
    stack: true,
  });
  if (opts.add) addRow();
  render();
}
