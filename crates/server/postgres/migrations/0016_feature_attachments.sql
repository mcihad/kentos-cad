-- KentOS CAD: an object's attachments (docs/adr/0225 §1): its photos and documents, in order, as the
-- contract writes them (`attachments`: each a name and one source, an embedded photo's library
-- asset, a linked file's path or a web address, and a note); null: none. The contract's rules
-- (`attachments_problem`: at most 64, a name, one source, an http or https address, short
-- texts) and “no attachments on a block's objects” are the application's check, as an
-- object's other fields' are; the column only keeps its shape. A copy
-- (kentos.duplicate_project, as in migration 0015) takes them too. A layer's actions and its
-- information card (docs/adr/0225 §4, §5) are its node's fields in the project's layer tree:
-- no column of their own.

alter table kentos.feature add column attachments jsonb
  check (attachments is null or (jsonb_typeof(attachments) = 'array' and jsonb_array_length(attachments) between 1 and 64));

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
                              properties, label, color, symbol, line_weight, label_pins, attachments, projection_version, created_by, updated_by)
  select p_dst_tenant, p_dst, f.id, f.layer_id, 1, f.kind, f.source_kind, f.srid, f.geom, f.cad_definition,
         f.properties, f.label, f.color, f.symbol, f.line_weight, f.label_pins, f.attachments, f.projection_version, me, me
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
