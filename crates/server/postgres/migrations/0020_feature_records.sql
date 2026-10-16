-- KentOS CAD: a record without a geometry (docs/adr/0410 §5): a table's row, its `geom` null, its source
-- `cad` with the kind alone. The kind check takes `record`, and `nurbs` (docs/adr/0259), which the
-- application has written since schema 47 while the check refused it.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record'));
