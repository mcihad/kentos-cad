import type { DataDefined, Effect, EffectType, GeneratorStep, GeneratorStepKind, GeometryGenerator, LineWave, MarkerLayer, TextMarker } from '../../model/style';
import { EFFECT_TYPES } from '../../model/style';
import { h, type Child } from '../dom';
import { icon } from '../icons';
import { PopupMenu } from '../widgets/PopupMenu';
import { checkbox, colorInput, dashInput, dataDefined, numberInput, pair, row, select, textInput, type FieldEnv } from './designerFields';
import {
  addStep,
  ANCHORS,
  BUFFER_SIDES,
  CAPS,
  CLASS_OF,
  DESIGNER_TEXTS as T,
  EFFECT_LABEL,
  effectOf,
  FONTS,
  moveStep,
  OPEN_SHAPES,
  PLACEMENTS,
  POSITIONS,
  putEffect,
  putStep,
  removeStep,
  RINGS,
  setEffect,
  setGeneratorKind,
  SHAPES,
  STEP_KINDS,
  STEP_LABEL,
  stepOutput,
  stepSummary,
  UNITS,
  VERTEX_CHOICES,
  WAVES,
  WEIGHTS,
  type AnyLayer,
  type Patch,
} from './designerModel';

/**
 * Each symbol layer type's property form (docs/adr/0094). What the types are
 * called, how a new layer starts, its summary and how a form's patch becomes
 * a layer are the model's (designerModel.ts); forms report patches, the
 * designer applies them to its draft.
 */

const unitOf = (l: AnyLayer) => (l.unit === 'px' ? 'px' : l.unit === 'm' ? 'm' : 'mm');

// ── Forms ──────────────────────────────────────────────────────────────

export interface FormEnv extends FieldEnv {
  /** Assets the symbol may draw with: SVG and raster drawings of the library. */
  assets: readonly { id: string; name: string; format: string }[];
  /** Adds an SVG file or a PNG/JPEG picture to the user's library; the new asset id, or null. */
  importSvg(): Promise<string | null>;
  /** Opens the SVG editor on a drawing (or a new one); the saved asset id, or null. */
  drawSvg(id?: string): Promise<string | null>;
  /** Where the layer sits: an area symbol's line layers may choose rings. */
  context: 'fill' | 'line' | 'marker';
  /** A geometry generator's chosen step (docs/adr/0226 §1), and choosing another. */
  step: number;
  chooseStep(i: number): void;
  /** Whether the layer takes draw effects: a symbol's own layers (docs/adr/0226 §2). */
  effects: boolean;
}

