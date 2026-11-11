-- KentOS CAD: free-standing railings (docs/adr/8045 §3): a railing's `geom` its line; its source `cad` with its own
-- fields (line, closed, height, top rail's width, infill, balusters' gap, offset). The kind check takes `railing`.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening', 'space', 'grid', 'column', 'stair', 'ramp',
                  'tag', 'viewmark', 'slab', 'beam', 'roof', 'railing'));
