import { ENTITY_KIND_LABEL, type EntityKind } from '../../model/entities';
import { WORKER_THRESHOLD } from '../../processing/runner';
import type { ExecutionTarget, FeaturesValue } from '../../processing/types';
import { pickObjectsPrompt } from '../../tools/pickObjectsTool';
import { TARGET_SHORT } from './targets';

/**
 * The processing dialog's words (ToolDialog, paramFields and their
 * rules, dialogPlan and fieldPlan); fixtures/processing/v1/dialog.json
 * holds them for the desktop.
 */

type ListedScope = Exclude<FeaturesValue['scope'], 'ids'>;

export const DIALOG_TEXTS = {
  sections: { input: 'Girdi', main: 'Ayarlar', output: 'Çıktı', advanced: 'Gelişmiş ayarlar' },
  optional: 'isteğe bağlı',
  footer: {
    run: 'Çalıştır',
    running: 'Çalışıyor…',
    close: 'Kapat',
    stop: 'Durdur',
    reset: 'Varsayılanlar',
    resetTitle: 'Bütün alanları varsayılan değerlerine döndürür',
  },
  actions: { zoom: 'Seçime yakınlaştır', select: 'Sonuçları seç', undo: 'Geri al' },
  side: {
    preview: 'Önizleme',
    targets: 'Nerede çalışır',
    aliases: 'Komut satırından',
    steps: 'Adımlar',
    editModel: 'Modeli düzenle',
    editCopy: 'Kopyasını düzenle',
    models: 'Modeller',
    noCategory: 'Genel',
  },
  previewBlocked: 'Önizleme için alanları düzeltin.',
  running: { here: 'Çalışıyor…', worker: 'Arka planda çalışıyor; sayfayı kullanmaya devam edebilirsiniz.' },
  fixFields: (n: number) => `Çalıştırmadan önce ${n} alanı düzeltin.`,
  targets: {
    auto: 'Otomatik',
    autoNow: (t: ExecutionTarget) => `şimdi: ${TARGET_SHORT[t]}`,
    autoModel: 'adım adım',
    hint: `${WORKER_THRESHOLD.toLocaleString('tr-TR')} ya da daha çok nesneli işler arka planda çalışır; sayfa donmaz.`,
    thisRun: 'bu çalıştırmada',
    soon: 'yakında',
  },
  features: {
    kinds: 'Türler',
    kindOff: 'Bu türü dışarıda bırak',
    kindOn: 'Bu türü de al',
    kindsNote: (kinds: readonly EntityKind[]) => `Uygun nesneler: ${kinds.map((k) => ENTITY_KIND_LABEL[k].toLocaleLowerCase('tr-TR')).join(', ')}`,
    noLayer: '—',
  },
  layer: {
    newHeader: 'Yeni katman',
    existingHeader: 'Mevcut katmanlar',
    newItem: (name: string) => `Yeni: ${name}`,
    suggested: 'Yeni katman',
    locked: 'kilitli',
    existing: '(mevcut)',
    fresh: '(yeni)',
  },
  point: { none: 'Henüz seçilmedi', show: 'Sahneden seç', again: 'Yeniden seç' },
  /** Sahneden seç beside a choice that picks a point, and beside the input objects' scope (docs/adr/0088). */
  pick: {
    choice: 'Sahneden seç',
    choiceTip: 'Başlangıcı çizimde gösterin: her nesnede o noktaya en yakın köşeden başlanır.',
    objects: 'Sahneden seç',
    objectsTip: 'Nesneleri çizimde tıklayarak ya da pencereyle seçin; Enter bitirir, Esc vazgeçer.',
    prompt: pickObjectsPrompt,
    picked: (n: number) => `${n} nesne seçildi.`,
  },
  field: {
    none: 'Bu nesnelerde öznitelik alanı yok',
    choose: 'Alan seçin',
    placeholder: 'Alan adı',
    count: (n: number) => `${n} nesne`,
    has: (n: number) => `${n} nesnede var; değeri değişir.`,
    fresh: 'Yeni alan: nesnelere eklenir.',
    missing: 'Bu nesnelerde böyle bir alan yok.',
  },
  expression: {
    fields: 'Alanlar',
    variables: 'Değişkenler',
    functions: 'İşlevler',
    chipTitle: (n: number) => `${n} nesnede var; ifadeye ekle`,
    more: (n: number) => `+${n}`,
  },
} as const;

/** The scopes' names on the features field's segmented control. */
export const SCOPE_SHORT: Record<ListedScope, string> = { selection: 'Seçili', visible: 'Görünen', all: 'Tümü', layer: 'Katman' };