export function layerForm(l: AnyLayer, set: (patch: Patch) => void, env: FormEnv): HTMLElement {
  const u = unitOf(l);
  const n = (label: string, value: number, key: string, opts: { step?: number; min?: number; max?: number; unit?: string | null } = {}) =>
    row(label, numberInput(value, (v) => set({ [key]: v }), { label, unit: opts.unit === null ? undefined : (opts.unit ?? u), step: opts.step, min: opts.min, max: opts.max }));
  const ddNum = (label: string, value: number | { expr: string; fallback?: number } | undefined, key: string, fallback: number, opts: { min?: number } = {}) =>
    row(label, dataDefined(value, fallback, (v) => set({ [key]: v }), (v, s) => numberInput(v, s, { label, unit: u, min: opts.min }), label));
  // Switched back from an expression without a fallback, a colour that must be one is ink (it became none, which the layer refused).
  const ddColor = (label: string, value: string | null | { expr: string; fallback?: string } | undefined, key: string, allowNone = false) =>
    row(label, dataDefined<string | null>(value ?? null, allowNone ? null : 'ink', (v) => set({ [key]: v }), (v, s) => colorInput(v, s, env, { label, allowNone }), label));
  const common: Child[] = [
    row('Birim', select(l.unit ?? 'mm', UNITS, (v) => set({ unit: v }), 'Birim'), 'Kâğıt mm çizim ölçeğiyle büyür; ekran px sabit kalır; harita m gerçek boydur.'),
    row('Saydamlık', numberInput(Math.round((1 - (l.opacity ?? 1)) * 100), (v) => set({ opacity: 1 - Math.min(100, Math.max(0, v)) / 100 }), { label: 'Saydamlık', unit: '%', step: 5, min: 0, max: 100 })),
    row('Görünür', dataDefined(l.enabled ?? true, true, (v) => set({ enabled: v }), (v, s) => checkbox(v, s, 'Çizilsin'), 'Görünür'), 'ƒ ile koşula bağlanabilir: ör. [Nitelik] = \'Arsa\''),
  ];
  const assetPick = (value: string, key: string) => {
    const options = [{ value: '', label: 'Çizim seçin…' }, ...env.assets.map((a) => ({ value: a.id, label: a.name }))];
    const sel = select(value, options, (v) => set({ [key]: v }), 'Çizim');
    const btn = (label: string, title: string, run: () => void) => {
      const b = h('button', { class: 'btn btn--small', type: 'button', title }, label);
      b.addEventListener('click', run);
      return b;
    };
    const current = env.assets.find((a) => a.id === value);
    return row(
      'Çizim',
      h(
        'div',
        { class: 'sdf__assetcol' },
        sel,
        h(
          'div',
          { class: 'sdf__assetrow' },
          btn('Yeni çizim…', 'SVG çizim düzenleyicisinde yeni bir çizim yapar', () => void env.drawSvg().then((id) => id && set({ [key]: id }))),
          current?.format === 'svg' ? btn('Düzenle…', 'Seçili çizimi düzenleyicide açar (sistem çiziminin kopyası)', () => void env.drawSvg(value).then((id) => id && set({ [key]: id }))) : null,
          btn('Dosya al…', 'Bilgisayardan SVG, PNG ya da JPEG alır (Kitaplığım\'a eklenir)', () => void env.importSvg().then((id) => id && set({ [key]: id }))),
        ),
      ),
    );
  };
  const specific: Child[] = [];
  switch (l.type) {
    case 'simpleFill':
      specific.push(ddColor('Renk', l.color, 'color'));
      break;
    case 'hatchFill':
      specific.push(
        ddColor('Renk', l.color, 'color'),
        n('Açı', l.angle, 'angle', { unit: '°', step: 5 }),
        pair(n('Aralık', l.spacing, 'spacing', { min: 0.01 }), n('Kalınlık', l.width, 'width', { min: 0 })),
        n('Kaydırma', l.offset ?? 0, 'offset'),
        row('Kesik', dashInput(l.dash, (v) => set({ dash: v }), 'Kesik'), 'Çizgi ve boşluk uzunlukları sırayla.'),
        l.dash?.length ? n('Kesik kaydırma', l.dashOffset ?? 0, 'dashOffset') : null,
      );
      break;
    case 'patternFill':
      specific.push(
        pair(n('Aralık Y (sağa)', l.spacingX, 'spacingX', { min: 0.01 }), n('Aralık X (yukarı)', l.spacingY, 'spacingY', { min: 0.01 })),
        row('Dizilim', checkbox(!!l.stagger, (v) => set({ stagger: v }), 'Şaşırtmalı (her iki satırda bir yarım kaydır)')),
        n('Açı', l.angle ?? 0, 'angle', { unit: '°', step: 5 }),
        pair(n('Kaydırma Y', l.offset?.[0] ?? 0, 'offsetX'), n('Kaydırma X', l.offset?.[1] ?? 0, 'offsetY')),
        pair(n('Dağınıklık', Math.round((l.jitter ?? 0) * 100), 'jitterPct', { unit: '%', min: 0, max: 100, step: 5 }), n('Doluluk', Math.round((l.coverage ?? 1) * 100), 'coveragePct', { unit: '%', min: 0, max: 100, step: 5 })),
        n('Rastgele tohum', l.seed ?? 0, 'seed', { unit: null, step: 1 }),
        h('div', { class: 'sdf__hint' }, 'Dağınıklık ve doluluk yalnızca şekil işaretlerinde uygulanır (kumsal, serbest noktalama).'),
      );
      break;
    case 'imageFill':
      specific.push(assetPick(l.asset, 'asset'), n('Döşeme genişliği', l.tileSize, 'tileSize', { min: 0.01 }), n('Açı', l.angle ?? 0, 'angle', { unit: '°', step: 5 }));
      break;
    case 'centroidMarker':
      specific.push(row('Konum', select(l.position ?? 'pointOnSurface', POSITIONS, (v) => set({ position: v }), 'Konum')));
      break;
    case 'simpleLine':
      specific.push(
        ddColor('Renk', l.color, 'color'),
        ddNum('Kalınlık', l.width, 'width', 0.25, { min: 0 }),
        row('Kesik', dashInput(l.dash, (v) => set({ dash: v }), 'Kesik'), 'Çizgi ve boşluk uzunlukları sırayla, en çok 8 değer.'),
        l.dash?.length ? n('Kesik kaydırma', l.dashOffset ?? 0, 'dashOffset') : null,
        pair(
          row('Uç', select(l.cap ?? 'butt', CAPS, (v) => set({ cap: v }), 'Uç')),
          ddNum('Kaydırma', l.offset, 'offset', 0),
        ),
        h('div', { class: 'sdf__hint' }, env.context === 'fill' ? 'Artı kaydırma çizgiyi alanın içine alır.' : 'Artı kaydırma çizim yönünün soluna alır.'),
        env.context === 'fill' ? row('Halkalar', select(l.rings ?? 'all', RINGS, (v) => set({ rings: v }), 'Halkalar')) : null,
        waveForm(l.wave, (w) => set({ wave: w }), u),
        // Softening on its own row, the page shift's two numbers under it (three in a row did not fit a narrow column).
        n('Yumuşatma', l.blur ?? 0, 'blur', { min: 0 }),
        pair(n('Gölge sağa', l.shift?.[0] ?? 0, 'shiftX'), n('Gölge yukarı', l.shift?.[1] ?? 0, 'shiftY')),
        h('div', { class: 'sdf__hint' }, 'Yumuşatma kenarı bu genişlikte soldurur; gölge kaydırması çizgiyi sayfada hep aynı yöne taşır (alt gölge için sağa ve aşağı).'),
      );
      break;
    case 'markerLine':
      specific.push(
        row('Yerleşim', select(l.placement, PLACEMENTS, (v) => set({ placement: v }), 'Yerleşim')),
        l.placement === 'interval' ? pair(n('Aralık', l.interval ?? 6, 'interval', { min: 0.01 }), n('İlk uzaklık', l.offsetAlong ?? 0, 'offsetAlong')) : null,
        pair(ddNum('Kaydırma', l.offset, 'offset', 0), row('Döndür', checkbox(l.rotate !== false, (v) => set({ rotate: v }), 'Çizgiyle dönsün'))),
        pair(n('Grup', l.group?.count ?? 1, 'groupCount', { unit: 'adet', min: 1, max: 20, step: 1 }), n('Grup aralığı', l.group?.spacing ?? 1, 'groupSpacing', { min: 0 })),
        env.context === 'fill' ? row('Halkalar', select(l.rings ?? 'all', RINGS, (v) => set({ rings: v }), 'Halkalar')) : null,
      );
      break;
    case 'shape':
      specific.push(
        row('Şekil', select(l.shape, SHAPES, (v) => set({ shape: v }), 'Şekil')),
        l.shape === 'rectangle' ? pair(ddNum('Genişlik', l.size, 'size', 3, { min: 0 }), n('Yükseklik', l.height ?? (typeof l.size === 'number' ? l.size : 3), 'height', { min: 0 })) : ddNum('Boyut', l.size, 'size', 3, { min: 0 }),
        ddColor('Dolgu', l.fill, 'fill', true),
        ddColor('Çizgi', l.stroke, 'stroke', true),
        n('Çizgi kalınlığı', l.strokeWidth ?? 0.2, 'strokeWidth', { min: 0 }),
        l.shape === 'gear'
          ? pair(n('Diş sayısı', l.teeth ?? 12, 'teeth', { unit: 'adet', min: 3, max: 64, step: 1 }), n('Diş derinliği', Math.round((l.teethDepth ?? 0.2) * 100), 'teethDepthPct', { unit: '%', min: 2, max: 60, step: 1 }))
          : null,
        l.shape === 'arc' ? n('Açıklık', l.sweep ?? 180, 'sweep', { unit: '°', min: 1, max: 360, step: 5 }) : null,
        !OPEN_SHAPES.has(l.shape) ? n('Delik', Math.round((l.hole ?? 0) * 100), 'holePct', { unit: '%', min: 0, max: 95, step: 5 }) : null,
        !OPEN_SHAPES.has(l.shape) ? h('div', { class: 'sdf__hint' }, 'Delik, yarıçapın yüzdesi kadar ortadan yuvarlak boşluk bırakır (dişli göbeği, pul).') : null,
        ...placementRows(l, set, u),
      );
      break;
    case 'svg':
      specific.push(assetPick(l.asset, 'asset'), ddNum('Genişlik', l.size, 'size', 5, { min: 0 }), ddColor('Renk (currentColor)', l.fill, 'fill', true), ddColor('İkinci renk', l.stroke, 'stroke', true), ...placementRows(l, set, u));
      break;
    case 'raster':
      specific.push(assetPick(l.asset, 'asset'), ddNum('Genişlik', l.size, 'size', 5, { min: 0 }), ...placementRows(l, set, u));
      break;
    case 'geometryGenerator':
      specific.push(...generatorForm(l, set, env, u));
      break;
    case 'text':
      specific.push(
        row('Metin', dataDefined(l.text, '', (v) => set({ text: v }), (v, s) => textInput(v, s, { label: 'Metin' }), 'Metin'), 'ƒ ile öznitelikten: ör. \'E=\' || [Emsal]'),
        ddNum('Harf yüksekliği', l.size, 'size', 3, { min: 0 }),
        pair(row('Yazı tipi', select(l.font ?? 'ui', FONTS, (v) => set({ font: v }), 'Yazı tipi')), row('Kalınlık', select(String(l.weight ?? 400), WEIGHTS, (v) => set({ weight: Number(v) as TextMarker['weight'] }), 'Kalınlık'))),
        row('Stil', checkbox(!!l.italic, (v) => set({ italic: v }), 'İtalik')),
        ddColor('Renk', l.color, 'color'),
        row('Hale', colorInput(l.halo?.color ?? null, (c) => set({ halo: c ? { color: c, width: l.halo?.width ?? 0.3 } : null }), env, { label: 'Hale rengi', allowNone: true })),
        l.halo ? n('Hale kalınlığı', l.halo.width, 'haloWidth', { min: 0 }) : null,
        ...placementRows(l, set, u),
      );
      break;
  }
  return h(
    'div',
    { class: 'sdf__form' },
    h('div', { class: 'sdf__group' }, specific),
    env.effects ? effectsForm(l, set, env, u) : null,
    h('div', { class: 'sdf__group sdf__group--common' }, h('div', { class: 'sdf__grouptitle' }, 'Genel'), common),
  );
}

