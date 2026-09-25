import type { CategoryNode } from '../processing/registry';
import type { ToolDescriptor, ToolGroup } from '../tools/Tool';
import { toolSections } from '../tools/sections';
import { menuBlocks, menuById, type MenuBlock, type SubmenuSpec } from './menus';
import { modelCommandId, processingCommandId } from './processing';
import { SHOW_ALL, type WorkspaceFilter } from './workspaces';
import type { Workspace } from '../model/projectSettings';

/**
 * The ribbon (Şerit) as data. Tabs are composed from the main menu
 * (app/menus.ts) and the tool catalog; they never list tools or menu
 * commands one by one, so anything added there shows up here by itself.
 * Only the Giriş tab hand-picks the everyday tools (each also lives in its
 * own tab) and the quick access bar starts with save, undo and redo.
 */
export type RibbonSource =
  /** The titled blocks of a main menu, one panel each (optionally only the named ones). */
  | { readonly menu: string; readonly sections?: readonly string[] }
  /** A tool group from the catalog, one panel per tool section. */
  | { readonly tools: ToolGroup }
  /**
   * A hand-picked panel; `more` names the tab that holds the whole family,
   * `compact` draws every button small (AutoCAD's Modify panel), and
   * `workspaces` shows it only in those work modes.
   */
  | {
      readonly pick: string;
      readonly icon: string;
      readonly commands: readonly string[];
      readonly more?: string;
      readonly compact?: boolean;
      readonly workspaces?: readonly Workspace[];
    }
  /** Panels with live fields rather than commands. */
  | { readonly builtin: BuiltinPanel };

export type BuiltinPanel = 'layers' | 'properties' | 'selection';

export interface RibbonTabSpec {
  readonly id: string;
  readonly label: string;
  /** The tab's name in a work mode where what is left of it is better called otherwise. */
  readonly labels?: Partial<Record<Workspace, string>>;
  readonly sources: readonly RibbonSource[];
  /** Commands that lead their panel (moved to its front). */
  readonly lead?: readonly string[];
  /** Panels that keep their labels longest when the window narrows (by title). */
  readonly keep?: readonly string[];
  /** A contextual tab: shown only while its subject exists (a selection under the select tool). */
  readonly contextual?: 'selection';
  /** Dialog launchers (the corner button of a panel title), by panel label. */
  readonly launchers?: Readonly<Record<string, RibbonLauncher>>;
}

export type RibbonLauncher = { readonly command: string; readonly args?: unknown; readonly title: string } | { readonly tab: string; readonly title: string };

/**
 * Commands other than tools that are the main thing of their panel, drawn
 * large wherever they appear (tools say so in the catalog, `primary`).
 */
export const PRIMARY_COMMANDS: ReadonlySet<string> = new Set([
  'file.new',
  'file.open',
  'file.save',
  'cloud.open',
  'cloud.upload',
  'edit.paste',
  'view.zoomExtents',
  'crs.set',
  'processing.toolbox',
  'style.manager',
  'style.svgEditor',
  'tools.options',
]);

/** Large buttons a panel may have; more would crowd out the small ones. */
export const MAX_LARGE = 4;

