// A municipality's fees and charges (docs/adr/9003): its yearly tariffs (the law's fees at its group's amounts, its
// council's charges), kept as data —
// inputs, rate tables, items — so a new fee is a new item, not a change to the program. A draft is what changes; one
// tariff is in force at a time; next year's is this year's copied and revalued. Its admins write it, its members read it.
import { fees as api } from '../../api/portal';
import type { FeeDefinition, FeeItem, FeeParameter, FeeParameterKind, FeeRule, FeeSchedule, FeeScheduleEdit, FeeTable, FeeTableRow } from '../../api/types';
import { go } from '../../app/router';
import { kindLabel, kinds, orgBySlug } from '../../app/state';
import { type Child, h } from '../../lib/dom';
import { day, decimal, money, moment, parseDecimal } from '../../lib/format';
import { confirm, openDialog } from '../../ui/dialog';
import { checkbox, checked, checkedValues, form, formError, grid, orNull, selectField, textField, textareaField, values } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { badge, button, empty, notice, pageHeader, panel, table, twoLine } from '../../ui/kit';
import { toast } from '../../ui/toast';
import type { PageFn } from '../page';

const STATE: Record<string, { label: string; tone: 'neutral' | 'success' | 'info' }> = {
  draft: { label: 'Taslak', tone: 'info' },
  active: { label: 'Yürürlükte', tone: 'success' },
  retired: { label: 'Geçmiş', tone: 'neutral' },
};

const KIND_LABEL: Record<FeeParameterKind, string> = { number: 'Sayı', choice: 'Seçenek', flag: 'Evet / hayır' };

function stateBadge(s: FeeSchedule): HTMLElement {
  const st = STATE[s.state] ?? STATE.draft;
  return badge(st.tone, st.label);
}

function isAdmin(role: string): boolean {
  return role === 'owner' || role === 'admin';
}

function period(s: FeeSchedule): string {
  return s.validTo ? `${day(s.validFrom)} – ${day(s.validTo)}` : `${day(s.validFrom)}’den`;
}

/** A new draft: its particulars, the rest from 2464's structure with every rate 0. */
function newTariff(tenant: string, base: string, year: number): void {
  const f = form(
    [
      grid(
        textField({ label: 'Adı', name: 'name', required: true, value: `${year} Ücret Tarifesi` }),
        textField({ label: 'Yılı', name: 'year', required: true, value: String(year), type: 'number' }),
        textField({ label: 'Yürürlüğe giriş', name: 'validFrom', required: true, type: 'date', value: `${year}-01-01` }),
        textField({ label: 'Dayanak karar', name: 'decision', hint: 'Meclis kararı ya da grup kararı: tarih ve sayı.' }),
      ),
      notice(
        'info',
        '2464 sayılı Kanun’un yapısıyla başlar.',
        'Bina inşaat harcı (Ek Madde 1, 5, 6), plan ve proje tasdik harcı, yapı kullanma izni harcı (m.80, m.84) ve istisna girdisi hazır gelir. Birim tutarlar 0’dır: harçlarda belediyenizin grubu için belirlenmiş yılın tutarlarını, ücretlerde meclis tarifesini yazın.',
      ),
      formError(),
    ],
    async (el) => {
      const v = values(el);
      const definition = await api.template(tenant);
      const made = await api.create(tenant, {
        name: v.name,
        year: Number(v.year),
        validFrom: v.validFrom,
        validTo: null,
        decision: orNull(v.decision),
        definition,
      });
      dialog.close();
      toast('Taslak tarife açıldı; birim tutarları yazın.');
      go(`${base}/harclar/${made.id}`);
    },
  );
  const dialog = openDialog({
    title: 'Yeni ücret tarifesi',
    body: f,
    footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dialog.close() }), button('Taslağı aç', { kind: 'primary', icon: 'plus', onClick: () => f.requestSubmit() })],
  });
}

