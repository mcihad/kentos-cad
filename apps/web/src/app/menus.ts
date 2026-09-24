import type { MenuItem } from '../ui/widgets/PopupMenu';
import type { CategoryNode } from '../processing/registry';
import type { AppContext } from './context';
import { modelCommandId, processingCommandId } from './processing';

/**
 * Declarative main menu. Strings are command ids; "-" is a separator;
 * "@processing" expands to the processing categories with their tools,
 * "@models" to the model library (run a model, or design a new one).
 */
export type MenuSpec = string | '-' | '@processing' | '@models' | { label: string; icon?: string; items: MenuSpec[] };

export interface TopMenu {
  id: string;
  label: string;
  items: MenuSpec[];
}

export const MAIN_MENU: TopMenu[] = [
  {
    id: 'file',
    label: 'Dosya',
    items: [
      'file.new',
      'file.open',
      '-',
      'file.save',
      'file.saveAs',
      '-',
      'cloud.open',
      'cloud.upload',
      'cloud.conflicts',
      'cloud.rename',
      'cloud.delete',
      '-',
      { label: 'İçe aktar', icon: 'import', items: ['file.import.dxf', 'file.import.ncz', 'file.import.shp', 'file.import.geojson', '-', 'file.import.ncn'] },
      { label: 'Dışa aktar', icon: 'export', items: ['file.export.dxf', 'file.export.geojson', 'file.export.pdf', '-', 'file.export.ncn'] },
      '-',
      'file.print',
      '-',
      'file.settings',
    ],
  },
  {
    id: 'edit',
    label: 'Düzen',
    items: ['edit.undo', 'edit.redo', '-', 'edit.cut', 'edit.copy', 'edit.paste', 'edit.pasteOriginal', 'tool.erase', '-', 'edit.selectAll', 'edit.deselect', 'edit.invertSelection'],
  },
  {
    id: 'view',
    label: 'Görünüm',
    items: [
      'view.zoomExtents',
      'tool.zoomWindow',
      'view.zoomSelection',
      'view.zoomIn',
      'view.zoomOut',
      'tool.pan',
      '-',
      'view.toolbox',
      'view.toolboxDock',
      'view.rightPanel',
      'view.bottomPanel',
      'view.coords',
      '-',
      { label: 'Tema', items: ['view.theme.dark', 'view.theme.light'] },
      { label: 'Çizim motoru', icon: 'chip', items: ['view.renderer.webgl2', 'view.renderer.webgpu'] },
      { label: 'Sembol boyutu', icon: 'styles', items: ['view.symbols.plot', 'view.symbols.screen'] },
    ],
  },
  {
    id: 'draw',
    label: 'Çizim',
    items: ['tool.point', 'tool.divide', '-', 'tool.line', 'tool.polyline', 'tool.parallel', 'tool.arc', 'tool.circle', 'tool.ellipse', 'tool.rectangle', 'tool.rectangle3', 'tool.regularPolygon', 'tool.polygon', 'tool.spline', '-', 'tool.perpIn', 'tool.perpOut', 'tool.xline', 'tool.ray', 'tool.donut', '-', 'tool.text', 'tool.dimension', 'tool.hatch', 'tool.revcloud'],
  },
  {
    id: 'modify',
    label: 'Değiştir',
    items: [
      'tool.move',
      'tool.copy',
      'tool.rotate',
      'tool.scale',
      'tool.mirror',
      'tool.stretch',
      'tool.array',
      'tool.arrayPolar',
      'tool.align',
      '-',
      'tool.offset',
      'tool.trim',
      'tool.extend',
      'tool.lengthen',
      'tool.break',
      'tool.fillet',
      'tool.chamfer',
      '-',
      'tool.join',
      'tool.explode',
      'tool.vertex',
      '-',
      { label: 'Alan işlemleri', icon: 'areaUnion', items: ['tool.boundary', 'tool.toArea', '-', 'tool.areaUnion', 'tool.areaIntersect', 'tool.areaSubtract', 'tool.areaSplit', '-', 'tool.toPolyline'] },
      '-',
      'tool.erase',
    ],
  },
  {
    id: 'map',
    label: 'Harita',
    items: ['tool.parcel', 'tool.subdivide', 'tool.stakeout', 'map.parcelReport', 'map.edgeLengths', '-', 'tool.spot', 'map.contours', 'map.profile', '-', 'map.sheet'],
  },
  {
    id: 'crs',
    label: 'Koordinat',
    items: ['crs.set', 'crs.transform', '-', 'crs.query', 'crs.points', 'view.coords'],
  },
  {
    id: 'analysis',
    label: 'Analiz',
    items: ['tool.measure', 'tool.area', '-', 'analysis.volume', 'analysis.slope'],
  },
  {
    id: 'processing',
    label: 'İşlemler',
    items: ['processing.toolbox', 'processing.history', '-', '@models', '@processing'],
  },
  {
    id: 'tools',
    label: 'Araçlar',
    items: ['commandline.focus', { label: 'Çizim yardımcıları', items: ['draft.snap', 'draft.grid', 'draft.ortho', 'draft.polar', 'draft.tracking'] }, '-', 'style.manager', 'style.svgEditor', 'style.layerStyle', 'style.legend', 'style.assign', 'style.clearSymbol', '-', 'help.shortcuts', 'server.check', 'tools.options'],
  },
  {
    id: 'help',
    label: 'Yardım',
    items: ['help.shortcuts', '-', 'help.about'],
  },
];

/** Resolves specs against the live command registry (enabled/checked/shortcut). */
export function resolveMenu(ctx: AppContext, specs: MenuSpec[]): MenuItem[] {
  return specs.flatMap((s): MenuItem | MenuItem[] => {
    if (s === '-') return { kind: 'separator' };
    if (s === '@processing') return processingMenu(ctx, ctx.processing.registry.tree());
    if (s === '@models')
      return {
        label: 'Modeller',
        icon: 'processing',
        items: () => [...ctx.processing.models.value.map((m) => commandItem(ctx, modelCommandId(m.id))), { kind: 'separator' }, commandItem(ctx, 'processing.newModel')],
      };
    if (typeof s === 'object') return { label: s.label, icon: s.icon, items: () => resolveMenu(ctx, s.items) };
    return commandItem(ctx, s);
  });
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
  const isRadio = (id.startsWith('view.theme.') && id !== 'view.theme.toggle') || id.startsWith('view.renderer.') || id.startsWith('view.symbols.');
  // Tools report "active" via isChecked, but in menus they read as actions.
  const isTool = id.startsWith('tool.');
  return {
    label: cmd.title,
    icon: checked === undefined || isRadio || isTool ? cmd.icon : undefined,
    shortcut: ctx.keymap.chordFor(id),
    checked: isTool ? undefined : checked,
    radio: isRadio,
    disabled: !ctx.commands.isEnabled(id),
    run: () => ctx.commands.execute(id),
    ...overrides,
  };
}
