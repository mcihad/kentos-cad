import type { Command } from '../core/commands';
import type { BackendKind } from '../render/types';
import { webgpuSupported } from '../render/webgpu/support';
import type { Entity } from '../model/entities';
import { entitiesDelete } from '../product/entitiesDelete';
import { treeLocked } from '../ui/layers/treeRights';
import { CoordinateReadTool } from '../tools/coordinateTool';
import { pasteEntities, PasteTool } from '../tools/editTools';
import { Signal, type ReadonlySignal } from '../core/signal';
import { TOOL_GROUP_LABEL, type ConfirmMods } from '../tools/Tool';
import type { AppContext } from './context';
import type { ThemeId } from './appearance';
import { PREF_KEYS, type OverlapMode } from './state';
import { settingDescriptor } from '../core/settings/schema';
import { checkExtent } from './extentCheck';
import { effectiveWorkspace, WORKSPACES, type WorkspaceSpec } from './workspaces';
import { lockCommands } from './lockCommands';

/** Features that exist in the menu but are not built yet say so plainly. */
function pending(ctx: AppContext, id: string, title: string, category: string, icon?: string): Command {
  return {
    id,
    title,
    category,
    icon,
    pending: true,
    run: () => ctx.log.warn(`“${title}” bu sürümde henüz kullanılamıyor.`),
  };
}

function toggle(id: string, title: string, s: Signal<boolean>, opts: Partial<Command> = {}): Command {
  return { id, title, run: () => s.set(!s.value), isChecked: () => s.value, watch: [s], ...opts };
}

/**
 * The snap kinds' toggles (docs/adr/0163 §6), each its preference, named and described as the settings are; the
 * menu's icon is named after the preference. Uç nokta takes Çeyrek with it, as the settings do.
 */
const SNAP_KIND_COMMANDS: readonly [kind: string, pref: SnapPref, aliases: string[]][] = [
  ['endpoint', 'snapEndpoint', []],
  ['midpoint', 'snapMidpoint', []],
  ['center', 'snapCenter', []],
  ['node', 'snapNode', []],
  ['intersection', 'snapIntersection', []],
  ['perpendicular', 'snapPerpendicular', []],
  ['tangent', 'snapTangent', []],
  ['nearest', 'snapNearest', []],
  ['centroid', 'snapCentroid', ['AGIRLIKMERKEZI']],
  ['extension', 'snapExtension', ['UZANTI']],
  ['parallel', 'snapParallel', ['PARALELKENET']],
  ['grid', 'snapGrid', ['KARELAJ']],
];

type SnapPref = 'snapEndpoint' | 'snapMidpoint' | 'snapCenter' | 'snapNode' | 'snapIntersection' | 'snapPerpendicular' | 'snapTangent' | 'snapNearest' | 'snapCentroid' | 'snapExtension' | 'snapParallel' | 'snapGrid';

function snapKindCommands(ctx: AppContext): Command[] {
  return SNAP_KIND_COMMANDS.map(([kind, pref, aliases]) => {
    const setting = settingDescriptor(PREF_KEYS[pref]);
    const title = setting?.title ?? kind;
    return toggle(`draft.snap.${kind}`, `Kenet: ${title}`, ctx.prefs[pref], {
      short: title,
      category: 'Çizim yardımcıları',
      icon: pref,
      description: setting?.description,
      ...(aliases.length ? { aliases } : {}),
    });
  });
}

/** One of the overlap control's modes as a radio command (docs/adr/0162 §1); a mode that avoids is the cell's next. */
function overlapMode(ctx: AppContext, mode: OverlapMode, title: string, icon: string, description: string): Command {
  const s = ctx.settings;
  return {
    id: `draft.overlap.${mode}`,
    title,
    category: 'Çizim yardımcıları',
    icon,
    description,
    run: () => {
      s.overlap.set(mode);
      if (mode !== 'allow') s.overlapLast.set(mode);
    },
    isChecked: () => s.overlap.value === mode,
    watch: [s.overlap],
  };
}

/**
 * A project type as a radio command (docs/adr/0165). Choosing one changes the
 * project setting (the project is unsaved afterwards, like any project
 * setting); an announced type says “Yakında” and changes nothing.
 */
function workspace(ctx: AppContext, w: WorkspaceSpec): Command {
  const setting = () => ctx.doc.settings.workspace;
  const soon = w.status === 'soon';
  return {
    id: `workspace.${w.id}`,
    title: w.label,
    category: 'Görünüm',
    icon: w.icon,
    description: w.description,
    aliases: [`MOD${w.label.replace(/\s+/g, '')}`],
    pending: soon || undefined,
    pendingNote: soon ? 'Yakında' : undefined,
    run: () => {
      if (soon) {
        ctx.log.info(`“${w.label}” proje türü yakında geliyor. Şimdilik CAD ya da CBS kullanın.`);
        return;
      }
      if (setting().value === w.id) return;
      setting().set(w.id);
      ctx.log.info(`Proje türü: ${w.label}. Şeridinde olmayan komutlar komut satırından ve kısayoluyla yine çalışır.`);
    },
    isChecked: () => effectiveWorkspace(setting().value).id === w.id,
    watch: [setting()],
  };
}

