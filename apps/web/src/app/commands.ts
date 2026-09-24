import type { Command } from '../core/commands';
import type { BackendKind } from '../render/types';
import { WebGPUBackend } from '../render/webgpu/WebGPUBackend';
import type { Entity } from '../model/entities';
import { pasteEntities, PasteTool } from '../tools/editTools';
import type { Signal } from '../core/signal';
import { TOOL_GROUP_LABEL } from '../tools/Tool';
import type { AppContext } from './context';
import type { Theme, UiScale } from './state';

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

/** Drawing engine choice: remembered in preferences, applied live by the viewport. */
function renderer(ctx: AppContext, kind: BackendKind, title: string, description: string): Command {
  return {
    id: `view.renderer.${kind}`,
    title,
    category: 'Görünüm',
    description,
    aliases: [kind.toUpperCase()],
    run: () => {
      ctx.prefs.rendererPreference.set(kind);
      void ctx.view.switchBackend(kind);
    },
    isEnabled: () => kind !== 'webgpu' || WebGPUBackend.isSupported(),
    isChecked: () => ctx.view.backendKind.value === kind,
    watch: [ctx.view.backendKind],
  };
}

export function applyTheme(ctx: AppContext, theme: Theme): void {
  document.documentElement.dataset.theme = theme;
  // Palette first: theme subscribers (layer swatches) read it.
  ctx.view.refreshPalette();
  ctx.ui.theme.set(theme);
}

/** Type scale multiplier; every font/size token is derived from --ui-scale. */
export const UI_SCALE: Record<UiScale, number> = { standard: 1, large: 1.08, xlarge: 1.16 };

export function applyUiScale(scale: UiScale): void {
  document.documentElement.style.setProperty('--ui-scale', String(UI_SCALE[scale] ?? 1));
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
  /** Komut ara — the ribbon's search box, or the command line in the classic shell. */
  searchCommands: () => void;
}

