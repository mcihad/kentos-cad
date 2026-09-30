-- KentOS CAD: a database project's block definitions (docs/adr/0144 §5), and
-- block inserts among its objects.
--
-- A definition is the contract's BlockDefinition as JSON (its id and name
-- beside it, for the rules and the list), numbers without the sign of zero
-- as an object's cad_definition. Its version is given as an object's is: a
-- made definition gets the commit's data revision, a change adds one. The
-- rules over the list (one name per project, Turkish case folded; known
-- blocks inside; no cycle; at most 16 levels) and "a definition in use is not
-- removed" are the commit's, under the project's lock.
--
-- An insert's source is its cad_definition; its geometry the collection of
-- its block's objects placed, redone when a definition it places changes.
--
-- A copy (kentos.duplicate_project, as in migration 0007) takes the
-- definitions too, each at version 1 in the source's order, and the objects'
-- own line weights, which migration 0011 added to the objects but not here.

create table kentos.block_definition (
  tenant_id uuid not null,
  project_id uuid not null,
  id uuid not null,
  -- The order they were made in (rows made in one transaction share created_at).
  seq bigint generated always as identity,
  name text not null check (length(name) between 1 and 255),
  definition jsonb not null,
  version bigint not null check (version > 0),
  created_by uuid not null,
  updated_by uuid not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, project_id, id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);

-- The project's own rows: seen and written only with the project in scope,
-- while the current user has a role in it (as the objects are).
alter table kentos.block_definition enable row level security;
create policy block_project on kentos.block_definition
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

grant select, insert, update, delete on kentos.block_definition to kentos_cad_app;

alter table kentos.feature drop constraint feature_kind_check;
alter table kentos.feature add constraint feature_kind_check
  check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline',
                  'xline', 'ray', 'text', 'dimension', 'hatch', 'insert'));

create or replace function kentos.duplicate_project(p_src_tenant uuid, p_src uuid, p_dst_tenant uuid, p_dst uuid, p_name text)
  returns bigint
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  src record;
  r text;
  copied bigint;
  blocks bigint;
begin
  if me is null then
    raise exception 'duplicate_project needs app.user_id';
  end if;
  select q.srid, q.settings, q.layers, q.active_layer, q.origin_x, q.origin_y, q.home_view, q.styles,
         q.owner_user_id, q.description, q.project_type, q.tags, q.storage, t.viewer_download
    into src
    from kentos.project q join kentos.tenant t on t.id = q.tenant_id
   where q.tenant_id = p_src_tenant and q.id = p_src and q.deleted_at is null;
  if not found then
    raise exception 'duplicate_project: no project % to copy', p_src;
  end if;
  r := kentos.project_role(p_src_tenant, p_src, src.owner_user_id);
  if r is null or (r in ('viewer', 'commenter') and not src.viewer_download) then
    raise exception 'duplicate_project: % may not copy %', me, p_src;
  end if;
  if not exists (select 1 from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
                  where m.tenant_id = p_dst_tenant and m.user_id = me and m.status = 'active' and t.status = 'active'
                    and m.role in ('owner', 'admin', 'project_manager')
                    and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)) then
    raise exception 'duplicate_project: % may not create projects in %', me, p_dst_tenant;
  end if;
  insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, home_view, styles,
                              created_by, owner_user_id, description, project_type, tags, storage)
  values (p_dst_tenant, p_dst, p_name, src.srid, src.settings, src.layers, src.active_layer, src.origin_x, src.origin_y,
          src.home_view, src.styles, me, me, src.description, src.project_type, src.tags, src.storage);
  insert into kentos.feature (tenant_id, project_id, id, layer_id, version, kind, source_kind, srid, geom, cad_definition,
                              properties, label, color, symbol, line_weight, projection_version, created_by, updated_by)
  select p_dst_tenant, p_dst, f.id, f.layer_id, 1, f.kind, f.source_kind, f.srid, f.geom, f.cad_definition,
         f.properties, f.label, f.color, f.symbol, f.line_weight, f.projection_version, me, me
    from kentos.feature f
   where f.tenant_id = p_src_tenant and f.project_id = p_src;
  get diagnostics copied = row_count;
  insert into kentos.block_definition (tenant_id, project_id, id, name, definition, version, created_by, updated_by)
  select p_dst_tenant, p_dst, b.id, b.name, b.definition, 1, me, me
    from kentos.block_definition b
   where b.tenant_id = p_src_tenant and b.project_id = p_src
   order by b.seq;
  get diagnostics blocks = row_count;
  -- A created object's version is its commit's data revision (docs/adr/0026): the copy is the first.
  if copied > 0 or blocks > 0 then
    update kentos.project set data_revision = 1 where tenant_id = p_dst_tenant and id = p_dst;
  end if;
  return copied;
end $$;

