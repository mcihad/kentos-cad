import type { LabelStyle } from '../../contracts/generated/LabelStyle';
import { h, type Child } from '../dom';
import { segmented, type Option } from '../widgets/controls';

/**
 * The Etiketler window's style form (docs/adr/0212 §4; the desktop's `labelling/form.rs`): one label style's fields
 * in five tabs, Metin, Yerleşim, Biçim, Sığdırma and Öncelik. Each control shows the style's value (absent: the
 * engine's default, said in its placeholder) and writes a patch; an emptied number takes its field away.
 */

export type LabelTab = 'text' | 'place' | 'look' | 'fit' | 'order';

export const LABEL_TABS: readonly [LabelTab, string][] = [
  ['text', 'Metin'],
  ['place', 'Yerleşim'],
  ['look', 'Biçim'],
  ['fit', 'Sığdırma'],
  ['order', 'Öncelik'],
];

/** A patch of a style: a field set, or taken away (undefined). */
export type StylePatch = { [K in keyof LabelStyle]?: LabelStyle[K] | undefined };

/** What a tab's controls need: the style, how to change it and the ε button for an expression field. */
export interface FormHost {
  readonly style: LabelStyle;
  readonly set: (patch: StylePatch) => void;
  readonly expression: (get: () => string, put: (v: string) => void) => HTMLElement;
}

/** A row: its name, its control and a hint under it. */
function row(label: string, control: Child, hint?: string): HTMLElement {
  return h(
    'label',
    { class: 'lbl-row' },
    h('span', { class: 'lbl-row__name' }, label),
    h('span', { class: 'lbl-row__control' }, control),
    hint ? h('span', { class: 'lbl-row__hint' }, hint) : null,
  );
}

/** A number that may be absent: empty takes the field away; one out of [min, max] is marked and not written. */
function optionalNumber(label: string, value: number | undefined, onChange: (v: number | undefined) => void, o: { min: number; max: number; unit?: string; placeholder?: string }): HTMLElement {
  const input = h('input', {
    class: 'field lbl-num num',
    value: value === undefined ? '' : String(value),
    inputmode: 'decimal',
    spellcheck: 'false',
    placeholder: o.placeholder ?? '',
    'aria-label': label,
  }) as HTMLInputElement;
  const commit = () => {
    const t = input.value.trim().replace(',', '.');
    if (!t) return void (input.classList.remove('is-invalid'), value !== undefined && onChange(undefined));
    const v = Number(t);
    if (!Number.isFinite(v) || v < o.min || v > o.max) return void input.classList.add('is-invalid');
    input.classList.remove('is-invalid');
    if (v !== value) onChange(v);
  };
  input.addEventListener('change', commit);
  input.addEventListener('keydown', (e) => e.key === 'Enter' && commit());
  return h('span', { class: 'lbl-numbox' }, input, o.unit ? h('span', { class: 'lbl-unit' }, o.unit) : null);
}

function check(label: string, checked: boolean, onChange: (v: boolean) => void): HTMLElement {
  const box = h('input', { type: 'checkbox', checked, 'aria-label': label }) as HTMLInputElement;
  box.addEventListener('change', () => onChange(box.checked));
  return h('label', { class: 'lbl-check' }, box, h('span', null, label));
}

function choice<T extends string>(label: string, options: Option<T>[], value: T, onChange: (v: T) => void): HTMLElement {
  return segmented({ label, options, value, onChange });
}

const THEME: readonly [string, string][] = [
  ['', 'Etiket rengi'],
  ['fg', 'Ana yazı'],
  ['fg-dim', 'Soluk'],
  ['paper', 'Kâğıt'],
  ['ink', 'Mürekkep'],
];

/** A colour: a theme's or a custom one (`#rrggbb`); empty, the default `none` names. */
function colour(label: string, value: string | undefined, onChange: (v: string | undefined) => void, none = 'Etiket rengi'): HTMLElement {
  const sel = h('select', { class: 'field lbl-select', 'aria-label': label }) as HTMLSelectElement;
  const custom = value !== undefined && value.startsWith('#');
  for (const [v, name] of THEME) sel.append(h('option', { value: v, selected: !custom && (value ?? '') === v }, v ? name : none));
  sel.append(h('option', { value: '#', selected: custom }, 'Özel…'));
  const picker = h('input', { type: 'color', class: 'lbl-colour', value: custom ? value!.slice(0, 7) : '#e5484d', hidden: !custom, 'aria-label': `${label}: özel renk` }) as HTMLInputElement;
  sel.addEventListener('change', () => {
    if (sel.value === '#') {
      picker.hidden = false;
      onChange(picker.value.toUpperCase());
    } else onChange(sel.value || undefined);
  });
  picker.addEventListener('input', () => onChange(picker.value.toUpperCase()));
  return h('span', { class: 'lbl-colourbox' }, sel, picker);
}

