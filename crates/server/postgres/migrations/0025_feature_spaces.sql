-- KentOS CAD: the space (docs/adr/0342 §1): its `geom` its area (parts and holes), its source `cad` with its parts, seed,
-- method, number, name, usage, level, height and finishes. The kind check takes `space`.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening', 'space'));
