/**
 * The settings service in the browser (docs/adr/0023): over localStorage,
 * with `?renderer=webgpu|webgl2` as this session's start preference for the
 * drawing engine (a session override, dropped once the user picks one), and
 * written out when the page goes away.
 */
import type { MessageLog } from '../state';
import { settingErrorMessage } from '../../core/settings/schema';
import { LEGACY_PREFS } from './legacy';
import { SETTINGS_BACKUP, SETTINGS_STORAGE, SettingsStore, type KeyValueStorage } from './store';

export function openBrowserSettings(): SettingsStore {
  let storage: KeyValueStorage | null;
  try {
    storage = window.localStorage;
  } catch {
    storage = null;
  }
  const store = SettingsStore.open(storage);
  const renderer = new URLSearchParams(location.search).get('renderer');
  if (renderer) store.set('graphics.backend', renderer, 'session');
  window.addEventListener('pagehide', () => store.flush());
  return store;
}

/** Says what opening the settings did: the one migration, a recovery, storage refused. */
export function reportSettingsOpen(store: SettingsStore, log: MessageLog): void {
  const { migrated, recovered, memoryOnly } = store.report;
  if (migrated) {
    const left = migrated.dropped.map((d) => d.key);
    log.info(`Uygulama ayarları tipli ayar deposuna taşındı (${migrated.moved} değer). Eski kayıt (${LEGACY_PREFS}) yedek olarak olduğu gibi duruyor.`);
    if (left.length) log.warn(`Taşınamayan ${left.length} değer varsayılanında kaldı: ${left.join(', ')}. Eski kayıtta duruyorlar; Araçlar → Uygulama ayarları'ndan yeniden seçin.`);
  }
  if (recovered?.code) log.warn(`Uygulama ayarları okunamadı: ${settingErrorMessage(recovered.code)} Kayıt ${SETTINGS_BACKUP} olarak saklandı; ayarlar ${migrated ? 'eski kayıttan' : 'varsayılanlardan'} yeniden kuruldu.`);
  else if (recovered) {
    const keys = recovered.diagnostics.map((d) => d.key);
    log.warn(`${keys.length} ayar geçersizdi ve varsayılanına döndü: ${keys.join(', ')}. Önceki kayıt ${SETTINGS_BACKUP} olarak saklandı.`);
  }
  if (memoryOnly) log.warn(`Tarayıcı ayarların saklanmasına izin vermiyor (${SETTINGS_STORAGE}); değişiklikler bu sekme kapanınca kaybolur. Özel pencere dışında açın ya da site verisine izin verin.`);
}
