import type { MenuItem } from '../ui/widgets/PopupMenu';
import type { CategoryNode } from '../processing/registry';
import type { ToolDescriptor } from '../tools/Tool';
import { parseToolRef, toolSections } from '../tools/sections';
import type { AppContext } from './context';
import { modelCommandId, processingCommandId } from './processing';
import { filterOf, SHOW_ALL, type WorkspaceFilter } from './workspaces';

/**
 * Declarative main menu: the single source for where commands live. The
 * classic menu bar and the ribbon (app/ribbon.ts) are both built from it.
 *
 * - A string is a command id, or a reference expanded on use:
 *   `@tools:draw` (every tool of a group, one section per tool section),
 *   `@tools:map/measure` (one section), `@processing` (the processing
 *   categories with their tools), `@models` (the model library).
 * - `{ section }` starts a titled block: a separator in the menu, a panel
 *   in the ribbon. Blocks with the same title merge, so a menu can add its
 *   own commands to a tool section (`Parsel`: the tools, then the report).
 * - "-" starts an untitled block.
 * - `{ label, items }` is a submenu (a drop-down button in the ribbon, or
 *   its blocks as panels when `inline`).
 *
 * Tools are never listed one by one: a tool added to tools/catalog.ts
 * appears in its group's menu and ribbon tab by itself.
 */
export type MenuSpec = string | MenuSection | SubmenuSpec;

export interface MenuSection {
  readonly section: string;
}

export interface SubmenuSpec {
  readonly label: string;
  readonly icon?: string;
  readonly items: readonly MenuSpec[];
  /** Ribbon: show the items as panels of their own instead of a drop-down button. */
  readonly inline?: boolean;
  /** Ribbon: the main thing of its panel, drawn large. */
  readonly primary?: boolean;
}

/** What a block holds once references are expanded: command ids, submenus, `@processing`, `@models`. */
export type MenuEntry = string | SubmenuSpec;

/** A titled (or untitled, label '') block of a menu. */
export interface MenuBlock {
  readonly label: string;
  readonly items: MenuEntry[];
}

export interface TopMenu {
  id: string;
  label: string;
  items: readonly MenuSpec[];
}

const sec = (section: string): MenuSection => ({ section });

