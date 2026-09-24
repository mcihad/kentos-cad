import { applyTheme, applyUiScale } from '../../app/commands';
import type { AppContext } from '../../app/context';
import { PREFERENCE_DEFAULTS, snapshot, type PreferencesData, type ShellKind, type Theme } from '../../app/state';
import type { Signal } from '../../core/signal';
import { crsBySrid } from '../../geo/crs';
import { WebGPUBackend } from '../../render/webgpu/WebGPUBackend';
import { h } from '../dom';
import { note, segmented, settingRow, stepper, toggleSwitch } from '../widgets/controls';
import { crsPicker } from './crsPicker';
import { group, SettingsShell, type DraftApi, type SectionDef } from './SettingsShell';

/** Application settings: this user, this browser, every project. */
interface AppDraft extends PreferencesData {
  theme: Theme;
}

export type AppSettingsSection = 'appearance' | 'snap' | 'newProjects' | 'engine';

export function openAppSettings(ctx: AppContext, section?: AppSettingsSection): void {
  const crsState = { query: '' };
  const initial: AppDraft = { ...snapshot(ctx.prefs), theme: ctx.ui.theme.value };

  const sections: SectionDef<AppDraft>[] = [
    {
      id: 'appearance',
      label: 'Görünüm',
      icon: 'appearance',
      title: 'Görünüm',
      lead: 'Tema, arayüz düzeni, yazı boyutu, artı imleç ve fare yardımcıları.',
      keys: ['theme', 'shell', 'uiScale', 'crosshair', 'cursorInput', 'hoverInfo'],
      render: (api) => appearance(api),
    },
    {
      id: 'snap',
      label: 'Kenetleme',
      icon: 'snap',
      title: 'Kenetleme ve seçim',
      lead: 'İmlecin hangi noktalara yapışacağı ve nesneleri ne kadar yakından yakalayacağı.',
      keys: ['snapEndpoint', 'snapMidpoint', 'snapCenter', 'snapNode', 'snapIntersection', 'snapPerpendicular', 'snapTangent', 'snapNearest', 'snapAperture', 'pickAperture', 'polarIncrement'],
      render: (api) => snap(api),
    },
    {
      id: 'newProjects',
      label: 'Yeni projeler',
      icon: 'fileNew',
      title: 'Yeni proje varsayılanları',
      lead: 'Oluşturacağınız her yeni projede başlangıçta kullanılacak koordinat sistemi.',
      keys: ['defaultSrid'],
      render: (api) => {
        const current = ctx.doc.crs.value;
        const openProject = h('button', { class: 'btn btn--small', type: 'button' }, 'Proje ayarlarını aç');
        openProject.addEventListener('click', () => ctx.commands.execute('file.settings'));
        return [
          crsPicker({
            value: api.draft.defaultSrid,
            initial: api.initial.defaultSrid,
            defaultSrid: api.initial.defaultSrid,
            mode: 'default',
            state: crsState,
            onChange: (srid, rerender) => api.set('defaultSrid', srid, rerender),
          }),
          group(
            'Açık proje',
            settingRow('Bu projenin sistemi', `${current.name} (EPSG:${current.srid}). Proje ayarlarından değiştirilir ve proje dosyasına kaydedilir.`, openProject),
          ),
        ];
      },
    },
    {
      id: 'engine',
      label: 'Çizim motoru',
      icon: 'chip',
      title: 'Çizim motoru',
      lead: 'Çizim alanını ekran kartında çizen arka uç ve çözünürlük tercihi.',
      keys: ['rendererPreference', 'hiDpi', 'symbolSize'],
      render: (api) => engine(api, ctx),
    },
  ];

  new SettingsShell<AppDraft>({
    title: 'Uygulama ayarları',
    scope: { icon: 'settings', title: 'Bu tarayıcıda saklanır', detail: 'Tüm projeler için geçerlidir' },
    sections,
    initial,
    defaults: { ...PREFERENCE_DEFAULTS, theme: 'dark' },
    section,
    onSave: (draft, init) => {
      const prefs = ctx.prefs as unknown as Record<string, Signal<unknown>>;
      for (const k of Object.keys(PREFERENCE_DEFAULTS) as (keyof PreferencesData)[]) prefs[k].set(draft[k]);
      if (draft.uiScale !== init.uiScale) applyUiScale(draft.uiScale);
      if (draft.theme !== init.theme) applyTheme(ctx, draft.theme);
      if (draft.defaultSrid !== init.defaultSrid) {
        const c = crsBySrid(draft.defaultSrid)!;
        ctx.log.info(`Yeni projeler ${c.name} (EPSG:${c.srid}) ile oluşturulacak. Açık projenin sistemi değişmedi.`);
      }
      ctx.log.success('Uygulama ayarları kaydedildi.');
    },
  });
}