export const orgFees: PageFn = async ({ params }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  const list = await api.schedules(org.tenantId);
  const admin = isAdmin(org.role);
  const active = list.find((s) => s.state === 'active');
  const year = new Date().getFullYear() + (new Date().getMonth() >= 9 ? 1 : 0);
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({
      title: 'Harç ve ücretler',
      lead: 'Yılın harç ve ücret tarifesi: harçlar 2464 sayılı Kanun’un ve belediyenizin grubu için belirlenen tutarlardır, ücretler meclis tarifesidir. Girdiler, birim tutar tabloları ve kalemler. Yürürlükteki tarife başvurulara tahakkukta kullanılır; yeni bir harç yeni bir kalemdir.',
      actions: admin ? [button('Yeni tarife', { kind: 'primary', icon: 'plus', onClick: () => newTariff(org.tenantId, base, year) })] : [],
    }),
    active
      ? h('div', { style: 'margin-bottom: 14px' }, notice('success', `Yürürlükte: ${active.name}`, `${period(active)}${active.decision ? ` · ${active.decision}` : ''} · ${active.definition.items.length} kalem`))
      : list.length
        ? h('div', { style: 'margin-bottom: 14px' }, notice('warning', 'Yürürlükte tarife yok.', 'Başvurulara harç tahakkuk edilemez; bir taslağı yürürlüğe koyun.'))
        : null,
    table(
      [
        { label: 'Tarife', cell: (s) => twoLine(s.name, s.decision ?? 'Dayanak karar yazılmadı') },
        { label: 'Yıl', cell: (s) => h('span', { class: 'num' }, String(s.year)), class: 'narrow' },
        { label: 'Yürürlük', cell: (s) => period(s), class: 'narrow' },
        { label: 'Kalem', cell: (s) => h('span', { class: 'num' }, String(s.definition.items.length)), class: 'narrow right' },
        { label: 'Durum', cell: stateBadge, class: 'narrow' },
        { label: 'Son değişiklik', cell: (s) => twoLine(moment(s.updatedAt), s.updatedByName), class: 'narrow' },
      ],
      list,
      {
        href: (s) => `${base}/harclar/${s.id}`,
        empty: empty(
          'receipt',
          'Henüz ücret tarifesi yok',
          'Yılın harç ve ücretlerini buraya yazın; başvurulara tahakkuk bu tarifeyle yapılır.',
          admin ? button('Yeni tarife', { kind: 'primary', icon: 'plus', onClick: () => newTariff(org.tenantId, base, year) }) : undefined,
        ),
      },
    ),
  );
  return { crumbs: [{ label: org.tenantName, href: base }, { label: 'Harç ve ücretler' }], body };
};

// ── One tariff ──────────────────────────────────────────────────────────────

function bracket(r: FeeTableRow, unit: string): string {
  const u = unit ? ` ${unit}` : '';
  if (r.from != null && r.to != null) return `${decimal(r.from)}–${decimal(r.to)}${u}`;
  if (r.from != null) return `${decimal(r.from)}${u} ve üstü`;
  if (r.to != null) return `${decimal(r.to)}${u}’ye kadar`;
  return 'Her miktar';
}

function numberInput(value: number | null, onChange: (v: number | null) => void, label: string, disabled: boolean): HTMLInputElement {
  const input = h('input', { class: 'input num fee-cell', inputmode: 'decimal', value: value == null ? '' : decimal(value), 'aria-label': label, disabled });
  input.addEventListener('change', () => {
    const v = parseDecimal(input.value);
    input.classList.toggle('is-invalid', input.value.trim() !== '' && v == null);
    if (input.value.trim() === '' || v != null) onChange(v);
  });
  return input;
}

function ruleText(def: FeeDefinition, rule: FeeRule): Child {
  const param = (k: string) => def.parameters.find((p) => p.key === k)?.label ?? k;
  switch (rule.kind) {
    case 'fixed':
      return `Maktu ${decimal(rule.amount)} TL`;
    case 'per_unit':
      return rule.table
        ? `${param(rule.quantity)} × ${def.tables.find((t) => t.key === rule.table)?.label ?? rule.table}`
        : `${param(rule.quantity)} × ${decimal(rule.rate ?? 0)} TL`;
    default:
      return h('code', { class: 'fee-code' }, rule.expression);
  }
}

/** A choice list as written in its box: one `value = Label` a line. */
function choicesText(p: FeeParameter): string {
  return p.choices.map((c) => `${c.value} = ${c.label}`).join('\n');
}

function parseChoices(text: string) {
  return text
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l) => {
      const [value, ...rest] = l.split('=');
      const label = rest.join('=').trim();
      return { value: value.trim(), label: label || value.trim() };
    });
}

/** Adds or changes an input. */
function parameterDialog(def: FeeDefinition, at: number | null, done: () => void): void {
  const p = at == null ? null : def.parameters[at];
  const f = form(
    [
      grid(
        textField({ label: 'Adı', name: 'label', required: true, value: p?.label ?? '' }),
        textField({ label: 'Anahtar', name: 'key', required: true, value: p?.key ?? '', hint: 'Formülde @anahtar; küçük harf, rakam ve _.' }),
        selectField({ label: 'Türü', name: 'kind', value: p?.kind ?? 'number', options: Object.entries(KIND_LABEL).map(([value, label]) => ({ value, label })) }),
        textField({ label: 'Birimi', name: 'unit', value: p?.unit ?? '', hint: 'Sayılar için: m², adet, m…' }),
      ),
      textareaField({ label: 'Seçenekler', name: 'choices', value: p ? choicesText(p) : '', rows: 3, hint: 'Seçenek türünde, her satıra bir tane: değer = Etiket (konut = Konut).' }),
      textField({ label: 'Açıklama', name: 'help', value: p?.help ?? '' }),
      checkbox('Zorunlu (girilmezse tahakkuk yapılmaz)', 'required', p?.required ?? true),
      formError(),
    ],
    async (el) => {
      const v = values(el);
      const next: FeeParameter = {
        key: v.key.trim(),
        label: v.label.trim(),
        kind: v.kind as FeeParameterKind,
        unit: orNull(v.unit),
        choices: v.kind === 'choice' ? parseChoices(v.choices ?? '') : [],
        required: checked(el, 'required'),
        help: orNull(v.help),
      };
      if (at == null) def.parameters.push(next);
      else def.parameters[at] = next;
      dialog.close();
      done();
    },
  );
  const dialog = openDialog({
    title: p ? `“${p.label}” girdisi` : 'Yeni girdi',
    body: f,
    footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dialog.close() }), button('Tamam', { kind: 'primary', icon: 'check', onClick: () => f.requestSubmit() })],
  });
}

