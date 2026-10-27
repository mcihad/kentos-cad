-- KentOS CAD: slabs (docs/adr/0722 §1): a slab's `geom` its area; its source `cad` with its own fields (role,
-- thickness, top, material, edge and its runs, excluded). The kind check takes `slab`.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening', 'space', 'grid', 'column', 'stair', 'ramp',
                  'tag', 'viewmark', 'slab'));