function textField(label: string, value: string | undefined, onChange: (v: string | undefined) => void, placeholder = ''): HTMLInputElement {
  const input = h('input', { class: 'field lbl-text', value: value ?? '', spellcheck: 'false', placeholder, 'aria-label': label }) as HTMLInputElement;
  input.addEventListener('change', () => onChange(input.value.trim() ? input.value : undefined));
  return input;
}

/** Metin: the text (a template or an expression), its size, weight, slant, colour and alignment. */
function textTab(f: FormHost): HTMLElement[] {
  const s = f.style;
  const byExpression = s.text !== undefined;
  const expr = textField('İfade', s.text, (v) => f.set({ text: v ?? '' }), "Ada || '/' || Parsel");
  const out = [
    row(
      'Kaynak',
      choice('Metnin kaynağı', [{ value: 'template', label: 'Şablon' }, { value: 'expression', label: 'İfade' }], byExpression ? 'expression' : 'template', (v) =>
        f.set(v === 'expression' ? { text: s.text ?? '$etiket' } : { text: undefined }),
      ),
    ),
    byExpression
      ? row('İfade', h('span', { class: 'lbl-exprbox' }, expr, f.expression(() => expr.value, (v) => f.set({ text: v }))), 'İfadeyle seç’in dili; $etiket nesnenin etiketi. $sıra ve $ölçek kullanılamaz.')
      : row('Şablon', textField('Şablon', s.template, (v) => f.set({ template: v }), '{label}'), '{label}: nesnenin etiketi; {Ad} gibi öznitelikler de yazılır.'),
    row('Boy', optionalNumber('Boy', s.size, (v) => f.set({ size: v ?? 10 }), { min: 1, max: 200, unit: 'px' })),
    row('Büyüme', optionalNumber('Büyüme', s.grow, (v) => f.set({ grow: v }), { min: 0, max: 1000, placeholder: '0' }), 'Yakınlaştıkça her px/m için boya eklenen.'),
    row('En büyük boy', optionalNumber('En büyük boy', s.maxSize, (v) => f.set({ maxSize: v }), { min: 1, max: 200, unit: 'px', placeholder: 'sınırsız' })),
    row(
      'Yazı',
      h(
        'span',
        { class: 'lbl-inline' },
        check('Kalın', (s.weight ?? 500) >= 600, (b) => f.set({ weight: b ? 600 : undefined })),
        check('Eğik', s.italic === true, (b) => f.set({ italic: b || undefined })),
      ),
    ),
    row('Renk', colour('Renk', s.color, (v) => f.set({ color: v }))),
    row(
      'Hiza',
      choice('Çok satırda hiza', [{ value: 'left', label: 'Sol' }, { value: 'center', label: 'Orta' }, { value: 'right', label: 'Sağ' }], s.align ?? 'center', (v) =>
        f.set({ align: v === 'center' ? undefined : v }),
      ),
      'Birden çok satırlı (yığılmış) etiketin satırları.',
    ),
  ];
  return out;
}

/** The old placement's modes when the style names none (docs/adr/0212 §3.3). */
function modes(s: LabelStyle): { point: string; line: string; area: string } {
  const old: Record<string, [string, string, string]> = {
    center: ['center', 'horizontal', 'horizontal'],
    corner: ['corner', 'horizontal', 'corner'],
    beside: ['around', 'horizontal', 'horizontal'],
    along: ['around', 'parallel', 'perimeter'],
  };
  const [p, l, a] = old[s.placement] ?? old.center;
  return { point: s.point ?? (p === 'corner' ? 'center' : p), line: s.line ?? l, area: s.area ?? a };
}

