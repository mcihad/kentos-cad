import type { CategoryNode } from '../processing/registry';
import type { ToolDescriptor, ToolGroup } from '../tools/Tool';
import { toolSections } from '../tools/sections';
import { menuBlocks, menuById, type MenuBlock, type SubmenuSpec } from './menus';
import { modelCommandId, processingCommandId } from './processing';

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
  /** A hand-picked panel; `more` names the tab that holds the whole family. */
  | { readonly pick: string; readonly icon: string; readonly commands: readonly string[]; readonly more?: string }
  /** Panels with live fields rather than commands. */
  | { readonly builtin: BuiltinPanel };

export type BuiltinPanel = 'layers' | 'properties' | 'selection';

export interface RibbonTabSpec {
  readonly id: string;
  readonly label: string;
  readonly sources: readonly RibbonSource[];
  /** Presentation only: commands drawn large besides each panel's first … */
  readonly large?: readonly string[];
  /** … commands drawn small even when first … */
  readonly small?: readonly string[];
  /** … and commands that lead their panel (moved to its front, drawn large). */
  readonly lead?: readonly string[];
  /** Panels that keep their labels longest when the window narrows (by title). */
  readonly keep?: readonly string[];
  /** A contextual tab: shown only while its subject exists (a selection under the select tool). */
  readonly contextual?: 'selection';
  /** Dialog launchers (the corner button of a panel title), by panel label. */
  readonly launchers?: Readonly<Record<string, RibbonLauncher>>;
}

export type RibbonLauncher = { readonly command: string; readonly args?: unknown; readonly title: string } | { readonly tab: string; readonly title: string };