export const MAIN_MENU: TopMenu[] = [
  {
    id: 'file',
    label: 'Dosya',
    items: [
      sec('Proje'),
      'file.new',
      'file.open',
      'file.start',
      sec('Kaydet'),
      'file.save',
      'file.saveAs',
      sec('Bulut'),
      'cloud.open',
      'cloud.upload',
      'cloud.conflicts',
      'cloud.rename',
      'cloud.delete',
      sec('Dosya alışverişi'),
      { label: 'İçe aktar', icon: 'import', items: ['file.import.dxf', 'file.import.ncz', 'file.import.shp', 'file.import.geojson', '-', 'file.import.ncn'] },
      { label: 'Dışa aktar', icon: 'export', items: ['file.export.dxf', 'file.export.geojson', 'file.export.pdf', '-', 'file.export.ncn'] },
      sec('Çıktı'),
      'file.print',
      sec('Ayarlar'),
      'file.settings',
    ],
  },
  {
    id: 'edit',
    label: 'Düzen',
    items: [
      sec('Geçmiş'),
      'edit.undo',
      'edit.redo',
      sec('Pano'),
      'edit.cut',
      'edit.copy',
      'edit.paste',
      'edit.pasteOriginal',
      'tool.erase',
      sec('Seçim'),
      'edit.selectAll',
      'edit.deselect',
      'edit.invertSelection',
    ],
  },
  {
    id: 'view',
    label: 'Görünüm',
    items: [
      sec('Yakınlaştır'),
      'view.zoomExtents',
      'tool.zoomWindow',
      'view.zoomSelection',
      'view.zoomIn',
      'view.zoomOut',
      'tool.pan',
      sec('Paneller'),
      'view.toolbox',
      'view.toolboxDock',
      'view.rightPanel',
      'view.bottomPanel',
      'view.coords',
      'view.ribbon',
      'view.keyTips',
      'view.fullscreen',
      sec('Görünüş'),
      { label: 'Çalışma modu', icon: 'modeHybrid', primary: true, items: ['workspace.hybrid', 'workspace.cad', 'workspace.gis', '-', 'workspace.plan3d', 'workspace.disaster'] },
      { label: 'Tema', icon: 'appearance', items: ['view.theme.dark', 'view.theme.light'] },
      { label: 'Çizim motoru', icon: 'chip', items: ['view.renderer.webgl2', 'view.renderer.webgpu'] },
      { label: 'Sembol boyutu', icon: 'styles', items: ['view.symbols.plot', 'view.symbols.screen'] },
      'view.lineWeights',
    ],
  },
  {
    id: 'draw',
    label: 'Çizim',
    items: ['@tools:draw', '@tools:annotate'],
  },
  {
    id: 'modify',
    label: 'Değiştir',
    items: ['@tools:transform', '@tools:modify', { label: 'Alan işlemleri', icon: 'areaUnion', inline: true, items: ['@tools:area'] }],
  },
  {
    id: 'map',
    label: 'Harita',
    items: ['@tools:map', sec('Parsel'), 'map.parcelReport', 'map.edgeLengths', sec('Arazi'), 'map.contours', 'map.profile', sec('Pafta'), 'map.sheet'],
  },
  {
    id: 'crs',
    label: 'Koordinat',
    items: [sec('Koordinat sistemi'), 'crs.set', 'crs.transform', sec('Koordinatlar'), 'crs.query', 'crs.points', 'view.coords'],
  },
  {
    id: 'analysis',
    label: 'Analiz',
    items: ['@tools:map/measure', sec('Arazi analizi'), 'analysis.volume', 'analysis.slope'],
  },
  {
    id: 'processing',
    label: 'İşlemler',
    items: [sec('İşlemler'), 'processing.toolbox', 'processing.history', sec('Modeller'), '@models', '@processing'],
  },
  {
    id: 'tools',
    label: 'Araçlar',
    items: [
      sec('Komut'),
      'commandline.focus',
      { label: 'Çizim yardımcıları', icon: 'snap', inline: true, items: ['draft.snap', 'draft.grid', 'draft.ortho', 'draft.polar', 'draft.tracking'] },
      sec('Stil'),
      'style.manager',
      'style.svgEditor',
      'style.layerStyle',
      'style.legend',
      'style.assign',
      'style.clearSymbol',
      sec('Uygulama'),
      'help.shortcuts',
      'server.check',
      'tools.options',
    ],
  },
  {
    id: 'help',
    label: 'Yardım',
    items: ['help.shortcuts', '-', 'help.about'],
  },
];

export const menuById = (id: string): TopMenu | undefined => MAIN_MENU.find((m) => m.id === id);

/** The main menus a work mode shows, in order. */
export const visibleMenus = (filter: WorkspaceFilter, menus: readonly TopMenu[] = MAIN_MENU): TopMenu[] => menus.filter((m) => filter.menu(m.id));

/**
 * Expands a menu into its blocks: tool references become the tools of the
 * catalog (section by section), blocks with the same title merge, and a
 * command is listed once (its first place wins).
 */
