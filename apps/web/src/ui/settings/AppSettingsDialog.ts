import { applyTheme, applyUiScale } from '../../app/commands';
import { applyAccent, applyUiFont } from '../../app/appearance';
import { accentPicker, drawingFontPicker, fontPicker } from './appearancePickers';
import type { AppContext } from '../../app/context';
import { PREF_KEYS, PREFERENCE_DEFAULTS, type PreferencesData, type ShellKind, type Theme } from '../../app/state';
import { crsBySrid } from '../../geo/crs';
import { settingDescriptor } from '../../core/settings/schema';
import { h } from '../dom';
import { note, segmented, settingRow, stepper, toggleSwitch } from '../widgets/controls';
import { crsPicker } from './crsPicker';
import { workspacePicker } from './workspacePicker';
import { workspaceById } from '../../app/workspaces';
import { group, SettingsShell, type DraftApi, type SectionDef } from './SettingsShell';
import { engine } from './engineSection';
import { settingsFile, type FileDraft, type FileState } from './settingsFileSection';

/** Application settings: this user, this browser, every project. */
export interface AppDraft extends PreferencesData, FileDraft {
  theme: Theme;
}

export type AppSettingsSection = 'appearance' | 'snap' | 'newProjects' | 'engine' | 'file';

const FIELDS = Object.keys(PREF_KEYS) as (keyof PreferencesData)[];