/**
 * Tam ekran: the whole app (not only the drawing) fills the screen; Esc or
 * the same command leaves it. The browser's F11 does the same on its own.
 */
function fullscreen(): Command {
  const on = new Signal(!!document.fullscreenElement);
  // The app's one global listener of this kind; it lives as long as the page.
  document.addEventListener('fullscreenchange', () => on.set(!!document.fullscreenElement));
  return {
    id: 'view.fullscreen',
    title: 'Tam ekran',
    category: 'Görünüm',
    icon: 'fullscreen',
    description: 'Uygulamayı ekranın tamamına yayar; Esc ya da aynı düğme çıkar.',
    aliases: ['TAMEKRAN', 'FULLSCREEN'],
    run: () => {
      if (document.fullscreenElement) void document.exitFullscreen();
      else void document.documentElement.requestFullscreen?.().catch(() => undefined);
    },
    isEnabled: () => document.fullscreenEnabled !== false,
    isChecked: () => on.value,
    watch: [on],
  };
}

/** Drawing engine choice: remembered in preferences, applied live by the viewport. */
function renderer(ctx: AppContext, kind: BackendKind, title: string, description: string): Command {
  return {
    id: `view.renderer.${kind}`,
    title,
    category: 'Görünüm',
    icon: kind === 'webgpu' ? 'rendererWebgpu' : 'rendererWebgl2',
    description,
    aliases: [kind.toUpperCase()],
    run: () => {
      // The viewport follows the preference's effective value; a device that cannot draw with it says why.
      ctx.prefs.rendererPreference.set(kind);
      const r = ctx.settingsStore.resolved('graphics.backend');
      if (r.effective !== kind) ctx.log.warn(`${title} kullanılamıyor${r.detail ? `: ${r.detail}` : ''}. ${String(r.effective).toUpperCase()} ile çiziliyor.`);
    },
    isEnabled: () => kind !== 'webgpu' || webgpuSupported(),
    isChecked: () => ctx.view.backendKind.value === kind,
    watch: [ctx.view.backendKind],
  };
}

/** Chooses the theme: kept as the setting `appearance.theme` (docs/adr/0126); createApp shows it as it changes. */
export function applyTheme(ctx: AppContext, theme: ThemeId): void {
  ctx.prefs.theme.set(theme);
}

/**
 * Shows the theme in use: the tokens (tokens.css) and the drawing's
 * palette, which theme subscribers (layer swatches) read after it.
 */
export function showTheme(ctx: AppContext): void {
  document.documentElement.dataset.theme = ctx.prefs.theme.value;
  // A CAD project's drawing is on slate whatever the theme (docs/adr/0165 §5).
  if (ctx.format.axes === 'cad') document.documentElement.dataset.canvas = 'slate';
  else delete document.documentElement.dataset.canvas;
  ctx.view.refreshPalette();
}

export interface CommandHooks {
  openShortcuts: () => void;
  openAbout: () => void;
  /** Uygulama ayarları — user preferences, stored in the browser. */
  openAppSettings: (section?: string) => void;
  /** Proje ayarları — CRS, units, plot scale; stored in the project file. */
  openProjectSettings: (section?: string) => void;
  /** Yeni proje — an empty drawing with the standard layers, a CRS and a plot scale. */
  openNewProject: () => void;
  focusCommandLine: () => void;
  /** Komut ara — the ribbon's search box. */
  searchCommands: () => void;
  /** Klavye ipuçları — letters over the ribbon's tabs and controls. */
  keyTips: () => void;
  /** Başlangıç ekranı — new, open, cloud and the recent files. */
  openStart: () => void;
  /**
   * A history in front of the drawing's: a sheet's while it is in front (app/sheet/install.ts). Geri al and
   * Yinele, from the quick access bar and their keys, go to it then (docs/sheet/integration.md §3, W-10);
   * set again whenever what it can do changes.
   */
  frontHistory?: ReadonlySignal<FrontHistory | null>;
  /**
   * What prints while something is in front of the drawing: a sheet's Yazdır while it is in front
   * (app/sheet/install.ts). Yazdır ve pafta (`file.print`, Ctrl+P) runs it then, and is not “Geliştirme
   * aşamasında” meanwhile (docs/sheet/integration.md §3, W-17); null: the drawing's own print, not here yet.
   */
  frontPrint?: ReadonlySignal<(() => void) | null>;
}

/** What Geri al and Yinele ask of a history in front of the drawing's. */
export interface FrontHistory {
  undo(): void;
  redo(): void;
  canUndo(): boolean;
  canRedo(): boolean;
}

