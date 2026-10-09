import '../../styles/networks.css';
import type { AppContext } from '../../app/context';
import { DisposableStore, listen } from '../../core/disposable';
import type { JunctionRole } from '../../contracts/generated/JunctionRole';
import type { NetworkCostKind } from '../../contracts/generated/NetworkCostKind';
import type { NetworkDef } from '../../contracts/generated/NetworkDef';
import { attributeFields, entityObjects } from '../../model/expression/builderObjects';
import { compileExpression, expressionError } from '../../model/expression/expression';
import type { NetworkCheck, NetworkProblem } from '../../model/networkAnswers';
import { NETWORK_PROBLEM_LABELS } from '../../model/networkAnswers';
import { DIRECTION_DEFAULTS, defOf, formExpressions, formOf, newForm, retyped, sameNetwork, type DirectionKind, type NetworkForm } from '../../model/networkForm';
import {
  JUNCTION_ROLE_LABELS,
  JUNCTION_ROLES,
  NETWORK_CONNECT_LABELS,
  NETWORK_CONNECTS,
  NETWORK_COST_KIND_LABELS,
  NETWORK_COST_KINDS,
  NETWORK_KIND_LABELS,
  NETWORK_KINDS,
  NETWORK_LAYER_LIMIT,
  NETWORK_COST_LIMIT,
  NETWORK_LIMIT,
  networksProblem,
  nextNetworkId,
} from '../../model/networkRules';
import { networkDefine } from '../../product/networkDefine';
import { builderButton } from '../expression/builderApi';
import { h, replaceChildren } from '../dom';
import { summaryLine } from '../io/common';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { askUnsaved } from '../widgets/confirm';

/** The window's title, which a trace names it by. */
export const NETWORKS_TITLE = 'Ağlar';

/**
 * Ağlar (docs/adr/0209 §10; the desktop's `networks/window.rs`): the project's networks on the left (Yeni ağ, Sil), the
 * chosen one's definition on the right: its name and kind, edge layers with their filters, junction layers with their
 * roles, filters and closed expressions, how ends meet and the tolerance, the direction, the costs and the closed
 * edges. Denetle builds the network as the window says it and lists its counts and problems; a problem's row selects
 * its objects, marks it in the drawing and goes there. The first thing wrong is said under the form and Kaydet waits;
 * Kaydet writes each changed network through `cad.network.define` (a project setting, not an undo step).
 */

const DIRECTIONS: readonly { value: DirectionKind; text: string }[] = [
  { value: 'both', text: 'Yok (iki yön)' },
  { value: 'digitized', text: 'Çizim yönü' },
  { value: 'field', text: 'Alandan' },
];

/** At most this many problems are listed (Denetle says how many more). */
const SHOWN_PROBLEMS = 300;

