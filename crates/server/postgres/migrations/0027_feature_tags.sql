-- KentOS CAD: tags (docs/adr/0344 §1): a tag's `geom` its middle; its source `cad` with its own fields (its object,
-- template, frame, words). The kind check takes `tag`. A schedule is a table: its source is the table's own.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening', 'space', 'grid', 'column', 'stair', 'ramp',
                  'tag'));
