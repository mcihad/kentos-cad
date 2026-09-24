import { describe, expect, it } from 'vitest';
import { BUILTIN_TOOLS } from '../processing/builtin';
import { BUILTIN_MODELS } from '../processing/builtin/models';
import { ProcessingRegistry } from '../processing/registry';
import { TOOL_CATALOG } from '../tools/catalog';
import { TOOL_SECTIONS, type ToolDescriptor } from '../tools/Tool';
import { toolSections } from '../tools/sections';
import { MAIN_MENU, menuBlocks, menuById, type MenuSpec } from './menus';
import { modelCommandId, processingCommandId } from './processing';

const registry = new ProcessingRegistry();
BUILTIN_TOOLS.forEach((t) => registry.register(t));

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

const allMenus = (tools: readonly ToolDescriptor[]) => MAIN_MENU.flatMap((m) => menuCommands(m.items, tools));

describe('tool sections', () => {
  it('each tool names a section of its own group', () => {
    for (const t of TOOL_CATALOG) {
      if (!t.section) continue;
      const own: Record<string, string> = (TOOL_SECTIONS as Record<string, Record<string, string>>)[t.group] ?? {};
      expect(own, `${t.id} → ${t.group}/${t.section}`).toHaveProperty(t.section);
    }
  });

  it('lists a group section by section in display order, then tools without one under the group name', () => {
    const extra: ToolDescriptor = { ...TOOL_CATALOG.find((t) => t.id === 'line')!, id: 'yeni', section: undefined };
    const sections = toolSections([...TOOL_CATALOG, extra], 'draw');
    expect(sections.map((s) => s.label)).toEqual(['Çizgi', 'Eğri', 'Şekil', 'Yardımcı', 'Nokta', 'Çizim']);
    expect(sections[0].tools.map((t) => t.id)).toEqual(['line', 'polyline', 'polygon', 'parallel']);
    expect(sections.at(-1)!.tools.map((t) => t.id)).toEqual(['yeni']);
  });
});

describe('menus from the catalog', () => {
  it('reach every tool in the catalog', () => {
    const reached = new Set(allMenus(TOOL_CATALOG));
    const missing = TOOL_CATALOG.map((t) => `tool.${t.id}`).filter((id) => id !== 'tool.select' && !reached.has(id));
    expect(missing).toEqual([]);
  });

  it('turn tool sections into blocks and merge blocks with the same title', () => {
    const draw = menuBlocks(menuById('draw')!.items, TOOL_CATALOG);
    expect(draw.map((b) => b.label)).toEqual(['Çizgi', 'Eğri', 'Şekil', 'Yardımcı', 'Nokta', 'Açıklama']);
    const map = menuBlocks(menuById('map')!.items, TOOL_CATALOG);
    expect(map.map((b) => b.label)).toEqual(['Parsel', 'Arazi', 'Ölçme', 'Pafta']);
    expect(map[0].items).toEqual(['tool.parcel', 'tool.subdivide', 'map.parcelReport', 'map.edgeLengths']);
  });

  it('list a command once per menu, and keep plain separators as untitled blocks', () => {
    for (const m of MAIN_MENU) {
      const ids = menuBlocks(m.items, TOOL_CATALOG).flatMap((b) => b.items.filter((e): e is string => typeof e === 'string'));
      expect(new Set(ids).size, m.id).toBe(ids.length);
    }
    expect(menuBlocks(['a', '-', 'b', { section: 'X' }, 'c', { section: 'X' }, 'd'], []).map((b) => [b.label, b.items])).toEqual([
      ['', ['a']],
      ['', ['b']],
      ['X', ['c', 'd']],
    ]);
  });

  it('shows a tool added to the catalog in the menu of its group, with nothing else to edit', () => {
    const added: ToolDescriptor = { ...TOOL_CATALOG.find((t) => t.id === 'arc')!, id: 'clothoid', label: 'Klotoid', section: 'curve' };
    const loose: ToolDescriptor = { ...TOOL_CATALOG.find((t) => t.id === 'trim')!, id: 'cleanup', label: 'Temizle', section: undefined };
    const tools = [...TOOL_CATALOG, added, loose];
    expect(menuBlocks(menuById('draw')!.items, tools).find((b) => b.label === 'Eğri')!.items).toContain('tool.clothoid');
    expect(menuBlocks(menuById('modify')!.items, tools).find((b) => b.label === 'Düzenle')!.items).toEqual(['tool.cleanup']);
  });
});