export const RIBBON_TABS: readonly RibbonTabSpec[] = [
  { id: 'file', label: 'Dosya', sources: [{ menu: 'file' }] },
  {
    id: 'home',
    label: 'Giriş',
    sources: [
      { menu: 'edit', sections: ['Pano'] },
      { tools: 'select' },
      { menu: 'edit', sections: ['Seçim'] },
      { pick: 'Çizim', icon: 'line', commands: ['tool.line', 'tool.polyline', 'tool.polygon', 'tool.circle', 'tool.arc', 'tool.rectangle'], more: 'draw' },
      {
        pick: 'Değiştir',
        icon: 'move',
        commands: ['tool.move', 'tool.copy', 'tool.rotate', 'tool.mirror', 'tool.scale', 'tool.offset', 'tool.trim', 'tool.extend', 'tool.fillet'],
        more: 'modify',
      },
      { pick: 'Açıklama', icon: 'text', commands: ['tool.text', 'tool.dimension', 'tool.hatch'], more: 'draw' },
      { builtin: 'layers' },
      { builtin: 'properties' },
    ],
    large: ['tool.polyline', 'tool.dimension'],
    small: ['tool.select'],
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

export type RibbonItem =
  | { readonly kind: 'command'; readonly id: string; readonly size: RibbonSize }
  | { readonly kind: 'menu'; readonly menu: SubmenuSpec; readonly size: RibbonSize }
  | { readonly kind: 'builtin'; readonly name: BuiltinPanel };

export type RibbonSize = 'large' | 'small';

export interface RibbonPanel {
  readonly label: string;
  readonly icon: string;
  readonly items: readonly RibbonItem[];
  readonly launcher?: RibbonLauncher;
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
}

type Draft = { label: string; icon?: string; entries: ({ kind: 'command'; id: string } | { kind: 'menu'; menu: SubmenuSpec } | { kind: 'builtin'; name: BuiltinPanel })[]; more?: string };

const BUILTIN_LABEL: Record<BuiltinPanel, { label: string; icon: string }> = {
  layers: { label: 'Katmanlar', icon: 'layers' },
  properties: { label: 'Özellikler', icon: 'styles' },
  selection: { label: 'Seçim', icon: 'select' },
};

/**
 * Tabs with their panels, sized for their widest layout. Within a tab
 * panels with the same title merge and a command appears once (its first
 * place wins); the ribbon then shrinks panels to fit the window.
 */
export function ribbonTabs(inputs: RibbonInputs, specs: readonly RibbonTabSpec[] = RIBBON_TABS): RibbonTab[] {
  return specs.map((spec) => {
    const drafts: Draft[] = [];
    const seen = new Set<string>();
    const panel = (label: string, icon?: string, more?: string): Draft => {
      const found = drafts.find((d) => d.label === label);
      if (found) return found;
      const d: Draft = { label, icon, entries: [], more };
      drafts.push(d);
      return d;
    };
    const command = (d: Draft, id: string) => {
      if (seen.has(id)) return;
      seen.add(id);
      d.entries.push({ kind: 'command', id });
    };
    for (const src of spec.sources) {
      if ('builtin' in src) {
        const b = BUILTIN_LABEL[src.builtin];
        panel(b.label, b.icon).entries.push({ kind: 'builtin', name: src.builtin });
      } else if ('pick' in src) {
        const d = panel(src.pick, src.icon, src.more);
        for (const id of src.commands) command(d, id);
      } else if ('tools' in src) {
        for (const s of toolSections(inputs.tools, src.tools)) {
          const d = panel(s.label);
          for (const t of s.tools) command(d, `tool.${t.id}`);
        }
      } else {
        const menu = menuById(src.menu);
        if (!menu) continue;
        const keep = (label: string) => !src.sections || src.sections.includes(label);
        for (const block of expandBlocks(menuBlocks(menu.items, inputs.tools), menu.label, inputs)) {
          if (!keep(block.label)) continue;
          const d = panel(block.label);
          for (const e of block.entries) {
            if (e.kind === 'command') command(d, e.id);
            else d.entries.push(e);
          }
        }
      }
    }
    const large = new Set(spec.large ?? []);
    const small = new Set(spec.small ?? []);
    const lead = new Set(spec.lead ?? []);
    const leads = (e: Entry) => e.kind === 'command' && lead.has(e.id);
    const panels = drafts
      .filter((d) => d.entries.length)
      .map((d): RibbonPanel => {
        const n = d.entries.length;
        const entries = [...d.entries.filter(leads), ...d.entries.filter((e) => !leads(e))];
        const items = entries.map((e, i): RibbonItem => {
          if (e.kind === 'builtin') return e;
          const id = e.kind === 'command' ? e.id : '';
          const size: RibbonSize = !small.has(id) && (i === 0 || n <= 2 || large.has(id)) ? 'large' : 'small';
          return e.kind === 'command' ? { kind: 'command', id: e.id, size } : { kind: 'menu', menu: e.menu, size };
        });
        const first = entries[0];
        const icon = d.icon ?? (first.kind === 'command' ? inputs.iconOf(first.id) : first.kind === 'menu' ? first.menu.icon : undefined) ?? 'more';
        const launcher = spec.launchers?.[d.label] ?? (d.more ? { tab: d.more, title: `Tüm araçlar: ${specs.find((s) => s.id === d.more)?.label ?? d.more} sekmesi` } : undefined);
        return { label: d.label, icon, items, launcher, keep: spec.keep?.includes(d.label) || undefined };
      });
    return { id: spec.id, label: spec.label, contextual: spec.contextual, panels };
  });
}

type Entry = Draft['entries'][number];

/**
 * Menu blocks as ribbon panels: an untitled block takes the menu's name,
 * an inline submenu contributes its own blocks, `@models` lists the model
 * library and `@processing` becomes one panel per category.
 */
function expandBlocks(blocks: readonly MenuBlock[], fallback: string, inputs: RibbonInputs): { label: string; entries: Entry[] }[] {
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
        out.push(...expandBlocks(menuBlocks(e.items, inputs.tools), e.label, inputs));
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