export function openAppSettings(ctx: AppContext, section?: AppSettingsSection): void {
  const crsState = { query: '' };
  const fileState: FileState = { report: null };
  const store = ctx.settingsStore;
  // The draft edits what was asked for (a device limit keeps only the value in use lower; SET-03).
  const requested = Object.fromEntries(FIELDS.map((f) => [f, store.requested(PREF_KEYS[f])])) as unknown as PreferencesData;
  const initial: AppDraft = { ...requested, theme: ctx.ui.theme.value, importText: null, importName: null, resetAll: false };

  const sections: SectionDef<AppDraft>[] = [
    {
      id: 'appearance',
      label: 'Görünüm',
      icon: 'appearance',
      title: 'Görünüm',
      lead: 'Tema, vurgu rengi, yazı tipi, arayüz düzeni, yazı boyutu, artı imleç ve fare yardımcıları.',
      keys: ['theme', 'accent', 'uiFont', 'shell', 'uiScale', 'crosshair', 'cursorInput', 'commandBar', 'hoverInfo', 'startScreen'],
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
      lead: 'Oluşturacağınız her yeni projede başlangıçta önerilecek çalışma modu, çizim yazı tipi ve koordinat sistemi.',
      keys: ['defaultSrid', 'defaultWorkspace', 'defaultDrawingFont'],
      render: (api) => {
        const current = ctx.doc.crs.value;
        const openProject = h('button', { class: 'btn btn--small', type: 'button' }, 'Proje ayarlarını aç');
        openProject.addEventListener('click', () => ctx.commands.execute('file.settings'));
        return [
          group(
            'Çalışma modu',
            workspacePicker({ value: api.draft.defaultWorkspace, compact: true, onChange: (id) => api.set('defaultWorkspace', id, false) }),
          ),
          group('Çizim yazı tipi', drawingFontPicker({ value: api.draft.defaultDrawingFont, onChange: (id) => api.set('defaultDrawingFont', id) })),
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
      lead: 'Çizim alanını ekran kartında çizen arka uç, kenar yumuşatma, çözünürlük, sembol boyutu ve çizgi kalınlığı. Bu cihaza özgüdür.',
      keys: ['rendererPreference', 'msaa', 'hiDpi', 'symbolSize', 'lineWeights'],
      render: (api) => engine(api, ctx),
    },
    {
      id: 'file',
      label: 'Ayar dosyası',
      icon: 'fileOpen',
      title: 'Ayar dosyası',
      lead: 'Ayarları bir dosyaya aktarın, başka bir tarayıcıdan ya da masaüstü uygulamasından alın, varsayılanlara döndürün.',
      keys: [],
      render: (api) => settingsFile(api, ctx, fileState),
    },
  ];

  new SettingsShell<AppDraft>({
    title: 'Uygulama ayarları',
    scope: { icon: 'settings', title: 'Bu tarayıcıda saklanır', detail: 'Tüm projeler için geçerlidir' },
    sections,
    initial,
    defaults: { ...PREFERENCE_DEFAULTS, theme: 'dark', importText: null, importName: null, resetAll: false },
    section,
    onSave: (draft, init) => {
      // An imported file replaces the stored values first; “Varsayılanlara döndür” forgets them.
      if (draft.importText !== null) store.importText(draft.importText);
      else if (draft.resetAll) store.reset();
      const base = Object.fromEntries(FIELDS.map((f) => [f, store.requested(PREF_KEYS[f])])) as unknown as PreferencesData;
      const changed = Object.fromEntries(FIELDS.filter((f) => draft[f] !== base[f]).map((f) => [PREF_KEYS[f], draft[f]]));
      const refused = Object.keys(store.choose(changed));
      if (refused.length) ctx.log.warn(`Kaydedilemeyen ayar: ${refused.join(', ')}. Değerleri denetleyip yeniden deneyin.`);
      if (draft.uiScale !== init.uiScale) applyUiScale(draft.uiScale);
      if (draft.accent !== init.accent) applyAccent(draft.accent);
      if (draft.theme !== init.theme) applyTheme(ctx, draft.theme);
      // The drawing's selection colour follows the accent.
      else if (draft.accent !== init.accent) ctx.view.refreshPalette();
      if (draft.uiFont !== init.uiFont) void applyUiFont(draft.uiFont).then(() => ctx.view.refreshFonts());
      if (draft.defaultSrid !== init.defaultSrid) {
        const c = crsBySrid(draft.defaultSrid)!;
        ctx.log.info(`Yeni projeler ${c.name} (EPSG:${c.srid}) ile oluşturulacak. Açık projenin sistemi değişmedi.`);
      }
      if (draft.defaultWorkspace !== init.defaultWorkspace) {
        ctx.log.info(`Yeni projeler “${workspaceById(draft.defaultWorkspace).label}” çalışma moduyla önerilecek. Açık projenin modu değişmedi.`);
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
      'Vurgu rengi',
      h('p', { class: 'sgroup__note' }, 'Çalışan araç, seçim, odak ve seçili öğeler bu renkle gösterilir; çizimdeki seçim rengi de ona uyar. Uyarılar turuncu, kenet işaretleri yeşil kalır.'),
      accentPicker({ value: d.accent, onChange: (v) => api.set('accent', v) }),
    ),
    group(
      'Yazı tipi',
      h('p', { class: 'sgroup__note' }, 'Menüler, paneller, pencereler ve çizim alanındaki işaret yazıları. Yazı tipleri uygulamayla birlikte gelir, internetten indirilmez; çizimdeki yazı nesneleri etkilenmez.'),
      fontPicker({ value: d.uiFont, onChange: (v) => api.set('uiFont', v) }),
    ),
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
            { value: 'small', label: 'Küçük' },
            { value: 'standard', label: 'Standart' },
            { value: 'large', label: 'Büyük' },
            { value: 'xlarge', label: 'Çok büyük' },
            { value: 'xxlarge', label: 'En büyük' },
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
        'Komut şeridi',
        'Komut çalışırken çizim alanının üstünde aracın adı, beklediği adım ve seçenekleri (kenar sayısı, yöntem, kopya…) düğme olarak gösterilir. Kapalıyken aynı seçenekler alttaki komut satırındadır.',
        toggleSwitch({ label: 'Komut şeridi', checked: d.commandBar, onChange: (v) => api.set('commandBar', v) }),
      ),
      settingRow(
        'Nesne bilgi kartı',
        'Seçim aracında bir nesnenin üzerinde durunca türü, katmanı, uzunluğu ya da alanı gösterilir.',
        toggleSwitch({ label: 'Nesne bilgi kartı', checked: d.hoverInfo, onChange: (v) => api.set('hoverInfo', v) }),
      ),
    ),
    group(
      'Açılış',
      settingRow(
        'Başlangıç ekranı',
        'Uygulama açılınca yeni proje, dosya aç, bulut ve son dosyalar gösterilir. Dosya → Başlangıç ekranı ile her zaman açılır.',
        toggleSwitch({ label: 'Başlangıç ekranı', checked: d.startScreen, onChange: (v) => api.set('startScreen', v) }),
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
          // The steps the typed schema allows (drafting.polarIncrement).
          options: (settingDescriptor('drafting.polarIncrement')?.choices ?? []).map((c) => ({ value: String(c.value), label: c.label })),
          onChange: (v) => api.set('polarIncrement', Number(v)),
        }),
      ),
    ),
    group(
      'Yakalama mesafesi',
      settingRow('Kenet yarıçapı', 'İmlecin bir noktaya yapışması için gereken yakınlık.', stepper({ label: 'Kenet yarıçapı', value: d.snapAperture, ...range('drafting.snapAperture'), unit: 'px', onChange: (v) => api.set('snapAperture', v) })),
      settingRow('Seçim yarıçapı', 'Tıklamanın bir çizgiyi yakalaması için gereken yakınlık.', stepper({ label: 'Seçim yarıçapı', value: d.pickAperture, ...range('drafting.pickAperture'), unit: 'px', onChange: (v) => api.set('pickAperture', v) })),
    ),
  ];
}

/** A number setting's bounds from the typed schema. */
function range(key: string): { min: number; max: number } {
  const d = settingDescriptor(key);
  return { min: d?.min ?? 0, max: d?.max ?? 100 };
}