export function menuBlocks(specs: readonly MenuSpec[], allTools: readonly ToolDescriptor[], filter: WorkspaceFilter = SHOW_ALL): MenuBlock[] {
  // The work mode's hidden tools and commands are left out (they still run from the command line).
  const tools = allTools.filter((t) => filter.tool(t));
  const out: { label: string; items: MenuEntry[] }[] = [];
  let current: { label: string; items: MenuEntry[] } | null = null;
  const seen = new Set<string>();
  const open = (label: string) => {
    const found = label ? out.find((b) => b.label === label) : undefined;
    current = found ?? { label, items: [] };
    if (!found) out.push(current);
    return current;
  };
  const add = (block: { items: MenuEntry[] }, entry: MenuEntry) => {
    // A submenu with nothing left to show in this mode is left out too.
    if (typeof entry === 'object' && !entry.inline && !menuBlocks(entry.items, tools, filter).length) return;
    if (typeof entry === 'string' && !entry.startsWith('@')) {
      if (seen.has(entry) || !filter.command(entry)) return;
      seen.add(entry);
    }
    block.items.push(entry);
  };
  for (const spec of specs) {
    if (typeof spec === 'object') {
      if ('section' in spec) open(spec.section);
      else add(current ?? open(''), spec);
    } else if (spec === '-') {
      current = null;
    } else if (spec.startsWith('@tools:')) {
      const ref = parseToolRef(spec.slice('@tools:'.length));
      if (!ref) continue;
      for (const s of toolSections(tools, ref.group)) {
        if (ref.section && s.id !== `${ref.group}/${ref.section}`) continue;
        const block = open(s.label);
        for (const t of s.tools) add(block, `tool.${t.id}`);
      }
      // What follows a tool reference starts a block of its own.
      current = null;
    } else add(current ?? open(''), spec);
  }
  return out.filter((b) => b.items.length);
}

/** Resolves specs against the live command registry (enabled/checked/shortcut). */
export function resolveMenu(ctx: AppContext, specs: readonly MenuSpec[]): MenuItem[] {
  const out: MenuItem[] = [];
  for (const block of menuBlocks(specs, ctx.tools.list(), filterOf(ctx))) {
    if (out.length) out.push({ kind: 'separator' });
    for (const e of block.items) out.push(...entryItems(ctx, e));
  }
  return out;
}

function entryItems(ctx: AppContext, e: MenuEntry): MenuItem[] {
  if (e === '@processing') return processingMenu(ctx, ctx.processing.registry.tree());
  if (e === '@models') return [modelsMenu(ctx)];
  if (typeof e === 'object') return [{ label: e.label, icon: e.icon, items: () => resolveMenu(ctx, e.items) }];
  return [commandItem(ctx, e)];
}

/** The model library: run a model, or design a new one. */
export function modelsMenu(ctx: AppContext): MenuItem {
  return {
    label: 'Modeller',
    icon: 'processing',
    items: () => [...ctx.processing.models.value.map((m) => commandItem(ctx, modelCommandId(m.id))), { kind: 'separator' }, commandItem(ctx, 'processing.newModel')],
  };
}

/** One submenu per processing category, its tools as commands (the registry decides what exists). */
function processingMenu(ctx: AppContext, nodes: CategoryNode[]): MenuItem[] {
  return nodes.map((n) => ({
    label: n.category.label,
    icon: n.category.icon,
    items: () => [...processingMenu(ctx, n.children), ...n.tools.map((t) => commandItem(ctx, processingCommandId(t.id)))],
  }));
}

export function commandItem(ctx: AppContext, id: string, overrides: Partial<MenuItem> = {}): MenuItem {
  const cmd = ctx.commands.get(id);
  if (!cmd) return { label: id, disabled: true };
  const checked = cmd.isChecked?.();
  const isRadio = (id.startsWith('view.theme.') && id !== 'view.theme.toggle') || id.startsWith('view.renderer.') || id.startsWith('view.symbols.') || id.startsWith('workspace.');
  // Tools report "active" via isChecked, but in menus they read as actions.
  const isTool = id.startsWith('tool.');
  return {
    label: cmd.title,
    icon: checked === undefined || isRadio || isTool ? cmd.icon : undefined,
    shortcut: ctx.keymap.chordFor(id),
    checked: isTool ? undefined : checked,
    radio: isRadio,
    disabled: !ctx.commands.isEnabled(id),
    hint: cmd.pendingNote,
    run: () => ctx.commands.execute(id),
    ...overrides,
  };
}
