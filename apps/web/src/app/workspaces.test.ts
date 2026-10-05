import { describe, expect, it } from 'vitest';
import { BUILTIN_TOOLS } from '../processing/builtin';
import { BUILTIN_MODELS } from '../processing/builtin/models';
import { ProcessingRegistry } from '../processing/registry';
import { WORKSPACE_IDS } from '../model/projectSettings';
import { TOOL_CATALOG } from '../tools/catalog';
import { ICONS } from '../ui/icons';
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

describe('project types (docs/adr/0165)', () => {
  it('have one entry per contract id; one not asked yet and the announced ones show as CBS', () => {
    expect(WORKSPACES.map((w) => w.id).sort()).toEqual([...WORKSPACE_IDS].sort());
    expect(WORKSPACES.filter((w) => w.status === 'ready').map((w) => w.id)).toEqual(['cad', 'gis']);
    expect(effectiveWorkspace('plan3d').id).toBe('gis');
    expect(effectiveWorkspace('disaster').id).toBe('gis');
    expect(effectiveWorkspace(null).id).toBe('gis');
    expect(effectiveWorkspace('cad').id).toBe('cad');
    for (const w of WORKSPACES) expect(w.highlights, w.id).toHaveLength(3);
  });

  it('loses no command: what one type leaves out the other shows', () => {
    const both = new Set([...menusOf(filterFor('cad')), ...menusOf(filterFor('gis'))]);
    expect(menusOf(SHOW_ALL).filter((id) => !both.has(id))).toEqual([]);
  });

  it('shows every tool it keeps in its menus and ribbon, and none it hides', () => {
    for (const w of WORKSPACES.filter((x) => x.status === 'ready')) {
      const filter = filterFor(w.id);
      const shown = TOOL_CATALOG.filter((t) => filter.tool(t)).map((t) => `tool.${t.id}`);
      const hidden = TOOL_CATALOG.filter((t) => !filter.tool(t)).map((t) => `tool.${t.id}`);
      const menus = new Set(menusOf(filter));
      const ribbon = new Set(ribbonOf(filter));
      // The select tool is Esc and the ribbon's first tool, not a menu item.
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

  it('gives CAD AutoCAD’s drafting tabs, measuring on Giriş with the survey computations under its ▾ (docs/adr/0165 §6)', () => {
    const cad = filterFor('cad');
    // Survey computations (Hesap) are measuring: CAD keeps them.
    expect(visibleMenus(cad).map((m) => m.id)).toEqual(['file', 'edit', 'view', 'draw', 'modify', 'calc', 'analysis', 'tools', 'help']);
    const tabs = tabsIn(cad);
    expect(tabs.filter((t) => !t.contextual).map((t) => t.label)).toEqual(['Dosya', 'Giriş', 'Ekle', 'Açıklama', 'Değiştir', 'Görünüm', 'Yönet', 'Çıktı']);
    expect(tabs.filter((t) => t.contextual).map((t) => t.label)).toEqual(['Seçim']);
    const home = tabs.find((t) => t.id === 'home')!;
    expect(home.panels.map((p) => p.label)).toEqual(['Pano', 'Seçim', 'Çizim', 'Değiştir', 'Açıklama', 'Katmanlar', 'Şablonlar', 'Blok', 'Özellikler', 'Ölçme']);
    const ölçme = home.panels.find((p) => p.label === 'Ölçme')!;
    expect(ölçme.overflow).toEqual(['calc.fieldbook', 'file.import.gnss', 'field.send', 'calc.traverse', 'calc.polar', 'calc.stakeout', 'calc.forward', 'calc.resection']);
    // The other drawing tools under Çizim's ▾: there is no drawing tab. Elips, Eğri and Nokta show.
    const çizim = home.panels.find((p) => p.label === 'Çizim')!;
    expect(çizim.overflow).toEqual(expect.arrayContaining(['tool.xline', 'tool.donut', 'tool.divide', 'tool.pointsBetween']));
    expect(panelCommands(çizim)).toEqual(expect.arrayContaining(['tool.ellipse', 'tool.spline', 'tool.point']));
    expect(çizim.overflow).not.toContain('tool.ellipse');
    // Açıklama a panel a kind, as AutoCAD's Annotate; a pick shows what it names, seldom used or not.
    const panel = (tab: string, label: string) => tabs.find((t) => t.id === tab)!.panels.find((p) => p.label === label)!;
    expect(tabs.find((t) => t.id === 'annotate')!.panels.map((p) => p.label)).toEqual(['Yazı', 'Ölçü', 'Kılavuz', 'Tarama', 'İşaretleme']);
    expect(panelCommands(panel('annotate', 'İşaretleme'))).toEqual(['tool.revcloud']);
    expect(panel('manage', 'Temizlik').overflow).toBeUndefined();
    expect(panelCommands(panel('manage', 'Temizlik'))).toEqual(['tool.cleanup', 'tool.topology', 'block.purge']);
    expect(cad.command('tool.parcel')).toBe(false);
    expect(cad.command('tool.hatch')).toBe(true);
  });

  it('gives CBS the map work’s tabs: Harita, Veri, Düzenle, Analiz, Ölçme (docs/adr/0165 §6)', () => {
    const tabs = tabsIn(filterFor('gis'));
    expect(tabs.filter((t) => !t.contextual).map((t) => t.label)).toEqual(['Dosya', 'Giriş', 'Harita', 'Veri', 'Düzenle', 'Analiz', 'Ölçme', 'Görünüm', 'Çıktı']);
    const panels = (id: string) => tabs.find((t) => t.id === id)!.panels.map((p) => p.label);
    expect(panels('survey')).toEqual(expect.arrayContaining(['Poligon', 'Nokta alımı', 'Kestirme', 'Noktalar']));
    expect(panels('analysis')).toEqual(expect.arrayContaining(['İşlemler', 'Modeller', 'Arazi analizi']));
    expect(panels('map')).toEqual(expect.arrayContaining(['Koordinat sistemi', 'Parsel', 'Ölçme', 'Stil']));
    // Blocks are the drawing's library, on Veri: Düzenle keeps to creating and changing objects.
    expect(panels('data')).toEqual(['Katman', 'Dosya alışverişi', 'Koordinatlar', 'Öznitelik', 'Blok']);
    expect(panels('edit')).not.toContain('Blok');
  });

  it('gives each type’s panels a title once a tab, a command once a tab and an icon that is drawn', () => {
    for (const id of ['cad', 'gis'] as const) {
      for (const t of tabsIn(filterFor(id))) {
        const labels = t.panels.map((p) => p.label);
        expect(new Set(labels).size, `${id} ${t.id}`).toBe(labels.length);
        const ids = t.panels.flatMap(panelCommands);
        expect(new Set(ids).size, `${id} ${t.id}`).toBe(ids.length);
        for (const p of t.panels) expect(ICONS, `${id} ${t.id} › ${p.label}`).toHaveProperty(p.icon);
      }
    }
  });

  it('points every “Tüm araçlar” of a type’s Giriş at a tab of that type', () => {
    for (const id of ['cad', 'gis'] as const) {
      const tabs = tabsIn(filterFor(id));
      for (const t of tabs)
        for (const p of t.panels) if (p.launcher && 'tab' in p.launcher) expect(tabs.some((x) => x.id === (p.launcher as { tab: string }).tab), `${id} ${t.id} › ${p.label}`).toBe(true);
    }
  });

  it('GIS keeps the map work in front and leaves out drafting-only tools', () => {
    const gis = filterFor('gis');
    const home = tabsIn(gis).find((t) => t.id === 'home')!;
    expect(home.panels.map((p) => p.label)).toContain('Harita');
    expect(panelCommands(home.panels.find((p) => p.label === 'Harita')!)).toEqual(['tool.parcel', 'tool.boundary', 'tool.areaUnion', 'tool.measure', 'tool.area']);
    const ribbon = new Set(ribbonOf(gis));
    for (const id of ['tool.hatch', 'tool.dimension', 'tool.rectangle', 'tool.xline', 'tool.fillet', 'tool.array']) expect(ribbon.has(id), id).toBe(false);
    for (const id of ['tool.line', 'tool.polyline', 'tool.polygon', 'tool.parcel', 'processing.toolbox', 'crs.set']) expect(ribbon.has(id), id).toBe(true);
    // A type's own panel is left out where no type filters (the inventory's places).
    expect(tabsIn(SHOW_ALL).find((t) => t.id === 'home')!.panels.some((p) => p.label === 'Harita')).toBe(false);
  });
});
