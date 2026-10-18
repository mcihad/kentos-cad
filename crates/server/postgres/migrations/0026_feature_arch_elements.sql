-- KentOS CAD: grids, columns, stairs and ramps (docs/adr/0343 §1): a grid's `geom` its line, a column's, a stair's and a
-- ramp's their outline; their source `cad` with their own fields. The kind check takes `grid`, `column`, `stair` and
-- `ramp`.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening', 'space', 'grid', 'column', 'stair', 'ramp'));
