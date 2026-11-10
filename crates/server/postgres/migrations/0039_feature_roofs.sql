-- KentOS CAD: roofs (docs/adr/8044 §1): a roof's `geom` its eaves' area; its source `cad` with its own fields (eaves
-- line, each edge's slope or gable end, eaves' offset, thickness, overhang, covering). The kind check takes `roof`.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening', 'space', 'grid', 'column', 'stair', 'ramp',
                  'tag', 'viewmark', 'slab', 'beam', 'roof'));