/** Adds or changes an item. */
function itemDialog(def: FeeDefinition, at: number | null, done: () => void): void {
  const it = at == null ? null : def.items[at];
  const numbers = def.parameters.filter((p) => p.kind === 'number');
  const rule = it?.rule ?? ({ kind: 'per_unit', quantity: numbers[0]?.key ?? '', table: def.tables[0]?.key ?? null, rate: null } as FeeRule);
  const kindSelect = selectField({
    label: 'Hesap',
    name: 'ruleKind',
    value: rule.kind,
    options: [
      { value: 'per_unit', label: 'Birim başına (miktar × birim tutar)' },
      { value: 'fixed', label: 'Maktu (sabit tutar)' },
      { value: 'formula', label: 'Formül' },
    ],
  });
  const perUnit = h(
    'div',
    { class: 'form-grid' },
    selectField({ label: 'Miktar', name: 'quantity', value: rule.kind === 'per_unit' ? rule.quantity : numbers[0]?.key, options: numbers.map((p) => ({ value: p.key, label: p.label })) }),
    selectField({
      label: 'Birim tutar',
      name: 'table',
      value: rule.kind === 'per_unit' ? (rule.table ?? '') : (def.tables[0]?.key ?? ''),
      options: [...def.tables.map((t) => ({ value: t.key, label: t.label })), { value: '', label: 'Sabit birim tutar (aşağıda)' }],
    }),
    textField({ label: 'Sabit birim tutar (TL)', name: 'rate', value: rule.kind === 'per_unit' && rule.rate != null ? decimal(rule.rate) : '' }),
  );
  const fixed = textField({ label: 'Tutar (TL)', name: 'amount', value: rule.kind === 'fixed' ? decimal(rule.amount) : '' });
  const formula = textareaField({
    label: 'Formül',
    name: 'expression',
    value: rule.kind === 'formula' ? rule.expression : '',
    rows: 2,
    hint: 'KentOS ifade dili: @insaat_alani * @bina_insaat_birim, durum eğer @bolge = \'ticaret\' ise 2 yoksa 1 son …',
  });
  const show = () => {
    const k = (kindSelect.querySelector('select') as HTMLSelectElement).value;
    perUnit.hidden = k !== 'per_unit';
    fixed.hidden = k !== 'fixed';
    formula.hidden = k !== 'formula';
  };
  kindSelect.querySelector('select')?.addEventListener('change', show);
  const appKinds = kinds('application');
  const f = form(
    [
      grid(
        textField({ label: 'Adı', name: 'label', required: true, value: it?.label ?? '' }),
        textField({ label: 'Kodu', name: 'code', required: true, value: it?.code ?? '', hint: 'Küçük harf, rakam ve _; tahakkukta satırın kimliği.' }),
        textField({ label: 'Yasal dayanak', name: 'legalBasis', value: it?.legalBasis ?? '', hint: '“2464 sayılı Kanun Ek Madde 1” ya da ücret tarifesinin meclis kararı.' }),
        selectField({ label: 'Grubu', name: 'group', value: it?.group ?? 'Harç', options: [{ value: 'Harç', label: 'Harç' }, { value: 'Ücret', label: 'Ücret' }, { value: 'Vergi', label: 'Vergi' }] }),
      ),
      kindSelect,
      perUnit,
      fixed,
      formula,
      textField({ label: 'Koşul', name: 'condition', value: it?.condition ?? '', hint: 'Boşsa her zaman; örnek: değil @kamu_istisnasi' }),
      grid(
        textField({ label: 'En az (TL)', name: 'minimum', value: it?.minimum != null ? decimal(it.minimum) : '' }),
        textField({ label: 'En çok (TL)', name: 'maximum', value: it?.maximum != null ? decimal(it.maximum) : '' }),
        textField({ label: 'KDV (%)', name: 'vatPercent', value: decimal(it?.vatPercent ?? 0) }),
      ),
      h(
        'div',
        { class: 'field', dataset: { field: 'appliesTo' } },
        h('div', { class: 'field-label' }, 'Dosya türleri'),
        h('div', { class: 'checks' }, ...appKinds.map((k) => checkbox(k.label, 'appliesTo', it?.appliesTo.includes(k.code) ?? false, k.code))),
        h('div', { class: 'field-hint' }, 'Hiçbiri seçilmezse her başvuruya uygulanır.'),
      ),
      checkbox('Karardan önce ödenir (ödenmeden başvuru onaylanmaz; 2464 Ek Madde 5)', 'beforeDecision', it?.beforeDecision ?? false),
      formError(),
    ],
    async (el) => {
      const v = values(el);
      const num = (field: string, text: string | undefined): number | null => {
        if (!text?.trim()) return null;
        const n = parseDecimal(text);
        if (n == null) throw Object.assign(new Error('Bir sayı yazın.'), { path: field });
        return n;
      };
      let next: FeeRule;
      if (v.ruleKind === 'fixed') next = { kind: 'fixed', amount: num('amount', v.amount) ?? 0 };
      else if (v.ruleKind === 'formula') next = { kind: 'formula', expression: v.expression ?? '' };
      else next = { kind: 'per_unit', quantity: v.quantity, table: v.table || null, rate: v.table ? null : num('rate', v.rate) };
      const item: FeeItem = {
        code: v.code.trim(),
        label: v.label.trim(),
        legalBasis: v.legalBasis ?? '',
        group: v.group,
        rule: next,
        appliesTo: checkedValues(el, 'appliesTo'),
        condition: orNull(v.condition),
        minimum: num('minimum', v.minimum),
        maximum: num('maximum', v.maximum),
        vatPercent: num('vatPercent', v.vatPercent) ?? 0,
        beforeDecision: checked(el, 'beforeDecision'),
      };
      if (at == null) def.items.push(item);
      else def.items[at] = item;
      dialog.close();
      done();
    },
  );
  show();
  const dialog = openDialog({
    title: it ? `“${it.label}”` : 'Yeni kalem',
    wide: true,
    body: f,
    footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dialog.close() }), button('Tamam', { kind: 'primary', icon: 'check', onClick: () => f.requestSubmit() })],
  });
}

