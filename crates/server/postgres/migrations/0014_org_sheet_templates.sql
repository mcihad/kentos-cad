-- KentOS CAD: an organisation's sheet template library (docs/sheet/design.md
-- §13, “Kurum şablonları”; the owner's decision of 2026-10-03).
--
-- A template kept in an organisation (tenant_id an organisation) is that
-- organisation's library's:
--
-- * It is published by the organisation's owner, its administrators and
--   the members who may create projects there (the rule of
--   `duplicate_project` in 0012: an active membership with a seat in an
--   active organisation, role owner, admin or project_manager): created
--   there directly, or copied there from one's own template
--   (`sheet.template.publish`, which keeps its source in `published_from`).
-- * The one who published it and the organisation's owner and
--   administrators edit and delete it; every other active member with a
--   seat sees and uses it. Guests (people outside it with a project's
--   grant), people who left it, a membership turned off or without a seat,
--   and a suspended organisation see nothing. A publisher who may no longer
--   publish there only sees it.
-- * It is not shared one by one: the grants of 0013 are a person's
--   templates' only.
--
-- `kentos.sheet_template_role` takes the template's tenant now and has the
-- organisation's branch; every policy of 0013 that asked it is made again
-- around the new one, so row-level security still rests on that one
-- function. A person's templates work exactly as in 0013. Whoever sees an
-- organisation's template sees who published it.
--
-- A new migration, not 0013 changed: 0013 may already be applied to a
-- database that is kept (a development database), and sqlx refuses to
-- migrate a database whose applied migration changed. If `main` has taken
-- these numbers by the time the branch is merged, 0013 and this file move
-- to the first free ones together, in this order (docs/sheet/integration.md
-- §4).

-- The template it was published from (one of the publisher's own), for an
-- organisation's template copied by `sheet.template.publish`.
alter table kentos.sheet_template
  add column published_from uuid references kentos.sheet_template (id) on delete set null;
create index sheet_template_tenant on kentos.sheet_template (tenant_id) where deleted_at is null;

-- ── Who may publish ──────────────────────────────────────────────────────

-- Whether the current user may publish into an organisation's library now:
-- its owner, administrators and the members who may create projects, with
-- an active membership and a seat, while the organisation is active.
create function kentos.sheet_template_can_publish(p_tenant uuid) returns boolean
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
    select exists (
      select 1 from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
       where m.tenant_id = p_tenant and m.user_id = kentos.current_user_id()
         and t.kind = 'organization' and t.status = 'active' and m.status = 'active'
         and m.role in ('owner', 'admin', 'project_manager')
         and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id))
  $$;

-- ── The role ─────────────────────────────────────────────────────────────

-- The current user's role in a template: 'owner', 'admin', 'editor',
-- 'viewer' or null. A person's template (its tenant a personal space):
-- 'owner' for its owner, else the grant's role (as in 0013). An
-- organisation's: nothing without an active membership with a seat in the
-- active organisation; then 'owner' for the one who published it while they
-- may still publish there, 'admin' for the organisation's owner and
-- administrators, 'viewer' for everyone else. It reads no template row (the
-- owner and the tenant are given), so the template's own policies can call it.
create function kentos.sheet_template_role(p_template uuid, p_owner uuid, p_tenant uuid) returns text
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  ws_kind text;
  ws_status text;
  member_role text;
  usable boolean;
  granted text;
begin
  if me is null or p_template is null or p_tenant is null then
    return null;
  end if;
  select kind, status into ws_kind, ws_status from kentos.tenant where id = p_tenant;
  if not found then
    return null;
  end if;
  if ws_kind = 'organization' then
    if ws_status <> 'active' then
      return null;
    end if;
    select m.role,
           m.status = 'active'
             and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
      into member_role, usable
      from kentos.membership m
     where m.tenant_id = p_tenant and m.user_id = me;
    if not coalesce(usable, false) then
      return null;
    end if;
    if p_owner = me and member_role in ('owner', 'admin', 'project_manager') then
      return 'owner';
    end if;
    if member_role in ('owner', 'admin') then
      return 'admin';
    end if;
    return 'viewer';
  end if;
  if p_owner = me then
    return 'owner';
  end if;
  select g.role into granted from kentos.sheet_template_grant g where g.template_id = p_template and g.user_id = me;
  return granted;
end $$;

-- Who should hear of a change to a template: a person's, its owner and
-- everyone it is shared with; an organisation's, every active member with a
-- seat (its publisher among them while they have one). Asked by someone with
-- a role in it.
create or replace function kentos.sheet_template_audience(p_template uuid) returns setof uuid
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  t_owner uuid;
  t_tenant uuid;
  ws_kind text;
