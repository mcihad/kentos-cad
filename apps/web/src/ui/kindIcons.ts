import type { EntityKind } from '../model/entities';

/**
 * An object kind's icon where a list names kinds (Seçim süzgeci's menu, Sıradakini seç's list; docs/adr/0187): its
 * drawing tool's. The desktop's is `kind_icon` in `apps/desktop/src/selection_commands.rs`.
 */
export const ENTITY_KIND_ICON: Record<EntityKind, string> = {
  point: 'point',
  line: 'line',
  polyline: 'polyline',
  polygon: 'polygon',
  circle: 'circle',
  arc: 'arc',
  ellipse: 'ellipse',
  spline: 'spline',
  xline: 'xline',
  ray: 'ray',
  text: 'text',
  dimension: 'dimension',
  hatch: 'hatch',
  insert: 'blockInsert',
  leader: 'leader',
  table: 'table',
  image: 'imageInsert',
  raster: 'rasterAdd',
  pointcloud: 'pointCloudAdd',
};
