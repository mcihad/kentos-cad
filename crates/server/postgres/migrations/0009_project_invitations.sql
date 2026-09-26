-- KentOS CAD: invitations by link and guests (docs/adr/0035; TODOS.md
-- CLOUD-16, CLOUD-17). The owner's decision of 26 September: the inviter
-- copies a link (no mail server); only the account with the invitation's
-- e-mail accepts it; someone outside the organisation becomes a guest who
-- reaches only the projects shared with them, and the organisation can turn
-- guests off.

-- ── A verified e-mail ────────────────────────────────────────────────────

-- Whether an account's e-mail is known to be its holder's: the OpenID
-- provider's `email_verified` at the last sign-in, or, for a local account,
-- an administrator who set it.
alter table kentos.app_user add column email_verified boolean not null default false;
update kentos.app_user set email_verified = true where issuer = 'kentos:local' and email is not null;

drop function kentos.resolve_identity(uuid, text, text, text, text);
create function kentos.resolve_identity(p_new_id uuid, p_issuer text, p_subject text, p_name text, p_email text, p_email_verified boolean)
  returns uuid
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  uid uuid;
  st text;
begin
  if p_issuer = 'kentos:local' then
    raise exception 'local accounts are not resolved as OpenID identities';
  end if;
  insert into kentos.app_user (id, issuer, subject, display_name, email, email_verified)
  values (p_new_id, p_issuer, p_subject, p_name, p_email, p_email is not null and coalesce(p_email_verified, false))
  on conflict (issuer, subject) do update set display_name = excluded.display_name, email = excluded.email,
    email_verified = excluded.email_verified
  returning id, status into uid, st;
  return case when st = 'active' then uid end;
end $$;
revoke all on function kentos.resolve_identity(uuid, text, text, text, text, boolean) from public;
grant execute on function kentos.resolve_identity(uuid, text, text, text, text, boolean) to kentos_cad_app;

-- ── Guests ───────────────────────────────────────────────────────────────

-- An organisation takes guests unless it turns them off; turned off, its
-- guests' grants stop working at once (and work again when it is turned on).
alter table kentos.tenant add column allow_guests boolean not null default true;

-- A guest's grant: given to someone outside the organisation by accepting an
-- invitation. Managing and sharing stay with members.
alter table kentos.project_grant
  add column guest boolean not null default false,
  add constraint project_grant_guest_role check (not guest or role in ('viewer', 'commenter', 'editor'));

-- As in migration 0004, with guests: in an organisation, someone who is not
-- a member at all works with a guest grant while the organisation takes
-- guests. A member whose membership or seat is gone gains nothing from one.
create or replace function kentos.project_role(p_tenant uuid, p_project uuid, p_owner uuid) returns text
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  ws_kind text;
  ws_status text;
  ws_owner uuid;
  ws_admins boolean;
  ws_guests boolean;
  member_role text;
  usable boolean;
  granted text;
  granted_guest boolean;
begin
  if me is null or p_tenant is null or p_project is null then
    return null;
  end if;
  select kind, status, owner_user_id, admins_access_all_projects, allow_guests
    into ws_kind, ws_status, ws_owner, ws_admins, ws_guests
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
  select g.role, g.guest into granted, granted_guest
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
    if member_role is null and ws_guests and coalesce(granted_guest, false) then
      return granted;
    end if;
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

-- ── Invitations ──────────────────────────────────────────────────────────

-- An invitation to one project for one e-mail, with the role it gives. Only
-- the SHA-256 of its token is kept; the token is shown to the inviter once.
-- It is used once: accepted, or revoked; one past `expires_at` is dead.
create table kentos.project_invitation (
  tenant_id uuid not null,
  project_id uuid not null,
  id uuid not null,
  email text not null check (email = lower(email) and length(email) between 3 and 254),
  role text not null check (role in ('viewer', 'commenter', 'editor')),
  token_hash bytea not null unique,
  state text not null default 'pending' check (state in ('pending', 'accepted', 'revoked')),
  created_by uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  expires_at timestamptz not null,
  accepted_by uuid references kentos.app_user (id),
  accepted_at timestamptz,
  revoked_by uuid references kentos.app_user (id),
  revoked_at timestamptz,
  primary key (tenant_id, project_id, id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);
-- One waiting invitation per e-mail in a project: a new one replaces it.
create unique index project_invitation_pending on kentos.project_invitation (tenant_id, project_id, email)
  where state = 'pending';

alter table kentos.project_invitation enable row level security;
create policy invitation_project on kentos.project_invitation
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

-- Accepting an invitation, for the current user, who has no role in its
-- project yet (so this runs as the owner). The token finds a waiting,
-- unexpired invitation; the account's e-mail must be the invitation's and
-- verified. In an organisation a member (active, with a seat) gets a plain
-- grant, anyone else a guest grant while the organisation takes guests; in a
-- personal space everyone gets a plain grant. An existing grant keeps the
-- stronger role (a guest's at most editor). The invitation is then used.
-- `outcome` says what happened: ok, missing, wrong_email, unverified, gone,
-- inactive (a member whose membership or seat is gone), guests_off, owner.
create function kentos.accept_invitation(p_token text)
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
  rank constant text[] := array['viewer', 'commenter', 'editor', 'manager'];
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
revoke all on function kentos.accept_invitation(text) from public;

grant execute on function kentos.accept_invitation(text) to kentos_cad_app;
grant select, insert, update on kentos.project_invitation to kentos_cad_app;