/** Adds a rate table. */
function tableDialog(def: FeeDefinition, done: () => void): void {
  const choices = def.parameters.filter((p) => p.kind === 'choice');
  const numbers = def.parameters.filter((p) => p.kind === 'number');
  const f = form(
    [
      grid(
        textField({ label: 'Adı', name: 'label', required: true }),
        textField({ label: 'Anahtar', name: 'key', required: true, hint: 'Formülde @anahtar.' }),
        selectField({ label: 'Seçeneğe göre', name: 'by', options: [{ value: '', label: 'Yok' }, ...choices.map((p) => ({ value: p.key, label: p.label }))] }),
        selectField({ label: 'Miktar dilimine göre', name: 'quantity', options: [{ value: '', label: 'Yok' }, ...numbers.map((p) => ({ value: p.key, label: p.label }))] }),
      ),
      formError(),
    ],
    async (el) => {
      const v = values(el);
      const by = v.by || null;
      const first = by ? (def.parameters.find((p) => p.key === by)?.choices ?? []) : [];
      const t: FeeTable = {
        key: v.key.trim(),
        label: v.label.trim(),
        by,
        quantity: v.quantity || null,
        rows: (first.length ? first.map((c) => c.value) : [null]).map((when) => ({ when, from: null, to: null, rate: 0, minRate: null, maxRate: null })),
      };
      def.tables.push(t);
      dialog.close();
      done();
    },
  );
  const dialog = openDialog({
    title: 'Yeni birim tutar tablosu',
    body: f,
    footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dialog.close() }), button('Tamam', { kind: 'primary', icon: 'check', onClick: () => f.requestSubmit() })],
  });
}