export const RIBBON_TABS: readonly RibbonTabSpec[] = [
  { id: 'file', label: 'Dosya', sources: [{ menu: 'file' }] },
  {
    id: 'home',
    label: 'Giriş',
    sources: [
      { menu: 'edit', sections: ['Pano'] },
      { tools: 'select' },
      { menu: 'edit', sections: ['Seçim'] },
      { pick: 'Çizim', icon: 'line', commands: ['tool.line', 'tool.polyline', 'tool.circle', 'tool.arc', 'tool.polygon', 'tool.rectangle'], more: 'draw' },
      {
        pick: 'Değiştir',
        icon: 'move',
        commands: ['tool.move', 'tool.copy', 'tool.rotate', 'tool.mirror', 'tool.scale', 'tool.offset', 'tool.trim', 'tool.extend', 'tool.fillet'],
        more: 'modify',
        compact: true,
      },
      { pick: 'Açıklama', icon: 'text', commands: ['tool.text', 'tool.dimension', 'tool.hatch'], more: 'draw' },
      { pick: 'Harita', icon: 'parcel', commands: ['tool.parcel', 'tool.boundary', 'tool.areaUnion', 'tool.measure', 'tool.area'], more: 'map', workspaces: ['gis'] },
      { builtin: 'layers' },
      { builtin: 'properties' },
    ],
    lead: ['edit.paste'],
    keep: ['Çizim', 'Değiştir'],
    launchers: {
      Katmanlar: { command: 'style.layerStyle', title: 'Katman stili…' },
      Özellikler: { command: 'file.settings', title: 'Proje ayarları: birimler, hassasiyet ve çizim ölçeği' },
    },
  },
  { id: 'draw', label: 'Çizim', sources: [{ menu: 'draw' }] },
  { id: 'modify', label: 'Değiştir', sources: [{ menu: 'modify' }] },
  {
    id: 'map',
    label: 'Harita',
    // CAD shows no map or coordinate menus: what is left is measuring.
    labels: { cad: 'Ölçme' },
    sources: [{ menu: 'map' }, { menu: 'crs' }, { menu: 'analysis' }],
    launchers: { 'Koordinat sistemi': { command: 'crs.set', title: 'Proje ayarları: koordinat sistemi' } },
  },
  {
    id: 'view',
    label: 'Görünüm',
    sources: [{ menu: 'view' }],
    launchers: { Görünüş: { command: 'tools.options', args: 'appearance', title: 'Uygulama ayarları: görünüm' } },
  },
  { id: 'processing', label: 'İşlemler', sources: [{ menu: 'processing' }] },
  {
    id: 'tools',
    label: 'Araçlar',
    sources: [{ menu: 'tools' }, { menu: 'help' }],
    launchers: { 'Çizim yardımcıları': { command: 'tools.options', args: 'snap', title: 'Uygulama ayarları: kenetleme' } },
  },
  {
    id: 'selection',
    label: 'Seçim',
    contextual: 'selection',
    sources: [
      { builtin: 'selection' },
      { menu: 'modify', sections: ['Dönüştür', 'Dizi', 'Nesne', 'Birleştir ve böl', 'Oluştur ve çevir'] },
      { menu: 'edit', sections: ['Pano'] },
      { pick: 'Sembol', icon: 'symbolAssign', commands: ['style.assign', 'style.clearSymbol'] },
    ],
  },
];

/** Always on the quick access bar; the user may add more (and remove what they added). */
export const QUICK_ACCESS: readonly string[] = ['file.save', 'edit.undo', 'edit.redo'];

/** One choice of a split button: a tool, or a tool started with one of its methods. */
export interface SplitEntry {
  readonly command: string;
  /** The prompt option sent right after the tool starts (ToolMethod.option). */
  readonly option?: string;
  /** The button's text: the tool's name (a method does not rename it). */
  readonly title: string;
  /** The list's text: the method, or the tool's name. */
  readonly label: string;
  readonly description?: string;
}

export type RibbonItem =
  | { readonly kind: 'command'; readonly id: string; readonly size: RibbonSize }
  /** A family of tools, or a tool's methods: the last chosen on top, the others under its arrow. */
  | { readonly kind: 'split'; readonly key: string; readonly entries: readonly SplitEntry[]; readonly size: RibbonSize }
  | { readonly kind: 'menu'; readonly menu: SubmenuSpec; readonly size: RibbonSize }
  | { readonly kind: 'builtin'; readonly name: BuiltinPanel };

export type RibbonSize = 'large' | 'small';

export interface RibbonPanel {
  readonly label: string;
  readonly icon: string;
  readonly items: readonly RibbonItem[];
  readonly launcher?: RibbonLauncher;
  /** Seldom used commands of the panel, listed under the ▾ beside its title. */
  readonly overflow?: readonly string[];
  /** Shrinks after the other panels of its tab. */
  readonly keep?: boolean;
}

export interface RibbonTab {
  readonly id: string;
  readonly label: string;
  readonly contextual?: 'selection';
  readonly panels: readonly RibbonPanel[];
}

/** What the tabs are derived from: the registered tools, the processing tree and the model library. */
export interface RibbonInputs {
  readonly tools: readonly ToolDescriptor[];
  readonly processing: readonly CategoryNode[];
  readonly models: readonly { readonly id: string }[];
  /** Icon of a command (panel icons follow their first item). */
  readonly iconOf: (id: string) => string | undefined;
  /** The project's work mode (everything when absent). */
  readonly filter?: WorkspaceFilter;
}

type Entry = { kind: 'command'; id: string } | { kind: 'menu'; menu: SubmenuSpec } | { kind: 'builtin'; name: BuiltinPanel };
type Draft = { label: string; icon?: string; entries: Entry[]; more?: string; compact?: boolean };

const BUILTIN_LABEL: Record<BuiltinPanel, { label: string; icon: string }> = {
  layers: { label: 'Katmanlar', icon: 'layers' },
  properties: { label: 'Özellikler', icon: 'styles' },
  selection: { label: 'Seçim', icon: 'select' },
};

