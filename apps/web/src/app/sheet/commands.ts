import type { AlignEdge } from '../../contracts/generated/sheet/AlignEdge';
import type { MapGrid } from '../../contracts/generated/sheet/MapGrid';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';
import type { Command } from '../../core/commands';
import type { Disposable } from '../../core/disposable';
import { copyItems, freeName, groupItem, withChildren } from '../../product/sheet/ops';

import { HAND, SELECT } from '../../product/sheet/state';
import { askRemove } from '../../ui/widgets/confirm';
import type { AppContext } from '../context';
import type { SheetService } from './service';

/**
 * The sheet layouts' commands (docs/sheet/design.md §11): every action of the
 * Pafta tab, the tabs, the panels and the keys is a command, so the ribbon,
 * the menus, the command line and the shortcuts all name the same one
 * (CLAUDE.md §4.5). Each change of the book is one list of the engine's
 * operations, one undo step; a refusal is said with the engine's words. A
 * command that cannot run now says why in its tooltip and, shortly, beside
 * it in a menu. The tools that add items are the mode's profile's
 * (`sheetToolCommands`, registered again when the profile changes).
 */

/** What the commands reach in the interface: the gallery, the desk while it is loaded, the sheet's windows. */
export interface SheetCommandHooks {
  gallery(): void;
  stage(): { fitPage(): void; realSize(): void; zoomChoice(): void; step(dir: 1 | -1): void; cancelDrag(): boolean } | null;
  /** A sheet window, loaded when first opened. */
  window(name: SheetWindow, args?: unknown): void;
  /** A name asked in a small window; null when it was left. */
  ask(title: string, label: string, value: string): Promise<string | null>;
}

export type SheetWindow = 'pageSetup' | 'variables' | 'saveTemplate' | 'export' | 'preflight' | 'importKpafta';

const P = 'Pafta';

