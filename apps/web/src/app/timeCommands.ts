// Zaman and Senaryo's commands (docs/adr/0210 §10; the desktop's `time_commands.rs`): the slider, Zaman ayarları,
// Zamanı karşılaştır, and the scenarios. The windows load on first use (CLAUDE.md §20); Yeni sürüm oluştur and Sona
// erdir are tools (tools/catalog.ts).

import type { LayerNode } from '../model/layers';
import { layerTimes, showTime, timePosition, timeValues } from '../model/time';
import { confirmDialog } from '../ui/widgets/confirm';
import type { AppContext } from './context';
import { applyScenario, pairsOf, scenarioAtHand, scenarioGroups, showBase, showScenario, shownScenario } from './scenarios';

const failed = (ctx: AppContext) => (e: Error) => ctx.log.error(`Pencere yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`);

/** The temporal layer a command means: the active one when it is, else the first temporal layer shown. */
export function temporalAtHand(ctx: AppContext): LayerNode | undefined {
  const layers = ctx.doc.layers;
  const active = layers.get(layers.active.value);
  if (active?.time) return active;
  return layers.leaves().find((l) => l.time && layers.isVisible(l.id));
}

/**
 * Zamanı karşılaştır's dates (docs/adr/0210 §8): with the slider open, the position before the one shown and that one;
 * closed, the layer's own times' first and last.
 */
function compareDates(ctx: AppContext, layer: LayerNode): [string, string] | null {
  const t = ctx.time;
  if (t.open.value) {
    const unit = t.step.value.unit;
    const k = t.position.value;
    return [showTime(timePosition(t.anchor.value, t.step.value, Math.max(0, k - 1)), unit), showTime(timePosition(t.anchor.value, t.step.value, k), unit)];
  }
  const rule = layer.time!;
  const list = ctx.doc.byLayer(layer.id);
  const { summary } = layerTimes(rule.end != null, !!rule.cumulative, timeValues(rule, list.map((e) => e.attrs)));
  return summary.extent ? [showTime(summary.extent[0], 'day'), showTime(summary.extent[1], 'day')] : null;
}