/**
 * Tabs with their panels, sized for their widest layout. Within a tab
 * panels with the same title merge and a command appears once (its first
 * place wins); the ribbon then shrinks panels to fit the window.
 *
 * Sizes follow meaning, not position (DESIGN.md §7.3.1): a panel's main
 * tools and commands (`primary` in the catalog, PRIMARY_COMMANDS) are
 * large, at most MAX_LARGE; the rest are small, three to a column; a panel
 * with no main item and one or two items draws them large; a `compact`
 * pick draws everything small. A tool family (`family`) is one split
 * button, and so is a tool with `methods`; `rare` tools go under the
 * panel's ▾. The work mode (`inputs.filter`) leaves out what it hides.
 */
export function ribbonTabs(inputs: RibbonInputs, specs: readonly RibbonTabSpec[] = RIBBON_TABS): RibbonTab[] {
  const filter = inputs.filter ?? SHOW_ALL;
  const tools = inputs.tools.filter((t) => filter.tool(t));
  const toolOf = new Map(tools.map((t) => [`tool.${t.id}`, t]));
  const family = (f: string) => tools.filter((t) => t.family === f);
  const tabs = specs.map((spec) => {
    const drafts: Draft[] = [];
    const seen = new Set<string>();
    const panel = (label: string, icon?: string, more?: string, compact?: boolean): Draft => {
      const found = drafts.find((d) => d.label === label);
      if (found) return found;
      const d: Draft = { label, icon, entries: [], more, compact };
      drafts.push(d);
      return d;
    };
    const command = (d: Draft, id: string) => {
      if (seen.has(id) || !filter.command(id)) return;
      const t = toolOf.get(id);
      // A family travels together: its other members join the first one's panel.
      const ids = t?.family ? family(t.family).map((m) => `tool.${m.id}`) : [id];
      for (const x of ids) {
        if (seen.has(x)) continue;
        seen.add(x);
        d.entries.push({ kind: 'command', id: x });
      }
    };
    for (const src of spec.sources) {
      if ('builtin' in src) {
        const b = BUILTIN_LABEL[src.builtin];
        panel(b.label, b.icon).entries.push({ kind: 'builtin', name: src.builtin });
      } else if ('pick' in src) {
        if (src.workspaces && !src.workspaces.includes(filter.id)) continue;
        const d = panel(src.pick, src.icon, src.more, src.compact);
        for (const id of src.commands) command(d, id);
      } else if ('tools' in src) {
        for (const s of toolSections(tools, src.tools)) {
          const d = panel(s.label);
          for (const t of s.tools) command(d, `tool.${t.id}`);
        }
      } else {
        const menu = menuById(src.menu);
        if (!menu || !filter.menu(menu.id)) continue;
        const keep = (label: string) => !src.sections || src.sections.includes(label);
        for (const block of expandBlocks(menuBlocks(menu.items, tools, filter), menu.label, inputs, tools, filter)) {
          if (!keep(block.label)) continue;
          const d = panel(block.label);
          for (const e of block.entries) {
            if (e.kind === 'command') command(d, e.id);
            else d.entries.push(e);
          }
        }
      }
    }
    const lead = new Set(spec.lead ?? []);
    const leads = (e: Entry) => e.kind === 'command' && lead.has(e.id);
    const panels = drafts
      .filter((d) => d.entries.length)
      .map((d): RibbonPanel => {
        const ordered = [...d.entries.filter(leads), ...d.entries.filter((e) => !leads(e))];
        const rare = (e: Entry) => e.kind === 'command' && !!toolOf.get(e.id)?.rare;
        // Rare tools go under the ▾, unless nothing would be left on the panel.
        const overflow = ordered.every(rare) ? [] : ordered.filter(rare).map((e) => (e.kind === 'command' ? e.id : ''));
        const shown = groupSplits(ordered.filter((e) => !overflow.includes(e.kind === 'command' ? e.id : '')), toolOf);
        const primary = (x: Pre) =>
          x.kind === 'command' ? !!toolOf.get(x.id)?.primary || PRIMARY_COMMANDS.has(x.id) : x.kind === 'split' ? x.members.some((m) => m.primary) : x.kind === 'menu' ? !!x.menu.primary : false;
        const anyPrimary = shown.some(primary);
        const items = shown.map((x): RibbonItem => {
          if (x.kind === 'builtin') return x;
          const size: RibbonSize = !d.compact && (primary(x) || (!anyPrimary && shown.length <= 2)) ? 'large' : 'small';
          if (x.kind === 'command') return { kind: 'command', id: x.id, size };
          if (x.kind === 'menu') return { kind: 'menu', menu: x.menu, size };
          return { kind: 'split', key: x.key, entries: x.entries, size };
        });
        const first = ordered[0];
        const icon = d.icon ?? (first.kind === 'command' ? inputs.iconOf(first.id) : first.kind === 'menu' ? first.menu.icon : undefined) ?? 'more';
        const launcher = spec.launchers?.[d.label] ?? (d.more ? { tab: d.more, title: `Tüm araçlar: ${specs.find((s) => s.id === d.more)?.label ?? d.more} sekmesi` } : undefined);
        return { label: d.label, icon, items, launcher, overflow: overflow.length ? overflow : undefined, keep: spec.keep?.includes(d.label) || undefined };
      });
    return { id: spec.id, label: spec.labels?.[filter.id] ?? spec.label, contextual: spec.contextual, panels };
  });
  // A tab the work mode leaves empty (İşlemler in CAD) is not shown.
  return tabs.filter((t) => t.panels.length);
}