export function registerSheetCommands(ctx: AppContext, sheets: SheetService, hooks: SheetCommandHooks): Disposable {
  const { state } = sheets;
  const watch = [state.open, state.engine, state.selection, state.book, state.tool, state.alignTo, sheets.history];
  /** Short words beside a row of a menu when the command cannot run now (its tooltip has the whole reason). */
  const hint = () => (state.sheet ? undefined : 'pafta açık değil');

  /** A command on the sheet in front. Extra conditions say their own reason. */
  const onSheet = (id: string, title: string, icon: string, run: (args?: unknown) => void, o: { short?: string; aliases?: string[]; description?: string; also?: () => string | null; checked?: () => boolean } = {}): Command => {
    const why = () => state.whyNot(true) ?? o.also?.() ?? null;
    return {
      id,
      title,
      short: o.short,
      category: P,
      icon,
      description: o.description,
      aliases: o.aliases,
      run,
      isEnabled: () => why() === null,
      whyDisabled: why,
      get pendingNote() {
        return why() ? hint() : undefined;
      },
      isChecked: o.checked,
      watch,
    };
  };
  /** The sheet in front, as the engine keeps it. */
  const sheet = (): Sheet | null => sheets.book()?.book.sheets.find((s) => s.id === state.open.value) ?? null;
  /** The sheet a tab's menu names, or the one in front. */
  const target = (args: unknown): Sheet | null => {
    const id = (args as { id?: string } | undefined)?.id ?? state.open.value;
    return sheets.book()?.book.sheets.find((s) => s.id === id) ?? null;
  };
  const chosen = () => [...state.selection.value];
  /** The chosen items that may change (a locked one is said, and left). */
  const free = (what: string): string[] => {
    const s = sheet();
    const ids = chosen();
    const locked = s ? s.items.filter((i) => ids.includes(i.id) && i.locked) : [];
    if (locked.length) ctx.log.warn(`${what}: “${locked[0].name}”${locked.length > 1 ? ` ve ${locked.length - 1} öğe daha` : ''} kilitli, dokunulmadı.`);
    return ids.filter((id) => !locked.some((l) => l.id === id));
  };
  const needsChoice = () => (state.selection.value.size ? null : 'Önce paftada bir öğe seçin.');
  const needsTwo = () => (state.selection.value.size > 1 ? null : 'En az iki öğe seçin.');
  const needsThree = () => (state.selection.value.size > 2 ? null : 'En az üç öğe seçin.');
  const owner = (s: Sheet) => ({ kind: 'sheet' as const, id: s.id });
  const apply = (ops: Op[], label?: string) => ops.length > 0 && sheets.apply(ops, label);

  const align = (edge: AlignEdge) => () => {
    const ids = free('Hizala');
    const to = state.alignTo.value;
    if (to === 'selection' && ids.length < 2) return ctx.log.warn('Seçime göre hizalamak için en az iki öğe seçin; tek öğe Sayfaya ya da Kenar boşluklarına göre hizalanır.');
    apply([{ op: 'align', ids, edge, to }]);
  };
  const distribute = (axis: 'x' | 'y', mode: 'centers' | 'gaps') => () => apply([{ op: 'distribute', ids: free('Dağıt'), axis, mode }]);
  const order = (to: 'front' | 'back' | 'forward' | 'backward') => () => apply([{ op: 'reorder', ids: chosen(), to }]);

  const commands: Command[] = [
    {
      id: 'sheet.model',
      title: 'Modele dön',
      short: 'Model',
      category: P,
      icon: 'sheetModel',
      aliases: ['MODEL', 'MODELE'],
      description: 'Çizim alanına döner; paftalar sekmelerinde kalır.',
      run: () => sheets.openSheet(null),
      isEnabled: () => state.open.value !== null,
      whyDisabled: () => (state.open.value === null ? 'Model zaten önde.' : null),
      watch,
    },
    {
      id: 'sheet.open',
      title: 'Paftayı aç',
      category: P,
      icon: 'sheetLayout',
      aliases: ['PAFTA', 'PAFTALAR', 'LAYOUT'],
      description: 'Bir paftayı öne getirir (verilmezse ilk paftayı).',
      run: (args) => {
        const id = (args as { id?: string } | undefined)?.id ?? state.book.value.sheets[0]?.id ?? state.waiting.value[0]?.id;
        if (id) sheets.openSheet(id);
        else ctx.log.warn('Bu projede pafta yok. Yeni pafta ekleyin ya da Pafta şablonları’ndan başlayın.');
      },
      watch,
    },
    {
      id: 'sheet.new',
      title: 'Yeni pafta',
      category: P,
      icon: 'sheetNew',
      aliases: ['YENIPAFTA', 'YP'],
      description: 'Çalışma modunun varsayılan şablonundan yeni bir pafta açar.',
      run: () => void sheets.newSheet().catch((e: Error) => sheets.report('Yeni pafta açılamadı', e)),
      isEnabled: () => state.whyNoEngine() === null,
      whyDisabled: () => state.whyNoEngine(),
      watch,
    },
    {
      id: 'sheet.fromTemplate',
      title: 'Şablondan pafta…',
      short: 'Şablondan',
      category: P,
      icon: 'sheetTemplate',
      aliases: ['SABLON', 'PAFTASABLONU'],
      description: 'Pafta şablonları: sistemin, sizin, kurumunuzun ve sizinle paylaşılanlar; kipinize ve proje türünüze göre sıralı.',
      run: () => hooks.gallery(),
    },
    {
      id: 'sheet.importKpafta',
      title: '.kpafta dosyasından…',
      category: P,
      icon: 'importKpafta',
      description: 'Bir .kpafta dosyasındaki paftaları ve resimlerini bu projeye alır.',
      run: () => hooks.window('importKpafta'),
      isEnabled: () => state.whyNoEngine() === null,
      whyDisabled: () => state.whyNoEngine(),
      watch,
    },
    onSheet('sheet.pageSetup', 'Sayfa ayarları…', 'sheetPage', () => hooks.window('pageSetup'), { short: 'Sayfa', description: 'Kâğıt, yön, boy ve kenar boşlukları; öğeler kısıtlarıyla yeniden yerleşir.' }),
    onSheet('sheet.variables', 'Değişkenler…', 'sheetVariables', () => hooks.window('variables'), { short: 'Değişkenler', description: 'Paftanın ve projenin değişkenleri (ada, parsel, mahalle …): metinlerde ve antette [% @ad %] olarak yazılır.' }),
    // On one sheet: the open one, or the one a tab's menu names.
    onSheet('sheet.rename', 'Paftaya ad ver…', 'edit', (args) => {
      const s = target(args);
      if (s) void hooks.ask('Paftaya ad ver', 'Ad', s.name).then((name) => name && name !== s.name && apply([{ op: 'renameSheet', id: s.id, name }]));
    }, { short: 'Ad ver' }),
    onSheet('sheet.duplicate', 'Paftayı çoğalt', 'copy', (args) => {
      const s = target(args);
      if (!s) return;
      const id = sheets.newId();
      const names = new Set(sheets.book()!.book.sheets.map((x) => x.name));
      if (apply([{ op: 'duplicateSheet', id: s.id, newId: id, name: freeName(s, s.name, names), itemIds: s.items.map(() => sheets.newId()) }])) sheets.openSheet(id);
    }, { short: 'Çoğalt' }),
    onSheet('sheet.moveLeft', 'Sola taşı', 'sheetMoveLeft', (args) => move(args, -1), { also: () => (index(null) > 0 ? null : 'Pafta zaten en solda.') }),
    onSheet('sheet.moveRight', 'Sağa taşı', 'sheetMoveRight', (args) => move(args, 1), { also: () => (index(null) < (sheets.book()?.book.sheets.length ?? 0) - 1 ? null : 'Pafta zaten en sağda.') }),
    onSheet('sheet.delete', 'Paftayı sil…', 'trash', (args) => {
      const s = target(args);
      if (!s) return;
      void askRemove({ title: 'Paftayı sil', message: `“${s.name}” silinsin mi?`, details: ['Geri al (Ctrl+Z) paftayı öğeleriyle geri getirir.'], action: 'Sil' }).then((ok) => ok && apply([{ op: 'removeSheet', id: s.id }]));
    }, { short: 'Sil' }),
    // Paper tools.
    onSheet('sheet.tool.select', 'Seç', 'select', () => state.tool.set(SELECT), { description: 'Öğeleri seçer, taşır, boyutlandırır, döndürür: tıklama, Shift ile ekleme, Ctrl ile çıkarma, sürükleyerek kutu.', checked: () => state.tool.value.kind === 'select' }),
    onSheet('sheet.tool.hand', 'El', 'pan', () => state.tool.set(HAND), { description: 'Kâğıdı kaydırır. Boşluk basılıyken her araçta geçici olarak el olur.', checked: () => state.tool.value.kind === 'hand' }),
    // Arranging.
    onSheet('sheet.align.left', 'Sola hizala', 'sheetAlignLeft', align('left'), { also: needsChoice }),
    onSheet('sheet.align.center', 'Yatayda ortala', 'sheetAlignCenter', align('center'), { also: needsChoice }),
    onSheet('sheet.align.right', 'Sağa hizala', 'sheetAlignRight', align('right'), { also: needsChoice }),
    onSheet('sheet.align.top', 'Üste hizala', 'sheetAlignTop', align('top'), { also: needsChoice }),
    onSheet('sheet.align.middle', 'Düşeyde ortala', 'sheetAlignMiddle', align('middle'), { also: needsChoice }),
    onSheet('sheet.align.bottom', 'Alta hizala', 'sheetAlignBottom', align('bottom'), { also: needsChoice }),
    alignTo('selection', 'Seçime göre'),
    alignTo('page', 'Sayfaya göre'),
    alignTo('margins', 'Kenar boşluklarına göre'),
    onSheet('sheet.distribute.hCenters', 'Yatayda ortaları eşit dağıt', 'sheetDistributeH', distribute('x', 'centers'), { also: needsThree }),
    onSheet('sheet.distribute.hGaps', 'Yatayda aralıkları eşitle', 'sheetDistributeHGaps', distribute('x', 'gaps'), { also: needsThree }),
    onSheet('sheet.distribute.vCenters', 'Düşeyde ortaları eşit dağıt', 'sheetDistributeV', distribute('y', 'centers'), { also: needsThree }),
    onSheet('sheet.distribute.vGaps', 'Düşeyde aralıkları eşitle', 'sheetDistributeVGaps', distribute('y', 'gaps'), { also: needsThree }),
    onSheet('sheet.matchSize.width', 'Aynı genişlik', 'sheetMatchWidth', () => apply([{ op: 'matchSize', ids: free('Aynı genişlik'), dimension: 'width' }]), { also: needsTwo }),
    onSheet('sheet.matchSize.height', 'Aynı yükseklik', 'sheetMatchHeight', () => apply([{ op: 'matchSize', ids: free('Aynı yükseklik'), dimension: 'height' }]), { also: needsTwo }),
    onSheet('sheet.order.front', 'Öne getir', 'sheetFront', order('front'), { also: needsChoice }),
    onSheet('sheet.order.forward', 'Bir öne', 'sheetForward', order('forward'), { also: needsChoice }),
    onSheet('sheet.order.backward', 'Bir arkaya', 'sheetBackward', order('backward'), { also: needsChoice }),
    onSheet('sheet.order.back', 'Arkaya gönder', 'sheetBack', order('back'), { also: needsChoice }),
    onSheet('sheet.group', 'Grupla', 'sheetGroup', () => {
      const s = sheet();
      if (!s) return;
      const id = sheets.newId();
      if (apply([{ op: 'group', ids: chosen(), group: groupItem(id, freeName(s, 'Grup')) }])) state.select([id]);
    }, { also: needsTwo }),
    onSheet('sheet.ungroup', 'Grubu çöz', 'sheetUngroup', () => {
      const s = sheet();
      const groups = s ? s.items.filter((i) => state.selection.value.has(i.id) && i.kind.type === 'group') : [];
      const children = s ? s.items.filter((i) => groups.some((g) => g.id === i.group)).map((i) => i.id) : [];
      if (apply(groups.map((g) => ({ op: 'ungroup', id: g.id })), 'Grubu çöz')) state.select(children);
    }, { also: () => (state.chosen.some((i) => i.kind === 'group') ? null : 'Önce bir grup seçin.') }),
    // The map's tools of the profile: a grid on the chosen map, the atlas (its plan comes in the next step).
    onSheet('sheet.grid', 'Karelaj', 'sheetGrid', () => {
      const s = sheet();
      const maps = s ? s.items.filter((i) => state.selection.value.has(i.id) && i.kind.type === 'map') : [];
      const grid = defaultGrid(sheets);
      if (!grid) return ctx.log.warn('Karelaj eklenemedi: çalışma modunun şablonlarında örnek karelaj yok.');
      const ops: Op[] = maps.map((m) => {
        const k = m.kind as Extract<typeof m.kind, { type: 'map' }>;
        return { op: 'setItemProps', id: m.id, patch: { kind: { grids: [...k.grids, { ...grid.grid, name: freeGridName(k.grids.map((g) => g.name), grid.grid.name) }], labelBand: Math.max(k.labelBand, grid.band) } } };
      });
      apply(ops, 'Karelaj ekle');
    }, {
      description: 'Seçili haritaya koordinat karelajı ekler: aralık ölçekten seçilir, yazılar çerçevenin yazı bandına yazılır.',
      also: () => toolReason(sheets, 'grid') ?? (state.chosen.some((i) => i.kind === 'map') ? null : 'Önce bir harita seçin.'),
    }),
    onSheet('sheet.atlas', 'Atlas', 'sheetAtlas', () => {}, {
      description: 'Kapsam katmanının her nesnesi için bir sayfa (pafta bölümleme).',
      also: () => toolReason(sheets, 'atlas') ?? 'Atlas düzenleyicisi ve önizlemesi sonraki adımda gelecek; çekirdeğin atlas planı hazır.',
    }),
    // On the chosen items.
    onSheet('sheet.renameItem', 'Öğeye ad ver', 'edit', () => {
      const s = sheet();
      const item = s?.items.find((i) => i.id === chosen()[0]);
      if (item) void hooks.ask('Öğeye ad ver', 'Ad (paftada tekil)', item.name).then((name) => name && name !== item.name && apply([{ op: 'renameItem', id: item.id, name }]));
    }, { short: 'Ad ver', also: () => (state.selection.value.size === 1 ? null : 'Tek bir öğe seçin.') }),
    onSheet('sheet.duplicateItems', 'Çoğalt', 'copy', () => {
      const s = sheet();
      if (!s) return;
      const copies = copyItems(s, chosen(), () => sheets.newId(), 5000);
      const tops = copies.filter((c) => !c.group || !copies.some((x) => x.id === c.group)).map((c) => c.id);
      if (apply([{ op: 'addItems', to: owner(s), items: copies }], `Çoğalt: ${copies.length === 1 ? copies[0].name : `${tops.length} öğe`}`)) state.select(tops);
    }, { also: needsChoice }),
    onSheet('sheet.deleteItems', 'Sil', 'trash', () => {
      const s = sheet();
      if (s) apply([{ op: 'removeItems', ids: withChildren(s, free('Sil')) }]);
    }, { also: needsChoice }),
    onSheet('sheet.hideItems', 'Gizle ya da göster', 'eyeOff', () => {
      const all = state.chosen.every((i) => i.hidden);
      apply([{ op: 'hide', ids: chosen(), value: !all }], all ? 'Göster' : 'Gizle');
    }, { also: needsChoice }),
    onSheet('sheet.lockItems', 'Kilitle ya da aç', 'lock', () => {
      const all = state.chosen.every((i) => i.locked);
      apply([{ op: 'lock', ids: chosen(), value: !all }], all ? 'Kilidi aç' : 'Kilitle');
    }, { also: needsChoice }),
    onSheet('sheet.nudge', 'Kaydır', 'move', (args) => {
      const a = (args ?? {}) as { dx?: number; dy?: number; step?: 'plain' | 'shift' | 'alt' };
      const engine = sheets.engine();
      if (!engine) return;
      // The engine's steps: 1 mm, Shift 10 mm, Alt 0.1 mm.
      const n = engine.info.nudge;
      const step = a.step === 'shift' ? n[1] : a.step === 'alt' ? n[2] : n[0];
      const ids = free('Kaydır');
      if (ids.length) apply([{ op: 'moveItems', ids, delta: [Math.sign(a.dx ?? 0) * step, Math.sign(a.dy ?? 0) * step] }]);
    }, { description: 'Ok tuşları 1 mm, Shift 10 mm, Alt 0.1 mm.', also: needsChoice }),
    onSheet('sheet.selectAll', 'Tümünü seç', 'selectAll', () => state.selectAll()),
    onSheet('sheet.escape', 'Seçimi bırak', 'deselect', () => {
      if (hooks.stage()?.cancelDrag()) return;
      if (state.tool.value.kind !== 'select') state.tool.set(SELECT);
      else state.clearSelection();
    }),
    {
      id: 'sheet.undo',
      title: 'Geri al (pafta)',
      category: P,
      icon: 'undo',
      description: 'Pafta kipinin kendi geri alma yığını: model alanından ayrıdır.',
      run: () => sheets.undo(),
      isEnabled: () => !!sheets.history.value?.canUndo.value,
      whyDisabled: () => (sheets.history.value?.canUndo.value ? null : 'Geri alınacak pafta işlemi yok.'),
      watch,
    },
    {
      id: 'sheet.redo',
      title: 'Yinele (pafta)',
      category: P,
      icon: 'redo',
      run: () => sheets.redo(),
      isEnabled: () => !!sheets.history.value?.canRedo.value,
      whyDisabled: () => (sheets.history.value?.canRedo.value ? null : 'Yinelenecek pafta işlemi yok.'),
      watch,
    },
    // The view of the paper.
    onSheet('sheet.zoomPage', 'Sayfayı sığdır', 'sheetZoomPage', () => hooks.stage()?.fitPage(), { short: 'Sığdır' }),
    onSheet('sheet.zoomReal', 'Gerçek boyut', 'sheetZoomReal', () => hooks.stage()?.realSize(), { short: 'Gerçek', description: '%100: kâğıdın bir milimetresi ekranda bir milimetre (96 dpi).' }),
    onSheet('sheet.zoomSelection', 'Seçime yakınlaş', 'zoomSelection', () => hooks.stage()?.zoomChoice(), { short: 'Seçim', also: needsChoice }),
    onSheet('sheet.zoomIn', 'Yakınlaştır', 'zoomIn', () => hooks.stage()?.step(1)),
    onSheet('sheet.zoomOut', 'Uzaklaştır', 'zoomOut', () => hooks.stage()?.step(-1)),
    // Output.
    onSheet('sheet.preflight', 'Ön denetim', 'sheetPreflight', () => hooks.window('preflight'), { description: 'Dışa aktarmadan önce: sayfa dışında kalan, örtülen, bağı kopuk öğeler, sığmayan yazılar, düşük çözünürlük …' }),
    onSheet('sheet.print', 'Yazdır…', 'print', () => hooks.window('export', { format: 'pdf', print: true }), { short: 'Yazdır', description: 'Paftayı PDF olarak hazırlar ve tarayıcının yazdırma penceresinde açar.' }),
    onSheet('sheet.export.pdf', 'PDF olarak…', 'exportPdf', () => hooks.window('export', { format: 'pdf' }), { description: 'Seçilen paftalar tek PDF’te, her biri bir sayfa: yazılar seçilebilir, haritalar vektör, konumlu haritalar GeoPDF.' }),
    onSheet('sheet.export.svg', 'SVG olarak…', 'exportSvg', () => hooks.window('export', { format: 'svg' })),
    onSheet('sheet.export.png', 'PNG olarak…', 'exportPng', () => hooks.window('export', { format: 'png' })),
    onSheet('sheet.export.kpafta', '.kpafta dosyası olarak…', 'exportKpafta', () => hooks.window('export', { format: 'kpafta' })),
    onSheet('sheet.saveTemplate', 'Şablon olarak kaydet', 'sheetSaveTemplate', () => hooks.window('saveTemplate'), {
      short: 'Şablon olarak kaydet',
      description: 'Paftadan bir şablon çıkarır: haritaların yeri atılır, ölçeği kalır; önce bu cihazda saklanır.',
    }),
  ];

  function alignTo(to: 'selection' | 'page' | 'margins', title: string): Command {
    const icon = { selection: 'sheetAlignToSelection', page: 'sheetAlignToPage', margins: 'sheetAlignToMargins' }[to];
    return onSheet(`sheet.alignTo.${to}`, title, icon, () => state.alignTo.set(to), { checked: () => state.alignTo.value === to });
  }
  function index(args: unknown): number {
    const s = target(args);
    return s ? (sheets.book()?.book.sheets.findIndex((x) => x.id === s.id) ?? -1) : -1;
  }
  function move(args: unknown, dir: -1 | 1): void {
    const s = target(args);
    const at = index(args);
    const n = sheets.book()?.book.sheets.length ?? 0;
    if (s && at + dir >= 0 && at + dir < n) apply([{ op: 'moveSheet', id: s.id, to: at + dir }]);
  }

  return ctx.commands.registerAll(commands);
}