/** The symbol classes a generator may draw with. */
const DRAW_KINDS: { value: 'fill' | 'line' | 'marker'; label: string }[] = [
  { value: 'fill', label: 'Alan' },
  { value: 'line', label: 'Çizgi' },
  { value: 'marker', label: 'İşaret' },
];

/** A step's icon in Adım ekle's menu (the desktop's `step_icon`): the app's icon of the same operation. */
const STEP_ICON: Record<GeneratorStepKind, string> = {
  centroid: 'snapCentroid',
  pointOnSurface: 'featureToPoint',
  vertices: 'verticesToPoints',
  boundary: 'polygonToLine',
  offset: 'offset',
  extend: 'extend',
  buffer: 'geoBuffer',
  envelope: 'stepEnvelope',
  hull: 'boundingGeometry',
  circle: 'circle',
  wedge: 'sector',
  polygon: 'regularPolygon',
  translate: 'move',
  rotate: 'rotate',
  scale: 'scale',
};

/**
 * Geometri üretici's form (docs/adr/0226 §1): the class of its symbol (whose layers are its child rows), its steps
 * in order with ↑, ↓ and ✕, Adım ekle, and the chosen step's values (ƒ: from each object's data).
 */
function generatorForm(l: GeometryGenerator, set: (patch: Patch) => void, env: FormEnv, u: string): Child[] {
  const steps = (next: GeometryGenerator | null) => next && set({ steps: next.steps });
  const btn = (name: string, label: string, run: () => void, enabled = true) => {
    const b = h('button', { class: 'ibtn ibtn--small', type: 'button', 'aria-label': label, title: label, disabled: !enabled }, icon(name, 14));
    b.addEventListener('click', (e) => {
      e.stopPropagation();
      run();
    });
    return b;
  };
  const chosen = Math.min(env.step, l.steps.length - 1);
  const list = l.steps.length
    ? h(
        'div',
        { class: 'sdf__steps', role: 'listbox', 'aria-label': T.steps },
        l.steps.map((st, i) => {
          const r = h(
            'div',
            { class: 'sdf__step', role: 'option', 'aria-selected': String(i === chosen), tabindex: i === chosen ? '0' : '-1' },
            h('span', { class: 'sdf__stepnum' }, `${i + 1}.`),
            h('span', { class: 'sdf__steptext' }, stepSummary(st, u)),
            btn('chevronUp', 'Yukarı', () => (steps(moveStep(l, i, -1)), env.chooseStep(i - 1)), i > 0),
            btn('chevronDown', 'Aşağı', () => (steps(moveStep(l, i, 1)), env.chooseStep(i + 1)), i < l.steps.length - 1),
            btn('trash', 'Adımı sil', () => (steps(removeStep(l, i)), env.chooseStep(Math.max(0, Math.min(i, l.steps.length - 2))))),
          );
          r.addEventListener('click', () => env.chooseStep(i));
          return r;
        }),
      )
    : h('p', { class: 'sdf__hint' }, T.noSteps);
  const add = h('button', { class: 'btn btn--small sdf__addstep', type: 'button', 'aria-haspopup': 'menu' }, icon('plus', 14), T.addStep, icon('chevronDown', 12));
  add.addEventListener('click', () => {
    const r = add.getBoundingClientRect();
    PopupMenu.open(
      STEP_KINDS.map((k) => ({ label: STEP_LABEL[k], icon: STEP_ICON[k], run: () => (steps(addStep(l, k)), env.chooseStep(l.steps.length)) })),
      { x: r.left, y: r.bottom + 4 },
    );
  });
  const left = stepOutput(l.steps, CLASS_OF[env.context]);
  const st = l.steps[chosen];
  return [
    row(T.drawsWith, select(l.symbol.type, DRAW_KINDS, (v) => set({ symbol: setGeneratorKind(l, v).symbol }), T.drawsWith), 'Üretilen geometriyi çizen sembol; katmanları listede bu katmanın altındadır.'),
    h('div', { class: 'sdf__grouptitle' }, T.steps),
    list,
    add,
    left === null ? h('div', { class: 'sdf__warn', role: 'status' }, T.nothingLeft) : null,
    st ? h('div', { class: 'sdf__stepform' }, h('div', { class: 'sdf__grouptitle' }, `${chosen + 1}. ${STEP_LABEL[st.kind]}`), ...stepFields(st, (next) => set({ steps: putStep(l, chosen, next).steps }), u)) : null,
  ];
}

