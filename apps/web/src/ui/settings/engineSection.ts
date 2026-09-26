/**
 * Uygulama ayarları → Çizim motoru (docs/adr/0023; TODOS.md SET-03, SET-05,
 * AA-01): the backend, the graphics presets that only fill values,
 * multisampling and the pixel ratio, each with the value asked for and the
 * value in use, and why they differ. The values come from the typed schema.
 */
import type { AppContext } from '../../app/context';
import type { ResolvedSetting } from '../../contracts/generated/ResolvedSetting';
import { matchingPreset } from '../../core/settings/rules';
import { resolveReasonMessage, settingDescriptor, SETTINGS_SCHEMA } from '../../core/settings/schema';
import { webgpuSupported } from '../../render/webgpu/support';
import { h, type Child } from '../dom';
import { note, segmented, settingRow, toggleSwitch } from '../widgets/controls';
import type { AppDraft } from './AppSettingsDialog';
import { group, type DraftApi } from './SettingsShell';

/** What the draft's graphics keys are called in the schema. */
const GRAPHICS: Readonly<Record<string, 'msaa' | 'hiDpi'>> = { 'graphics.msaa': 'msaa', 'graphics.hiDpi': 'hiDpi' };

/** A value as the schema labels it (“4×”, “WebGPU”). */
const choiceLabel = (key: string, value: unknown) => settingDescriptor(key)?.choices?.find((c) => c.value === value)?.label ?? String(value);
const samplesLabel = (n: number) => choiceLabel('graphics.msaa', n);

/** The value in use against the one asked for: a line when equal, a warning with the reason when not. */
export function effectiveNote(r: ResolvedSetting, label: (v: ResolvedSetting['effective']) => string, detail: string): HTMLElement {
  if (r.effective === r.requested) return note('info', `Kullanılan: ${label(r.effective)}. ${detail}`);
  const why = r.reason ? resolveReasonMessage(r.reason) : '';
  return note('warn', h('b', null, `İstenen ${label(r.requested)}, kullanılan ${label(r.effective)}.`), ` ${why}${r.detail ? ` ${r.detail}` : ''}`);
}

export function engine(api: DraftApi<AppDraft>, ctx: AppContext): Child[] {
  const d = api.draft;
  const store = ctx.settingsStore;
  const gpu = webgpuSupported();
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
  const backend = store.preview('graphics.backend', d.rendererPreference);
  const msaa = store.preview('graphics.msaa', d.msaa);
  const msaaDesc = settingDescriptor('graphics.msaa')!;
  const hiDpiDesc = settingDescriptor('graphics.hiDpi')!;
  const running = ctx.view.backendLabel.value;
  const preset = matchingPreset(SETTINGS_SCHEMA, 'graphics', (key) => (GRAPHICS[key] ? d[GRAPHICS[key]] : undefined));
  const supported = backend.effective === ctx.view.backendKind.value ? ctx.view.sampleCounts : [];
  // Memory of the drawing's colour targets at this window's size: the multisampled one and the two kept copies.
  const dpr = d.hiDpi ? window.devicePixelRatio || 1 : 1;
  const pixels = ctx.view.camera.width * ctx.view.camera.height * dpr * dpr;
  const megabytes = Math.max(1, Math.round((pixels * 4 * (Number(msaa.effective) + 2)) / 1e6));
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
      effectiveNote(backend, (v) => choiceLabel('graphics.backend', v), `Şu an çalışan: ${running}. Seçim Kaydet ile hemen uygulanır; motor başlatılamazsa WebGL2’ye dönülür.`),
    ),
    group(
      'Kenar yumuşatma ve çözünürlük',
      settingRow(
        'Hazır ayar',
        'Hızlı, Dengeli ve Kaliteli yalnız aşağıdaki iki değeri doldurur; her biri ayrıca değiştirilebilir. Çizimin kaydını ve hassasiyetini değiştirmez.',
        segmented({
          label: 'Grafik hazır ayarı',
          options: [
            ...SETTINGS_SCHEMA.presets.filter((p) => p.group === 'graphics').map((p) => ({ value: p.id, label: p.title, hint: p.description })),
            { value: 'custom', label: 'Özel', disabled: true, hint: 'Değerler hiçbir hazır ayara uymuyor.' },
          ],
          value: preset?.id ?? 'custom',
          onChange: (id) => {
            const values = SETTINGS_SCHEMA.presets.find((p) => p.id === id)?.values ?? {};
            for (const [key, value] of Object.entries(values)) if (GRAPHICS[key]) api.set(GRAPHICS[key], value as never, false);
            api.set('msaa', api.draft.msaa);
          },
        }),
      ),
      settingRow(
        msaaDesc.title,
        msaaDesc.description,
        segmented({
          label: msaaDesc.title,
          options: (msaaDesc.choices ?? []).map((c) => {
            const n = c.value as number;
            const missing = supported.length > 0 && !supported.includes(n);
            return { value: String(n), label: c.label, hint: missing ? `${running} bu sayıyı desteklemiyor; istenirse desteklenen en yakın alt değer kullanılır.` : undefined };
          }),
          value: String(d.msaa),
          onChange: (v) => api.set('msaa', Number(v)),
        }),
      ),
      effectiveNote(msaa, (v) => samplesLabel(Number(v)), supported.length ? `${running}: ${supported.map(samplesLabel).join(', ')}.` : ''),
      settingRow(hiDpiDesc.title, hiDpiDesc.description, toggleSwitch({ label: hiDpiDesc.title, checked: d.hiDpi, onChange: (v) => api.set('hiDpi', v) })),
      note('info', `Çizim hedefleri bu pencere boyutunda yaklaşık ${megabytes} MB ekran belleği tutar (${d.hiDpi && dpr > 1 ? `${dpr}× piksel yoğunluğu, ` : ''}${samplesLabel(Number(msaa.effective)).toLocaleLowerCase('tr')} kenar yumuşatma). Değişiklik Kaydet ile hemen uygulanır; çizim ya da pencere yeniden açılmaz.`),
    ),
    group(
      'Semboller ve çizgiler',
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
      settingRow(
        'Çizgi kalınlığı',
        'Katman çizgileri kalınlıklarıyla çizilir. Kapalıyken hepsi ince çizilir (durum çubuğunda Kalınlık).',
        toggleSwitch({ label: 'Çizgi kalınlığını göster', checked: d.lineWeights, onChange: (v) => api.set('lineWeights', v) }),
      ),
    ),
  ];
}