/** Yerleşim: where points', lines' and areas' labels go, their side, distance, repetition and the line's bends. */
function placeTab(f: FormHost): HTMLElement[] {
  const s = f.style;
  const m = modes(s);
  return [
    row('Nokta', choice('Noktanın etiketi', [{ value: 'around', label: 'Çevresinde' }, { value: 'center', label: 'Üstünde' }], m.point as 'around' | 'center', (v) => f.set({ point: v })), 'Çevresinde: sağ üst, sol üst … sekiz yerden boş olan.'),
    row(
      'Çizgi',
      choice(
        'Çizginin etiketi',
        [
          { value: 'parallel', label: 'Paralel' },
          { value: 'curved', label: 'Kıvrık' },
          { value: 'horizontal', label: 'Yatay' },
          { value: 'contour', label: 'Eş yükselti' },
        ],
        m.line as 'parallel' | 'curved' | 'horizontal' | 'contour',
        (v) => f.set({ line: v }),
      ),
      'Eş yükselti: kıvrık, üstü yokuş yukarı, kot çoklu çizginin ilk kotu.',
    ),
    row(
      'Alan',
      choice(
        'Alanın etiketi',
        [
          { value: 'horizontal', label: 'Yatay' },
          { value: 'free', label: 'Eğik' },
          { value: 'perimeter', label: 'Çevre' },
          { value: 'boundary', label: 'Sınır' },
          { value: 'parcel', label: 'Parsel' },
          { value: 'corner', label: 'Köşe' },
        ],
        m.area as 'horizontal' | 'free' | 'perimeter' | 'boundary' | 'parcel' | 'corner',
        (v) => f.set({ area: v }),
      ),
      'Parsel: yalnız içine; sığmazsa yığar, kısaltır, küçültür. Sınır: kenar boyunca içeride.',
    ),
    row(
      'Konum',
      choice(
        'Çizgiye göre konum',
        [
          { value: 'on', label: 'Üstünde' },
          { value: 'above', label: 'Üstte' },
          { value: 'below', label: 'Altta' },
          { value: 'sides', label: 'İki yanda' },
        ],
        s.position ?? (m.area === 'boundary' ? 'above' : 'on'),
        (v) => f.set({ position: v }),
      ),
      'Alanın çevresinde üstte içeri, altta dışarı demektir.',
    ),
    row('Uzaklık', optionalNumber('Uzaklık', s.distance, (v) => f.set({ distance: v }), { min: 0, max: 500, unit: 'px', placeholder: '2' })),
    row('Yineleme', optionalNumber('Yineleme', s.repeat, (v) => f.set({ repeat: v }), { min: 20, max: 100_000, unit: 'px', placeholder: 'yok' }), 'Uzun çizgide her bu kadar pikselde bir etiket.'),
    row('Harf açısı', optionalNumber('Harfler arası en büyük açı', s.maxAngle, (v) => f.set({ maxAngle: v }), { min: 5, max: 90, unit: '°', placeholder: '25' })),
    row(
      'Seçenekler',
      h(
        'span',
        { class: 'lbl-checks' },
        check('Çevre ve sınırda kıvrık', s.curved === true, (b) => f.set({ curved: b || undefined })),
        check('Bağlı çizgileri birleştir', s.mergeLines === true, (b) => f.set({ mergeLines: b || undefined })),
        check('Yalnız içine sığarsa', s.inside === true, (b) => f.set({ inside: b || undefined })),
        check('Sığmazsa dışarıda', s.outside === true, (b) => f.set({ outside: b || undefined })),
      ),
    ),
  ];
}

