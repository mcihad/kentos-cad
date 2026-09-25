-- KentOS CAD: who owns a project and who may use it (docs/adr/0015-project-ownership-and-access.md).
--
-- Until now a tenant's every member saw every project of the tenant. From here:
--
-- * A tenant is an organisation (today's tenants) or a person's personal space,
--   opened by the server the first time the person needs it
--   (`kentos.ensure_personal_tenant`). Its only member is its owner; others
--   reach its projects through grants.
-- * Every project has an owner; its other users hold a grant (viewer,
--   commenter, editor, manager). An organisation's owners and admins also
--   reach unshared projects while its policy `admins_access_all_projects` is on
--   (the default). `kentos.project_role` is the one place that works out
--   the role; the server's `access.rs` maps roles to permissions.
-- * Row-level security follows: a project row is visible only to someone with
--   a role in it, and the rows that belong to a project (objects, command log,
--   audit, events, the log's horizon, grants) only inside a transaction scoped
--   to that project (`app.project_id`, set per transaction like the tenant)
--   and only while its user can see the project. A mistake in the server's
--   code cannot open another project's rows.
--
-- Existing projects: the creator becomes the owner, and every current member
-- gets the grant matching their tenant role on every existing project of the
-- tenant (viewer → viewer, editor → editor, project_manager → manager, whatever
-- the membership's status: status and seat are still checked on each use, as
-- before). Owners and admins keep their reach through the policy. So nobody
-- loses or gains access to an existing project; new projects are the owner's
-- (and the policy's) until they are shared.

-- ── Workspaces ───────────────────────────────────────────────────────────

alter table kentos.tenant
  add column kind text not null default 'organization' check (kind in ('personal', 'organization')),
  -- A personal space's person; a person has at most one.
  add column owner_user_id uuid unique references kentos.app_user (id),
  -- The organisation's owners and admins work in projects not shared with them (manager's rights and
  -- deleting). The owner's decision: on unless the organisation turns it off.
  add column admins_access_all_projects boolean not null default true,
  -- Viewers and commenters may download and export (the recommended default).
  add column viewer_download boolean not null default true,
  add constraint tenant_personal_owner check ((kind = 'personal') = (owner_user_id is not null));

-- ── Owners and grants ────────────────────────────────────────────────────

alter table kentos.project add column owner_user_id uuid references kentos.app_user (id);
update kentos.project set owner_user_id = created_by;
alter table kentos.project alter column owner_user_id set not null;
create index project_owner on kentos.project (owner_user_id);

-- A person's role in a project besides its owner (groups come in a later slice).
create table kentos.project_grant (
  tenant_id uuid not null,
  project_id uuid not null,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  role text not null check (role in ('viewer', 'commenter', 'editor', 'manager')),
  -- The grant ends by itself at this moment (null: until revoked).
  expires_at timestamptz,
  -- Null for the grants this migration made from tenant roles.
  granted_by uuid references kentos.app_user (id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, project_id, user_id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);
create index project_grant_user on kentos.project_grant (user_id);

-- Today's access as explicit grants (see the header), recorded in each project's audit.
insert into kentos.project_grant (tenant_id, project_id, user_id, role)
select p.tenant_id, p.id, m.user_id,
       case m.role when 'viewer' then 'viewer' when 'editor' then 'editor' else 'manager' end
  from kentos.project p
  join kentos.membership m on m.tenant_id = p.tenant_id
 where m.role in ('viewer', 'editor', 'project_manager')
   and m.user_id <> p.owner_user_id;

insert into kentos.audit_event (tenant_id, project_id, action, detail)
select g.tenant_id, g.project_id, 'project.access.migrate',
       jsonb_build_object('migration', '0004_project_access',
                          'grants', jsonb_agg(jsonb_build_object('userId', g.user_id, 'role', g.role) order by g.user_id))
  from kentos.project_grant g
 group by g.tenant_id, g.project_id;

-- ── The project in scope and the role in it ──────────────────────────────

create function kentos.current_project() returns uuid
  language sql stable
  as $$ select nullif(current_setting('app.project_id', true), '')::uuid $$;

-- The current user's role in a project, or null: 'owner', 'policy' (an
-- organisation owner's or admin's reach through the policy: manager's rights
-- and deleting), or the grant's role. It reads no project row (the owner is
-- given), so project policies can call it; it runs as the owner and returns
-- nothing about anyone but the current user.
--
-- * The tenant must be active.
-- * Personal space: its owner owns every project in it; anyone else needs an
--   unexpired grant (no membership).
-- * Organisation: an active membership with a seat first, whatever the role
--   comes from; then ownership, the policy, a grant, in that order (each
--   gives at least the rights of the ones after it).
--
-- The work does not depend on whether the project exists (the owner is null
-- then), so an answer's timing does not tell a real id from a guessed one.
create function kentos.project_role(p_tenant uuid, p_project uuid, p_owner uuid) returns text
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  ws_kind text;
  ws_status text;
  ws_owner uuid;
  ws_admins boolean;
  member_role text;
  usable boolean;
  granted text;
begin
  if me is null or p_tenant is null or p_project is null then
    return null;
  end if;
  select kind, status, owner_user_id, admins_access_all_projects
    into ws_kind, ws_status, ws_owner, ws_admins
    from kentos.tenant where id = p_tenant;
  if not found or ws_status <> 'active' then
    return null;
  end if;
  select m.role,
         m.status = 'active'
           and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
    into member_role, usable
    from kentos.membership m
   where m.tenant_id = p_tenant and m.user_id = me;
  select g.role into granted
    from kentos.project_grant g
   where g.tenant_id = p_tenant and g.project_id = p_project and g.user_id = me
     and (g.expires_at is null or g.expires_at > now());
  if ws_kind = 'personal' then
    if me = ws_owner then
      return 'owner';
    end if;
    return granted;
  end if;
  if not coalesce(usable, false) then
    return null;
  end if;
  if p_owner = me then
    return 'owner';
  end if;
  if ws_admins and member_role in ('owner', 'admin') then
    return 'policy';
  end if;
  return granted;
end $$;

-- What opening a project needs to know, for the current user: nothing when
-- they have no role in it (or it does not exist; the same work either way).
create function kentos.project_access(p_tenant uuid, p_project uuid)
  returns table (role text, name text, deleted boolean, tenant_name text, tenant_kind text, viewer_download boolean)
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  p_name text;
  p_deleted boolean;
  p_owner uuid;
  t_name text;
  t_kind text;
  t_download boolean;
  r text;
begin
  select q.name, q.deleted_at is not null, q.owner_user_id into p_name, p_deleted, p_owner
    from kentos.project q where q.tenant_id = p_tenant and q.id = p_project;
  select t.name, t.kind, t.viewer_download into t_name, t_kind, t_download
    from kentos.tenant t where t.id = p_tenant;
  r := kentos.project_role(p_tenant, p_project, p_owner);
  if r is null or p_name is null then
    return;
  end if;
  return query select r, p_name, p_deleted, t_name, t_kind, t_download;
end $$;

-- Whether the project in scope is one the current user has a role in (the
-- project policy below decides). Policies call it as `(select …)`, so it runs
-- once per statement, not once per row.
create function kentos.project_readable() returns boolean
  language sql stable
  as $$ select exists (select 1 from kentos.project p
                        where p.tenant_id = kentos.current_tenant() and p.id = kentos.current_project()) $$;

-- ── Personal spaces ──────────────────────────────────────────────────────

-- Opens the current user's personal space if it is not there yet and returns
-- its id. Only this function opens one (the server's role cannot create
-- tenants): the kind, owner, one seat and the owner's membership are fixed
-- here. Safe to call again and at once from two requests.
create function kentos.ensure_personal_tenant(p_new_id uuid, p_name text) returns uuid
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  tid uuid;
begin
  if me is null then
    raise exception 'ensure_personal_tenant needs app.user_id';
  end if;
  if not exists (select 1 from kentos.app_user where id = me and status = 'active') then
    raise exception 'account % is not active', me;
  end if;
  select id into tid from kentos.tenant where owner_user_id = me;
  if found then
    return tid;
  end if;
  insert into kentos.tenant (id, slug, name, seat_limit, kind, owner_user_id)
  values (p_new_id, 'kisisel-' || replace(me::text, '-', ''), p_name, 1, 'personal', me)
  on conflict (owner_user_id) do nothing
  returning id into tid;
  if tid is null then
    -- Another request opened it in the meantime.
    select id into tid from kentos.tenant where owner_user_id = me;
    return tid;
  end if;
  insert into kentos.membership (tenant_id, user_id, role) values (tid, me, 'owner');
  insert into kentos.seat_allocation (tenant_id, user_id) values (tid, me);
  insert into kentos.audit_event (tenant_id, actor, action) values (tid, me, 'tenant.personal.open');
  return tid;
end $$;

-- ── Row-level security ───────────────────────────────────────────────────

drop policy project_tenant on kentos.project;
-- Visible (and changed) only by someone with a role in it; inside a project's
-- scope, only that project. A new project's owner is the one creating it.
create policy project_visible on kentos.project for select
  using (tenant_id = kentos.current_tenant()
         and (kentos.current_project() is null or id = kentos.current_project())
         and kentos.project_role(tenant_id, id, owner_user_id) is not null);
create policy project_insert on kentos.project for insert
  with check (tenant_id = kentos.current_tenant()
              and owner_user_id = kentos.current_user_id()
              and (kentos.current_project() is null or id = kentos.current_project())
              and kentos.project_role(tenant_id, id, owner_user_id) = 'owner');
create policy project_update on kentos.project for update
  using (tenant_id = kentos.current_tenant()
         and (kentos.current_project() is null or id = kentos.current_project())
         and kentos.project_role(tenant_id, id, owner_user_id) is not null)
  with check (tenant_id = kentos.current_tenant()
              and (kentos.current_project() is null or id = kentos.current_project())
              and kentos.project_role(tenant_id, id, owner_user_id) is not null);

drop policy feature_tenant on kentos.feature;
create policy feature_project on kentos.feature
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

drop policy command_log_tenant on kentos.command_log;
create policy command_log_project on kentos.command_log
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));
-- A retried create looks up the project its key made before that project is in scope: only one's own.
create policy command_log_own_create on kentos.command_log for select
  using (tenant_id = kentos.current_tenant() and actor = kentos.current_user_id() and command_name = 'project.create');

