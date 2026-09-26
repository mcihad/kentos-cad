-- KentOS CAD: the project catalog (docs/adr/0028-project-catalog.md).
--
-- * What describes a project in the catalog (TODOS.md CLOUD-02, CLOUD-03): a
--   description, a type and tags beside its name. The type is a label for
--   finding and ordering work: it is not a claim that the project meets a
--   regulation, and it does not decide how the project is stored. Existing
--   projects become the general type ('cad', "Genel CAD"). catalog_version
--   counts changes of the name, description, type and tags (optimistic
--   concurrency of catalog edits); the drawing's own metadata keeps
--   meta_version.
-- * Lifecycle (CLOUD-05): an archived project (archived_at, archived_by) is
--   read-only and listed apart; a project in the trash is the soft delete of
--   0002 (deleted_at, deleted_by), and purge_after is when the retention
--   removes it for good, fixed when it is moved there. A project deleted
--   before this migration has no purge_after: it was deleted under the old
--   rule (kept for the operator) and the retention never removes it.
-- * Per person (CLOUD-04): the projects one opened lately and one's
--   favourites, seen by that person only.
-- * Three functions do what the server's role may not do by itself, each
--   checking the current user's role like row-level security would:
--   kentos.duplicate_project reads one project and writes another in one
--   statement; kentos.purge_project removes a project from the trash for good
--   (its objects, grants, log and events; the audit stays and records it);
--   kentos.purge_trash does the same for the projects whose retention ended.

-- ── Catalog metadata and lifecycle ───────────────────────────────────────

alter table kentos.project
  add column description text not null default '' check (length(description) <= 2000),
  add column project_type text not null default 'cad'
    check (project_type in ('cad', 'gis', 'landReadjustment', 'zoningPlan', 'subdivision', 'road', 'architecture')),
  add column tags text[] not null default '{}' check (cardinality(tags) <= 12 and array_position(tags, null) is null),
  add column catalog_version bigint not null default 1,
  add column archived_at timestamptz,
  add column archived_by uuid references kentos.app_user (id),
  add column purge_after timestamptz,
  add constraint project_archived_by check ((archived_at is null) = (archived_by is null)),
  -- Only a project in the trash is waiting to be removed.
  add constraint project_purge_after check (purge_after is null or deleted_at is not null);

-- What opening a project needs to know (0004), now with whether it is archived.
drop function kentos.project_access(uuid, uuid);
create function kentos.project_access(p_tenant uuid, p_project uuid)
  returns table (role text, name text, deleted boolean, archived boolean, tenant_name text, tenant_kind text, viewer_download boolean)
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  p_name text;
  p_deleted boolean;
  p_archived boolean;
  p_owner uuid;
  t_name text;
  t_kind text;
  t_download boolean;
  r text;
begin
  select q.name, q.deleted_at is not null, q.archived_at is not null, q.owner_user_id
    into p_name, p_deleted, p_archived, p_owner
    from kentos.project q where q.tenant_id = p_tenant and q.id = p_project;
  select t.name, t.kind, t.viewer_download into t_name, t_kind, t_download
    from kentos.tenant t where t.id = p_tenant;
  r := kentos.project_role(p_tenant, p_project, p_owner);
  if r is null or p_name is null then
    return;
  end if;
  return query select r, p_name, p_deleted, p_archived, t_name, t_kind, t_download;
end $$;
revoke all on function kentos.project_access(uuid, uuid) from public;
grant execute on function kentos.project_access(uuid, uuid) to kentos_cad_app;

-- ── Per person: recently opened and favourite projects ───────────────────