function appearance(api: DraftApi<AppDraft>) {
  const d = api.draft;
  const themeCard = (theme: Theme, label: string) => {
    const b = h(
      'button',
      { class: 'theme-card', type: 'button', role: 'radio', 'aria-checked': String(d.theme === theme), 'data-preview': theme },
      h(
        'div',
        { class: 'theme-card__mock', 'aria-hidden': 'true' },
        h('div', { class: 'mock__bar' }),
        h(
          'div',
          { class: 'mock__body' },
          h('div', { class: 'mock__tools' }, h('i'), h('i'), h('i')),
          h('div', { class: 'mock__canvas' }, h('b'), h('b'), h('b')),
          h('div', { class: 'mock__panel' }, h('i'), h('i'), h('i'), h('i')),
        ),
      ),
      h('span', { class: 'theme-card__label' }, label),
    );
    b.addEventListener('click', () => api.set('theme', theme));
    return b;
  };
  // Miniature workbenches in the current theme: menu bar, toolbar and toolbox, or tabs over a ribbon.
  const shellCard = (shell: ShellKind, label: string, detail: string) => {
    const i = (n: number, cls?: string) => Array.from({ length: n }, (_, k) => h('i', { class: k === 1 && cls ? cls : null }));
    const top =
      shell === 'classic'
        ? [h('div', { class: 'lmock__menu' }, ...i(5)), h('div', { class: 'lmock__toolbar' }, ...i(6))]
        : [
            h('div', { class: 'lmock__tabs' }, ...i(5, 'on')),
            h(
              'div',
              { class: 'lmock__ribbon' },
              h('b', { class: 'lmock__big' }),
              h('span', { class: 'lmock__smalls' }, ...i(3)),
              h('b', { class: 'lmock__big' }),
              h('span', { class: 'lmock__smalls' }, ...i(3)),
              h('span', { class: 'lmock__smalls' }, ...i(3)),
            ),
          ];
    const b = h(
      'button',
      { class: 'theme-card layout-card', type: 'button', role: 'radio', 'aria-checked': String(d.shell === shell), dataset: { shell } },
      h(
        'div',
        { class: 'layout-card__mock', 'aria-hidden': 'true' },
        ...top,
        h('div', { class: 'lmock__body' }, shell === 'classic' ? h('div', { class: 'lmock__toolbox' }, ...i(6)) : null, h('div', { class: 'lmock__canvas' }), h('div', { class: 'lmock__panel' })),
      ),
      h('span', { class: 'theme-card__label' }, label),
      h('span', { class: 'layout-card__detail' }, detail),
    );
    b.addEventListener('click', () => api.set('shell', shell));
    return b;
  };
  return [
    group('Tema', h('div', { class: 'theme-cards', role: 'radiogroup', 'aria-label': 'Tema' }, themeCard('dark', 'Koyu grafit'), themeCard('light', 'Açık pafta'))),
    group(
      'Arayüz düzeni',
      h(
        'div',
        { class: 'theme-cards', role: 'radiogroup', 'aria-label': 'Arayüz düzeni' },
        shellCard('classic', 'Klasik', 'Menüler, araç çubuğu ve kayan araç kutusu'),
        shellCard('ribbon', 'Şerit', 'Sekmeli şerit; dar pencerede kendini sığdırır'),
      ),
      note('info', 'İkisi aynı araç ve komutları sunar; yeni bir araç ikisinde de kendiliğinden yer alır. Görünüm → Şerit arayüzü komutuyla da geçilir.'),
    ),
    group(
      'Yazı ve imleç',
      settingRow(
        'Yazı boyutu',
        'Menüler, paneller ve komut satırı. Çizim etiketleri etkilenmez.',
        segmented({
          label: 'Yazı boyutu',
          value: d.uiScale,
          options: [
            { value: 'standard', label: 'Standart' },
            { value: 'large', label: 'Büyük' },
            { value: 'xlarge', label: 'Çok büyük' },
          ],
          onChange: (v) => api.set('uiScale', v),
        }),
      ),
      settingRow(
        'Artı imleç',
        'Çizim alanındaki imlecin kol uzunluğu.',
        segmented({
          label: 'Artı imleç',
          value: d.crosshair,
          options: [
            { value: 'small', label: 'Küçük' },
            { value: 'medium', label: 'Orta' },
            { value: 'full', label: 'Tam ekran' },
          ],
          onChange: (v) => api.set('crosshair', v),
        }),
      ),
    ),
    group(
      'Fare yardımcıları',
      settingRow(
        'İmleç yanında değer girişi',
        'Komut sırasında yazılan mesafe ve koordinatlar imlecin yanında açılır; kapalıyken komut satırına gider.',
        toggleSwitch({ label: 'İmleç yanında değer girişi', checked: d.cursorInput, onChange: (v) => api.set('cursorInput', v) }),
      ),
      settingRow(
        'Nesne bilgi kartı',
        'Seçim aracında bir nesnenin üzerinde durunca türü, katmanı, uzunluğu ya da alanı gösterilir.',
        toggleSwitch({ label: 'Nesne bilgi kartı', checked: d.hoverInfo, onChange: (v) => api.set('hoverInfo', v) }),
      ),
    ),
  ];
}