export const orgFeeSchedule: PageFn = async ({ params, reload }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  const s = await api.schedule(org.tenantId, params.id);
  const admin = isAdmin(org.role);
  const editable = admin && s.state === 'draft';
  const def: FeeDefinition = structuredClone(s.definition);
  const status = h('span', { class: 'muted' });
  const touched = () => {
    status.textContent = 'Kaydedilmemiş değişiklik var.';
    void tryNow();
  };

  // The particulars.
  const head = form(
    [
      grid(
        textField({ label: 'Adı', name: 'name', required: true, value: s.name }),
        textField({ label: 'Yılı', name: 'year', required: true, type: 'number', value: String(s.year) }),
        textField({ label: 'Yürürlüğe giriş', name: 'validFrom', type: 'date', required: true, value: s.validFrom }),
        textField({ label: 'Bitiş', name: 'validTo', type: 'date', value: s.validTo ?? '', hint: 'Boşsa yerini başka tarife alana kadar.' }),
        textField({ label: 'Dayanak karar', name: 'decision', value: s.decision ?? '' }),
        selectField({
          label: 'Yuvarlama',
          name: 'rounding',
          value: def.rounding,
          options: [
            { value: 'kurus', label: 'Kuruşa (yarım kuruş yukarı)' },
            { value: 'lira', label: 'Liraya (yarım lira yukarı)' },
            { value: 'lira_down', label: 'Liraya, aşağı' },
          ],
        }),
      ),
      textareaField({ label: 'Ödeme notu', name: 'paymentNote', value: def.paymentNote ?? '', rows: 2, hint: 'Tahakkukta meslek mensubuna görünür: nereye, nasıl ödenir.' }),
      formError(),
    ],
    async () => save(),
  );
  head.querySelectorAll('input,select,textarea').forEach((el) => {
    (el as HTMLInputElement).disabled = !editable;
    el.addEventListener('change', () => (status.textContent = 'Kaydedilmemiş değişiklik var.'));
  });

  // The inputs.
  const paramsBox = h('div');
  const drawParams = () =>
    paramsBox.replaceChildren(
      table(
        [
          { label: 'Girdi', cell: (p) => twoLine(p.label, h('code', { class: 'fee-code' }, `@${p.key}`)) },
          { label: 'Türü', cell: (p) => `${KIND_LABEL[p.kind]}${p.unit ? `, ${p.unit}` : ''}`, class: 'narrow' },
          { label: 'Seçenekler', cell: (p) => (p.choices.length ? p.choices.map((c) => c.label).join(', ') : '—') },
          { label: 'Zorunlu', cell: (p) => (p.required ? 'Evet' : 'Hayır'), class: 'narrow' },
          {
            label: '',
            class: 'narrow',
            cell: (p) =>
              editable
                ? h(
                    'div',
                    { class: 'row-actions' },
                    button('Düzenle', { kind: 'ghost', onClick: () => parameterDialog(def, def.parameters.indexOf(p), () => (drawAll(), touched())) }),
                    button('Kaldır', { kind: 'ghost', icon: 'trash', onClick: () => (def.parameters.splice(def.parameters.indexOf(p), 1), drawAll(), touched()) }),
                  )
                : null,
          },
        ],
        def.parameters,
        { empty: empty('list', 'Girdi yok', 'Tahakkukta sorulacak bilgiler: inşaat alanı, yapının kullanımı, bölge…') },
      ),
    );

  // The rate tables.
  const tablesBox = h('div', { class: 'stack' });
  const drawTables = () =>
    tablesBox.replaceChildren(
      ...def.tables.map((t) => {
        const by = def.parameters.find((p) => p.key === t.by);
        const qty = def.parameters.find((p) => p.key === t.quantity);
        const unit = qty?.unit ?? '';
        const choiceLabel = (v: string | null) => (v == null ? 'Hepsi' : (by?.choices.find((c) => c.value === v)?.label ?? v));
        const rowsTable = table(
          [
            ...(by
              ? [
                  {
                    label: by.label,
                    cell: (r: FeeTableRow) =>
                      editable
                        ? (() => {
                            const sel = h('select', { class: 'input fee-cell', 'aria-label': by.label }, ...by.choices.map((c) => h('option', { value: c.value, selected: c.value === r.when }, c.label)));
                            sel.addEventListener('change', () => ((r.when = sel.value), touched()));
                            return sel;
                          })()
                        : choiceLabel(r.when),
                  },
                ]
              : []),
            ...(qty
              ? [
                  {
                    label: `${qty.label} dilimi`,
                    cell: (r: FeeTableRow) =>
                      editable
                        ? h(
                            'div',
                            { class: 'fee-range' },
                            numberInput(r.from, (v) => ((r.from = v), touched()), 'Dilimin başı', false),
                            h('span', { class: 'muted' }, '–'),
                            numberInput(r.to, (v) => ((r.to = v), touched()), 'Dilimin sonu', false),
                            unit ? h('span', { class: 'muted' }, unit) : null,
                          )
                        : bracket(r, unit),
                  },
                ]
              : []),
            {
              label: unit ? `Birim tutar (TL / ${unit})` : 'Birim tutar (TL)',
              class: editable ? undefined : 'right',
              cell: (r: FeeTableRow) => (editable ? numberInput(r.rate, (v) => ((r.rate = v ?? 0), touched()), 'Birim tutar', false) : h('span', { class: 'num' }, decimal(r.rate))),
            },
            {
              label: 'En az – en çok (TL)',
              class: editable ? undefined : 'right',
              cell: (r: FeeTableRow) =>
                editable
                  ? h(
                      'div',
                      { class: 'fee-range' },
                      numberInput(r.minRate, (v) => ((r.minRate = v), touched()), 'En az', false),
                      h('span', { class: 'muted' }, '–'),
                      numberInput(r.maxRate, (v) => ((r.maxRate = v), touched()), 'En çok', false),
                    )
                  : r.minRate != null || r.maxRate != null
                    ? `${r.minRate != null ? decimal(r.minRate) : '…'} – ${r.maxRate != null ? decimal(r.maxRate) : '…'}`
                    : '—',
            },
            {
              label: '',
              class: 'narrow',
              cell: (r: FeeTableRow) =>
                editable ? button('Satırı kaldır', { kind: 'ghost', icon: 'trash', onClick: () => (t.rows.splice(t.rows.indexOf(r), 1), drawTables(), touched()) }) : null,
            },
          ],
          t.rows,
        );
        return h(
          'div',
          { class: 'fee-table' },
          h(
            'div',
            { class: 'fee-table-head' },
            h('div', null, h('strong', null, t.label), ' ', h('code', { class: 'fee-code' }, `@${t.key}`)),
            h('span', { class: 'muted' }, by || qty ? `Satırlar: ${[by?.label, qty ? `${qty.label} dilimi` : null].filter(Boolean).join(' ve ')}` : 'Tek tutar'),
            editable
              ? h(
                  'span',
                  { class: 'row-actions' },
                  button('Satır ekle', { kind: 'ghost', icon: 'plus', onClick: () => (t.rows.push({ when: t.rows.at(-1)?.when ?? null, from: null, to: null, rate: 0, minRate: null, maxRate: null }), drawTables(), touched()) }),
                  button('Tabloyu kaldır', { kind: 'ghost', icon: 'trash', onClick: () => (def.tables.splice(def.tables.indexOf(t), 1), drawAll(), touched()) }),
                )
              : null,
          ),
          rowsTable,
        );
      }),
      ...(def.tables.length ? [] : [empty('grid', 'Tablo yok', 'Dilimli ya da seçeneğe göre değişen birim tutarlar tablolarda durur.')]),
    );

  // The items.
  const itemsBox = h('div');
  const appKindLabel = (code: string) => kindLabel('application', code);
  const drawItems = () =>
    itemsBox.replaceChildren(
      table(
        [
          { label: 'Kalem', cell: (i) => twoLine(i.label, i.legalBasis || i.group) },
          { label: 'Hesap', cell: (i) => ruleText(def, i.rule) },
          { label: 'Dosya türleri', cell: (i) => (i.appliesTo.length ? i.appliesTo.map(appKindLabel).join(', ') : 'Her başvuru') },
          { label: 'Koşul', cell: (i) => (i.condition ? h('code', { class: 'fee-code' }, i.condition) : '—') },
          {
            label: 'Sınır / KDV',
            class: 'narrow',
            cell: (i) => [i.minimum != null ? `en az ${decimal(i.minimum)}` : null, i.maximum != null ? `en çok ${decimal(i.maximum)}` : null, i.vatPercent ? `KDV %${decimal(i.vatPercent)}` : null].filter(Boolean).join(' · ') || '—',
          },
          { label: '', class: 'narrow', cell: (i) => (i.beforeDecision ? badge('warning', 'Karardan önce') : null) },
          {
            label: '',
            class: 'narrow',
            cell: (i) =>
              editable
                ? h(
                    'div',
                    { class: 'row-actions' },
                    button('Düzenle', { kind: 'ghost', onClick: () => itemDialog(def, def.items.indexOf(i), () => (drawItems(), touched())) }),
                    button('Kaldır', { kind: 'ghost', icon: 'trash', onClick: () => (def.items.splice(def.items.indexOf(i), 1), drawItems(), touched()) }),
                  )
                : null,
          },
        ],
        def.items,
        { empty: empty('receipt', 'Kalem yok', 'Harç ve ücret kalemlerini ekleyin.') },
      ),
    );

  // Trying it: what the tariff as it is on the page makes of a kind's inputs.
  const appKinds = kinds('application');
  const tryKind = h('select', { class: 'input', style: 'max-width: 340px', 'aria-label': 'Dosya türü' }, ...appKinds.map((k) => h('option', { value: k.code, selected: k.code === 'building_permit' }, k.label)));
  const tryInputs = h('div', { class: 'form-grid' });
  const tryOut = h('div', { class: 'stack' });
  const given = new Map<string, string | boolean>([
    ['yapi_turu', 'konut'],
    ['insaat_alani', '152,5'],
    ['bolge', 'konut'],
  ]);
  const drawTryInputs = () =>
    tryInputs.replaceChildren(
      ...def.parameters.map((p) => {
        const id = `try-${p.key}`;
        let control: HTMLElement;
        if (p.kind === 'choice') {
          const sel = h('select', { class: 'input', id }, h('option', { value: '' }, 'Seçin'), ...p.choices.map((c) => h('option', { value: c.value, selected: given.get(p.key) === c.value }, c.label)));
          sel.addEventListener('change', () => (given.set(p.key, sel.value), void tryNow()));
          control = sel;
        } else if (p.kind === 'flag') {
          const box = h('input', { type: 'checkbox', id, checked: given.get(p.key) === true });
          box.addEventListener('change', () => (given.set(p.key, box.checked), void tryNow()));
          control = h('label', { class: 'check' }, box, ' Evet');
        } else {
          const input = h('input', { class: 'input num', id, inputmode: 'decimal', value: String(given.get(p.key) ?? '') });
          input.addEventListener('input', () => (given.set(p.key, input.value), void tryNow()));
          control = p.unit ? h('div', { class: 'fee-range' }, input, h('span', { class: 'muted' }, p.unit)) : input;
        }
        return h('div', { class: 'field' }, h('label', { class: 'field-label', for: id }, p.label), control);
      }),
    );
  let seq = 0;
  const tryNow = async () => {
    const mine = ++seq;
    const inputs: Record<string, number | string | boolean> = {};
    for (const p of def.parameters) {
      const v = given.get(p.key);
      if (v == null || v === '') continue;
      if (p.kind === 'number') {
        const n = parseDecimal(String(v));
        if (n != null) inputs[p.key] = n;
      } else inputs[p.key] = v;
    }
    try {
      const r = await api.tryOut(org.tenantId, { definition: def, kind: tryKind.value, inputs });
      if (mine !== seq) return;
      tryOut.replaceChildren(linesTable(r.lines, r.total), ...Object.values(r.missing).map((m) => h('div', { class: 'field-hint' }, icon('alert', 14), ' ', m)));
    } catch (e) {
      if (mine !== seq) return;
      tryOut.replaceChildren(notice('warning', 'Tarife bu hâliyle uygulanamıyor.', e instanceof Error ? e.message : String(e)));
    }
  };
  tryKind.addEventListener('change', () => void tryNow());

  const drawAll = () => {
    drawParams();
    drawTables();
    drawItems();
    drawTryInputs();
  };
  drawAll();
  void tryNow();

  const collect = (): FeeScheduleEdit => {
    const v = values(head);
    def.rounding = v.rounding as FeeDefinition['rounding'];
    def.paymentNote = orNull(v.paymentNote);
    return { name: v.name, year: Number(v.year), validFrom: v.validFrom, validTo: orNull(v.validTo), decision: orNull(v.decision), definition: def };
  };
  const save = async () => {
    await api.update(org.tenantId, s.id, collect());
    toast('Tarife kaydedildi.');
    reload();
  };
  const saveClick = async () => {
    try {
      await save();
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), 'danger');
    }
  };
  const activate = async () => {
    const ok = await confirm({
      title: 'Tarife yürürlüğe girsin mi?',
      text: 'Bundan sonraki tahakkuklar bu tarifeyle yapılır; yürürlükteki tarife geçmişe düşer. Yapılmış tahakkuklar değişmez.',
      confirm: 'Yürürlüğe koy',
    });
    if (!ok) return;
    try {
      await api.update(org.tenantId, s.id, collect());
      await api.activate(org.tenantId, s.id);
      toast('Tarife yürürlükte.');
      reload();
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), 'danger');
    }
  };
  const remove = async () => {
    if (!(await confirm({ title: 'Taslak silinsin mi?', text: `“${s.name}” silinir.`, confirm: 'Sil', danger: true }))) return;
    await api.remove(org.tenantId, s.id);
    toast('Taslak silindi.');
    go(`${base}/harclar`);
  };
  const copy = () => {
    const f = form(
      [
        grid(
          textField({ label: 'Yılı', name: 'year', type: 'number', required: true, value: String(s.year + 1) }),
          textField({ label: 'Yürürlüğe giriş', name: 'validFrom', type: 'date', required: true, value: `${s.year + 1}-01-01` }),
          textField({ label: 'Yeniden değerleme katsayısı', name: 'factor', required: true, value: '1,0', hint: 'Bütün tutarlar bununla çarpılır; yüzde 25 artış için 1,25. Formüllerin içindeki sayılar çarpılmaz.' }),
        ),
        formError(),
      ],
      async (el) => {
        const v = values(el);
        const factor = parseDecimal(v.factor);
        if (factor == null) throw Object.assign(new Error('Katsayıyı sayı olarak yazın.'), { path: 'factor' });
        const made = await api.copy(org.tenantId, s.id, { year: Number(v.year), validFrom: v.validFrom, factor });
        dialog.close();
        toast('Yeni yılın taslağı açıldı.');
        go(`${base}/harclar/${made.id}`);
      },
    );
    const dialog = openDialog({
      title: 'Yeni yıla kopyala',
      lead: 'Tarifenin bütün tutarları katsayıyla çarpılıp kuruşa yuvarlanır; yeni tarife taslak olarak açılır.',
      body: f,
      footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dialog.close() }), button('Kopyala', { kind: 'primary', icon: 'copy', onClick: () => f.requestSubmit() })],
    });
  };

  const json = h('textarea', { class: 'input fee-json', rows: 16, spellcheck: false, 'aria-label': 'Tarifenin JSON metni', readonly: !editable });
  json.value = JSON.stringify(def, null, 2);
  const applyJson = async () => {
    try {
      const parsed = JSON.parse(json.value) as FeeDefinition;
      await api.update(org.tenantId, s.id, { ...collect(), definition: parsed });
      toast('Tarife JSON metninden kaydedildi.');
      reload();
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), 'danger');
    }
  };

  const actions: Child[] = [];
  if (editable) {
    actions.push(button('Sil', { kind: 'ghost', icon: 'trash', onClick: () => void remove() }));
    actions.push(button('Kaydet', { kind: 'secondary', icon: 'check', onClick: () => void saveClick() }));
    actions.push(button('Yürürlüğe koy', { kind: 'primary', icon: 'stamp', onClick: () => void activate() }));
  } else if (admin) {
    actions.push(button('Yeni yıla kopyala', { icon: 'copy', onClick: copy }));
  }
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({
      title: s.name,
      back: { label: 'Harç ve ücretler', href: `${base}/harclar` },
      lead: h('span', { class: 'party' }, stateBadge(s), h('span', null, `${period(s)} · son değişiklik ${moment(s.updatedAt)}, ${s.updatedByName}`)),
      actions: [status, ...actions],
    }),
    editable ? null : h('div', { style: 'margin-bottom: 14px' }, notice('info', s.state === 'active' ? 'Yürürlükteki tarife değişmez.' : 'Geçmiş tarife değişmez.', admin ? 'Değiştirmek için yeni yıla kopyalayın; kopya taslak olarak açılır.' : null)),
    panel('Künye', head),
    panel('Girdiler', paramsBox, { note: 'Tahakkukta sorulanlar', actions: editable ? [button('Girdi ekle', { kind: 'ghost', icon: 'plus', onClick: () => parameterDialog(def, null, () => (drawAll(), touched())) })] : [] }),
    panel('Birim tutar tabloları', tablesBox, { note: 'Yılın tutarları', actions: editable ? [button('Tablo ekle', { kind: 'ghost', icon: 'plus', onClick: () => tableDialog(def, () => (drawAll(), touched())) })] : [] }),
    panel('Kalemler', itemsBox, { note: `${def.items.length} kalem`, actions: editable ? [button('Kalem ekle', { kind: 'ghost', icon: 'plus', onClick: () => itemDialog(def, null, () => (drawItems(), touched())) })] : [] }),
    panel('Dene', h('div', { class: 'stack' }, h('div', { class: 'toolbar' }, tryKind, h('span', { class: 'muted' }, 'Sayfadaki hâliyle hesaplanır; kaydedilmez.')), tryInputs, tryOut)),
    h(
      'details',
      { class: 'panel fee-advanced' },
      h('summary', null, 'Gelişmiş: tarifenin JSON metni'),
      h('div', { class: 'panel-body stack' }, h('div', { class: 'field-hint' }, 'Tarifenin tamamı tek bir belge; API ile de yazılır (docs/api). Kaydederken sunucu doğrular ve yanlışın yerini söyler.'), json, editable ? h('div', null, button('JSON metnini kaydet', { kind: 'secondary', onClick: () => void applyJson() })) : null),
    ),
  );
  return { crumbs: [{ label: org.tenantName, href: base }, { label: 'Harç ve ücretler', href: `${base}/harclar` }, { label: s.name }], body };
};