/** A step's own values. */
function stepFields(st: GeneratorStep, put: (next: GeneratorStep) => void, u: string): Child[] {
  const dd = (label: string, key: string, fallback: number, unit: string | undefined, opts: { min?: number; step?: number } = {}) =>
    row(
      label,
      dataDefined(
        (st as unknown as Record<string, DataDefined<number> | undefined>)[key],
        fallback,
        (v) => put({ ...st, [key]: v } as GeneratorStep),
        (v, s) => numberInput(v, s, { label, unit, min: opts.min, step: opts.step }),
        label,
      ),
    );
  switch (st.kind) {
    case 'vertices':
      return [row('Köşeler', select(st.which ?? 'all', VERTEX_CHOICES, (v) => put({ ...st, which: v }), 'Köşeler'))];
    case 'offset':
      return [dd('Uzaklık', 'distance', 1, u), h('div', { class: 'sdf__hint' }, 'Artı çizginin soluna, alanın içine.')];
    case 'extend':
      return [pair(dd('Baştan', 'start', 0, u, { min: 0 }), dd('Sondan', 'end', 0, u, { min: 0 }))];
    case 'buffer':
      return [
        dd('Uzaklık', 'distance', 2, u),
        row('Yan', select(st.side ?? 'both', BUFFER_SIDES, (v) => put({ ...st, side: v }), 'Yan'), 'Eksi uzaklık alanı içe küçültür; yan yalnız çizgilerde.'),
      ];
    case 'circle':
      return [dd('Yarıçap', 'radius', 2, u, { min: 0 })];
    case 'wedge':
      return [
        pair(dd('Yön', 'direction', 0, '°', { step: 5 }), dd('Genişlik', 'width', 60, '°', { min: 0, step: 5 })),
        pair(dd('Yarıçap', 'radius', 5, u, { min: 0 }), dd('İç yarıçap', 'inner', 0, u, { min: 0 })),
        h('div', { class: 'sdf__hint' }, 'Yön kuzeyden saat yönünde (ör. fotoğrafın Yön özniteliği: ƒ ile [Yön]).'),
      ];
    case 'polygon':
      return [
        pair(dd('Köşe sayısı', 'sides', 6, 'adet', { min: 3, step: 1 }), dd('Yarıçap', 'radius', 2, u, { min: 0 })),
        dd('Döndürme', 'rotation', 0, '°', { step: 5 }),
        h('div', { class: 'sdf__hint' }, 'İlk köşe kuzeyde; döndürme saat yönünün tersine.'),
      ];
    case 'translate':
      return [pair(dd('Y (sağa)', 'dx', 0, u), dd('X (yukarı)', 'dy', 0, u))];
    case 'rotate':
      return [dd('Açı', 'angle', 45, '°', { step: 5 }), h('div', { class: 'sdf__hint' }, 'Saat yönünün tersine, ağırlık merkezinde.')];
    case 'scale':
      return [pair(dd('Yatay', 'x', 1, undefined, { step: 0.1 }), dd('Düşey', 'y', 1, undefined, { step: 0.1 })), h('div', { class: 'sdf__hint' }, 'Ağırlık merkezinde; biri boşsa öbürü.')];
    default:
      return [h('p', { class: 'sdf__hint' }, 'Bu adımın değeri yok.')];
  }
}