function snap(api: DraftApi<AppDraft>) {
  const d = api.draft;
  type SnapKey = 'snapEndpoint' | 'snapMidpoint' | 'snapCenter' | 'snapNode' | 'snapIntersection' | 'snapPerpendicular' | 'snapTangent' | 'snapNearest';
  const kind = (key: SnapKey, label: string, desc: string, glyph: string) =>
    settingRow(
      label,
      desc,
      h('div', { class: 'snap-toggle' }, h('span', { class: `snap-glyph snap-glyph--${glyph}`, 'aria-hidden': 'true' }), toggleSwitch({ label, checked: d[key], onChange: (v) => api.set(key, v) })),
    );
  return [
    group(
      'Kenet türleri',
      kind('snapEndpoint', 'Uç nokta', 'Çizgi, parsel ve bina köşeleri; dairelerin çeyrek noktaları.', 'square'),
      kind('snapMidpoint', 'Orta nokta', 'Kenarların ve yayların tam ortası.', 'triangle'),
      kind('snapCenter', 'Merkez', 'Daire ve yay merkezleri.', 'circle'),
      kind('snapNode', 'Nokta', 'Poligon, kot ve tekil noktalar.', 'circle'),
      kind('snapIntersection', 'Kesişim', 'İki kenarın kesiştiği nokta.', 'cross'),
      kind('snapPerpendicular', 'Dik', 'Son noktadan kenara inen dikmenin ayağı.', 'perp'),
      kind('snapTangent', 'Teğet', 'Son noktadan daire ya da yaya çizilen teğetin değme noktası.', 'circle'),
      kind('snapNearest', 'En yakın', 'Kenar üzerindeki en yakın nokta; başka kenet yoksa devreye girer.', 'nearest'),
      note('info', 'Durum çubuğundaki Kenet düğmesi (F3) seçili türlerin tümünü birlikte açıp kapatır.'),
    ),
    group(
      'Kutupsal izleme',
      settingRow(
        'Açı adımı',
        'Kutupsal izleme (F10) açıkken imleç bu açının katlarına kilitlenir. Orto açıksa önceliklidir.',
        segmented({
          label: 'Açı adımı',
          value: String(d.polarIncrement),
          options: [15, 30, 45, 90].map((v) => ({ value: String(v), label: `${v}°` })),
          onChange: (v) => api.set('polarIncrement', Number(v)),
        }),
      ),
    ),
    group(
      'Yakalama mesafesi',
      settingRow('Kenet yarıçapı', 'İmlecin bir noktaya yapışması için gereken yakınlık.', stepper({ label: 'Kenet yarıçapı', value: d.snapAperture, min: 4, max: 30, unit: 'px', onChange: (v) => api.set('snapAperture', v) })),
      settingRow('Seçim yarıçapı', 'Tıklamanın bir çizgiyi yakalaması için gereken yakınlık.', stepper({ label: 'Seçim yarıçapı', value: d.pickAperture, min: 2, max: 15, unit: 'px', onChange: (v) => api.set('pickAperture', v) })),
    ),
  ];
}