/** An assessment's lines with its total, as the dossier and the try panel show them. */
export function linesTable(lines: { label: string; legalBasis: string; basis: string; amount: number; vat: number; beforeDecision: boolean }[], total: number): HTMLElement {
  const vat = lines.some((l) => l.vat > 0);
  return h(
    'div',
    { class: 'table-wrap' },
    h(
      'table',
      { class: 'table fee-lines' },
      h('thead', null, h('tr', null, h('th', null, 'Kalem'), h('th', null, 'Hesap'), vat ? h('th', { class: 'right' }, 'KDV') : null, h('th', { class: 'right' }, 'Tutar'))),
      h(
        'tbody',
        null,
        ...lines.map((l) =>
          h(
            'tr',
            null,
            h('td', null, twoLine(h('span', null, l.label, l.beforeDecision ? h('span', { class: 'fee-flag' }, ' · karardan önce') : null), l.legalBasis)),
            h('td', null, l.basis),
            vat ? h('td', { class: 'right num' }, l.vat ? money(l.vat) : '—') : null,
            h('td', { class: 'right num' }, money(l.amount)),
          ),
        ),
        lines.length ? null : h('tr', null, h('td', { colspan: vat ? 4 : 3, class: 'muted' }, 'Bu girdilerle uygulanan kalem yok.')),
      ),
      h('tfoot', null, h('tr', null, h('th', { colspan: vat ? 3 : 2 }, 'Toplam'), h('th', { class: 'right num' }, money(total)))),
    ),
  );
}