/** Opens Ağlar on the project's networks; `chosen`: the network to show first. */
export function openNetworksDialog(ctx: AppContext, chosenId?: string): void {
  const { doc, format } = ctx;
  const d = new DisposableStore();
  const saved: readonly NetworkDef[] = doc.settings.networks.value;
  let forms: NetworkForm[] = saved.map((n) => formOf(n, (m) => format.fromMetres(m)));
  let chosen = Math.max(0, forms.findIndex((f) => f.id === chosenId));
  if (!forms.length) chosen = -1;
  /** Denetle's answer for the chosen network as it was when asked. */
  let checked: { key: string; value: NetworkCheck | null; words: string | null } | null = null;
  let done = false;

  const list = h('div', { class: 'net-list', role: 'listbox', 'aria-label': 'Ağlar' });
  const form = h('div', { class: 'net-form' });
  const summary = h('div', { class: 'io-summary' });
  const button = (words: string, glyph: string | null, cls = 'btn') => h('button', { class: cls, type: 'button' }, glyph ? icon(glyph, 14) : null, words) as HTMLButtonElement;
  const add = button('Yeni ağ', 'plus');
  const remove = button('Sil', 'erase');
  const save = button('Kaydet', null, 'btn btn--primary');
  const cancel = button('Vazgeç', null);

  const layers = () => doc.layers.leaves();
  const layerOptions = (current: string) => {
    const known = layers().map((l) => ({ value: l.id, text: doc.layers.path(l.id) }));
    if (current && !known.some((o) => o.value === current)) known.push({ value: current, text: `(çizimde yok: ${current})` });
    return [{ value: '', text: '—' }, ...known];
  };
  const select = (label: string, value: string, options: readonly { value: string; text: string }[], set: (v: string) => void, cls = 'field') => {
    const el = h('select', { class: cls, 'aria-label': label }, options.map((o) => h('option', { value: o.value, selected: o.value === value }, o.text))) as HTMLSelectElement;
    el.addEventListener('change', () => set(el.value));
    return el;
  };
  const input = (label: string, value: string, set: (v: string) => void, opts: { placeholder?: string; list?: string; cls?: string } = {}) => {
    const el = h('input', { class: opts.cls ?? 'field', type: 'text', value, 'aria-label': label, placeholder: opts.placeholder ?? '', spellcheck: 'false', list: opts.list ?? null }) as HTMLInputElement;
    el.addEventListener('input', () => {
      set(el.value);
      check();
    });
    return el;
  };
  const row = (label: string, ...controls: HTMLElement[]) => h('div', { class: 'net-row' }, h('span', { class: 'net-row__label' }, label), h('div', { class: 'net-row__controls' }, controls));
  const head = (label: string, action?: HTMLElement) => h('div', { class: 'net-head' }, h('span', null, label), action ?? null);
  const del = (label: string, run: () => void) => {
    const b = h('button', { class: 'ibtn', type: 'button', 'aria-label': label, title: label }, icon('close', 12)) as HTMLButtonElement;
    b.addEventListener('click', run);
    return b;
  };

  /** The attribute names of a network's layers' objects (a sample of each layer), for fields and the builder. */
  const fieldNames = (f: NetworkForm): { name: string; count: number }[] => {
    const counts = new Map<string, number>();
    for (const id of new Set([...f.edges.map((e) => e.layer), ...f.junctions.map((j) => j.layer)])) for (const e of doc.byLayer(id).slice(0, 2000)) for (const k of Object.keys(e.attrs)) counts.set(k, (counts.get(k) ?? 0) + 1);
    return [...counts].map(([name, count]) => ({ name, count })).sort((a, b) => a.name.localeCompare(b.name, 'tr'));
  };
  const exprField = (label: string, value: string, set: (v: string) => void, f: NetworkForm, layer: () => string, placeholder: string) => {
    const el = input(label, value, set, { placeholder, cls: 'field net-expr' });
    const b = builderButton({
      get: () => el.value,
      set: (v) => {
        el.value = v;
        set(v);
        check();
      },
      fields: () => attributeFields(fieldNames(f)),
      objects: () => entityObjects(doc.byLayer(layer()).slice(0, 5000), (id) => doc.layers.get(id)?.name ?? id, { geometry: { evaluateExpression: (...a) => ctx.view.evaluateExpression(...a) } }),
      context: `${NETWORKS_TITLE} · ${label}`,
      fail: (m) => ctx.log.error(m),
    });
    return h('span', { class: 'net-exprbox' }, el, b);
  };

  function renderList(): void {
    replaceChildren(
      list,
      ...(forms.length
        ? forms.map((f, i) => {
            const item = h(
              'button',
              { class: `net-item${i === chosen ? ' is-selected' : ''}`, type: 'button', role: 'option', 'aria-selected': String(i === chosen) },
              h('span', { class: 'net-item__name' }, f.name.trim() || 'Adsız ağ'),
              h('span', { class: 'net-item__meta' }, `${NETWORK_KIND_LABELS[f.kind]}, ${f.edges.length} kenar katmanı`),
            );
            item.addEventListener('click', () => {
              chosen = i;
              checked = null;
              render();
            });
            return item;
          })
        : [h('p', { class: 'net-empty' }, 'Ağ yok. Yeni ağ ile yol ya da şebeke ağı tanımlayın.')]),
    );
    remove.disabled = chosen < 0;
    add.disabled = forms.length >= NETWORK_LIMIT;
  }

  function renderForm(): void {
    const f = forms[chosen];
    if (!f) {
      replaceChildren(form, h('p', { class: 'net-empty' }, 'Soldan bir ağ seçin ya da Yeni ağ ile ekleyin.'));
      return;
    }
    const names = fieldNames(f);
    const listId = 'net-fields';
    const datalist = h('datalist', { id: listId }, names.map((n) => h('option', { value: n.name })));
    const parts: HTMLElement[] = [datalist];
    parts.push(row('Ad', input('Ad', f.name, (v) => ((f.name = v), renderList()))));
    parts.push(
      row(
        'Tür',
        select(
          'Tür',
          f.kind,
          NETWORK_KINDS.map((k) => ({ value: k, text: NETWORK_KIND_LABELS[k] })),
          (v) => {
            forms[chosen] = retyped(f, v as NetworkForm['kind']);
            render();
          },
        ),
      ),
    );
    // Edge layers.
    const addEdge = button('Katman ekle', 'plus', 'btn btn--small');
    addEdge.disabled = f.edges.length >= NETWORK_LAYER_LIMIT;
    addEdge.addEventListener('click', () => {
      f.edges.push({ layer: doc.layers.active.value, filter: '' });
      render();
    });
    parts.push(head('Kenar katmanları', addEdge));
    f.edges.forEach((e, i) =>
      parts.push(
        h(
          'div',
          { class: 'net-line net-line--edge' },
          select('Kenar katmanı', e.layer, layerOptions(e.layer), (v) => ((e.layer = v), render())),
          exprField(`${i + 1}. kenar süzgeci`, e.filter, (v) => (e.filter = v), f, () => e.layer, 'süzgeç (boş: hepsi)'),
          del('Kenar katmanını kaldır', () => (f.edges.splice(i, 1), render())),
        ),
      ),
    );
    if (!f.edges.length) parts.push(h('p', { class: 'net-empty' }, 'Kenar katmanı yok: ağın çizgileri, çoklu çizgileri ve yayları bir katmanda olmalı.'));
    // Junction layers.
    const addJunction = button('Katman ekle', 'plus', 'btn btn--small');
    addJunction.disabled = f.junctions.length >= NETWORK_LAYER_LIMIT;
    addJunction.addEventListener('click', () => {
      f.junctions.push({ layer: '', role: f.kind === 'utility' ? 'valve' : 'junction', filter: '', closed: '' });
      render();
    });
    parts.push(head('Düğüm katmanları', addJunction));
    f.junctions.forEach((j, i) =>
      parts.push(
        h(
          'div',
          { class: 'net-line net-line--junction' },
          select('Düğüm katmanı', j.layer, layerOptions(j.layer), (v) => ((j.layer = v), render())),
          select(
            'Rol',
            j.role,
            JUNCTION_ROLES.map((r) => ({ value: r, text: JUNCTION_ROLE_LABELS[r] })),
            (v) => ((j.role = v as JunctionRole), check()),
          ),
          exprField(`${i + 1}. düğüm süzgeci`, j.filter, (v) => (j.filter = v), f, () => j.layer, 'süzgeç'),
          exprField(`${i + 1}. düğüm kapalı`, j.closed, (v) => (j.closed = v), f, () => j.layer, 'kapalı ifadesi'),
          del('Düğüm katmanını kaldır', () => (f.junctions.splice(i, 1), render())),
        ),
      ),
    );
    if (!f.junctions.length) parts.push(h('p', { class: 'net-empty' }, 'Düğüm katmanı yok: bağlantılar çizgilerin uçlarındadır. Vanalar ve kaynaklar için nokta katmanı ekleyin.'));
    // Connecting.
    parts.push(head('Bağlanma'));
    parts.push(
      row(
        'Uçlar',
        select(
          'Bağlanma',
          f.connect,
          NETWORK_CONNECTS.map((c) => ({ value: c, text: NETWORK_CONNECT_LABELS[c] })),
          (v) => ((f.connect = v as NetworkForm['connect']), check()),
        ),
        h('span', { class: 'net-unit' }, 'Tolerans'),
        input('Tolerans', f.tolerance, (v) => (f.tolerance = v), { cls: 'field num net-num' }),
        h('span', { class: 'net-unit' }, format.lengthUnitLabel),
      ),
    );
    // Direction.
    parts.push(head('Yön'));
    parts.push(
      row(
        'Yön',
        select('Yön', f.direction, DIRECTIONS, (v) => {
          f.direction = v as DirectionKind;
          if (f.direction === 'field' && !f.field && !f.forward && !f.backward && !f.shut) Object.assign(f, DIRECTION_DEFAULTS);
          render();
        }),
      ),
    );
    if (f.direction === 'field') {
      parts.push(row('Alan', input('Yön alanı', f.field, (v) => (f.field = v), { list: listId })));
      parts.push(row('İleri', input('İleri değerleri', f.forward, (v) => (f.forward = v), { placeholder: 'FT, ileri' })));
      parts.push(row('Geri', input('Geri değerleri', f.backward, (v) => (f.backward = v), { placeholder: 'TF, geri' })));
      parts.push(row('Kapalı', input('Kapalı değerleri', f.shut, (v) => (f.shut = v), { placeholder: 'N, kapalı' })));
    }
    // Costs.
    const addCost = button('Maliyet ekle', 'plus', 'btn btn--small');
    addCost.disabled = f.costs.length >= NETWORK_COST_LIMIT;
    addCost.addEventListener('click', () => {
      f.costs.push({ name: f.costs.length ? `Maliyet ${f.costs.length + 1}` : 'Süre', kind: f.costs.length ? 'field' : 'speed', field: '', unit: '', speed: '' });
      render();
    });
    parts.push(head('Maliyetler (Uzunluk her ağda vardır)', addCost));
    f.costs.forEach((c, i) =>
      parts.push(
        h(
          'div',
          { class: 'net-line net-line--cost' },
          input('Maliyetin adı', c.name, (v) => (c.name = v)),
          select(
            'Maliyetin türü',
            c.kind,
            NETWORK_COST_KINDS.map((k) => ({ value: k, text: NETWORK_COST_KIND_LABELS[k] })),
            (v) => ((c.kind = v as NetworkCostKind), render()),
          ),
          input('Alan', c.field, (v) => (c.field = v), { list: listId, placeholder: c.kind === 'speed' ? 'hız alanı' : 'maliyet alanı' }),
          c.kind === 'speed'
            ? h('span', { class: 'net-exprbox' }, input('Varsayılan hız', c.speed, (v) => (c.speed = v), { placeholder: '50', cls: 'field num net-num' }), h('span', { class: 'net-unit' }, 'km/sa'))
            : input('Birim', c.unit, (v) => (c.unit = v), { placeholder: 'birim' }),
          del('Maliyeti kaldır', () => (f.costs.splice(i, 1), render())),
        ),
      ),
    );
    // Closed edges.
    parts.push(head('Kapalı kenarlar'));
    parts.push(row('İfade', exprField('Kapalı kenarlar', f.closed, (v) => (f.closed = v), f, () => f.edges[0]?.layer ?? '', 'ifade (boş: hiçbiri)')));
    // Denetle.
    const run = button('Denetle', 'networks');
    run.addEventListener('click', () => runCheck());
    parts.push(head('Denetle', run));
    parts.push(checkView());
    replaceChildren(form, ...parts);
  }

  /** Denetle's answer: its counts, its parts and its problems, each a row that shows it in the drawing. */
  function checkView(): HTMLElement {
    const f = forms[chosen];
    const box = h('div', { class: 'net-check' });
    if (!checked || !f) {
      box.append(h('p', { class: 'net-empty' }, 'Denetle ağı bu tanımla kurar: düğüm, parça ve uzunluğunu, kopuk parçaları ve sorunları gösterir.'));
      return box;
    }
    if (checked.words) {
      box.append(summaryLine(checked.value ? 'info' : 'error', checked.words));
      if (!checked.value) return box;
    }
    const c = checked.value!;
    const detached = c.parts.slice(1);
    box.append(
      summaryLine(
        c.problems.length ? 'warn' : 'ok',
        `${c.nodes} düğüm, ${c.pieces} parça, ${format.length(c.length)}; ${c.parts.length} bağlı parça${detached.length ? ` (kopuk ${detached.length}: ${format.length(detached.reduce((s, p) => s + p[1], 0))})` : ''}; ${c.deadEnds} çıkmaz uç.`,
      ),
    );
    if (c.counts.length) box.append(h('p', { class: 'net-counts' }, c.counts.map((k) => `${NETWORK_PROBLEM_LABELS[k.kind]}: ${k.count}`).join(', ')));
    if (!c.problems.length) return box;
    const rows = h('div', { class: 'net-problems', role: 'list' });
    for (const p of c.problems.slice(0, SHOWN_PROBLEMS)) {
      const b = h(
        'button',
        { class: 'net-problem', type: 'button', role: 'listitem' },
        h('span', { class: 'net-problem__kind' }, NETWORK_PROBLEM_LABELS[p.kind]),
        h('span', { class: 'net-problem__at' }, format.point({ x: p.x, y: p.y })),
        h('span', { class: 'net-problem__ids' }, problemDetail(p)),
      );
      b.addEventListener('click', () => show(p));
      rows.append(b);
    }
    box.append(rows);
    if (c.problems.length > SHOWN_PROBLEMS) box.append(h('p', { class: 'net-empty' }, `İlk ${SHOWN_PROBLEMS} sorun gösteriliyor; ${c.problems.length - SHOWN_PROBLEMS} sorun daha var.`));
    return box;
  }

  /** A problem's objects and value in words. */
  function problemDetail(p: NetworkProblem): string {
    const ids = p.ids.length ? `${p.ids.length} nesne` : '';
    const value = p.value ?? null;
    if (p.kind === 'nearMiss' && value !== null) return `${ids}; ${format.length(value)} arayla`;
    if (p.kind === 'short' && value !== null) return `${ids}; ${format.length(value)}`;
    if (p.kind === 'detached' && value !== null) return format.length(value);
    if (p.kind === 'unread') {
      const f = forms[chosen];
      const what = p.cost === null || p.cost === undefined ? 'yön' : (f?.costs[p.cost - 1]?.name ?? 'maliyet');
      return `${ids}; ${what} okunamadı`;
    }
    return ids;
  }

  /** A problem's objects selected, the problem marked and the view on it. */
  function show(p: NetworkProblem): void {
    const ids = p.ids.filter((id) => doc.get(id));
    ctx.selection.set(ids);
    ctx.selection.problem.set({ at: { x: p.x, y: p.y }, label: NETWORK_PROBLEM_LABELS[p.kind], regions: [], edges: [] });
    const r = 25;
    ctx.view.zoomToBox({ minX: p.x - r, minY: p.y - r, maxX: p.x + r, maxY: p.y + r }, 96);
  }

  function runCheck(): void {
    const f = forms[chosen];
    if (!f) return;
    const def = defOf(f, (v) => format.toMetres(v));
    if ('problem' in def) {
      checked = { key: '', value: null, words: def.problem };
      return render();
    }
    const key = JSON.stringify(def);
    checked = { key, value: null, words: 'Ağ kuruluyor ve denetleniyor…' };
    render();
    ctx.networks
      .ask<NetworkCheck>(def.id, { kind: 'check' }, def)
      .then(
        ({ value }) => {
          if (checked?.key !== key) return;
          checked = { key, value, words: null };
        },
        (e: Error) => {
          if (checked?.key === key) checked = { key, value: null, words: e.message };
        },
      )
      .finally(() => {
        if (forms[chosen] === f) render();
      });
  }

  /** The networks the window writes, or the first thing wrong. */
  function defs(): NetworkDef[] | string {
    const out: NetworkDef[] = [];
    for (const f of forms) {
      const d = defOf(f, (v) => format.toMetres(v));
      if ('problem' in d) return d.problem;
      for (const x of formExpressions(f)) {
        const r = compileExpression(x.text);
        if (!r.ok) return `“${d.name}” ağının ${x.what.toLocaleLowerCase('tr-TR')}: ${expressionError(r)}`;
      }
      out.push(d);
    }
    const wrong = networksProblem(out);
    return wrong ? `${wrong.charAt(0).toLocaleUpperCase('tr-TR')}${wrong.slice(1)}.` : out;
  }

  const changed = (): boolean => {
    const now = defs();
    if (typeof now === 'string') return true;
    return now.length !== saved.length || now.some((n, i) => !sameNetwork(n, saved[i]));
  };

  function check(): void {
    const now = defs();
    const missing = forms.flatMap((f) => [...f.edges.map((e) => e.layer), ...f.junctions.map((j) => j.layer)]).filter((l) => l && !doc.layers.get(l));
    if (typeof now === 'string') replaceChildren(summary, summaryLine('error', now));
    else if (missing.length) replaceChildren(summary, summaryLine('warn', `Çizimde olmayan katman: ${[...new Set(missing)].join(', ')}; ağ kurulurken atlanır.`));
    else replaceChildren(summary, summaryLine('ok', now.length ? `${now.length} ağ. En kısa yol, Hizmet alanı ve Şebeke izleme bu ağları kullanır.` : 'Ağ yok.'));
    save.disabled = typeof now === 'string' || !changed();
  }

  function render(): void {
    renderList();
    renderForm();
    check();
  }

  /** Kaydet: each network taken away is removed and each new or changed one written, in the list's order. */
  function write(): boolean {
    const now = defs();
    if (typeof now === 'string') return false;
    const cx = { doc };
    const refused = (r: { status: string; error?: { message: string } }, words: string): boolean => {
      ctx.log.warn(r.error?.message ?? words);
      return false;
    };
    for (const old of saved)
      if (!now.some((n) => n.id === old.id)) {
        const r = networkDefine.execute(cx, { operation: 'remove', id: old.id });
        if (r.status !== 'completed') return refused(r as { status: string; error?: { message: string } }, 'Ağ silinemedi.');
      }
    for (const n of now) {
      const old = saved.find((o) => o.id === n.id);
      if (old && sameNetwork(old, n)) continue;
      const r = networkDefine.execute(cx, { operation: 'set', network: n });
      if (r.status !== 'completed') return refused(r as { status: string; error?: { message: string } }, 'Ağ yazılamadı.');
      for (const w of r.warnings) ctx.log.warn(w.message);
    }
    ctx.log.info(`Ağlar kaydedildi: ${now.length} ağ.`);
    return true;
  }

  add.addEventListener('click', () => {
    const ids = forms.map((f) => ({ id: f.id }) as NetworkDef);
    let name = 'Yeni ağ';
    for (let k = 2; forms.some((f) => f.name.trim() === name); k++) name = `Yeni ağ ${k}`;
    const active = doc.layers.active.value;
    forms.push(newForm(nextNetworkId(ids), name, 'road', doc.layers.get(active) ? active : '', (m) => format.fromMetres(m)));
    chosen = forms.length - 1;
    checked = null;
    render();
  });
  remove.addEventListener('click', () => {
    if (chosen < 0) return;
    forms.splice(chosen, 1);
    forms = [...forms];
    chosen = Math.min(chosen, forms.length - 1);
    checked = null;
    render();
  });

  const dialog = new Dialog({
    title: NETWORKS_TITLE,
    width: 1000,
    className: 'dialog--io dialog--networks',
    content: [h('div', { class: 'net-panes' }, h('div', { class: 'net-left' }, list, h('div', { class: 'net-actions' }, add, remove)), form), summary],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
    beforeClose: () => {
      if (done || !changed()) return true;
      void askUnsaved({ name: NETWORKS_TITLE, after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat', canSave: typeof defs() !== 'string' }).then((a) => {
        if (a === 'stay') return;
        if (a === 'save' && !write()) return;
        done = true;
        dialog.close();
      });
      return false;
    },
    onClose: () => d.dispose(),
  });
  d.add(
    listen(save, 'click', () => {
      if (!write()) return;
      done = true;
      dialog.close();
    }),
  );
  d.add(
    listen(cancel, 'click', () => {
      done = true;
      dialog.close();
    }),
  );
  render();
}