/** Biçim: the halo, the background, the shadow and the callout. */
function lookTab(f: FormHost): HTMLElement[] {
  const s = f.style;
  const bg = s.background;
  const shadow = s.shadow;
  const callout = s.callout;
  return [
    row(
      'Hale',
      h(
        'span',
        { class: 'lbl-inline' },
        optionalNumber('Halenin genişliği', s.halo?.width, (v) => f.set({ halo: v === undefined ? undefined : { ...s.halo, width: v } }), { min: 0, max: 10, unit: 'px', placeholder: '1.5' }),
        colour('Halenin rengi', s.halo?.color, (c) => f.set({ halo: { width: s.halo?.width ?? 1.5, ...(c && { color: c }) } }), 'Zemin rengi'),
      ),
    ),
    row(
      'Zemin',
      choice(
        'Zemin',
        [
          { value: 'none', label: 'Yok' },
          { value: 'rect', label: 'Dikdörtgen' },
          { value: 'round', label: 'Yuvarlak' },
          { value: 'ellipse', label: 'Elips' },
        ],
        bg?.shape ?? 'none',
        (v) => f.set({ background: v === 'none' ? undefined : { fill: 'paper', ...bg, shape: v } }),
      ),
    ),
    ...(bg
      ? [
          row(
            'Zeminin rengi',
            h(
              'span',
              { class: 'lbl-inline' },
              colour('Zeminin dolgusu', bg.fill, (c) => f.set({ background: { ...bg, fill: c } }), 'Dolgusuz'),
              colour('Zeminin çizgisi', bg.stroke, (c) => f.set({ background: { ...bg, stroke: c } }), 'Çizgisiz'),
              optionalNumber('Zeminin payı', bg.padding, (v) => f.set({ background: { ...bg, padding: v } }), { min: 0, max: 50, unit: 'px', placeholder: '2' }),
            ),
          ),
        ]
      : []),
    row(
      'Gölge',
      h(
        'span',
        { class: 'lbl-inline' },
        check('Gölge', !!shadow, (b) => f.set({ shadow: b ? { dx: 1, dy: -1, opacity: 0.5 } : undefined })),
        ...(shadow
          ? [
              optionalNumber('Gölgenin sağa kayması', shadow.dx, (v) => f.set({ shadow: { ...shadow, dx: v ?? 0 } }), { min: -50, max: 50, unit: 'px' }),
              optionalNumber('Gölgenin yukarı kayması', shadow.dy, (v) => f.set({ shadow: { ...shadow, dy: v ?? 0 } }), { min: -50, max: 50, unit: 'px' }),
              optionalNumber('Gölgenin opaklığı', shadow.opacity, (v) => f.set({ shadow: { ...shadow, opacity: v } }), { min: 0, max: 1, placeholder: '0.5' }),
            ]
          : []),
      ),
    ),
    row(
      'Çağrı çizgisi',
      choice(
        'Çağrı çizgisi',
        [
          { value: 'none', label: 'Yok' },
          { value: 'straight', label: 'Düz' },
          { value: 'manhattan', label: 'Dik açılı' },
        ],
        callout?.kind ?? 'none',
        (v) => f.set({ callout: v === 'none' ? undefined : { ...callout, kind: v } }),
      ),
      'Etiketi nesnesinden uzakta kalınca (dışarıda, elle taşınmış) nesneye bağlar.',
    ),
    ...(callout
      ? [
          row(
            'Çizginin görünüşü',
            h(
              'span',
              { class: 'lbl-inline' },
              colour('Çağrı çizgisinin rengi', callout.color, (c) => f.set({ callout: { ...callout, color: c } }), 'Yazının rengi'),
              optionalNumber('Çağrı çizgisinin kalınlığı', callout.width, (v) => f.set({ callout: { ...callout, width: v } }), { min: 0.1, max: 10, unit: 'px', placeholder: '1' }),
              optionalNumber('Çağrı çizgisinin en kısası', callout.minLength, (v) => f.set({ callout: { ...callout, minLength: v } }), { min: 0, max: 1000, unit: 'px', placeholder: '6' }),
            ),
          ),
        ]
      : []),
  ];
}

/** The abbreviation dictionary as the window writes it: a word and its short form a line, `Caddesi = Cd.`. */
export function dictionaryText(s: LabelStyle): string {
  return (s.abbreviate?.words ?? []).map((w) => `${w.word} = ${w.short}`).join('\n');
}

/** A dictionary's lines read back; a line without “=” is left out. */
export function readDictionary(text: string): { word: string; short: string }[] {
  const out: { word: string; short: string }[] = [];
  for (const line of text.split('\n')) {
    const i = line.indexOf('=');
    if (i < 0) continue;
    const word = line.slice(0, i).trim();
    const short = line.slice(i + 1).trim();
    if (word || short) out.push({ word, short });
  }
  return out;
}