/** Why a tool of the profile cannot be used now (hidden or off in this mode, with the profile's reason); null when it can. */
function toolReason(sheets: SheetService, id: string): string | null {
  const t = sheets.tools.value.find((x) => x.id === id);
  if (!t) return 'Bu çalışma modunda bu araç yok.';
  return t.state === 'enabled' ? null : (t.reason ?? 'Bu projede kullanılamaz.');
}

/**
 * The grid a new one starts as: the first map grid of the mode's templates,
 * in the gallery's order (engine data; the engine has no tool that makes a
 * grid by itself), with that map's label band.
 */
function defaultGrid(sheets: SheetService): { grid: MapGrid; band: number } | null {
  const engine = sheets.engine();
  if (!engine) return null;
  const order = sheets.profile.value.gallery;
  const all = engine.systemTemplates();
  const ranked = [...all].sort((a, b) => (order.indexOf(a.meta.id) + 1 || 999) - (order.indexOf(b.meta.id) + 1 || 999));
  for (const t of ranked)
    for (const i of t.sheet.items) if (i.kind.type === 'map' && i.kind.grids.length) return { grid: i.kind.grids[0], band: i.kind.labelBand };
  return null;
}

const freeGridName = (names: readonly string[], name: string): string => {
  if (!names.includes(name)) return name;
  for (let n = 2; ; n++) if (!names.includes(`${name} ${n}`)) return `${name} ${n}`;
};