/** A layer's draw effects (docs/adr/0226 §2): each type's switch, its values when it is on. */
function effectsForm(l: AnyLayer, set: (patch: Patch) => void, env: FormEnv, u: string): HTMLElement {
  const put = (e: Effect) => set({ effects: putEffect(l, e).effects });
  const rows: Child[] = [h('div', { class: 'sdf__grouptitle' }, T.effects)];
  for (const type of EFFECT_TYPES) {
    const e = effectOf(l, type);
    const on = !!e && e.enabled !== false;
    rows.push(checkbox(on, (v) => set({ effects: setEffect(l, type, v).effects }), EFFECT_LABEL[type]));
    if (!on || !e) continue;
    rows.push(h('div', { class: 'sdf__effect' }, effectFields(type, e, put, env, u)));
  }
  return h('div', { class: 'sdf__group' }, rows);
}

function effectFields(type: EffectType, e: Effect, put: (e: Effect) => void, env: FormEnv, u: string): Child[] {
  const label = EFFECT_LABEL[type];
  const n = (name: string, key: string, value: number, unit: string, opts: { min?: number; max?: number; step?: number } = {}) =>
    row(name, numberInput(value, (v) => put({ ...e, [key]: v } as Effect), { label: `${label}: ${name}`, unit, ...opts }));
  const transparency = (opacity: number) =>
    row('Saydamlık', numberInput(Math.round((1 - opacity) * 100), (v) => put({ ...e, opacity: 1 - Math.min(100, Math.max(0, v)) / 100 } as Effect), { label: `${label}: Saydamlık`, unit: '%', step: 5, min: 0, max: 100 }));
  const color = (value: string) => row('Renk', colorInput(value, (c) => c && put({ ...e, color: c } as Effect), env, { label: `${label}: Renk` }));
  if (e.type === 'dropShadow' || e.type === 'innerShadow')
    return [
      pair(n('Uzaklık', 'distance', e.distance ?? (e.type === 'dropShadow' ? 1 : 0.5), u, { min: 0 }), n('Açı', 'angle', e.angle ?? 135, '°', { step: 5 })),
      n('Bulanıklık', 'blur', e.blur ?? (e.type === 'dropShadow' ? 1.5 : 1), u, { min: 0 }),
      pair(color(e.color ?? '#000000'), transparency(e.opacity ?? 0.5)),
    ];
  if (e.type === 'outerGlow' || e.type === 'innerGlow')
    return [pair(n('Yayılma', 'spread', e.spread ?? 0.5, u, { min: 0 }), n('Bulanıklık', 'blur', e.blur ?? (e.type === 'outerGlow' ? 1.5 : 1), u, { min: 0 })), pair(color(e.color ?? '#FFFFFF'), transparency(e.opacity ?? 0.5))];
  return [n('Bulanıklık', 'blur', e.blur ?? 1, u, { min: 0 })];
}