export function registerCoreCommands(ctx: AppContext, hooks: CommandHooks): void {
  const { commands, doc, selection, settings, ui, view, tools, log } = ctx;
  const selected = () => [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
  const F = 'Dosya';
  const print = pending(ctx, 'file.print', 'Yazdır ve pafta çıktısı…', F, 'print');
  const E = 'Düzen';
  const V = 'Görünüm';
  const K = 'Koordinat';
  const A = 'Analiz';
  const M = 'Harita';

  commands.registerAll([
    // Dosya
    {
      id: 'file.new',
      title: 'Yeni proje…',
      category: F,
      icon: 'fileNew',
      description:
        'Boş bir çizim açar: standart katman ağacı, seçilen koordinat sistemi ve çizim ölçeği. Kaydedilmemiş değişiklikler varsa önce sorar; açık bulut projesi kapanır.',
      aliases: ['YENI', 'NEW'],
      run: () => hooks.openNewProject(),
      isEnabled: () => !ctx.files.busy.value,
      watch: [ctx.files.busy],
    },
    {
      id: 'file.open',
      title: 'Aç…',
      category: F,
      icon: 'fileOpen',
      description: 'Bir KentOS çizimi (.kcad) açar; kaydedilmemiş değişiklikler varsa önce sorar.',
      aliases: ['AC', 'OPEN'],
      run: () => void ctx.files.open(),
      isEnabled: () => !ctx.files.busy.value,
      watch: [ctx.files.busy],
    },
    {
      id: 'file.start',
      title: 'Başlangıç ekranı…',
      short: 'Başlangıç',
      category: F,
      icon: 'history',
      description: 'Yeni proje, dosya aç, bulut projeleri ve son açılan ya da kaydedilen dosyalar.',
      aliases: ['BASLANGIC', 'SON', 'RECENT'],
      run: hooks.openStart,
    },
    {
      id: 'file.save',
      title: 'Kaydet',
      category: F,
      icon: 'save',
      description:
        'Bulut projesinde bekleyen değişiklikleri hemen gönderir (zaten kendiliğinden kaydedilir). Bulut dosya projesinde yeni bir revizyon yazar. Yerel çizimi .kcad dosyasına yazar; dosya yazılamazsa değişiklikler kaydedilmemiş sayılır.',
      aliases: ['KAYDET', 'SAVE'],
      run: () => {
        if (!ctx.cloud.project.value) return void ctx.files.save();
        // A file project: a new revision on the one the drawing came from (docs/adr/0038).
        const file = ctx.cloud.file.value;
        const state = ctx.cloud.sync.value?.state.value ?? file?.endedBy ?? undefined;
        if (file && !file.endedBy) return void ctx.cloud.saveFile();
        if (state === 'deleted' || state === 'revoked') {
          // Nothing can be saved to a deleted project, or one this account may not reach: the next useful step is a local file.
          ctx.log.warn(
            state === 'deleted'
              ? 'Bu bulut projesi silindi; çizim buluta kaydedilemez. Yerel bir dosyaya kaydedin.'
              : 'Bu bulut projesine erişiminiz kaldırıldı; çizim buluta kaydedilemez. Yerel bir dosyaya kaydedin.',
          );
          return void ctx.files.saveAs();
        }
        if (state === 'archived') {
          // Nothing is saved to an archived project (docs/adr/0028): the drawing goes to a local file and leaves the project.
          ctx.log.warn('Bu bulut projesi arşivlenmiş; çizim buluta kaydedilemez. Yerel bir dosyaya kaydedin.');
          return void ctx.files.saveAs().then((saved) => saved && (ctx.cloud.sync.value?.state.value ?? ctx.cloud.file.value?.endedBy) === 'archived' && ctx.cloud.detach());
        }
        void ctx.cloud.flush().then((ok) =>
          ok ? ctx.log.success('Buluta kaydedildi.') : ctx.log.warn('Bulut kaydı tamamlanamadı; durum çubuğundaki kayıt durumuna bakın.'),
        );
      },
      isEnabled: () => !ctx.files.busy.value,
      watch: [ctx.files.busy],
    },
    {
      id: 'file.saveAs',
      title: 'Farklı kaydet…',
      category: F,
      icon: 'saveAs',
      description: 'Çizimi yeni bir .kcad dosyasına yazar ve bundan sonra oraya kaydeder.',
      aliases: ['FARKLIKAYDET', 'SAVEAS'],
      run: () => void ctx.files.saveAs(),
      isEnabled: () => !ctx.files.busy.value,
      watch: [ctx.files.busy],
    },
    pending(ctx, 'file.export.pdf', 'PDF pafta…', F, 'exportPdf'),
    {
      ...print,
      short: 'Yazdır',
      // A sheet in front prints itself (W-17); the drawing's own print is not here yet.
      get pending() {
        return !hooks.frontPrint?.value;
      },
      run: (args?: unknown) => (hooks.frontPrint?.value ?? print.run)(args),
      watch: hooks.frontPrint ? [hooks.frontPrint] : [],
    },
    { id: 'file.settings', title: 'Proje ayarları…', category: F, icon: 'folder', aliases: ['PROJE'], run: () => hooks.openProjectSettings() },

    // Düzen
    {
      id: 'edit.undo',
      title: 'Geri al',
      category: E,
      icon: 'undo',
      run: () => {
        const front = hooks.frontHistory?.value;
        if (front) return front.undo();
        // While a command runs, its own newest step goes first (docs/adr/0018); so does the point calculator's,
        // over a waiting grip too.
        if ((tools.activeId.value !== 'select' || tools.nested) && tools.active.undoStep?.()) return;
        const label = doc.undo();
        if (label) log.info(`Geri alındı: ${label}`);
        selection.retain((id) => !!doc.get(id));
      },
      isEnabled: () => hooks.frontHistory?.value?.canUndo() ?? (doc.canUndo.value || (tools.activeId.value !== 'select' && (tools.active.pointCount ?? 0) > 0)),
      // The prompt changes with every point a command takes.
      watch: [doc.canUndo, tools.prompt, ...(hooks.frontHistory ? [hooks.frontHistory] : [])],
    },
    {
      id: 'edit.redo',
      title: 'Yinele',
      category: E,
      icon: 'redo',
      run: () => {
        const front = hooks.frontHistory?.value;
        if (front) return front.redo();
        const label = doc.redo();
        if (label) log.info(`Yinelendi: ${label}`);
        selection.retain((id) => !!doc.get(id));
      },
      isEnabled: () => hooks.frontHistory?.value?.canRedo() ?? doc.canRedo.value,
      watch: [doc.canRedo, ...(hooks.frontHistory ? [hooks.frontHistory] : [])],
    },
    {
      id: 'edit.cut',
      title: 'Kes',
      category: E,
      icon: 'cut',
      run: () => {
        const ents = selected();
        const editable = ents.filter((e) => !doc.layers.isLocked(e.layerId));
        if (editable.length < ents.length) log.warn(`${ents.length - editable.length} nesne kilitli katmanda olduğu için kesilmedi.`);
        if (!editable.length) return;
        const extent = ctx.view.extent(editable.map((e) => e.id));
        // Deleted through cad.entities.delete (docs/adr/0029), as Sil deletes; the step keeps the name “Kes”.
        const uids = editable.map((e) => doc.uidOf(e.id)).filter((uid): uid is string => uid !== undefined);
        const result = doc.transact('Kes', () => entitiesDelete.execute({ doc }, { uids }));
        if (result.status !== 'completed') {
          if ('error' in result) log.warn(result.error.message);
          return;
        }
        ctx.clipboard.set(editable, extent);
        selection.retain((id) => !!doc.get(id));
        log.success(`${editable.length} nesne panoya kesildi.`);
      },
      isEnabled: () => selection.size > 0,
      watch: [selection.ids],
    },
    {
      id: 'edit.copy',
      title: 'Panoya kopyala',
      category: E,
      icon: 'copy',
      run: () => {
        const ents = selected();
        if (!ents.length) return;
        ctx.clipboard.set(ents, ctx.view.extent(ents.map((e) => e.id)));
        log.success(`${ents.length} nesne panoya kopyalandı.`);
      },
      isEnabled: () => selection.size > 0,
      watch: [selection.ids],
    },
    {
      id: 'edit.paste',
      title: 'Yapıştır',
      category: E,
      icon: 'paste',
      aliases: ['YAPISTIR', 'PASTE'],
      run: () => {
        const { items, base } = ctx.clipboard.get();
        if (items.length) tools.run(new PasteTool(ctx, items, base), 'Yapıştır');
      },
      isEnabled: () => ctx.clipboard.count.value > 0,
      watch: [ctx.clipboard.count],
    },
    {
      id: 'edit.pasteOriginal',
      title: 'Özgün koordinatlara yapıştır',
      short: 'Yerine yapıştır',
      category: E,
      icon: 'pasteOriginal',
      aliases: ['PASTEORIG'],
      description: 'Panodaki nesneleri kopyalandıkları koordinatlara yapıştırır (başka projeye aktarırken).',
      run: () => {
        const ids = pasteEntities(ctx, ctx.clipboard.get().items, 0, 0);
        if (ids.length) selection.set(ids);
      },
      isEnabled: () => ctx.clipboard.count.value > 0,
      watch: [ctx.clipboard.count],
    },
    {
      id: 'edit.selectAll',
      title: 'Tümünü seç',
      category: E,
      icon: 'selectAll',
      run: () => {
        const ids = [...doc.all()].filter((e) => doc.layers.isVisible(e.layerId)).map((e) => e.id);
        selection.set(ids);
        log.info(`${ids.length} nesne seçildi.`);
      },
    },
    { id: 'edit.deselect', title: 'Seçimi kaldır', category: E, icon: 'deselect', run: () => selection.clear(), isEnabled: () => selection.size > 0, watch: [selection.ids] },
    {
      id: 'edit.invertSelection',
      title: 'Seçimi ters çevir',
      category: E,
      icon: 'invertSelection',
      run: () => selection.set([...doc.all()].filter((e) => doc.layers.isVisible(e.layerId) && !selection.has(e.id)).map((e) => e.id)),
    },

    // Görünüm
    { id: 'view.zoomExtents', title: 'Tümünü göster', category: V, icon: 'zoomExtents', aliases: ['ZE', 'TUMU', 'LIMITBUL'], run: () => view.zoomExtents() },
    {
      id: 'view.previous',
      title: 'Önceki görünüm',
      category: V,
      icon: 'viewPrevious',
      description: 'Görünümü bir önceki yakınlaştırma ve kaydırma durumuna döndürür (en çok 30 görünüm tutulur). Başka bir çizim açılınca geçmiş boşalır.',
      aliases: ['ZP', 'ONCEKIGORUNUM', 'ONCEKIPENCERE'],
      run: () => void view.viewBack(),
      isEnabled: () => view.navigation.history.canBack.value,
      watch: [view.navigation.history.canBack],
    },
    {
      id: 'view.next',
      title: 'Sonraki görünüm',
      category: V,
      icon: 'viewNext',
      description: 'Önceki görünümle geri dönülen görünüme yeniden gider. Yeni bir yakınlaştırma ya da kaydırma sonraki görünümleri siler.',
      aliases: ['ZN', 'SONRAKIGORUNUM', 'SONRAKIPENCERE'],
      run: () => void view.viewForward(),
      isEnabled: () => view.navigation.history.canForward.value,
      watch: [view.navigation.history.canForward],
    },
    {
      id: 'view.extentCheck',
      title: 'Kapsam denetimi',
      category: V,
      icon: 'extentCheck',
      description: 'Görünen nesnelerden çizimin geri kalanından çok uzakta olanları seçer (yanlış koordinat sistemiyle gelen ya da sıfıra düşen nesneler gibi) ve kaç tane olduklarını söyler. Nesneleri taşımaz, silmez.',
      aliases: ['KAPSAM', 'KAPSAMDENETIM', 'EXTENTCHECK'],
      run: () => checkExtent(ctx),
    },
    { id: 'view.zoomIn', title: 'Yakınlaştır', category: V, icon: 'zoomIn', run: () => view.zoomBy(1.5) },
    { id: 'view.zoomOut', title: 'Uzaklaştır', category: V, icon: 'zoomOut', run: () => view.zoomBy(1 / 1.5) },
    {
      id: 'view.zoomSelection',
      title: 'Seçime yakınlaştır',
      category: V,
      icon: 'zoomSelection',
      run: () => view.zoomToSelection(),
      isEnabled: () => selection.size > 0,
      watch: [selection.ids],
    },
    toggle('view.rightPanel', 'Katman ve öznitelik paneli', ui.rightVisible, { category: V, icon: 'panelRight', short: 'Katman paneli' }),
    toggle('view.bottomPanel', 'Komut geçmişi paneli', ui.bottomExpanded, { category: V, icon: 'panelBottom', short: 'Komut geçmişi' }),
    {
      id: 'view.theme.dark',
      title: 'Koyu',
      category: V,
      icon: 'moon',
      run: () => applyTheme(ctx, 'dark'),
      isChecked: () => ctx.prefs.theme.value === 'dark',
      watch: [ctx.prefs.theme],
    },
    {
      id: 'view.theme.light',
      title: 'Açık',
      category: V,
      icon: 'sun',
      run: () => applyTheme(ctx, 'light'),
      isChecked: () => ctx.prefs.theme.value === 'light',
      watch: [ctx.prefs.theme],
    },
    ...WORKSPACES.map((w) => workspace(ctx, w)),
    renderer(ctx, 'webgl2', 'WebGL2', 'Tüm güncel tarayıcılarda çalışır. Varsayılan çizim motoru.'),
    renderer(ctx, 'webgpu', 'WebGPU', 'Yeni nesil grafik arayüzü. Tarayıcı ve ekran kartı desteklemelidir.'),
    {
      id: 'view.symbols.plot',
      title: 'Çizim ölçeğinde',
      category: V,
      icon: 'symbolsPlot',
      description: 'Semboller basılı paftadaki boylarında durur; harita yaklaştıkça büyür, uzaklaştıkça küçülür.',
      aliases: ['SEMBOLOLCEK'],
      run: () => ctx.prefs.symbolSize.set('plot'),
      isChecked: () => ctx.prefs.symbolSize.value === 'plot',
      watch: [ctx.prefs.symbolSize],
    },
    {
      id: 'view.symbols.screen',
      title: 'Ekranda sabit',
      category: V,
      icon: 'symbolsScreen',
      description: 'Semboller her yakınlıkta ekranda aynı boyda kalır (gezinmek için); basılı boyları görmek için Çizim ölçeğinde seçin.',
      aliases: ['SEMBOLEKRAN'],
      run: () => ctx.prefs.symbolSize.set('screen'),
      isChecked: () => ctx.prefs.symbolSize.value === 'screen',
      watch: [ctx.prefs.symbolSize],
    },
    toggle('view.lineWeights', 'Çizgi kalınlığını göster', ctx.prefs.lineWeights, {
      category: V,
      icon: 'lineWeight',
      short: 'Kalınlık',
      description: 'Katman çizgileri kalınlıklarıyla çizilir. Kapalıyken hepsi ince çizilir (AutoCAD LWT); hassas çalışmada kalın sınırlar noktaları örtmez. Kitaplık sembolleri kendi kalınlığını korur.',
      aliases: ['KALINLIK', 'LWT', 'LWDISPLAY'],
    }),
    // Between the light theme and the dark ones (night and high contrast are dark).
    { id: 'view.theme.toggle', title: 'Temayı değiştir', category: V, icon: 'appearance', run: () => applyTheme(ctx, ctx.prefs.theme.value === 'light' ? 'dark' : 'light') },
    {
      id: 'view.ribbonCollapse',
      title: 'Şeridi daralt',
      category: V,
      icon: 'chevronUp',
      aliases: ['SERITDARALT'],
      description: 'Şeritte yalnız sekmeler kalır; bir sekmeye tıklayınca şerit çizimin üstünde açılır, komuttan sonra kapanır. Sekmeye çift tıklamak da daraltır ya da açar.',
      run: () => ui.ribbonCollapsed.set(!ui.ribbonCollapsed.value),
      isChecked: () => ui.ribbonCollapsed.value,
      watch: [ui.ribbonCollapsed],
    },
    {
      id: 'view.commandSearch',
      title: 'Komut ara',
      category: V,
      icon: 'search',
      aliases: ['ARA', 'SEARCH'],
      description: 'Bir komutu adıyla ya da takma adıyla bulup çalıştırır: şeridin sekme satırındaki arama kutusu.',
      run: hooks.searchCommands,
    },
    {
      id: 'view.keyTips',
      title: 'Şerit harf ipuçları',
      short: 'Harf ipuçları',
      category: V,
      icon: 'keyboard',
      aliases: ['KEYTIPS', 'HARFLER'],
      description: 'Şeridin sekmelerinde ve düğmelerinde harfler gösterir: sekmenin harfine, sonra düğmenin harflerine basınca çalışır. Alt tuşuna tek başına basıp bırakmak da açar; Esc bir düzey geri gider.',
      run: hooks.keyTips,
    },
    fullscreen(),
    {
      id: 'view.coords',
      title: 'Koordinat listesi',
      category: V,
      icon: 'table',
      run: () => {
        ui.bottomTab.set('coords');
        ui.bottomExpanded.set(true);
      },
    },
    {
      // Nokta editörü (docs/adr/0153 §1): the bottom panel's Noktalar tab.
      id: 'point.editor',
      title: 'Nokta editörü',
      short: 'Noktalar',
      category: 'Koordinat',
      icon: 'pointEditor',
      aliases: ['NOKTAEDITORU', 'NE', 'NOKTALAR'],
      description: 'Çizimdeki bütün noktalar alt panelde bir tabloda: ad, Y, X, Z, kod ve katman; sıralanır, aranır, süzülür. Satıra tıklamak noktayı seçer, Göster ona yakınlaştırır.',
      run: () => {
        ui.bottomTab.set('points');
        ui.bottomExpanded.set(true);
      },
    },

    // Çizim yardımcıları
    toggle('draft.snap', 'Kenetleme', settings.snap, {
      category: 'Çizim yardımcıları',
      icon: 'snap',
      description: 'İmleç uç, orta, merkez, kesişim gibi noktalara yapışır. Tek seferlik kenet için Shift + sağ tık.',
    }),
    // The snap kinds one by one (docs/adr/0163 §6): the Kenet cell's right-click menu and the command line.
    ...snapKindCommands(ctx),
    toggle('draft.snap.self', 'Çizilmekte olan nesneye kenet', ctx.prefs.snapSelf, {
      short: 'Çizilmekte olan nesneye',
      category: 'Çizim yardımcıları',
      icon: 'snap',
      description: 'Çizilen yolun önceki köşelerine ve kenarlarına da kenetlenir; kapalıyken yalnız çizimdeki nesnelere.',
    }),
    toggle('draft.grid', 'Izgara', settings.grid, { category: 'Çizim yardımcıları', icon: 'grid' }),
    toggle('draft.ortho', 'Orto', settings.ortho, { category: 'Çizim yardımcıları', icon: 'ortho', description: 'Yeni nokta son noktanın tam yatayına ya da dikeyine düşer. Shift basılıyken tersine döner.' }),
    toggle('draft.rightAngle', 'Dik açı', settings.rightAngle, {
      category: 'Çizim yardımcıları',
      icon: 'rightAngle',
      description: 'Aracın ikinci kenarından başlayarak her kenar öncekine dik çizilir; hangi yöne gideceğini imleç seçer. Elle kilitlenen doğrultu önce gelir. Kapalı alanda Dik kapat (D) son köşeyi ilk kenara dik kapatır.',
    }),
    toggle('draft.polar', 'Kutupsal izleme', settings.polar, {
      category: 'Çizim yardımcıları',
      icon: 'polar',
      description: 'Son noktadan açı adımlarında (ayarlardan, varsayılan 45°) kılavuz çıkar; imleç yaklaşınca yapışır.',
    }),
    toggle('draft.tracking', 'Nesne izleme', settings.tracking, {
      category: 'Çizim yardımcıları',
      icon: 'tracking',
      description: 'Bir kenet noktasının üzerinde kısa süre bekleyin: o noktadan yatay ve dikey kılavuzlar çıkar, imleç bu hizalara ve kesişimlerine yapışır.',
    }),
    // Topolojik düzenleme (docs/adr/0160 §1): a drafting aid, off at the start of every session.
    toggle('draft.topology', 'Topolojik düzenleme', settings.topology, {
      category: 'Çizim yardımcıları',
      icon: 'topologyEdit',
      aliases: ['TOPOLOJIKDUZENLEME', 'TOPODUZENLE'],
      description:
        'Tutamaçla taşınan köşe, kenar ortasından eklenen köşe ve biçimlenen yay, görünen ve kilitsiz katmanlardaki komşu nesnelerin ortak köşe ve kenarlarında da birlikte değişir; hepsi tek adımda geri alınır.',
    }),
    toggle('draft.topologyPoints', 'Topolojik düzenlemede noktalar da', settings.topologyPoints, {
      short: 'Noktalar da',
      category: 'Çizim yardımcıları',
      icon: 'topologyPoints',
      description: 'Topolojik düzenlemede nokta nesneleri de ortak köşe sayılır ve köşeyle birlikte taşınır; kapalıyken ölçü noktaları yerinde kalır.',
    }),
    // Çakışma denetimi (docs/adr/0162 §1): a drafting aid; the cell's click goes between Serbest and the last mode that avoids.
    {
      id: 'draft.overlap',
      title: 'Çakışmayı önle',
      short: 'Çakışma',
      category: 'Çizim yardımcıları',
      icon: 'overlap',
      aliases: ['CAKISMA', 'CAKISMAYIONLE'],
      description:
        'Açıkken çizilen yeni alan, komşu alanlarla örtüşen kısmı çıkarılarak yazılır; ortak sınır komşunun sınırı olur. Kendi katmanında ya da seçili katmanlarda önlenir (sağ tık menüsü).',
      run: () => {
        const next = settings.overlap.value === 'allow' ? settings.overlapLast.value : 'allow';
        settings.overlap.set(next);
        ctx.log.info(next === 'allow' ? 'Çakışma serbest.' : `Çakışma önleniyor: ${next === 'layer' ? 'kendi katmanında' : 'seçili katmanlarda'}.`);
      },
      isChecked: () => settings.overlap.value !== 'allow',
      watch: [settings.overlap],
    },
    overlapMode(ctx, 'allow', 'Serbest', 'overlapAllow', 'Yeni alan komşularıyla örtüşebilir; olduğu gibi yazılır.'),
    overlapMode(ctx, 'layer', 'Kendi katmanında önle', 'overlapLayer', 'Yeni alan, yazılacağı katmandaki görünen alanlarla örtüşen kısmı çıkarılarak yazılır.'),
    overlapMode(ctx, 'layers', 'Seçili katmanlarda önle', 'overlapLayers', 'Yeni alan, seçilen katmanlardaki görünen alanlarla örtüşen kısmı çıkarılarak yazılır; katmanlar Çakışma hücresinin menüsünden seçilir.'),
    // The digitizing locks (docs/adr/0166 §6): on the running command, without ending it.
    ...lockCommands(ctx),

    // Harita / Koordinat / Analiz
    pending(ctx, 'map.contours', 'Eşyükselti üret…', M, 'contours'),
    pending(ctx, 'map.profile', 'Boy kesit al…', M, 'profile'),
    pending(ctx, 'map.sheet', 'Pafta bölümlemesi…', M, 'sheet'),
    { ...pending(ctx, 'map.parcelReport', 'Parsel alan çizelgesi', M, 'parcelReport'), short: 'Alan çizelgesi' },
    { id: 'crs.set', title: 'Koordinat sistemi…', category: K, icon: 'crs', aliases: ['SRID', 'EPSG'], run: () => hooks.openProjectSettings('crs') },
    { ...pending(ctx, 'crs.transform', 'Datum dönüşümü (ED50 ↔ TUREF)…', K, 'crsTransform'), short: 'Datum dönüşümü' },
    {
      id: 'crs.query',
      title: 'Koordinat oku',
      short: 'Koordinat oku',
      category: K,
      icon: 'crsQuery',
      description:
        'Tıklanan (kenetli) noktanın Y ve X’ini, noktanın kotu varsa onu da, projenin ikinci koordinat sistemi varsa o sistemdeki değerlerini de doğruluğuyla iletiye yazar; her tıklama bir okumadır, Esc bitirir. Çizime bir şey yazılmaz.',
      aliases: ['KOORDINATOKU', 'ID', 'NOKTAOKU', 'XYZSOR'],
      run: () => tools.run(new CoordinateReadTool(ctx), 'Koordinat oku'),
    },
    pending(ctx, 'analysis.volume', 'Hacim hesabı…', A, 'volume'),
    pending(ctx, 'analysis.slope', 'Eğim analizi…', A, 'slope'),

    // Katmanlar
    {
      id: 'layer.new',
      title: 'Yeni katman',
      category: 'Katman',
      icon: 'layerAdd',
      // One undo step, “Katman ekle”: undo takes the layer away and makes the one before active again.
      run: () => {
        const locked = treeLocked(ctx);
        if (locked) return log.warn(locked);
        const node = doc.addLayer({ name: doc.layers.uniqueName('Yeni katman') }, doc.layers.active.value, { activate: true });
        log.success(`“${node.name}” katmanı eklendi ve etkin yapıldı.`);
      },
      isEnabled: () => !treeLocked(ctx),
      whyDisabled: () => treeLocked(ctx),
      watch: [ctx.cloud.project],
    },
    {
      id: 'layer.newGroup',
      title: 'Yeni grup',
      category: 'Katman',
      icon: 'folderAdd',
      // One undo step, “Grup ekle”.
      run: () => {
        const locked = treeLocked(ctx);
        if (locked) return log.warn(locked);
        const node = doc.addLayer({ name: doc.layers.uniqueName('Yeni grup'), type: 'group', children: [] }, null);
        log.success(`“${node.name}” grubu eklendi.`);
      },
      isEnabled: () => !treeLocked(ctx),
      whyDisabled: () => treeLocked(ctx),
      watch: [ctx.cloud.project],
    },
    { id: 'layer.showAll', title: 'Tüm katmanları göster', short: 'Katmanları göster', category: 'Katman', icon: 'layersShowAll', run: () => doc.layers.showAll() },

    // Araç akışı
    { id: 'tool.cancel', title: 'İptal', category: 'Komut', icon: 'close', run: () => tools.exit() },
    {
      id: 'tool.confirm',
      title: 'Onayla',
      category: 'Komut',
      icon: 'check',
      // Shift+Enter comes with `{ shift: true }` (app/keybindings.ts).
      run: (args) => {
        const t = tools.active;
        t.confirm ? t.confirm(args as ConfirmMods | undefined) : tools.repeatLast();
      },
    },
    { id: 'tool.repeat', title: 'Son komutu yinele', category: 'Komut', icon: 'repeat', run: () => tools.repeatLast() },
    { id: 'commandline.focus', title: 'Komut satırına git', category: 'Araçlar', icon: 'terminal', run: hooks.focusCommandLine },

    // Yardım
    { id: 'help.shortcuts', title: 'Klavye kısayolları', category: 'Yardım', icon: 'keyboard', aliases: ['KISAYOL', 'KEYS'], run: hooks.openShortcuts },
    { id: 'help.about', title: 'KentOS CAD hakkında', category: 'Yardım', icon: 'info', run: hooks.openAbout },
    { id: 'tools.options', title: 'Uygulama ayarları…', category: 'Araçlar', icon: 'settings', aliases: ['AYARLAR', 'OPTIONS'], run: (section) => hooks.openAppSettings(typeof section === 'string' ? section : undefined) },
    {
      id: 'server.check',
      title: 'Sunucu bağlantısını denetle',
      short: 'Sunucuyu denetle',
      category: 'Araçlar',
      icon: 'server',
      description: 'KentOS sunucusuna şimdi sorar. Çizim sunucusuz da çalışır; kayıt yerel .kcad dosyasına yapılır.',
      aliases: ['SUNUCU', 'SERVER'],
      run: () =>
        void ctx.server.check().then((s) => {
          const why = ctx.server.detail.value;
          if (s === 'online') ctx.log.success(`Sunucu bağlı: ${ctx.server.health.value?.service} ${ctx.server.health.value?.version}.`);
          else if (s === 'incompatible') ctx.log.warn(`Sunucu uyumsuz. ${why}`);
          else ctx.log.info(`Sunucu yok. ${why} Çizim sunucusuz çalışmaya devam ediyor.`);
        }),
      isEnabled: () => ctx.server.state.value !== 'checking',
      watch: [ctx.server.state],
    },
  ]);

  // Every tool becomes a command: the ribbon, menus, keymap and command line share it.
  for (const d of tools.list()) {
    commands.register({
      id: `tool.${d.id}`,
      title: d.label,
      category: TOOL_GROUP_LABEL[d.group],
      icon: d.icon,
      description: d.description,
      aliases: d.aliases,
      pending: !d.ready || undefined,
      run: () => tools.activate(d.id),
      isChecked: () => tools.activeId.value === d.id,
      watch: [tools.activeId],
    });
  }
}