export function registerCoreCommands(ctx: AppContext, hooks: CommandHooks): void {
  const { commands, doc, selection, settings, ui, view, tools, log } = ctx;
  const selected = () => [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
  const toolboxShown = () => (ctx.prefs.shell.value === 'ribbon' ? ui.ribbonToolbox : ui.toolboxVisible);
  const F = 'Dosya';
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
      id: 'file.save',
      title: 'Kaydet',
      category: F,
      icon: 'save',
      description:
        'Bulut projesinde bekleyen değişiklikleri hemen gönderir (zaten kendiliğinden kaydedilir). Yerel çizimi .kcad dosyasına yazar; dosya yazılamazsa değişiklikler kaydedilmemiş sayılır.',
      aliases: ['KAYDET', 'SAVE'],
      run: () => {
        if (!ctx.cloud.project.value) return void ctx.files.save();
        if (ctx.cloud.sync.value?.state.value === 'deleted') {
          // Nothing can be saved to a deleted project: the next useful step is a local file.
          ctx.log.warn('Bu bulut projesi silindi; çizim buluta kaydedilemez. Yerel bir dosyaya kaydedin.');
          return void ctx.files.saveAs();
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
    pending(ctx, 'file.import.ncz', 'Netcad NCZ…', F),
    pending(ctx, 'file.import.shp', 'Shapefile…', F),
    pending(ctx, 'file.import.geojson', 'GeoJSON…', F),
    pending(ctx, 'file.export.dxf', 'DXF…', F),
    pending(ctx, 'file.export.geojson', 'GeoJSON…', F),
    pending(ctx, 'file.export.pdf', 'PDF pafta…', F),
    { ...pending(ctx, 'file.print', 'Yazdır ve pafta çıktısı…', F, 'print'), short: 'Yazdır' },
    { id: 'file.settings', title: 'Proje ayarları…', category: F, icon: 'folder', aliases: ['PROJE'], run: () => hooks.openProjectSettings() },

    // Düzen
    {
      id: 'edit.undo',
      title: 'Geri al',
      category: E,
      icon: 'undo',
      run: () => {
        const label = doc.undo();
        if (label) log.info(`Geri alındı: ${label}`);
        selection.retain((id) => !!doc.get(id));
      },
      isEnabled: () => doc.canUndo.value,
      watch: [doc.canUndo],
    },
    {
      id: 'edit.redo',
      title: 'Yinele',
      category: E,
      icon: 'redo',
      run: () => {
        const label = doc.redo();
        if (label) log.info(`Yinelendi: ${label}`);
        selection.retain((id) => !!doc.get(id));
      },
      isEnabled: () => doc.canRedo.value,
      watch: [doc.canRedo],
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
        ctx.clipboard.set(editable, ctx.view.extent(editable.map((e) => e.id)));
        doc.transact('Kes', () => doc.remove(editable.map((e) => e.id)));
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
    { id: 'view.zoomExtents', title: 'Tümünü göster', category: V, icon: 'zoomExtents', aliases: ['ZE', 'TUMU'], run: () => view.zoomExtents() },
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
    {
      id: 'view.toolbox',
      title: 'Araç kutusu',
      category: V,
      icon: 'toolbox',
      // The ribbon holds every tool: next to it the toolbox stays off until asked for, remembered apart.
      run: () => toolboxShown().set(!toolboxShown().value),
      isChecked: () => toolboxShown().value,
      watch: [ui.toolboxVisible, ui.ribbonToolbox, ctx.prefs.shell],
    },
    toggle('view.toolboxDock', 'Araç kutusunu kenara sabitle', ui.toolboxDocked, { category: V, icon: 'dock', short: 'Kenara sabitle' }),
    toggle('view.rightPanel', 'Katman ve öznitelik paneli', ui.rightVisible, { category: V, icon: 'panelRight', short: 'Katman paneli' }),
    toggle('view.bottomPanel', 'Komut geçmişi paneli', ui.bottomExpanded, { category: V, icon: 'panelBottom', short: 'Komut geçmişi' }),
    {
      id: 'view.theme.dark',
      title: 'Koyu',
      category: V,
      icon: 'moon',
      run: () => applyTheme(ctx, 'dark'),
      isChecked: () => ui.theme.value === 'dark',
      watch: [ui.theme],
    },
    {
      id: 'view.theme.light',
      title: 'Açık',
      category: V,
      icon: 'sun',
      run: () => applyTheme(ctx, 'light'),
      isChecked: () => ui.theme.value === 'light',
      watch: [ui.theme],
    },
    renderer(ctx, 'webgl2', 'WebGL2', 'Tüm güncel tarayıcılarda çalışır. Varsayılan çizim motoru.'),
    renderer(ctx, 'webgpu', 'WebGPU', 'Yeni nesil grafik arayüzü. Tarayıcı ve ekran kartı desteklemelidir.'),
    {
      id: 'view.symbols.plot',
      title: 'Çizim ölçeğinde',
      category: V,
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
      description: 'Semboller her yakınlıkta ekranda aynı boyda kalır (gezinmek için); basılı boyları görmek için Çizim ölçeğinde seçin.',
      aliases: ['SEMBOLEKRAN'],
      run: () => ctx.prefs.symbolSize.set('screen'),
      isChecked: () => ctx.prefs.symbolSize.value === 'screen',
      watch: [ctx.prefs.symbolSize],
    },
    { id: 'view.theme.toggle', title: 'Temayı değiştir', category: V, run: () => applyTheme(ctx, ui.theme.value === 'dark' ? 'light' : 'dark') },
    {
      id: 'view.ribbon',
      title: 'Şerit arayüzü',
      category: V,
      icon: 'ribbon',
      aliases: ['SERIT', 'RIBBON'],
      description: 'Menüler, araç çubuğu ve araç kutusu yerine sekmeli şerit: aynı araçlar ve komutlar. Uygulama ayarları → Görünüm’den de seçilir.',
      run: () => ctx.prefs.shell.set(ctx.prefs.shell.value === 'ribbon' ? 'classic' : 'ribbon'),
      isChecked: () => ctx.prefs.shell.value === 'ribbon',
      watch: [ctx.prefs.shell],
    },
    {
      id: 'view.ribbonCollapse',
      title: 'Şeridi daralt',
      category: V,
      icon: 'chevronUp',
      aliases: ['SERITDARALT'],
      description: 'Şeritte yalnız sekmeler kalır; bir sekmeye tıklayınca şerit çizimin üstünde açılır, komuttan sonra kapanır. Sekmeye çift tıklamak da daraltır ya da açar.',
      run: () => ui.ribbonCollapsed.set(!ui.ribbonCollapsed.value),
      isEnabled: () => ctx.prefs.shell.value === 'ribbon',
      isChecked: () => ui.ribbonCollapsed.value,
      watch: [ctx.prefs.shell, ui.ribbonCollapsed],
    },
    {
      id: 'view.commandSearch',
      title: 'Komut ara',
      category: V,
      icon: 'search',
      aliases: ['ARA', 'SEARCH'],
      description: 'Bir komutu adıyla ya da takma adıyla bulup çalıştırır: şeritte arama kutusu, klasik arayüzde komut satırı.',
      run: hooks.searchCommands,
    },
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

    // Çizim yardımcıları
    toggle('draft.snap', 'Kenetleme', settings.snap, {
      category: 'Çizim yardımcıları',
      icon: 'snap',
      description: 'İmleç uç, orta, merkez, kesişim gibi noktalara yapışır. Tek seferlik kenet için Shift + sağ tık.',
    }),
    toggle('draft.grid', 'Izgara', settings.grid, { category: 'Çizim yardımcıları', icon: 'grid' }),
    toggle('draft.ortho', 'Orto', settings.ortho, { category: 'Çizim yardımcıları', icon: 'ortho', description: 'Yeni nokta son noktanın tam yatayına ya da dikeyine düşer. Shift basılıyken tersine döner.' }),
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

    // Harita / Koordinat / Analiz
    pending(ctx, 'map.contours', 'Eşyükselti üret…', M, 'contours'),
    pending(ctx, 'map.profile', 'Boy kesit al…', M, 'profile'),
    pending(ctx, 'map.sheet', 'Pafta bölümlemesi…', M, 'sheet'),
    { ...pending(ctx, 'map.parcelReport', 'Parsel alan çizelgesi', M, 'parcelReport'), short: 'Alan çizelgesi' },
    { id: 'crs.set', title: 'Koordinat sistemi…', category: K, icon: 'crs', aliases: ['SRID', 'EPSG'], run: () => hooks.openProjectSettings('crs') },
    { ...pending(ctx, 'crs.transform', 'Datum dönüşümü (ED50 ↔ TUREF)…', K, 'crsTransform'), short: 'Datum dönüşümü' },
    pending(ctx, 'crs.query', 'Koordinat sorgula', K, 'crsQuery'),
    pending(ctx, 'analysis.volume', 'Hacim hesabı…', A, 'volume'),
    pending(ctx, 'analysis.slope', 'Eğim analizi…', A, 'slope'),

    // Katmanlar
    {
      id: 'layer.new',
      title: 'Yeni katman',
      category: 'Katman',
      icon: 'layerAdd',
      run: () => {
        const active = doc.layers.active.value;
        const node = doc.layers.add({ name: doc.layers.uniqueName('Yeni katman') }, active);
        doc.layers.setActive(node.id);
        log.success(`“${node.name}” katmanı eklendi ve etkin yapıldı.`);
      },
    },
    {
      id: 'layer.newGroup',
      title: 'Yeni grup',
      category: 'Katman',
      icon: 'folderAdd',
      run: () => {
        const node = doc.layers.add({ name: doc.layers.uniqueName('Yeni grup'), type: 'group', children: [] }, null);
        log.success(`“${node.name}” grubu eklendi.`);
      },
    },
    { id: 'layer.showAll', title: 'Tüm katmanları göster', short: 'Katmanları göster', category: 'Katman', icon: 'eye', run: () => doc.layers.showAll() },

    // Araç akışı
    { id: 'tool.cancel', title: 'İptal', category: 'Komut', run: () => tools.exit() },
    {
      id: 'tool.confirm',
      title: 'Onayla',
      category: 'Komut',
      run: () => {
        const t = tools.active;
        t.confirm ? t.confirm() : tools.repeatLast();
      },
    },
    { id: 'tool.repeat', title: 'Son komutu yinele', category: 'Komut', run: () => tools.repeatLast() },
    { id: 'commandline.focus', title: 'Komut satırına git', category: 'Araçlar', icon: 'terminal', run: hooks.focusCommandLine },

    // Yardım
    { id: 'help.shortcuts', title: 'Klavye kısayolları', category: 'Yardım', icon: 'keyboard', aliases: ['KISAYOL', 'KEYS'], run: hooks.openShortcuts },
    { id: 'help.about', title: 'KentOS CAD hakkında', category: 'Yardım', icon: 'info', run: hooks.openAbout },
    { id: 'tools.options', title: 'Uygulama ayarları…', category: 'Araçlar', icon: 'settings', aliases: ['AYARLAR', 'OPTIONS'], run: () => hooks.openAppSettings() },
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

  // Every tool becomes a command: menus, toolbox, keymap and command line share it.
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
