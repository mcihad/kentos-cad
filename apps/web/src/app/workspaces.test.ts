import { describe, expect, it } from 'vitest';
import { BUILTIN_TOOLS } from '../processing/builtin';
import { BUILTIN_MODELS } from '../processing/builtin/models';
import { ProcessingRegistry } from '../processing/registry';
import { WORKSPACE_IDS } from '../model/projectSettings';
import { TOOL_CATALOG } from '../tools/catalog';
import { menuBlocks, visibleMenus, type MenuSpec } from './menus';
import { panelCommands, RIBBON_TABS, ribbonTabs } from './ribbon';
import { effectiveWorkspace, SHOW_ALL, WORKSPACES, workspaceById, workspaceFilter, type WorkspaceFilter } from './workspaces';

const registry = new ProcessingRegistry();
BUILTIN_TOOLS.forEach((t) => registry.register(t));
const tabsIn = (filter: WorkspaceFilter) => ribbonTabs({ tools: TOOL_CATALOG, processing: registry.tree(), models: BUILTIN_MODELS, iconOf: () => undefined, filter });
const filterFor = (id: (typeof WORKSPACE_IDS)[number]) => workspaceFilter(workspaceById(id), TOOL_CATALOG);

/** Commands the shown menus reach (submenus too; processing and models by reference). */
function menuCommands(specs: readonly MenuSpec[], filter: WorkspaceFilter): string[] {
  return menuBlocks(specs, TOOL_CATALOG, filter).flatMap((b) => b.items.flatMap((e): string[] => (typeof e === 'object' ? menuCommands(e.items, filter) : e.startsWith('@') ? [] : [e])));
}
const menusOf = (filter: WorkspaceFilter) => visibleMenus(filter).flatMap((m) => menuCommands(m.items, filter));
const ribbonOf = (filter: WorkspaceFilter) =>
  tabsIn(filter)
    .filter((t) => !t.contextual)
    .flatMap((t) => t.panels.flatMap((p) => [...panelCommands(p), ...p.items.flatMap((i) => (i.kind === 'menu' ? menuCommands(i.menu.items, filter) : []))]));

describe('work modes', () => {
  it('have one entry per contract id; announced modes show as hybrid', () => {
    expect(WORKSPACES.map((w) => w.id).sort()).toEqual([...WORKSPACE_IDS].sort());
    expect(WORKSPACES.filter((w) => w.status === 'ready').map((w) => w.id)).toEqual(['hybrid', 'cad', 'gis']);
    expect(effectiveWorkspace('plan3d').id).toBe('hybrid');
    expect(effectiveWorkspace('disaster').id).toBe('hybrid');
    expect(effectiveWorkspace('gis').id).toBe('gis');
    for (const w of WORKSPACES) expect(w.highlights, w.id).toHaveLength(3);
  });

  it('hybrid shows exactly what an unfiltered interface shows', () => {
    expect(tabsIn(filterFor('hybrid'))).toEqual(tabsIn(SHOW_ALL));
    expect(menusOf(filterFor('hybrid'))).toEqual(menusOf(SHOW_ALL));
  });

  it('shows every tool it keeps in its menus and ribbon, and none it hides', () => {
    for (const w of WORKSPACES.filter((x) => x.status === 'ready')) {
      const filter = filterFor(w.id);
      const shown = TOOL_CATALOG.filter((t) => filter.tool(t)).map((t) => `tool.${t.id}`);
      const hidden = TOOL_CATALOG.filter((t) => !filter.tool(t)).map((t) => `tool.${t.id}`);
      const menus = new Set(menusOf(filter));
      const ribbon = new Set(ribbonOf(filter));
      // The select tool is Esc and the toolbox's first button, not a menu item.
      expect(shown.filter((id) => !menus.has(id) && id !== 'tool.select'), `${w.id} menus`).toEqual([]);
      expect(shown.filter((id) => !ribbon.has(id)), `${w.id} ribbon`).toEqual([]);
      expect(hidden.filter((id) => menus.has(id) || ribbon.has(id)), `${w.id} hidden`).toEqual([]);
      // Every menu command it keeps is on the ribbon too.
      expect([...menus].filter((id) => !ribbon.has(id) && !['edit.undo', 'edit.redo'].includes(id)), `${w.id} commands`).toEqual([]);
    }
  });

  it('names only known menus, tools and commands', () => {
    const groups = new Set(TOOL_CATALOG.flatMap((t) => [t.group, `${t.group}/${t.section ?? ''}`]));
    const tools = new Set(TOOL_CATALOG.map((t) => `tool.${t.id}`));
    const commands = new Set(menusOf(SHOW_ALL));
    for (const w of WORKSPACES) {
      for (const m of w.hide?.menus ?? []) expect(visibleMenus(SHOW_ALL).some((x) => x.id === m), `${w.id}: ${m}`).toBe(true);
      for (const t of w.hide?.tools ?? []) expect(groups.has(t) || tools.has(t), `${w.id}: ${t}`).toBe(true);
      for (const c of w.hide?.commands ?? []) expect(commands.has(c), `${w.id}: ${c}`).toBe(true);
    }
    for (const spec of RIBBON_TABS) {
      for (const id of Object.keys(spec.labels ?? {})) expect(WORKSPACE_IDS, spec.id).toContain(id);
      for (const src of spec.sources) if ('pick' in src) for (const id of src.workspaces ?? []) expect(WORKSPACE_IDS).toContain(id);
    }
  });

  it('CAD leaves out the map, coordinate and processing menus; what is left of the map tab is measuring', () => {
    const cad = filterFor('cad');
    expect(visibleMenus(cad).map((m) => m.id)).toEqual(['file', 'edit', 'view', 'draw', 'modify', 'analysis', 'tools', 'help']);
    const tabs = tabsIn(cad);
    expect(tabs.some((t) => t.id === 'processing')).toBe(false);
    const map = tabs.find((t) => t.id === 'map')!;
    expect(map.label).toBe('Ölçme');
    expect(map.panels.flatMap(panelCommands)).toEqual(['tool.measure', 'tool.area']);
    expect(cad.command('tool.parcel')).toBe(false);
    expect(cad.command('tool.hatch')).toBe(true);
  });

  it('GIS keeps the map work in front and leaves out drafting-only tools', () => {
    const gis = filterFor('gis');
    const home = tabsIn(gis).find((t) => t.id === 'home')!;
    expect(home.panels.map((p) => p.label)).toContain('Harita');
    expect(panelCommands(home.panels.find((p) => p.label === 'Harita')!)).toEqual(['tool.parcel', 'tool.boundary', 'tool.areaUnion', 'tool.measure', 'tool.area']);
    const ribbon = new Set(ribbonOf(gis));
    for (const id of ['tool.hatch', 'tool.dimension', 'tool.rectangle', 'tool.xline', 'tool.fillet', 'tool.array']) expect(ribbon.has(id), id).toBe(false);
    for (const id of ['tool.line', 'tool.polyline', 'tool.polygon', 'tool.parcel', 'processing.toolbox', 'crs.set']) expect(ribbon.has(id), id).toBe(true);
    expect(tabsIn(filterFor('hybrid')).find((t) => t.id === 'home')!.panels.some((p) => p.label === 'Harita')).toBe(false);
  });
});