export function registerTimeCommands(ctx: AppContext): void {
  const { time, doc, log } = ctx;
  const hasScenarios = () => scenarioGroups(doc.layers).length > 0;
  const noScenario = () => (hasScenarios() ? null : 'Projede senaryo yok; önce Senaryo oluştur ile bir senaryo yapın.');
  ctx.commands.registerAll([
    {
      id: 'time.slider',
      title: 'Zaman sürgüsü',
      category: 'Harita',
      icon: 'timeSlider',
      description: 'Zamansal katmanları bir anda ya da aralıkta gösterir: çizim alanının altındaki sürgüyle adım adım ilerler, oynatır. Çizim değişmez.',
      aliases: ['ZAMAN', 'ZAMANSURGUSU', 'TIMESLIDER'],
      isChecked: () => time.open.value,
      watch: [time.open],
      run: () => {
        const why = time.toggle();
        if (why) log.warn(why);
      },
    },
    {
      id: 'time.layer',
      title: 'Zaman ayarları…',
      short: 'Zaman ayarları',
      category: 'Harita',
      icon: 'timeLayer',
      description: 'Katmanın nesnelerinin başlangıç, bitiş ve kimlik alanlarını seçer; katmanı zamansal yapar ya da zamanını kaldırır.',
      aliases: ['ZAMANAYARLARI', 'ZAMANAYAR'],
      run: (args) =>
        void import('../ui/time/TimeLayerDialog').then((m) => m.openTimeLayer(ctx, typeof args === 'string' ? args : undefined), failed(ctx)),
    },
    {
      id: 'time.compare',
      title: 'Zamanı karşılaştır…',
      short: 'Zamanı karşılaştır',
      category: 'Harita',
      icon: 'timeCompare',
      description: 'Zamansal bir katmanın iki tarihteki hâlini Veri karşılaştır ile karşılaştırır: eklenen, silinen ve değişen nesneler.',
      aliases: ['ZAMANKARSILASTIR'],
      run: () => {
        const layer = temporalAtHand(ctx);
        if (!layer) return void log.warn('Zamansal katman yok; önce bir katmana Zaman ayarları’ndan başlangıç alanı verin.');
        const dates = compareDates(ctx, layer);
        if (!dates) return void log.warn(`“${layer.name}” katmanında zamanı olan nesne yok.`);
        void import('../ui/data/DataCompareDialog').then(
          (m) => m.openDataCompare(ctx, { oldNode: layer.id, newNode: layer.id, oldDate: dates[0], newDate: dates[1], ...(layer.time?.key && { key: layer.time.key }) }),
          failed(ctx),
        );
      },
    },
    {
      id: 'scenario.create',
      title: 'Senaryo oluştur…',
      short: 'Senaryo oluştur',
      category: 'Harita',
      icon: 'scenarioCreate',
      description: 'Seçilen ana katmanların kopyalarıyla bir senaryo grubu yapar: öneriler Mevcut durum’dan ayrı çizilir, gösterilir ve karşılaştırılır.',
      aliases: ['SENARYO', 'SENARYOOLUSTUR'],
      run: () => void import('../ui/time/ScenarioDialog').then((m) => m.openScenarioCreate(ctx), failed(ctx)),
    },
    {
      id: 'scenario.show',
      title: 'Senaryoyu göster',
      category: 'Harita',
      icon: 'scenarioShow',
      description: 'Seçili ya da etkin katmanın senaryosunu gösterir: senaryonun katmanları görünür, yerine geçtiği ana katmanlar gizlenir.',
      aliases: ['SENARYOGOSTER'],
      isEnabled: hasScenarios,
      whyDisabled: noScenario,
      watch: [doc.layers.version],
      run: (args) => {
        const id = typeof args === 'string' ? args : scenarioAtHand(ctx)?.id;
        if (id) showScenario(ctx, id);
      },
    },
    {
      id: 'scenario.base',
      title: 'Mevcut durum',
      category: 'Harita',
      icon: 'scenarioBase',
      description: 'Senaryoları gizler, yerine geçtikleri ana katmanları gösterir.',
      aliases: ['MEVCUTDURUM'],
      isEnabled: hasScenarios,
      whyDisabled: noScenario,
      isChecked: () => hasScenarios() && shownScenario(doc.layers).kind === 'base',
      watch: [doc.layers.version],
      run: () => showBase(ctx),
    },
    {
      id: 'scenario.compare',
      title: 'Senaryoyu karşılaştır…',
      short: 'Senaryoyu karşılaştır',
      category: 'Harita',
      icon: 'scenarioCompare',
      description: 'Ana katmanı senaryodaki karşılığıyla Veri karşılaştır ile karşılaştırır.',
      aliases: ['SENARYOKARSILASTIR'],
      isEnabled: hasScenarios,
      whyDisabled: noScenario,
      watch: [doc.layers.version],
      run: (args) => {
        const layers = doc.layers;
        const group = typeof args === 'string' ? layers.get(args) : scenarioAtHand(ctx);
        if (!group) return;
        const pairs = pairsOf(layers, group.id);
        const active = layers.active.value;
        const pair = pairs.find(([b, l]) => b === active || l === active) ?? pairs[0];
        if (!pair) return void log.warn(`“${group.name}” senaryosunda bir ana katmanın yerine geçen katman yok; karşılaştırılacak bir şey yok.`);
        void import('../ui/data/DataCompareDialog').then((m) => m.openDataCompare(ctx, { oldNode: pair[0], newNode: pair[1] }), failed(ctx));
      },
    },
    {
      id: 'scenario.apply',
      title: 'Senaryoyu uygula…',
      short: 'Senaryoyu uygula',
      category: 'Harita',
      icon: 'scenarioApply',
      description: 'Senaryonun katmanlarını ana katmanlara yazar: ana katmanların nesneleri senaryodakilerle değişir, senaryo grubu sıradan gruba döner. Tek adımda geri alınır.',
      aliases: ['SENARYOUYGULA'],
      isEnabled: hasScenarios,
      whyDisabled: noScenario,
      watch: [doc.layers.version],
      run: (args) => {
        const layers = doc.layers;
        const group = typeof args === 'string' ? layers.get(args) : scenarioAtHand(ctx);
        if (!group) return;
        const pairs = pairsOf(layers, group.id);
        const objects = pairs.reduce((n, [b]) => n + doc.byLayer(b).length, 0);
        void confirmDialog({
          title: 'Senaryoyu uygula',
          message: `“${group.name}” senaryosu Mevcut durum’a yazılsın mı?`,
          details: [
            ...pairs.map(([b, l]) => `“${layers.path(b)}”: ${doc.byLayer(b).length} nesne silinir, “${layers.get(l)?.name ?? l}” katmanının ${doc.byLayer(l).length} nesnesi gelir.`),
            ...(pairs.length ? [] : ['Senaryoda bir ana katmanın yerine geçen katman yok; grubu sıradan gruba döner.']),
            `Toplam ${objects} nesne silinir; tek adımda geri alınır.`,
          ],
          answers: [
            { value: 'cancel', label: 'Vazgeç' },
            { value: 'apply', label: 'Uygula', kind: 'primary' },
          ],
          cancel: 'cancel',
        }).then((a) => {
          if (a === 'apply') applyScenario(ctx, group.id);
        });
      },
    },
  ]);
}
