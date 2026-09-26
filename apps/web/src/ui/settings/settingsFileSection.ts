/**
 * Uygulama ayarları → Ayar dosyası (TODOS.md SET-04, docs/adr/0023): the
 * saved settings as a `kentos.settings` file (the desktop app reads the same
 * file), a file's values taken into the window, every setting back to its
 * default, and where the settings are kept: the one migration from the older
 * store and a recovery's backup. Like the rest of the window this works on the
 * draft: an import or a reset is applied on Kaydet.
 */
import type { AppContext } from '../../app/context';
import { LEGACY_PREFS } from '../../app/settings/legacy';
import { SETTINGS_BACKUP, SETTINGS_STORAGE } from '../../app/settings/store';
import { PREF_KEYS, PREFERENCE_DEFAULTS, type PreferencesData } from '../../app/state';
import { readSettingsFile, resolveSettings } from '../../core/settings/rules';
import { SETTINGS_SCHEMA, settingErrorMessage } from '../../core/settings/schema';
import { h, type Child } from '../dom';
import { saveExport } from '../io/save';
import { note, settingRow } from '../widgets/controls';
import type { AppDraft } from './AppSettingsDialog';
import { group, type DraftApi } from './SettingsShell';

/** The draft's part of an import or a reset, applied on Kaydet. */
export interface FileDraft {
  /** A settings file's text: on Kaydet it replaces the stored values (its values already fill the draft). */
  importText: string | null;
  importName: string | null;
  /** “Varsayılanlara döndür”: on Kaydet the stored values are forgotten. */
  resetAll: boolean;
}

/** What the last file chosen in this window said (kept beside the draft, not in it). */
export interface FileState {
  report: { readonly name: string; readonly error: string | null; readonly left: readonly string[] } | null;
}

const KIND = { description: 'KentOS ayarları', accept: { 'application/json': ['.json'] } };
const FIELDS = Object.keys(PREF_KEYS) as (keyof PreferencesData)[];

const button = (label: string, run: () => void) => {
  const b = h('button', { class: 'btn btn--small', type: 'button' }, label);
  b.addEventListener('click', run);
  return b;
};

export function settingsFile(api: DraftApi<AppDraft>, ctx: AppContext, state: FileState): Child[] {
  const store = ctx.settingsStore;
  const d = api.draft;
  const fill = (values: (f: keyof PreferencesData) => unknown, file: Partial<AppDraft>) => {
    for (const f of FIELDS) api.set(f, values(f) as never, false);
    for (const [k, v] of Object.entries(file)) api.set(k as keyof AppDraft, v as never, false);
    api.set('resetAll', api.draft.resetAll);
  };
  const exportFile = async () => {
    const name = await saveExport(ctx, new TextEncoder().encode(store.exportText()), 'kentos-ayarlar.json', KIND);
    if (name) ctx.log.success(`Uygulama ayarları dışa aktarıldı: ${name}.`);
  };
  const importFile = async () => {
    const picked = await ctx.files.pickForImport(KIND);
    if (!picked) return;
    const text = new TextDecoder().decode(picked.bytes);
    const read = readSettingsFile(text, SETTINGS_SCHEMA);
    if (!read.ok) {
      state.report = { name: picked.name, error: settingErrorMessage(read.code), left: [] };
      api.set('importName', api.draft.importName);
      return;
    }
    // The file's values as they resolve on their own (no session, no device limits): what Kaydet will ask for.
    const r = resolveSettings(SETTINGS_SCHEMA, { user: read.file.user, device: read.file.device });
    state.report = { name: picked.name, error: null, left: read.diagnostics.filter((x) => x.code !== 'unknown_key').map((x) => x.key) };
    fill((f) => r.settings.get(PREF_KEYS[f])?.requested, { importText: text, importName: picked.name, resetAll: false });
  };
  const reset = () => {
    state.report = null;
    fill((f) => PREFERENCE_DEFAULTS[f], { importText: null, importName: null, resetAll: true });
  };

  const report = state.report;
  const pending = d.importText !== null ? note('info', h('b', null, `“${d.importName}” okundu.`), ' Değerleri pencerede; Kaydet ile uygulanır, Vazgeç ile hiçbiri değişmez.') : d.resetAll ? note('info', h('b', null, 'Bütün ayarlar varsayılanlarında.'), ' Kaydet ile saklanan değerler silinir.') : null;
  const problem = report?.error
    ? note('warn', h('b', null, `“${report.name}” alınamadı.`), ` ${report.error}`)
    : report?.left.length
      ? note('warn', `Dosyadaki ${report.left.length} değer geçersiz ve alınmadı: ${report.left.join(', ')}. Bu ayarlar varsayılanında kalır.`)
      : null;
  const migrated = store.migrations.map((m) =>
    note(
      'info',
      `${new Date(m.at).toLocaleString('tr-TR')}: ${m.from} kaydından ${m.moved} değer taşındı; eski kayıt yedek olarak olduğu gibi duruyor.`,
      m.dropped.length ? ` Taşınamayanlar: ${m.dropped.map((x) => x.key).join(', ')}.` : '',
    ),
  );
  const { recovered, memoryOnly } = store.report;
  return [
    group(
      'Dışa ve içe aktarma',
      settingRow('Dışa aktar', 'Kaydedilmiş uygulama ayarlarını bir kentos.settings dosyasına yazar. Masaüstü uygulaması da aynı dosyayı okur.', button('Dışa aktar…', () => void exportFile())),
      settingRow('İçe aktar', 'Bir ayar dosyasının değerleri bu pencereye gelir; Kaydet ile uygulanır. Geçersiz değerler alınmaz ve adıyla söylenir.', button('İçe aktar…', () => void importFile())),
      pending,
      problem,
    ),
    group('Varsayılanlar', settingRow('Varsayılanlara döndür', 'Bütün bölümlerdeki ayarlar varsayılanına döner; Kaydet ile saklanan değerler silinir.', button('Varsayılanlara döndür', reset))),
    group(
      'Kayıt',
      note('info', `Ayarlar bu tarayıcıda ${SETTINGS_STORAGE} anahtarında saklanır. Çizim motoru ayarları bu cihaza özgüdür; öbürleri sizin tercihinizdir.`),
      ...migrated,
      recovered ? note('warn', `Açılışta kayıt okunamadı ya da geçersiz değer içeriyordu; önceki metin ${SETTINGS_BACKUP} anahtarında saklandı.`) : null,
      memoryOnly || store.inMemory ? note('warn', `Tarayıcı ayarların saklanmasına izin vermiyor; değişiklikler bu sekme kapanınca kaybolur. Eski kayıt: ${LEGACY_PREFS}.`) : null,
    ),
  ];
}