function placementRows(l: MarkerLayer, set: (p: Patch) => void, u: string): Child[] {
  return [
    row(
      'Döndürme',
      dataDefined(l.rotation ?? 0, 0, (v) => set({ rotation: v }), (v, s) => numberInput(v, s, { label: 'Döndürme', unit: '°', step: 5 }), 'Döndürme'),
      'Saat yönünün tersine; çizgi boyunca işaretlerde çizginin yönüne eklenir.',
    ),
    pair(
      row('Kaydırma Y', numberInput(l.offset?.[0] ?? 0, (v) => set({ offsetX: v }), { label: 'Kaydırma Y', unit: u })),
      row('Kaydırma X', numberInput(l.offset?.[1] ?? 0, (v) => set({ offsetY: v }), { label: 'Kaydırma X', unit: u })),
    ),
    row('Çapa', select(l.anchor ?? 'center', ANCHORS, (v) => set({ anchor: v }), 'Çapa'), 'Noktanın işaretin neresine düştüğü.'),
  ];
}

function waveForm(w: LineWave | undefined, set: (w: LineWave | undefined) => void, u: string): HTMLElement {
  const host = h('div', { class: 'sdf__wave' });
  const render = (cur: LineWave | undefined) => {
    const shape = select(cur?.shape ?? 'none', WAVES as { value: string; label: string }[], (v) => {
      // A new shape keeps the wave's other settings (where the first wave starts was dropped).
      const next = v === 'none' ? undefined : { ...cur, shape: v as LineWave['shape'], length: cur?.length ?? 5, amplitude: cur?.amplitude ?? 0.8, connect: cur?.connect ?? true };
      set(next);
      render(next);
    }, 'Dalga');
    const parts: Child[] = [row('Biçim', shape)];
    if (cur) {
      const upd = (patch: Partial<LineWave>) => {
        cur = { ...cur!, ...patch };
        set(cur);
      };
      parts.push(
        pair(row('Dalga boyu', numberInput(cur.length, (v) => upd({ length: v }), { label: 'Dalga boyu', unit: u, min: 0.01 })), row('Genlik', numberInput(cur.amplitude, (v) => upd({ amplitude: v }), { label: 'Genlik', unit: u, min: 0 }))),
        pair(row('Tekrar', numberInput(cur.spacing ?? cur.length, (v) => upd({ spacing: v }), { label: 'Tekrar', unit: u, min: 0.01 })), row('Aralar', checkbox(cur.connect !== false, (v) => upd({ connect: v }), 'Düz çizgiyle bağla'))),
        pair(
          row('Evre', checkbox(cur.offsetAlong !== undefined, (v) => (upd({ offsetAlong: v ? 0 : undefined }), render(cur)), 'Çizgi başından')),
          cur.offsetAlong !== undefined ? row('İlk dalga', numberInput(cur.offsetAlong, (v) => upd({ offsetAlong: v }), { label: 'İlk dalga', unit: u, min: 0 })) : null,
        ),
        h('div', { class: 'sdf__hint' }, 'Çizgi başından başlayan dalgalar, aynı aralık ve ilk uzaklıkla yerleşen işaretlerle adım adım gider; kapalıyken dalgalar çizgiye ortalanır.'),
      );
    }
    host.replaceChildren(h('div', { class: 'sdf__grouptitle' }, 'Dalga'), ...parts.flat().filter(Boolean) as Node[]);
  };
  render(w);
  return host;
}
