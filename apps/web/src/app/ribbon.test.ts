import { describe, expect, it } from 'vitest';
import { BUILTIN_TOOLS } from '../processing/builtin';
import { BUILTIN_MODELS } from '../processing/builtin/models';
import { ProcessingRegistry } from '../processing/registry';
import { TOOL_CATALOG } from '../tools/catalog';
import type { ToolDescriptor } from '../tools/Tool';
import { MAIN_MENU, menuBlocks, menuById, type MenuSpec } from './menus';
import { modelCommandId, processingCommandId } from './processing';
import { QUICK_ACCESS, RIBBON_TABS, ribbonTabs, type RibbonInputs, type RibbonTab } from './ribbon';

const registry = new ProcessingRegistry();
BUILTIN_TOOLS.forEach((t) => registry.register(t));

function inputs(tools: readonly ToolDescriptor[] = TOOL_CATALOG): RibbonInputs {
  return { tools, processing: registry.tree(), models: BUILTIN_MODELS, iconOf: () => undefined };
}

/** Every command a menu reaches, through its submenus and references. */
function menuCommands(specs: readonly MenuSpec[], tools: readonly ToolDescriptor[]): string[] {
  return menuBlocks(specs, tools).flatMap((b) =>
    b.items.flatMap((e): string[] => {
      if (typeof e === 'object') return menuCommands(e.items, tools);
      if (e === '@processing') return registry.list().map((t) => processingCommandId(t.id));
      if (e === '@models') return ['processing.newModel', ...BUILTIN_MODELS.map((m) => modelCommandId(m.id))];
      return [e];
    }),
  );
}

/** Every command a tab set reaches: its buttons and what its drop-down buttons open. */
function ribbonCommands(tabs: readonly RibbonTab[], tools: readonly ToolDescriptor[]): string[] {
  return tabs.flatMap((t) =>
    t.panels.flatMap((p) =>
      p.items.flatMap((i): string[] => {
        if (i.kind === 'command') return [i.id];
        if (i.kind === 'menu') return menuCommands(i.menu.items, tools);
        return [];
      }),
    ),
  );
}

const allMenus = (tools: readonly ToolDescriptor[]) => MAIN_MENU.flatMap((m) => menuCommands(m.items, tools));

describe('ribbon', () => {
  const tabs = ribbonTabs(inputs());
  const regular = tabs.filter((t) => !t.contextual);

  it('reaches every tool and every menu command (the quick access bar holds undo and redo)', () => {
    const reached = new Set([...ribbonCommands(regular, TOOL_CATALOG), ...QUICK_ACCESS]);
    expect(TOOL_CATALOG.map((t) => `tool.${t.id}`).filter((id) => !reached.has(id))).toEqual([]);
    expect([...new Set(allMenus(TOOL_CATALOG))].filter((id) => !reached.has(id))).toEqual([]);
  });

  it('shows a tool added to the catalog in its menu and ribbon tab, with nothing else to edit', () => {
    const added: ToolDescriptor = { ...TOOL_CATALOG.find((t) => t.id === 'arc')!, id: 'clothoid', label: 'Klotoid', section: 'curve' };
    const loose: ToolDescriptor = { ...TOOL_CATALOG.find((t) => t.id === 'trim')!, id: 'cleanup', label: 'Temizle', section: undefined };
    const tools = [...TOOL_CATALOG, added, loose];
    expect(allMenus(tools)).toEqual(expect.arrayContaining(['tool.clothoid', 'tool.cleanup']));
    const withNew = ribbonTabs(inputs(tools));
    const draw = withNew.find((t) => t.id === 'draw')!;
    expect(draw.panels.find((p) => p.label === 'Eğri')!.items).toContainEqual({ kind: 'command', id: 'tool.clothoid', size: 'small' });
    const modify = withNew.find((t) => t.id === 'modify')!;
    expect(modify.panels.find((p) => p.label === 'Düzenle')!.items).toContainEqual({ kind: 'command', id: 'tool.cleanup', size: 'large' });
  });

  it('builds the processing tab from the registry and the model library', () => {
    const processing = tabs.find((t) => t.id === 'processing')!;
    const ids = ribbonCommands([processing], TOOL_CATALOG);
    for (const t of registry.list()) expect(ids).toContain(processingCommandId(t.id));
    for (const m of BUILTIN_MODELS) expect(ids).toContain(modelCommandId(m.id));
    const categories = registry.tree().map((n) => n.category.label);
    expect(processing.panels.map((p) => p.label)).toEqual(expect.arrayContaining(categories));
  });

  it('has unique panel titles and lists a command once in each tab', () => {
    for (const t of tabs) {
      const labels = t.panels.map((p) => p.label);
      expect(new Set(labels).size, t.id).toBe(labels.length);
      const ids = t.panels.flatMap((p) => p.items.flatMap((i) => (i.kind === 'command' ? [i.id] : [])));
      expect(new Set(ids).size, t.id).toBe(ids.length);
    }
  });

  it('draws the first item of a panel large (or its leading command), every item of a panel of one or two, and what the tab asks small', () => {
    const home = tabs.find((t) => t.id === 'home')!;
    const draw = home.panels.find((p) => p.label === 'Çizim')!;
    expect(draw.items.map((i) => (i.kind === 'command' ? `${i.id}:${i.size}` : ''))).toEqual([
      'tool.line:large',
      'tool.polyline:large',
      'tool.polygon:small',
      'tool.circle:small',
      'tool.arc:small',
      'tool.rectangle:small',
    ]);
    expect(draw.launcher).toEqual({ tab: 'draw', title: 'Tüm araçlar: Çizim sekmesi' });
    const pano = home.panels.find((p) => p.label === 'Pano')!;
    expect(pano.items[0]).toEqual({ kind: 'command', id: 'edit.paste', size: 'large' });
    expect(pano.items.slice(1).every((i) => i.kind === 'command' && i.size === 'small')).toBe(true);
    expect(home.panels.find((p) => p.label === 'Seçim')!.items.every((i) => i.kind === 'command' && i.size === 'small')).toBe(true);
    const ölçme = tabs.find((t) => t.id === 'map')!.panels.find((p) => p.label === 'Ölçme')!;
    expect(ölçme.items.every((i) => i.kind === 'command' && i.size === 'large')).toBe(true);
  });

  it('names only known tabs, commands and menus', () => {
    const known = new Set([...TOOL_CATALOG.map((t) => `tool.${t.id}`), ...allMenus(TOOL_CATALOG), 'style.assign', 'style.clearSymbol']);
    for (const spec of RIBBON_TABS) {
      for (const src of spec.sources) {
        if ('menu' in src) expect(menuById(src.menu), src.menu).toBeTruthy();
        if ('pick' in src) for (const id of src.commands) expect(known.has(id), id).toBe(true);
        if ('pick' in src && src.more) expect(RIBBON_TABS.some((t) => t.id === src.more)).toBe(true);
      }
      for (const [label, l] of Object.entries(spec.launchers ?? {})) {
        expect(tabs.find((t) => t.id === spec.id)!.panels.some((p) => p.label === label), `${spec.id}: ${label}`).toBe(true);
        if ('command' in l) expect(known.has(l.command), l.command).toBe(true);
      }
    }
    for (const id of QUICK_ACCESS) expect(known.has(id), id).toBe(true);
  });
});
