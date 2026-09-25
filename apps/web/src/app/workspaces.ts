import { WORKSPACE_IDS, type Workspace } from '../model/projectSettings';
import type { ToolDescriptor } from '../tools/Tool';
import type { AppContext } from './context';

/**
 * Work modes (Çalışma modu): one project, one data model, several ways of
 * presenting it. A mode decides which main menus, ribbon tabs and panels,
 * and toolbox tools are shown; it never changes what the data means, and
 * every command still runs from the command line and its shortcut in every
 * mode. The mode is a project setting (`ProjectSettings.workspace`), asked
 * when a project is created and changed from the status bar or
 * Görünüm → Çalışma modu.
 *
 * A new mode is one entry here (and its id in the contract enum,
 * crates/shared/contracts `Workspace`). Modes announced but not built yet
 * carry `status: 'soon'`: they are shown with "Yakında" and cannot be
 * chosen; a file naming one opens in the hybrid presentation.
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
    id: 'hybrid',
    label: 'Hibrit',
    title: 'CAD + CBS',
    icon: 'modeHybrid',
    description: 'Hassas çizim ile coğrafi veri bir arada: bütün menüler, sekmeler ve araçlar.',
    highlights: ['Bütün çizim ve düzenleme araçları', 'Harita, parsel ve koordinat işleri', 'İşlem araçları ve modeller'],
    status: 'ready',
  },
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
        'tool.revcloud',
        'tool.hatch',
        'transform/array',
        'modify/corner',
        'tool.stretch',
        'tool.lengthen',
      ],
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
  throw new Error('app/workspaces.ts: çalışma modları sözleşmeyle aynı değil');
}

export const workspaceById = (id: Workspace): WorkspaceSpec => WORKSPACES.find((w) => w.id === id)!;

/** The mode the interface shows for a project's setting: an announced mode shows as hybrid. */
export function effectiveWorkspace(id: Workspace): WorkspaceSpec {
  const w = workspaceById(id);
  return w.status === 'ready' ? w : workspaceById('hybrid');
}

/** What a mode shows: the menus, ribbon and toolbox ask it item by item. */
export interface WorkspaceFilter {
  readonly id: Workspace;
  menu(id: string): boolean;
  tool(t: ToolDescriptor): boolean;
  /** A command id; `tool.x` asks the tool's own entry (its group and section). */
  command(id: string): boolean;
}

/** Everything shown: hybrid, and callers that do not filter. */
export const SHOW_ALL: WorkspaceFilter = { id: 'hybrid', menu: () => true, tool: () => true, command: () => true };

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

/** The filter of the open project's mode (menus, ribbon and toolbox rebuild when the setting changes). */
export function filterOf(ctx: AppContext): WorkspaceFilter {
  return workspaceFilter(effectiveWorkspace(ctx.doc.settings.workspace.value), ctx.tools.list());
}
