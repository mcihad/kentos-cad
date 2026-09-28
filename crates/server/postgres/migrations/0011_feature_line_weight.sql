-- KentOS CAD: an object's own line weight, paper millimetres as a layer's
-- (docs/adr/0139); null: its layer's ("katmana göre"). What a DXF's group
-- 370 and a Netcad pen give an object; 100 mm is the heaviest a pen reaches.

alter table kentos.feature add column line_weight double precision
  check (line_weight is null or (line_weight >= 0 and line_weight <= 100));
