-- KentOS CAD: the wall (docs/adr/0339 §4): its `geom` the body its faces close, its source `cad` with the wall's
-- path, width, justification, components and architectural information. The kind check takes `wall`.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall'));