begin
  select owner_user_id, tenant_id into t_owner, t_tenant from kentos.sheet_template where id = p_template;
  if t_owner is null or kentos.sheet_template_role(p_template, t_owner, t_tenant) is null then
    return;
  end if;
  select kind into ws_kind from kentos.tenant where id = t_tenant;
  if ws_kind = 'organization' then
    return query select m.user_id from kentos.membership m
                  where m.tenant_id = t_tenant and m.status = 'active'
                    and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id);
    return;
  end if;
  return query select t_owner union select g.user_id from kentos.sheet_template_grant g where g.template_id = p_template;
end $$;

-- An audit record of a template change, in the audit of the template's
-- space (no project). Only someone with a role in the template writes one,
-- and only as themselves.
create or replace function kentos.sheet_template_audit(p_template uuid, p_action text, p_request text, p_revision integer, p_detail jsonb)
  returns void
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  t_tenant uuid;
  t_owner uuid;
begin
  select tenant_id, owner_user_id into t_tenant, t_owner from kentos.sheet_template where id = p_template;
  if t_tenant is null or me is null or kentos.sheet_template_role(p_template, t_owner, t_tenant) is null then
    raise exception 'no role in sheet template %', p_template;
  end if;
  insert into kentos.audit_event (tenant_id, project_id, actor, action, request_id, data_revision, detail)
  values (t_tenant, null, me, p_action, p_request, p_revision, p_detail);
end $$;

-- ── Row-level security, around the new role ──────────────────────────────

drop policy sheet_template_visible on kentos.sheet_template;
create policy sheet_template_visible on kentos.sheet_template for select
  using (kentos.sheet_template_role(id, owner_user_id, tenant_id) is not null);
-- A new template is its creator's: in their own personal space, or in an
-- organisation (the command's scope) they may publish into.
drop policy sheet_template_insert on kentos.sheet_template;
create policy sheet_template_insert on kentos.sheet_template for insert
  with check (owner_user_id = kentos.current_user_id()
              and tenant_id = kentos.current_tenant()
              and (exists (select 1 from kentos.tenant t where t.id = tenant_id and t.owner_user_id = kentos.current_user_id())
                   or kentos.sheet_template_can_publish(tenant_id)));
drop policy sheet_template_update on kentos.sheet_template;
create policy sheet_template_update on kentos.sheet_template for update
  using (kentos.sheet_template_role(id, owner_user_id, tenant_id) in ('owner', 'admin', 'editor'))
  with check (kentos.sheet_template_role(id, owner_user_id, tenant_id) in ('owner', 'admin', 'editor'));

drop policy sheet_template_revision_insert on kentos.sheet_template_revision;
create policy sheet_template_revision_insert on kentos.sheet_template_revision for insert
  with check (author_user_id = kentos.current_user_id()
              and exists (select 1 from kentos.sheet_template t
                           where t.id = template_id
                             and kentos.sheet_template_role(t.id, t.owner_user_id, t.tenant_id) in ('owner', 'admin', 'editor')));

-- The grants of one's own templates in one's personal space; only the owner
-- gives and takes them (an organisation's template is not shared one by one).
drop policy sheet_template_grant_owner on kentos.sheet_template_grant;
create policy sheet_template_grant_owner on kentos.sheet_template_grant
  using (exists (select 1 from kentos.sheet_template t join kentos.tenant k on k.id = t.tenant_id
                  where t.id = template_id and t.owner_user_id = kentos.current_user_id() and k.kind = 'personal'))
  with check (granted_by = kentos.current_user_id()
              and exists (select 1 from kentos.sheet_template t join kentos.tenant k on k.id = t.tenant_id
                           where t.id = template_id and t.owner_user_id = kentos.current_user_id() and k.kind = 'personal'));

-- Whether `p_user` published a template the current user sees in an
-- organisation's library (the current user an active member with a seat of
-- the active organisation, as the role's branch asks): the publisher is then
-- named to them, in the list and in the template's details. One indexed
-- look per account row, not the role of every template.
create function kentos.sheet_template_publisher_seen(p_user uuid) returns boolean
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
    select exists (
      select 1 from kentos.sheet_template t
        join kentos.tenant k on k.id = t.tenant_id
        join kentos.membership m on m.tenant_id = t.tenant_id and m.user_id = kentos.current_user_id()
       where t.owner_user_id = p_user and t.deleted_at is null
         and k.kind = 'organization' and k.status = 'active' and m.status = 'active'
         and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id))
  $$;
create policy app_user_sheet_template_publisher on kentos.app_user for select
  using (kentos.sheet_template_publisher_seen(id));

drop function kentos.sheet_template_role(uuid, uuid);

-- ── What the server's role may do ────────────────────────────────────────

revoke all on function kentos.sheet_template_role(uuid, uuid, uuid) from public;
revoke all on function kentos.sheet_template_can_publish(uuid) from public;
revoke all on function kentos.sheet_template_publisher_seen(uuid) from public;
grant execute on function kentos.sheet_template_role(uuid, uuid, uuid) to kentos_cad_app;
grant execute on function kentos.sheet_template_can_publish(uuid) to kentos_cad_app;
grant execute on function kentos.sheet_template_publisher_seen(uuid) to kentos_cad_app;