function engine(api: DraftApi<AppDraft>, ctx: AppContext) {
  const d = api.draft;
  const gpu = WebGPUBackend.isSupported();
  const card = (value: 'webgl2' | 'webgpu', title: string, desc: string, badge: string, disabled: boolean) => {
    const b = h(
      'button',
      { class: 'engine-card', type: 'button', role: 'radio', 'aria-checked': String(d.rendererPreference === value), disabled },
      h('span', { class: 'engine-card__radio', 'aria-hidden': 'true' }),
      h('span', { class: 'engine-card__text' }, h('span', { class: 'engine-card__title' }, title, h('span', { class: 'engine-card__badge' }, badge)), h('span', { class: 'engine-card__desc' }, desc)),
    );
    b.addEventListener('click', () => api.set('rendererPreference', value));
    return b;
  };
  return [
    group(
      'Arka uç',
      h(
        'div',
        { class: 'engine-cards', role: 'radiogroup', 'aria-label': 'Çizim arka ucu' },
        card('webgl2', 'WebGL2', 'Tüm güncel tarayıcılarda çalışır. Varsayılan ve önerilen.', 'Varsayılan', false),
        card(
          'webgpu',
          'WebGPU',
          gpu ? 'Yeni nesil grafik arayüzü. Aynı çizimi üretir; büyük veride daha verimli olması hedeflenir.' : 'Bu tarayıcı WebGPU sunmuyor. Chrome ya da Edge’in güncel sürümünü kullanın.',
          gpu ? 'Deneysel' : 'Desteklenmiyor',
          !gpu,
        ),
      ),
      note('info', `Şu an çalışan: ${ctx.view.backendLabel.value}. Seçim Kaydet ile hemen uygulanır; motor başlatılamazsa WebGL2’ye dönülür.`),
    ),
    group(
      'Çözünürlük',
      settingRow(
        'Yüksek çözünürlük (HiDPI)',
        'Retina ve 4K ekranlarda keskin çizgiler. Kapatıldığında daha az piksel çizilir; çok büyük çizimlerde kaydırma akıcılaşır.',
        toggleSwitch({ label: 'Yüksek çözünürlük', checked: d.hiDpi, onChange: (v) => api.set('hiDpi', v) }),
      ),
    ),
    group(
      'Sembol boyutu',
      settingRow(
        'Semboller',
        'Çizim ölçeğinde: basılı paftadaki boyları, harita ile büyür ve küçülür (yönetmelik ölçüleri böyle görünür). Ekranda sabit: her yakınlıkta aynı boy, gezinmek için.',
        segmented({
          label: 'Sembol boyutu',
          options: [
            { value: 'plot', label: 'Çizim ölçeğinde' },
            { value: 'screen', label: 'Ekranda sabit' },
          ],
          value: d.symbolSize,
          onChange: (v) => api.set('symbolSize', v),
        }),
      ),
    ),
  ];
}