drop policy audit_tenant on kentos.audit_event;
create policy audit_project on kentos.audit_event
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

drop policy outbox_tenant on kentos.outbox_event;
create policy outbox_project on kentos.outbox_event
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

drop policy outbox_horizon_tenant on kentos.outbox_horizon;
create policy outbox_horizon_project on kentos.outbox_horizon for select
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

alter table kentos.project_grant enable row level security;
-- One's own grants, in any scope (the "shared with me" list starts from them).
create policy grant_own on kentos.project_grant for select
  using (user_id = kentos.current_user_id());
-- A project's grants inside its scope, for those who can see it; written only there.
create policy grant_project on kentos.project_grant
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

-- A project's grantees are named in its scope, members of the tenant or not
-- (someone a personal space's owner shared with is not a member of it).
drop policy app_user_visible on kentos.app_user;
create policy app_user_visible on kentos.app_user for select
  using (id = kentos.current_user_id()
         or exists (select 1 from kentos.membership m where m.user_id = app_user.id and m.tenant_id = kentos.current_tenant())
         or exists (select 1 from kentos.project_grant g
                     where g.user_id = app_user.id and g.tenant_id = kentos.current_tenant() and g.project_id = kentos.current_project()));

-- ── What the server's role may do ────────────────────────────────────────

revoke all on function kentos.project_role(uuid, uuid, uuid) from public;
revoke all on function kentos.project_access(uuid, uuid) from public;
revoke all on function kentos.ensure_personal_tenant(uuid, text) from public;
grant execute on function kentos.current_project(), kentos.project_readable() to kentos_cad_app;
grant execute on function kentos.project_role(uuid, uuid, uuid), kentos.project_access(uuid, uuid) to kentos_cad_app;
grant execute on function kentos.ensure_personal_tenant(uuid, text) to kentos_cad_app;
grant select, insert, update, delete on kentos.project_grant to kentos_cad_app;