/** Sığdırma: stacking, abbreviating and shrinking a label that does not fit. */
function fitTab(f: FormHost): HTMLElement[] {
  const s = f.style;
  const stack = s.stack;
  const abbr = s.abbreviate;
  const dict = h('textarea', { class: 'field lbl-dict', rows: 4, spellcheck: 'false', placeholder: 'Caddesi = Cd.\nSokak = Sk.', 'aria-label': 'Kısaltma sözlüğü' }, dictionaryText(s)) as HTMLTextAreaElement;
  dict.addEventListener('change', () => {
    const words = readDictionary(dict.value);
    f.set({ abbreviate: words.length ? { ...(abbr?.always && { always: true }), words } : undefined });
  });
  return [
    row(
      'Yığma',
      choice('Yığma', [{ value: 'none', label: 'Yok' }, { value: 'ifNeeded', label: 'Gerekirse' }, { value: 'always', label: 'Her zaman' }], stack?.mode ?? 'none', (v) =>
        f.set({ stack: v === 'none' ? undefined : { chars: stack?.chars ?? 12, ...(stack?.at !== undefined && { at: stack.at }), mode: v } }),
      ),
      'Uzun etiket satırlara bölünür.',
    ),
    ...(stack
      ? [
          row(
            'Satır',
            h(
              'span',
              { class: 'lbl-inline' },
              optionalNumber('Satırın en çok harfi', stack.chars, (v) => f.set({ stack: { ...stack, chars: v ?? 12 } }), { min: 2, max: 500, unit: 'harf' }),
              textField('Bölme karakterleri', stack.at, (v) => f.set({ stack: { ...stack, at: v } }), 'boşluk'),
            ),
          ),
        ]
      : []),
    row(
      'Kısaltma',
      choice('Kısaltma', [{ value: 'none', label: 'Yok' }, { value: 'ifNeeded', label: 'Gerekirse' }, { value: 'always', label: 'Her zaman' }], !abbr ? 'none' : abbr.always ? 'always' : 'ifNeeded', (v) =>
        f.set({ abbreviate: v === 'none' ? undefined : { words: abbr?.words?.length ? abbr.words : [{ word: 'Caddesi', short: 'Cd.' }], ...(v === 'always' && { always: true }) } }),
      ),
    ),
    ...(abbr ? [row('Sözlük', dict, 'Her satır bir sözcük: “Sözcük = Kısa”. Yalnız tam sözcükler kısalır.')] : []),
    row(
      'Küçültme',
      optionalNumber('En küçük boy oranı', s.shrink === undefined ? undefined : Math.round(s.shrink * 100), (v) => f.set({ shrink: v === undefined || v >= 100 ? undefined : v / 100 }), {
        min: 50,
        max: 100,
        unit: '%',
        placeholder: '100',
      }),
      'Sığmayan etiket bu orana kadar adım adım küçülür.',
    ),
  ];
}

/** Öncelik: priority, overlap, duplicates and when a class is drawn at all. */
function orderTab(f: FormHost): HTMLElement[] {
  const s = f.style;
  return [
    row('Öncelik', optionalNumber('Öncelik', s.priority, (v) => f.set({ priority: v === undefined ? undefined : Math.round(v) }), { min: 0, max: 10, placeholder: '5' }), '0–10: yüksek olan önce yerleşir; engelin ağırlığından küçük olan engeli örtemez.'),
    row(
      'Çakışma',
      choice('Çakışma', [{ value: 'never', label: 'Çakışmasın' }, { value: 'ifNeeded', label: 'Gerekirse' }, { value: 'always', label: 'Her zaman' }], s.overlap ?? 'never', (v) =>
        f.set({ overlap: v === 'never' ? undefined : v }),
      ),
      'Gerekirse: yer bulamazsa öbür etiketlerin üstüne yazılır.',
    ),
    row('Yinelenenler', optionalNumber('Yinelenenlerin uzaklığı', s.duplicates, (v) => f.set({ duplicates: v }), { min: 1, max: 10_000, unit: 'px', placeholder: 'ayıklanmaz' }), 'Aynı metin bu uzaklıktan yakınsa yazılmaz.'),
    row('En küçük nesne', optionalNumber('En küçük nesne', s.minFeaturePx, (v) => f.set({ minFeaturePx: v }), { min: 0, max: 100_000, unit: 'px', placeholder: '0' }), 'Ekranda bundan küçük nesnenin etiketi yazılmaz.'),
    row(
      'Ölçek aralığı',
      h(
        'span',
        { class: 'lbl-inline' },
        optionalNumber('En küçük ölçek', s.minScale, (v) => f.set({ minScale: v }), { min: 0, max: 1e9, unit: 'px/m', placeholder: 'sınırsız' }),
        optionalNumber('En büyük ölçek', s.maxScale, (v) => f.set({ maxScale: v }), { min: 0, max: 1e9, unit: 'px/m', placeholder: 'sınırsız' }),
      ),
      'Etiket yalnız bu yakınlıklar arasında yazılır.',
    ),
  ];
}

/** A tab's rows. */
export function tabRows(tab: LabelTab, f: FormHost): HTMLElement[] {
  switch (tab) {
    case 'text':
      return textTab(f);
    case 'place':
      return placeTab(f);
    case 'look':
      return lookTab(f);
    case 'fit':
      return fitTab(f);
    case 'order':
      return orderTab(f);
  }
}
