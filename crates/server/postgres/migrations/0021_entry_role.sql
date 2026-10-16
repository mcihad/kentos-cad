-- KentOS CAD: the Veri girişi role (docs/adr/0411 §2): `entry`, between a commenter and an editor. It edits
-- existing records' attributes through a published form only; the application gives it `forms.use` and no
-- `feature.write`. A grant, a guest's grant and an invitation may give it; accepting an invitation keeps
-- the stronger role by the order viewer < commenter < entry < editor < manager.

alter table kentos.project_grant drop constraint project_grant_role_check;
alter table kentos.project_grant add constraint project_grant_role_check
  check (role in ('viewer', 'commenter', 'entry', 'editor', 'manager'));
alter table kentos.project_grant drop constraint project_grant_guest_role;
alter table kentos.project_grant add constraint project_grant_guest_role
  check (not guest or role in ('viewer', 'commenter', 'entry', 'editor'));
alter table kentos.project_invitation drop constraint project_invitation_role_check;
alter table kentos.project_invitation add constraint project_invitation_role_check
  check (role in ('viewer', 'commenter', 'entry', 'editor'));

-- As in migration 0009, with `entry` in the roles' order.
create or replace function kentos.accept_invitation(p_token text)
  returns table (outcome text, tenant_id uuid, project_id uuid, invitation_id uuid, role text, guest boolean)
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
-- The answer's column names are also the tables' own: inside, a name is the table's column.
#variable_conflict use_column
declare
  me constant uuid := kentos.current_user_id();
  inv record;
  acct record;
  ws record;
  owner_id uuid;
  member_role text;
  member_ok boolean;
  is_guest boolean := false;
  held text;
  rank constant text[] := array['viewer', 'commenter', 'entry', 'editor', 'manager'];
  given text;
begin
  if me is null then
    raise exception 'accept_invitation needs app.user_id';
  end if;
  select i.tenant_id, i.project_id, i.id, i.email, i.role, i.state, i.expires_at, i.created_by into inv
    from kentos.project_invitation i
   where i.token_hash = public.digest(p_token, 'sha256')
   for update;
  if not found or inv.state <> 'pending' or inv.expires_at <= now() then
    return query select 'missing'::text, null::uuid, null::uuid, null::uuid, null::text, null::boolean;
    return;
  end if;
  select u.email, u.email_verified into acct from kentos.app_user u where u.id = me;
  if acct.email is null or lower(acct.email) <> inv.email then
    return query select 'wrong_email'::text, null::uuid, null::uuid, null::uuid, null::text, null::boolean;
    return;
  end if;
  if not acct.email_verified then
    return query select 'unverified'::text, null::uuid, null::uuid, null::uuid, null::text, null::boolean;
    return;
  end if;
  select p.owner_user_id into owner_id
    from kentos.project p
   where p.tenant_id = inv.tenant_id and p.id = inv.project_id and p.deleted_at is null;
  if not found then
    return query select 'gone'::text, null::uuid, null::uuid, null::uuid, null::text, null::boolean;
    return;
  end if;
  select t.kind, t.status, t.allow_guests into ws from kentos.tenant t where t.id = inv.tenant_id;
  if ws.status <> 'active' then
    return query select 'gone'::text, null::uuid, null::uuid, null::uuid, null::text, null::boolean;
    return;
  end if;
  if owner_id = me then
    update kentos.project_invitation set state = 'accepted', accepted_by = me, accepted_at = now()
     where tenant_id = inv.tenant_id and project_id = inv.project_id and id = inv.id;
    return query select 'owner'::text, inv.tenant_id, inv.project_id, inv.id, 'owner'::text, false;
    return;
  end if;
  if ws.kind = 'organization' then
    select m.role,
           m.status = 'active'
             and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
      into member_role, member_ok
      from kentos.membership m
     where m.tenant_id = inv.tenant_id and m.user_id = me;
    if member_role is not null and not member_ok then
      return query select 'inactive'::text, null::uuid, null::uuid, null::uuid, null::text, null::boolean;
      return;
    end if;
    if member_role is null then
      if not ws.allow_guests then
        return query select 'guests_off'::text, null::uuid, null::uuid, null::uuid, null::text, null::boolean;
        return;
      end if;
      is_guest := true;
    end if;
  end if;
  select g.role into held
    from kentos.project_grant g
   where g.tenant_id = inv.tenant_id and g.project_id = inv.project_id and g.user_id = me
     and (g.expires_at is null or g.expires_at > now());
  given := case when held is not null and array_position(rank, held) > array_position(rank, inv.role)
                then held else inv.role end;
  if is_guest and given = 'manager' then
    given := 'editor';
  end if;
  insert into kentos.project_grant (tenant_id, project_id, user_id, role, granted_by, guest)
  values (inv.tenant_id, inv.project_id, me, given, inv.created_by, is_guest)
  on conflict on constraint project_grant_pkey do update set
    role = excluded.role, guest = excluded.guest, granted_by = excluded.granted_by,
    -- A kept grant keeps its end; one the invitation gives has none.
    expires_at = case when kentos.project_grant.role = excluded.role and held is not null
                      then kentos.project_grant.expires_at end,
    updated_at = now();
  update kentos.project_invitation set state = 'accepted', accepted_by = me, accepted_at = now()
   where tenant_id = inv.tenant_id and project_id = inv.project_id and id = inv.id;
  return query select 'ok'::text, inv.tenant_id, inv.project_id, inv.id, given, is_guest;
end $$;