/** Every command a panel reaches: its buttons, the choices of its split buttons and its ▾ list. */
export function panelCommands(p: RibbonPanel): string[] {
  const ids = p.items.flatMap((i) => (i.kind === 'command' ? [i.id] : i.kind === 'split' ? i.entries.map((e) => e.command) : []));
  return [...new Set([...ids, ...(p.overflow ?? [])])];
}

/** An entry before sizing: families and tools with methods become splits. */
type Pre =
  | { kind: 'command'; id: string }
  | { kind: 'menu'; menu: SubmenuSpec }
  | { kind: 'builtin'; name: BuiltinPanel }
  | { kind: 'split'; key: string; entries: SplitEntry[]; members: ToolDescriptor[] };

/** A tool's choices: its methods, or the tool itself. */
function splitEntries(t: ToolDescriptor): SplitEntry[] {
  const command = `tool.${t.id}`;
  return t.methods?.length
    ? t.methods.map((m) => ({ command, option: m.option, title: t.label, label: m.label, description: m.description }))
    : [{ command, title: t.label, label: t.label }];
}

/** Family members (at the first one's place) and tools with methods as split buttons. */
function groupSplits(entries: readonly Entry[], toolOf: ReadonlyMap<string, ToolDescriptor>): Pre[] {
  const out: Pre[] = [];
  const families = new Map<string, Extract<Pre, { kind: 'split' }>>();
  for (const e of entries) {
    const t = e.kind === 'command' ? toolOf.get(e.id) : undefined;
    if (!t || (!t.family && !t.methods?.length)) {
      out.push(e);
      continue;
    }
    const key = t.family ?? t.id;
    const found = families.get(key);
    if (found) {
      found.members.push(t);
      found.entries.push(...splitEntries(t));
      continue;
    }
    const split: Extract<Pre, { kind: 'split' }> = { kind: 'split', key, entries: splitEntries(t), members: [t] };
    families.set(key, split);
    out.push(split);
  }
  // A family with one member left in this mode and no methods is a plain button.
  return out.map((x) => (x.kind === 'split' && x.entries.length === 1 ? { kind: 'command', id: x.entries[0].command } : x));
}

/**
 * Menu blocks as ribbon panels: an untitled block takes the menu's name,
 * an inline submenu contributes its own blocks, `@models` lists the model
 * library and `@processing` becomes one panel per category.
 */
function expandBlocks(blocks: readonly MenuBlock[], fallback: string, inputs: RibbonInputs, tools: readonly ToolDescriptor[], filter: WorkspaceFilter): { label: string; entries: Entry[] }[] {
  const out: { label: string; entries: Entry[] }[] = [];
  for (const block of blocks) {
    let cur: { label: string; entries: Entry[] } = { label: block.label || fallback, entries: [] };
    out.push(cur);
    for (const e of block.items) {
      if (typeof e === 'object') {
        if (!e.inline) {
          cur.entries.push({ kind: 'menu', menu: e });
          continue;
        }
        out.push(...expandBlocks(menuBlocks(e.items, tools, filter), e.label, inputs, tools, filter));
      } else if (e === '@models') {
        cur.entries.push({ kind: 'command', id: 'processing.newModel' }, ...inputs.models.map((m) => ({ kind: 'command' as const, id: modelCommandId(m.id) })));
        continue;
      } else if (e === '@processing') {
        for (const node of inputs.processing) out.push({ label: node.category.label, entries: categoryTools(node).map((id) => ({ kind: 'command' as const, id })) });
      } else {
        cur.entries.push({ kind: 'command', id: e });
        continue;
      }
      // After panels of their own, the block's remaining items continue in a new one under its name.
      cur = { label: block.label || fallback, entries: [] };
      out.push(cur);
    }
  }
  return out;
}

/** A category's tools and those of its sub-categories, as commands. */
function categoryTools(node: CategoryNode): string[] {
  return [...node.tools.map((t) => processingCommandId(t.id)), ...node.children.flatMap(categoryTools)];
}
