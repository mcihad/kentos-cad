import { WORKSPACE_IDS, type Workspace } from '../model/projectSettings';
import type { ToolDescriptor } from '../tools/Tool';
import type { AppContext } from './context';

/**
 * Project types (docs/adr/0165): CAD and CBS, each with its own scene, axes
 * and ribbon; the tools both use are in both. The type is a project setting
 * (`ProjectSettings.workspace`), chosen when a project is created; it never
 * changes what the data means, and every command still runs from the
 * command line and its shortcut in both. There is no hybrid type: a project
 * whose type is not asked yet (an older file) shows as CBS until it is.
 *
 * A new type is one entry here (and its id in the contract enum,
 * crates/shared/contracts `Workspace`). Types announced but not built yet
 * carry `status: 'soon'`: they are shown with "Yakında" and cannot be
 * chosen; a file naming one shows as CBS.
 */

export type WorkspaceStatus = 'ready' | 'soon';

/**
 * What a mode leaves out. Entries in `tools` name a tool group (`draw`), a
 * group's section (`draw/shape`) or one tool (`tool.hatch`).
 */
export interface WorkspaceHide {
  readonly menus?: readonly string[];
  readonly tools?: readonly string[];
  readonly commands?: readonly string[];
}

export interface WorkspaceSpec {
  readonly id: Workspace;
  /** Short name: status bar, menus. */
  readonly label: string;
  /** What it is, one line (cards, tooltips). */
  readonly title: string;
  readonly icon: string;
  readonly description: string;
  /** Three short points for the mode cards. */
  readonly highlights: readonly string[];
  readonly status: WorkspaceStatus;
  readonly hide?: WorkspaceHide;
}

export const WORKSPACES: readonly WorkspaceSpec[] = [
  {
    id: 'cad',
    label: 'CAD',
    title: 'Teknik çizim',
    icon: 'modeCad',
    description: 'Çizim, düzenleme ve ölçülendirmeye odaklı sade arayüz; harita ve işlem menüleri gizlenir.',
    highlights: ['Çizim, değiştir ve alan işlemleri', 'Ölçü, yazı ve tarama', 'Mesafe ve alan ölçme'],
    status: 'ready',
    hide: {
      menus: ['map', 'crs', 'processing'],
      tools: ['map/parcel', 'map/field'],
      commands: ['analysis.volume', 'analysis.slope'],
    },
  },
  {
    id: 'gis',
    label: 'CBS',
    title: 'Coğrafi bilgi sistemi',
    icon: 'modeGis',
    description: 'Katman, öznitelik ve harita işlerine odaklı arayüz; ileri çizim araçları gizlenir.',
    highlights: ['Parsel, kot ve ölçme araçları', 'Koordinat sistemi ve dosya alışverişi', 'İşlem araçları, stiller ve modeller'],
    status: 'ready',
    hide: {
      tools: [
        'draw/shape',
        'draw/construction',
        'tool.ellipse',
        'tool.spline',
        'tool.donut',
        'tool.parallel',
        'tool.dimension',
        // The rest of the dimension family and CAD's annotation (docs/adr/0146, 0147, 0182): CAD's interface only.
        'tool.dimContinue',
        'tool.dimBaseline',
        'tool.quickDimension',
        'tool.mtext',
        'tool.leader',
        'tool.placeTextFile',
        'tool.revcloud',
        'tool.hatch',
        'transform/array',
        'modify/corner',
        'tool.stretch',
        'tool.lengthen',
      ],
      // Yazı ve ölçü stilleri (docs/adr/0183 §4) and Tablo (docs/adr/0184): CAD's interface only.
      commands: ['style.textStyles', 'style.dimensionStyles', 'table.insert', 'table.edit', 'table.update'],
    },
  },
  {
    id: 'plan3d',
    label: '3D Plan',
    title: 'İmar planından 3D kent tasarımı',
    icon: 'modePlan3d',
    description: 'Parsel ve imar kurallarından yapı kütleleri, senaryolar ve 3D görünüm.',
    highlights: ['İmar kısıtlarından yapı kütlesi', 'Senaryo karşılaştırma', '3D sahne ve arazi'],
    status: 'soon',
  },
  {
    id: 'disaster',
    label: 'Afet Analizi',
    title: 'Afet ve risk analizi',
    icon: 'modeDisaster',
    description: 'Deprem, sel ve heyelan riskinin parsel ve yapı düzeyinde analizi.',
    highlights: ['Risk ve tehlike katmanları', 'Toplanma alanı ve erişim', 'Etkilenen yapı ve parsel raporu'],
    status: 'soon',
  },
];

// Every id the contract knows has an entry, and no entry is unknown to it.
if (WORKSPACES.length !== WORKSPACE_IDS.length || !WORKSPACE_IDS.every((id) => WORKSPACES.some((w) => w.id === id))) {
  throw new Error('app/workspaces.ts: proje türleri sözleşmeyle aynı değil');
}

export const workspaceById = (id: Workspace): WorkspaceSpec => WORKSPACES.find((w) => w.id === id)!;

/** The type a project whose type is not asked yet, or is an announced one, shows as (docs/adr/0165 §1). */
export const FALLBACK_WORKSPACE: Workspace = 'gis';

/** The type the interface shows for a project's setting: one not asked yet, or announced, shows as CBS. */
export function effectiveWorkspace(id: Workspace | null): WorkspaceSpec {
  const w = id === null ? undefined : workspaceById(id);
  return w && w.status === 'ready' ? w : workspaceById(FALLBACK_WORKSPACE);
}

/** What a type shows: the ribbon and its menus ask it item by item (null: no filter). */
export interface WorkspaceFilter {
  readonly id: Workspace | null;
  menu(id: string): boolean;
  tool(t: ToolDescriptor): boolean;
  /** A command id; `tool.x` asks the tool's own entry (its group and section). */
  command(id: string): boolean;
}

/** Everything shown: callers that do not filter (a type's own lists, the inventory's places). */
export const SHOW_ALL: WorkspaceFilter = { id: null, menu: () => true, tool: () => true, command: () => true };

export function workspaceFilter(spec: WorkspaceSpec, tools: readonly ToolDescriptor[]): WorkspaceFilter {
  const hide = spec.hide;
  if (!hide) return { ...SHOW_ALL, id: spec.id };
  const menus = new Set(hide.menus ?? []);
  const toolRefs = new Set(hide.tools ?? []);
  const commands = new Set(hide.commands ?? []);
  const tool = (t: ToolDescriptor) => !toolRefs.has(t.group) && !toolRefs.has(`${t.group}/${t.section ?? ''}`) && !toolRefs.has(`tool.${t.id}`);
  const byId = new Map(tools.map((t) => [`tool.${t.id}`, t]));
  return {
    id: spec.id,
    menu: (id) => !menus.has(id),
    tool,
    command: (id) => {
      if (commands.has(id)) return false;
      const t = byId.get(id);
      return t ? tool(t) : true;
    },
  };
}

/** The filter of the open project's mode (the ribbon and its menus rebuild when the setting changes). */
export function filterOf(ctx: AppContext): WorkspaceFilter {
  return workspaceFilter(effectiveWorkspace(ctx.doc.settings.workspace.value), ctx.tools.list());
}
