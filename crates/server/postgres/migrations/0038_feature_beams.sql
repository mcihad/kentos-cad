-- KentOS CAD: beams (docs/adr/8041 §1): a beam's `geom` its band; its source `cad` with its own fields (axis, width,
-- height, justification, top's offset, number, material). The kind check takes `beam`.

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'nurbs',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image',
                  'raster', 'pointcloud', 'record', 'wall', 'opening', 'space', 'grid', 'column', 'stair', 'ramp',
                  'tag', 'viewmark', 'slab', 'beam'));
