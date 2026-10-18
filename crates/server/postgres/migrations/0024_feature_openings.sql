-- KentOS CAD: the opening (docs/adr/0341 §1): a door, a window or a bare opening in a wall; its `geom` its jambs'
-- footprint, its source `cad` with its wall's id, its place along it, its width and the rest of its fields. The kind
-- check takes `opening`.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening'));