create table kentos.project_recent (
  tenant_id uuid not null,
  project_id uuid not null,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  opened_at timestamptz not null default now(),
  primary key (tenant_id, project_id, user_id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);
create index project_recent_user on kentos.project_recent (user_id, opened_at desc);

create table kentos.project_favorite (
  tenant_id uuid not null,
  project_id uuid not null,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (tenant_id, project_id, user_id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);
create index project_favorite_user on kentos.project_favorite (user_id);

-- One's own rows are seen in any scope: a list joins them with the projects
-- row-level security shows, so a project one lost is not listed. They are
-- written only for oneself, inside the project's scope, while one sees it.
alter table kentos.project_recent enable row level security;
create policy recent_own on kentos.project_recent for select
  using (user_id = kentos.current_user_id());
create policy recent_add on kentos.project_recent for insert
  with check (user_id = kentos.current_user_id() and tenant_id = kentos.current_tenant()
              and project_id = kentos.current_project() and (select kentos.project_readable()));
create policy recent_touch on kentos.project_recent for update
  using (user_id = kentos.current_user_id() and tenant_id = kentos.current_tenant() and project_id = kentos.current_project())
  with check (user_id = kentos.current_user_id() and tenant_id = kentos.current_tenant()
              and project_id = kentos.current_project() and (select kentos.project_readable()));

alter table kentos.project_favorite enable row level security;
create policy favorite_own on kentos.project_favorite for select
  using (user_id = kentos.current_user_id());
create policy favorite_add on kentos.project_favorite for insert
  with check (user_id = kentos.current_user_id() and tenant_id = kentos.current_tenant()
              and project_id = kentos.current_project() and (select kentos.project_readable()));
create policy favorite_remove on kentos.project_favorite for delete
  using (user_id = kentos.current_user_id() and tenant_id = kentos.current_tenant() and project_id = kentos.current_project());

-- ── Duplicating ──────────────────────────────────────────────────────────

-- Copies project p_src into a new project p_dst of p_dst_tenant, owned by the
-- current user: the name given, the settings, layer tree, styles, origin,
-- home view, description, type and tags, and every object with its
-- persistent id at version 1 (the copy's first commit, data revision 1).
-- Not copied: history (command log, events, audit), grants, favourites and
-- recents, the archived state. Returns how many objects were copied.
--
-- The current user must see the source (and may download it: viewers and
-- commenters not while the organisation's viewer_download is off) and may
-- create projects in the destination (owner, admin or project manager with
-- an active membership and a seat). The server checks the same before (the
-- rules of access.rs and tenancy.rs); these checks keep a mistake in it from
-- copying a project it should not.
create function kentos.duplicate_project(p_src_tenant uuid, p_src uuid, p_dst_tenant uuid, p_dst uuid, p_name text)
  returns bigint
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  src record;
  r text;
  copied bigint;
begin
  if me is null then
    raise exception 'duplicate_project needs app.user_id';
  end if;
  select q.srid, q.settings, q.layers, q.active_layer, q.origin_x, q.origin_y, q.home_view, q.styles,
         q.owner_user_id, q.description, q.project_type, q.tags, t.viewer_download
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
                              created_by, owner_user_id, description, project_type, tags)
  values (p_dst_tenant, p_dst, p_name, src.srid, src.settings, src.layers, src.active_layer, src.origin_x, src.origin_y,
          src.home_view, src.styles, me, me, src.description, src.project_type, src.tags);
  insert into kentos.feature (tenant_id, project_id, id, layer_id, version, kind, source_kind, srid, geom, cad_definition,
                              properties, label, color, symbol, projection_version, created_by, updated_by)
  select p_dst_tenant, p_dst, f.id, f.layer_id, 1, f.kind, f.source_kind, f.srid, f.geom, f.cad_definition,
         f.properties, f.label, f.color, f.symbol, f.projection_version, me, me
    from kentos.feature f
   where f.tenant_id = p_src_tenant and f.project_id = p_src;
  get diagnostics copied = row_count;
  -- A created object's version is its commit's data revision (docs/adr/0026): the copy is the first.
  if copied > 0 then
    update kentos.project set data_revision = 1 where tenant_id = p_dst_tenant and id = p_dst;
  end if;
  return copied;
end $$;

-- ── Removing for good ────────────────────────────────────────────────────

-- Removes a project with everything that belongs to it: objects, grants,
-- favourites and recents go with its row; its command log, events and the
-- log's horizon here. The audit stays. Called only by the two functions
-- below (not granted to the server's role).
create function kentos.remove_project(p_tenant uuid, p_project uuid) returns void
  language sql volatile set search_path = pg_catalog, pg_temp
  as $$
    delete from kentos.outbox_event where tenant_id = p_tenant and project_id = p_project;
    delete from kentos.outbox_horizon where tenant_id = p_tenant and project_id = p_project;
    delete from kentos.command_log where tenant_id = p_tenant and project_id = p_project;
    delete from kentos.project where tenant_id = p_tenant and id = p_project;
  $$;
revoke all on function kentos.remove_project(uuid, uuid) from public;

-- A person removes a project in the trash for good: only its owner, or an
-- organisation's owner or admin under its policy (the roles with
-- project.delete). Nothing happens to a project that is not there; one that
-- is not in the trash is refused. The audit records who, the name and how
-- many objects went. Returns the name and the number of objects.
create function kentos.purge_project(p_tenant uuid, p_project uuid, p_request text)
  returns table (purged_name text, purged_objects bigint)
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  p record;
  r text;
  n bigint;
begin
  if me is null then
    raise exception 'purge_project needs app.user_id';
  end if;
  select q.name, q.owner_user_id, q.deleted_at into p
    from kentos.project q where q.tenant_id = p_tenant and q.id = p_project for update;
  if not found then
    return;
  end if;
  r := kentos.project_role(p_tenant, p_project, p.owner_user_id);
  if r is null or r not in ('owner', 'policy') then
    raise exception 'purge_project: % may not delete %', me, p_project;
  end if;
  if p.deleted_at is null then
    raise exception 'purge_project: % is not in the trash', p_project;
  end if;
  select count(*) into n from kentos.feature f where f.tenant_id = p_tenant and f.project_id = p_project;
  perform kentos.remove_project(p_tenant, p_project);
  insert into kentos.audit_event (tenant_id, project_id, actor, action, request_id, detail)
  values (p_tenant, p_project, me, 'project.purge', p_request,
          jsonb_build_object('name', p.name, 'objects', n, 'trashedAt', p.deleted_at));
  return query select p.name::text, n;
end $$;

-- The retention: removes for good at most p_batch projects whose time in the
-- trash is over (purge_after, fixed when they were moved there), of every
-- tenant, oldest first; each is audited without an actor. Never one moved to
-- the trash less than a day ago, whatever its purge_after says. Returns how
-- many went: a full batch means there may be more.
create function kentos.purge_trash(p_batch integer) returns bigint
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  p record;
  objects bigint;
  n bigint := 0;
begin
  for p in
    select q.tenant_id, q.id, q.name, q.deleted_at, q.purge_after
      from kentos.project q
     where q.deleted_at is not null and q.purge_after is not null and q.purge_after <= now()
       and q.deleted_at <= now() - interval '1 day'
     order by q.purge_after
     limit greatest(p_batch, 1)
     for update skip locked
  loop
    select count(*) into objects from kentos.feature f where f.tenant_id = p.tenant_id and f.project_id = p.id;
    perform kentos.remove_project(p.tenant_id, p.id);
    insert into kentos.audit_event (tenant_id, project_id, action, detail)
    values (p.tenant_id, p.id, 'project.purge',
            jsonb_build_object('name', p.name, 'objects', objects, 'trashedAt', p.deleted_at, 'purgeAfter', p.purge_after, 'by', 'retention'));
    n := n + 1;
  end loop;
  return n;
end $$;

-- ── What the server's role may do ────────────────────────────────────────

revoke all on function kentos.duplicate_project(uuid, uuid, uuid, uuid, text) from public;
revoke all on function kentos.purge_project(uuid, uuid, text) from public;
revoke all on function kentos.purge_trash(integer) from public;
grant execute on function kentos.duplicate_project(uuid, uuid, uuid, uuid, text) to kentos_cad_app;
grant execute on function kentos.purge_project(uuid, uuid, text) to kentos_cad_app;
grant execute on function kentos.purge_trash(integer) to kentos_cad_app;
grant select, insert, update on kentos.project_recent to kentos_cad_app;
grant select, insert, delete on kentos.project_favorite to kentos_cad_app;
