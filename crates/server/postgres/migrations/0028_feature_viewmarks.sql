-- KentOS CAD: view marks (docs/adr/0345 §1): a mark's `geom` its cut line or its point; its source `cad` with its own
-- fields (form, points, target, words). The kind check takes `viewmark`. A named view's id lives in the project's
-- settings and needs no column.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening', 'space', 'grid', 'column', 'stair', 'ramp',
                  'tag', 'viewmark'));
