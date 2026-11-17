-- KentOS cloud portal (docs/adr/9000): the server's own administrators, organisations described for the portal
-- (municipalities first), professionals who sign up and are verified, and the dossiers a municipality and a
-- professional exchange. A dossier is a professional's application to a municipality or a municipality's transfer of a
-- project to a professional. Each dossier has its packages of files in rounds, its messages and its timeline. Each
-- person has their own notifications.
--
-- Accounts, memberships and seats stay out of the server role's reach (0001). What the portal changes in them goes
-- through the security-definer functions below. Each one checks who calls it: a platform administrator, the
-- organisation's owner or admin, or, when signing up, whatever the platform's setting allows. Each one also records
-- what it did. They answer with a code (null: done) that the server words for the person; they raise only on misuse.

-- ── Platform administrators, settings and record ─────────────────────────

-- The people who run this server (not a role in any organisation). Granted with `kentosd platform-admin`.
create table kentos.platform_admin (
  user_id uuid primary key references kentos.app_user (id) on delete cascade,
  granted_at timestamptz not null default now()
);

create function kentos.is_platform_admin() returns boolean
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select exists (select 1 from kentos.platform_admin a join kentos.app_user u on u.id = a.user_id
                  where a.user_id = kentos.current_user_id() and u.status = 'active')
$$;

-- One row: what the platform allows. Signing up is closed until an administrator opens it (docs/adr/0007).
create table kentos.platform_setting (
  id boolean primary key default true check (id),
  signup_open boolean not null default false,
  updated_at timestamptz not null default now(),
  updated_by uuid references kentos.app_user (id) on delete set null
);
insert into kentos.platform_setting (id) values (true);

-- What administrators did to organisations, accounts and professionals, and who signed up.
create table kentos.platform_event (
  id bigint generated always as identity primary key,
  at timestamptz not null default now(),
  actor_id uuid references kentos.app_user (id) on delete set null,
  actor_name text not null,
  action text not null,
  target_kind text not null check (target_kind in ('tenant', 'user', 'professional', 'setting')),
  target_id uuid,
  target_label text not null default '',
  detail jsonb not null default '{}'::jsonb
);
create index platform_event_at on kentos.platform_event (at desc);

create function kentos.platform_log(p_action text, p_kind text, p_target uuid, p_label text, p_detail jsonb)
  returns void
  language sql security definer set search_path = pg_catalog, pg_temp
  as $$
  insert into kentos.platform_event (actor_id, actor_name, action, target_kind, target_id, target_label, detail)
  select kentos.current_user_id(),
         coalesce((select display_name from kentos.app_user where id = kentos.current_user_id()), 'Kayıt'),
         p_action, p_kind, p_target, coalesce(p_label, ''), coalesce(p_detail, '{}'::jsonb)
$$;

-- ── Accounts opened by signing up ────────────────────────────────────────

-- A local account an administrator opens has a verified e-mail (0010). One its owner opens by signing up does not,
-- until an administrator verifies the person: an invitation to that address would otherwise go to whoever typed it.
alter table kentos.app_user add column signup boolean not null default false;

create or replace function kentos.local_email_verified() returns trigger
  language plpgsql set search_path = pg_catalog, pg_temp
  as $$
begin
  if new.issuer = 'kentos:local' then
    new.email_verified := new.email is not null and not new.signup;
  end if;
  return new;
end $$;

-- A local account with its password (bcrypt, 0001). Not callable by the server: the functions below call it.
create function kentos.create_local_account(p_id uuid, p_login text, p_password text, p_name text, p_email text,
                                            p_signup boolean)
  returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  if exists (select 1 from kentos.local_credential where lower(login) = lower(p_login)) then
    return 'login_taken';
  end if;
  begin
    insert into kentos.app_user (id, issuer, subject, display_name, email, signup)
    values (p_id, 'kentos:local', p_id::text, p_name, nullif(p_email, ''), p_signup);
    insert into kentos.local_credential (user_id, login, password_hash)
    values (p_id, p_login, public.crypt(p_password, public.gen_salt('bf', 12)));
  exception when unique_violation then
    return 'login_taken';
  end;
  return null;
end $$;

-- ── Organisations as the portal sees them ────────────────────────────────

alter table kentos.tenant
  add column category text not null default 'other' check (category in ('municipality', 'office', 'other')),
  add column province text check (province is null or length(province) between 2 and 60),
  add column district text check (district is null or length(district) between 2 and 60);

-- A role's rank: each has the rights of the ones below it.
create function kentos.role_rank(p_role text) returns integer
  language sql immutable set search_path = pg_catalog, pg_temp
  as $$
  select case p_role when 'owner' then 5 when 'admin' then 4 when 'project_manager' then 3
                     when 'editor' then 2 when 'viewer' then 1 else 0 end
$$;

-- The caller's role in an organisation they may use now (an active membership with a seat in an active
-- organisation), or null.
create function kentos.member_role(p_tenant uuid) returns text
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select m.role from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
   where m.tenant_id = p_tenant and m.user_id = kentos.current_user_id()
     and m.status = 'active' and t.status = 'active' and t.kind = 'organization'
     and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
$$;

-- What an organisation's admins say about it on the portal: whether it takes applications and of which kinds
-- (none listed: every kind), and how to reach it.
create table kentos.tenant_portal (
  tenant_id uuid primary key references kentos.tenant (id) on delete cascade,
  accepts_applications boolean not null default false,
  application_kinds text[] not null default '{}' check (array_position(application_kinds, null) is null),
  contact_email text check (contact_email is null or length(contact_email) <= 200),
  contact_phone text check (contact_phone is null or length(contact_phone) <= 40),
  address text check (address is null or length(address) <= 400),
  updated_at timestamptz not null default now(),
  updated_by uuid references kentos.app_user (id) on delete set null
);

-- ── Professionals ────────────────────────────────────────────────────────

-- An architect, surveyor or engineer: their chamber registration as they gave it, and whether an administrator has
-- checked it. Only a verified professional applies to a municipality or is sent work.
create table kentos.professional (
  user_id uuid primary key references kentos.app_user (id) on delete cascade,
  profession text not null check (profession in ('architect', 'surveyor', 'civil_engineer', 'mechanical_engineer',
    'electrical_engineer', 'urban_planner', 'landscape_architect', 'geological_engineer')),
  chamber_no text not null check (length(chamber_no) between 1 and 40),
  office_name text check (office_name is null or length(office_name) <= 200),
  phone text check (phone is null or length(phone) <= 40),
  province text check (province is null or length(province) <= 60),
  -- Whether municipalities find them in their directory (they always see whoever has a dossier with them).
  listed boolean not null default true,
  status text not null default 'pending' check (status in ('pending', 'verified', 'rejected', 'suspended')),
  status_note text not null default '' check (length(status_note) <= 1000),
  reviewed_by uuid references kentos.app_user (id) on delete set null,
  reviewed_at timestamptz,
  created_at timestamptz not null default now()
);
create index professional_status on kentos.professional (status, created_at);

-- ── Dossiers ─────────────────────────────────────────────────────────────

-- One matter between a municipality and a professional. `round` counts the packages sent so far; the one being
-- prepared is `round + 1`. The number is given when it is first sent, per organisation and year.
create table kentos.dossier (
  id uuid primary key,
  tenant_id uuid not null references kentos.tenant (id) on delete cascade,
  professional_id uuid not null references kentos.app_user (id),
  direction text not null check (direction in ('application', 'transfer')),
  year integer,
  number integer check (number is null or number > 0),
  kind text not null,
  title text not null check (length(title) between 3 and 200),
  description text not null default '' check (length(description) <= 8000),
  -- Where: province, district, neighbourhood, block (ada), parcel, address; what was given.
  site jsonb not null default '{}'::jsonb check (jsonb_typeof(site) = 'object'),
  -- An application's building owner (3194 m.22: the owner or their representative applies); a name only.
  owner_name text check (owner_name is null or length(owner_name) <= 200),
  status text not null,
  round integer not null default 0 check (round >= 0),
  assignee_id uuid references kentos.app_user (id) on delete set null,
  assignee_name text,
  due_on date,
  created_by uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  sent_at timestamptz,
  round_sent_at timestamptz,
  closed_at timestamptz,
  unique (tenant_id, year, number),
  check ((year is null) = (number is null)),
  check (
    (direction = 'application'
      and status in ('draft', 'submitted', 'in_review', 'changes_requested', 'approved', 'rejected', 'withdrawn')
      and kind in ('architectural_project', 'structural_project', 'mechanical_project', 'electrical_project',
                   'building_permit', 'occupancy_permit', 'application_sketch', 'site_sketch', 'subdivision',
                   'plan_amendment', 'other'))
    or (direction = 'transfer'
      and status in ('draft', 'sent', 'accepted', 'declined', 'delivered', 'revision_requested', 'completed',
                     'cancelled')
      and kind in ('survey', 'project_preparation', 'project_revision', 'technical_review', 'other'))
  )
);
create index dossier_tenant on kentos.dossier (tenant_id, updated_at desc);
create index dossier_professional on kentos.dossier (professional_id, updated_at desc);

create table kentos.dossier_message (
  id uuid primary key,
  dossier_id uuid not null references kentos.dossier (id) on delete cascade,
  round integer not null check (round >= 0),
  side text not null check (side in ('municipality', 'professional')),
  -- A message; the deficiencies a municipality lists at once (3194 m.22); a decision's reason; a note to a step.
  kind text not null default 'message' check (kind in ('message', 'deficiency', 'decision', 'note')),
  author_id uuid references kentos.app_user (id) on delete set null,
  author_name text not null,
  body text not null check (length(body) between 1 and 8000),
  at timestamptz not null default now()
);
create index dossier_message_dossier on kentos.dossier_message (dossier_id, at);

-- A file of a package (its round) or attached to a message. A pending one may still be removed; a sent one is kept as
-- it was sent: its bytes are checked against `sha256` when read.
create table kentos.dossier_file (
  id uuid primary key,
  dossier_id uuid not null references kentos.dossier (id) on delete cascade,
  side text not null check (side in ('municipality', 'professional')),
  purpose text not null check (purpose in ('package', 'attachment')),
  round integer check (round is null or round > 0),
  message_id uuid references kentos.dossier_message (id) on delete cascade,
  state text not null default 'pending' check (state in ('pending', 'final')),
  doc_type text not null default 'other' check (doc_type ~ '^[a-z_]{2,40}$'),
  name text not null check (length(name) between 1 and 200),
  media_type text not null check (length(media_type) between 3 and 120),
  size bigint not null check (size >= 0),
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  blob_key text not null,
  -- Taken from a cloud project: its organisation, id, name and revision.
  source jsonb,
  added_by uuid not null references kentos.app_user (id),
  added_by_name text not null,
  added_at timestamptz not null default now(),
  check ((purpose = 'package') = (round is not null)),
  check (purpose = 'package' or state = 'pending' or message_id is not null)
);
create index dossier_file_dossier on kentos.dossier_file (dossier_id, round, added_at);

create table kentos.dossier_event (
  id bigint generated always as identity primary key,
  dossier_id uuid not null references kentos.dossier (id) on delete cascade,
  at timestamptz not null default now(),
  actor_id uuid references kentos.app_user (id) on delete set null,
  actor_name text not null,
  side text not null check (side in ('municipality', 'professional')),
  action text not null,
  round integer not null default 0,
  detail jsonb not null default '{}'::jsonb
);
create index dossier_event_dossier on kentos.dossier_event (dossier_id, id);

-- ── Notifications ────────────────────────────────────────────────────────

create table kentos.notification (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  at timestamptz not null default now(),
  kind text not null,
  dossier_id uuid references kentos.dossier (id) on delete cascade,
  -- The organisation the recipient opens it in (a member's); none for a professional's own.
  tenant_id uuid references kentos.tenant (id) on delete cascade,
  title text not null,
  body text not null default '',
  read_at timestamptz
);
create index notification_user on kentos.notification (user_id, at desc);

-- ── Who sees what ────────────────────────────────────────────────────────

-- Whether the caller sees this professional (profile and account name): themselves; an administrator; a member of a
-- municipality, when the professional is verified and listed; a member of an organisation they have a dossier with.
create function kentos.professional_visible(p_user uuid) returns boolean
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select p_user = kentos.current_user_id()
      or kentos.is_platform_admin()
      or (exists (select 1 from kentos.professional p
                   where p.user_id = p_user and p.status = 'verified' and p.listed)
          and exists (select 1 from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
                       where m.user_id = kentos.current_user_id() and m.status = 'active'
                         and t.status = 'active' and t.category = 'municipality'))
      or exists (select 1 from kentos.dossier d
                  where d.professional_id = p_user and kentos.member_role(d.tenant_id) is not null
                    and not (d.direction = 'application' and d.status = 'draft'))
$$;

-- The caller's side in a dossier, or null: the professional's, or the organisation's for a member who may use it.
create function kentos.dossier_side(p_dossier uuid) returns text
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select case when d.professional_id = kentos.current_user_id()
                   and not (d.direction = 'transfer' and d.status = 'draft') then 'professional'
              when kentos.member_role(d.tenant_id) is not null
                   and not (d.direction = 'application' and d.status = 'draft') then 'municipality' end
    from kentos.dossier d where d.id = p_dossier
$$;

alter table kentos.platform_setting enable row level security;
create policy platform_setting_read on kentos.platform_setting for select using (true);

alter table kentos.platform_event enable row level security;
create policy platform_event_read on kentos.platform_event for select using (kentos.is_platform_admin());

alter table kentos.platform_admin enable row level security;

create policy tenant_platform on kentos.tenant for select using (kentos.is_platform_admin());
-- A municipality's name and place are public: professionals pick the one they apply to.
create policy tenant_municipality on kentos.tenant for select
  using (kind = 'organization' and category = 'municipality' and status = 'active'
         and kentos.current_user_id() is not null);
create policy membership_platform on kentos.membership for select using (kentos.is_platform_admin());
create policy seat_platform on kentos.seat_allocation for select using (kentos.is_platform_admin());
create policy app_user_platform on kentos.app_user for select using (kentos.is_platform_admin());
create policy app_user_professional on kentos.app_user for select using (kentos.professional_visible(id));

alter table kentos.tenant_portal enable row level security;
create policy tenant_portal_read on kentos.tenant_portal for select using (kentos.current_user_id() is not null);
create policy tenant_portal_insert on kentos.tenant_portal for insert
  with check (kentos.role_rank(kentos.member_role(tenant_id)) >= 4 or kentos.is_platform_admin());
create policy tenant_portal_update on kentos.tenant_portal for update
  using (kentos.role_rank(kentos.member_role(tenant_id)) >= 4 or kentos.is_platform_admin())
  with check (kentos.role_rank(kentos.member_role(tenant_id)) >= 4 or kentos.is_platform_admin());

alter table kentos.professional enable row level security;
create policy professional_read on kentos.professional for select using (kentos.professional_visible(user_id));
create policy professional_own on kentos.professional for update
  using (user_id = kentos.current_user_id()) with check (user_id = kentos.current_user_id());

alter table kentos.dossier enable row level security;
-- Each side sees its dossiers; a draft only its own side (see dossier_side).
create policy dossier_read on kentos.dossier for select using (kentos.dossier_side(id) is not null);
-- A professional opens their own application; a municipality's project manager opens a transfer. Both as drafts.
create policy dossier_open_application on kentos.dossier for insert
  with check (direction = 'application' and status = 'draft' and professional_id = kentos.current_user_id()
              and created_by = kentos.current_user_id());
create policy dossier_open_transfer on kentos.dossier for insert
  with check (direction = 'transfer' and status = 'draft' and created_by = kentos.current_user_id()
              and kentos.role_rank(kentos.member_role(tenant_id)) >= 3);
create policy dossier_change on kentos.dossier for update
  using (kentos.dossier_side(id) is not null)
  with check (professional_id = kentos.current_user_id() or kentos.member_role(tenant_id) is not null);
create policy dossier_drop_draft on kentos.dossier for delete
  using (status = 'draft'
         and ((direction = 'application' and professional_id = kentos.current_user_id())
              or (direction = 'transfer' and kentos.role_rank(kentos.member_role(tenant_id)) >= 3)));
alter table kentos.dossier_message enable row level security;
create policy dossier_message_read on kentos.dossier_message for select
  using (exists (select 1 from kentos.dossier d where d.id = dossier_id));
create policy dossier_message_write on kentos.dossier_message for insert
  with check (author_id = kentos.current_user_id() and exists (select 1 from kentos.dossier d where d.id = dossier_id));

-- A sent file is seen by both sides; one not sent yet only by the side preparing it.
alter table kentos.dossier_file enable row level security;
create policy dossier_file_read on kentos.dossier_file for select
  using (case state when 'final' then exists (select 1 from kentos.dossier d where d.id = dossier_id)
                    else kentos.dossier_side(dossier_id) = side end);
create policy dossier_file_add on kentos.dossier_file for insert
  with check (added_by = kentos.current_user_id() and state = 'pending' and kentos.dossier_side(dossier_id) = side);
create policy dossier_file_change on kentos.dossier_file for update
  using (state = 'pending' and kentos.dossier_side(dossier_id) = side)
  with check (kentos.dossier_side(dossier_id) = side);
create policy dossier_file_drop on kentos.dossier_file for delete
  using (state = 'pending' and kentos.dossier_side(dossier_id) = side);

alter table kentos.dossier_event enable row level security;
create policy dossier_event_read on kentos.dossier_event for select
  using (exists (select 1 from kentos.dossier d where d.id = dossier_id));
create policy dossier_event_write on kentos.dossier_event for insert
  with check (actor_id = kentos.current_user_id() and exists (select 1 from kentos.dossier d where d.id = dossier_id));

alter table kentos.notification enable row level security;
create policy notification_own on kentos.notification for select using (user_id = kentos.current_user_id());
create policy notification_read on kentos.notification for update
  using (user_id = kentos.current_user_id()) with check (user_id = kentos.current_user_id());

-- ── Notifying ────────────────────────────────────────────────────────────

-- Notifies the other side of a dossier the caller is a party to: the professional, or the organisation (its assignee
-- while they may use it, else every member who may work on dossiers: editor and above). Never the caller. Returns how
-- many were notified.
create function kentos.notify_dossier(p_dossier uuid, p_audience text, p_kind text, p_title text, p_body text)
  returns integer
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  d record;
  n integer := 0;
begin
  if kentos.dossier_side(p_dossier) is null then
    raise exception 'notify_dossier: the caller is not a party to the dossier';
  end if;
  select * into d from kentos.dossier where id = p_dossier;
  if p_audience = 'professional' then
    if d.professional_id is distinct from kentos.current_user_id() then
      insert into kentos.notification (user_id, kind, dossier_id, title, body)
      values (d.professional_id, p_kind, p_dossier, p_title, coalesce(p_body, ''));
      n := 1;
    end if;
  elsif p_audience = 'municipality' then
    with usable as (
      select m.user_id from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
       where m.tenant_id = d.tenant_id and m.status = 'active' and t.status = 'active'
         and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
         and kentos.role_rank(m.role) >= 2
    ), audience as (
      select user_id from usable where user_id = d.assignee_id
      union
      select user_id from usable where not exists (select 1 from usable u where u.user_id = d.assignee_id)
    )
    insert into kentos.notification (user_id, kind, dossier_id, tenant_id, title, body)
    select a.user_id, p_kind, p_dossier, d.tenant_id, p_title, coalesce(p_body, '')
      from audience a where a.user_id is distinct from kentos.current_user_id();
    get diagnostics n = row_count;
  else
    raise exception 'notify_dossier: unknown audience %', p_audience;
  end if;
  return n;
end $$;

-- The dossier's number, given the first time it is sent: the next of its organisation's year, one at a time.
create function kentos.dossier_assign_number(p_dossier uuid) returns integer
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  d record;
  y integer := extract(year from now())::integer;
  given integer;
begin
  select * into d from kentos.dossier where id = p_dossier for update;
  if not found then
    raise exception 'dossier_assign_number: no dossier %', p_dossier;
  end if;
  if not ((d.direction = 'application' and d.professional_id = kentos.current_user_id())
          or (d.direction = 'transfer' and kentos.role_rank(kentos.member_role(d.tenant_id)) >= 3)) then
    raise exception 'dossier_assign_number: the caller does not send this dossier';
  end if;
  if d.number is not null then
    return d.number;
  end if;
  perform pg_advisory_xact_lock(hashtextextended('kentos.dossier_number:' || d.tenant_id::text || ':' || y, 0));
  select coalesce(max(number), 0) + 1 into given from kentos.dossier where tenant_id = d.tenant_id and year = y;
  update kentos.dossier set year = y, number = given where id = p_dossier;
  return given;
end $$;

-- ── Administering the platform ───────────────────────────────────────────

create function kentos.platform_create_account(p_id uuid, p_login text, p_password text, p_name text, p_email text)
  returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  code text;
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  code := kentos.create_local_account(p_id, p_login, p_password, p_name, p_email, false);
  if code is null then
    perform kentos.platform_log('user.create', 'user', p_id, p_name, jsonb_build_object('login', p_login));
  end if;
  return code;
end $$;

create function kentos.platform_create_tenant(p_id uuid, p_slug text, p_name text, p_category text,
                                              p_province text, p_district text, p_seats integer, p_owner uuid)
  returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  if exists (select 1 from kentos.tenant where slug = p_slug) then
    return 'slug_taken';
  end if;
  if p_owner is not null and not exists (select 1 from kentos.app_user where id = p_owner and status = 'active') then
    return 'not_found';
  end if;
  insert into kentos.tenant (id, slug, name, seat_limit, category, province, district)
  values (p_id, p_slug, p_name, p_seats, p_category, nullif(p_province, ''), nullif(p_district, ''));
  insert into kentos.tenant_portal (tenant_id, accepts_applications, updated_by)
  values (p_id, p_category = 'municipality', kentos.current_user_id());
  if p_owner is not null then
    insert into kentos.membership (tenant_id, user_id, role, status) values (p_id, p_owner, 'owner', 'active');
    if p_seats > 0 then
      insert into kentos.seat_allocation (tenant_id, user_id) values (p_id, p_owner);
    end if;
  end if;
  perform kentos.platform_log('tenant.create', 'tenant', p_id, p_name,
    jsonb_build_object('slug', p_slug, 'category', p_category, 'seats', p_seats, 'owner', p_owner));
  return null;
end $$;

create function kentos.platform_update_tenant(p_tenant uuid, p_name text, p_category text, p_province text,
                                              p_district text, p_seats integer, p_status text)
  returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  t record;
  used integer;
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  select * into t from kentos.tenant where id = p_tenant and kind = 'organization' for update;
  if not found then
    return 'not_found';
  end if;
  select count(*) into used from kentos.seat_allocation where tenant_id = p_tenant;
  if p_seats < used then
    return 'seats_below_used';
  end if;
  update kentos.tenant
     set name = p_name, category = p_category, province = nullif(p_province, ''), district = nullif(p_district, ''),
         seat_limit = p_seats, status = p_status
   where id = p_tenant;
  perform kentos.platform_log('tenant.update', 'tenant', p_tenant, p_name, jsonb_strip_nulls(jsonb_build_object(
    'name', case when t.name <> p_name then p_name end,
    'category', case when t.category <> p_category then p_category end,
    'seats', case when t.seat_limit <> p_seats then p_seats end,
    'status', case when t.status <> p_status then p_status end)));
  return null;
end $$;

create function kentos.platform_set_user_status(p_user uuid, p_status text) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  u record;
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  if p_user = kentos.current_user_id() then
    return 'self';
  end if;
  select * into u from kentos.app_user where id = p_user for update;
  if not found then
    return 'not_found';
  end if;
  update kentos.app_user set status = p_status where id = p_user;
  if p_status = 'disabled' then
    update kentos.auth_session set revoked_at = now() where user_id = p_user and revoked_at is null;
  end if;
  perform kentos.platform_log(case p_status when 'disabled' then 'user.disable' else 'user.enable' end,
    'user', p_user, u.display_name, '{}'::jsonb);
  return null;
end $$;

create function kentos.platform_reset_password(p_user uuid, p_password text) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  name text;
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  update kentos.local_credential set password_hash = public.crypt(p_password, public.gen_salt('bf', 12)),
         updated_at = now()
   where user_id = p_user;
  if not found then
    return 'not_local';
  end if;
  update kentos.auth_session set revoked_at = now() where user_id = p_user and revoked_at is null;
  select display_name into name from kentos.app_user where id = p_user;
  perform kentos.platform_log('user.password', 'user', p_user, name, '{}'::jsonb);
  return null;
end $$;

create function kentos.platform_review_professional(p_user uuid, p_status text, p_note text) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  name text;
  before text;
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  select p.status, u.display_name into before, name
    from kentos.professional p join kentos.app_user u on u.id = p.user_id where p.user_id = p_user for update of p;
  if not found then
    return 'not_found';
  end if;
  update kentos.professional
     set status = p_status, status_note = coalesce(p_note, ''), reviewed_by = kentos.current_user_id(),
         reviewed_at = now()
   where user_id = p_user;
  -- Verifying the person attests their account, e-mail included (see local_email_verified).
  if p_status = 'verified' then
    update kentos.app_user set signup = false where id = p_user;
  end if;
  insert into kentos.notification (user_id, kind, title, body)
  values (p_user, 'professional.' || p_status,
          case p_status when 'verified' then 'Meslek mensubu kaydınız doğrulandı'
                        when 'rejected' then 'Meslek mensubu kaydınız onaylanmadı'
                        when 'suspended' then 'Meslek mensubu kaydınız askıya alındı'
                        else 'Meslek mensubu kaydınız yeniden incelemede' end,
          coalesce(p_note, ''));
  perform kentos.platform_log('professional.' || p_status, 'professional', p_user, name,
    jsonb_build_object('before', before, 'note', coalesce(p_note, '')));
  return null;
end $$;

-- Opens or closes signing up, recorded.
create function kentos.platform_set_signup(p_open boolean) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  update kentos.platform_setting set signup_open = p_open, updated_at = now(), updated_by = kentos.current_user_id();
  perform kentos.platform_log('setting.signup', 'setting', null, 'Meslek mensubu kaydı',
    jsonb_build_object('open', p_open));
  return null;
end $$;

-- Accounts as an administrator lists them, with their sign-in name and what they are on the platform.
create function kentos.platform_users(p_query text, p_limit integer)
  returns table (id uuid, login text, display_name text, email text, status text, created_at timestamptz,
                 signup boolean, platform_admin boolean, profession text, professional_status text,
                 organisations bigint)
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  if not kentos.is_platform_admin() then
    return;
  end if;
  return query
  select u.id, c.login, u.display_name, u.email, u.status, u.created_at, u.signup,
         exists (select 1 from kentos.platform_admin a where a.user_id = u.id),
         p.profession, p.status,
         (select count(*) from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
           where m.user_id = u.id and t.kind = 'organization')
    from kentos.app_user u
    left join kentos.local_credential c on c.user_id = u.id
    left join kentos.professional p on p.user_id = u.id
   where coalesce(p_query, '') = ''
      or u.display_name ilike '%' || p_query || '%' or u.email ilike '%' || p_query || '%'
      or c.login ilike '%' || p_query || '%'
   order by u.created_at desc
   limit greatest(1, least(p_limit, 500));
end $$;

-- Organisations with what they use: seats, members, projects and dossiers.
create function kentos.platform_tenants()
  returns table (id uuid, slug text, name text, category text, province text, district text, status text,
                 seat_limit integer, seats_used bigint, members bigint, projects bigint, dossiers bigint,
                 accepts_applications boolean, created_at timestamptz)
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  if not kentos.is_platform_admin() then
    return;
  end if;
  return query
  select t.id, t.slug, t.name, t.category, t.province, t.district, t.status, t.seat_limit,
         (select count(*) from kentos.seat_allocation s where s.tenant_id = t.id),
         (select count(*) from kentos.membership m where m.tenant_id = t.id),
         (select count(*) from kentos.project p where p.tenant_id = t.id and p.deleted_at is null),
         (select count(*) from kentos.dossier d where d.tenant_id = t.id and d.status <> 'draft'),
         coalesce(tp.accepts_applications, false), t.created_at
    from kentos.tenant t left join kentos.tenant_portal tp on tp.tenant_id = t.id
   where t.kind = 'organization'
   order by t.name;
end $$;

-- The platform at a glance, for its administrators.
create function kentos.platform_summary() returns jsonb
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  if not kentos.is_platform_admin() then
    return null;
  end if;
  return jsonb_build_object(
    'municipalities', (select count(*) from kentos.tenant where kind = 'organization' and category = 'municipality'),
    'organisations', (select count(*) from kentos.tenant where kind = 'organization'),
    'suspendedOrganisations', (select count(*) from kentos.tenant where kind = 'organization' and status = 'suspended'),
    'accounts', (select count(*) from kentos.app_user),
    'disabledAccounts', (select count(*) from kentos.app_user where status = 'disabled'),
    'professionals', (select count(*) from kentos.professional where status = 'verified'),
    'pendingProfessionals', (select count(*) from kentos.professional where status = 'pending'),
    'seatsUsed', (select count(*) from kentos.seat_allocation s join kentos.tenant t on t.id = s.tenant_id
                   where t.kind = 'organization'),
    'seatLimit', (select coalesce(sum(seat_limit), 0) from kentos.tenant where kind = 'organization'),
    'openDossiers', (select count(*) from kentos.dossier where status not in ('draft') and closed_at is null),
    'dossiersThisMonth', (select count(*) from kentos.dossier
                           where sent_at >= date_trunc('month', now())),
    'projects', (select count(*) from kentos.project where deleted_at is null),
    'activeSessions', (select count(*) from kentos.auth_session where revoked_at is null and expires_at > now()));
end $$;

-- ── Signing up ───────────────────────────────────────────────────────────

-- A professional's own account and profile, while the platform takes sign-ups (or opened by an administrator).
create function kentos.register_professional(p_id uuid, p_login text, p_password text, p_name text, p_email text,
                                             p_profession text, p_chamber_no text, p_office text, p_phone text,
                                             p_province text)
  returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  admin boolean := kentos.is_platform_admin();
  code text;
begin
  if not admin and not (select signup_open from kentos.platform_setting) then
    return 'signup_closed';
  end if;
  code := kentos.create_local_account(p_id, p_login, p_password, p_name, p_email, not admin);
  if code is not null then
    return code;
  end if;
  insert into kentos.professional (user_id, profession, chamber_no, office_name, phone, province)
  values (p_id, p_profession, p_chamber_no, nullif(p_office, ''), nullif(p_phone, ''), nullif(p_province, ''));
  perform kentos.platform_log('professional.register', 'professional', p_id, p_name,
    jsonb_build_object('profession', p_profession, 'chamberNo', p_chamber_no, 'byAdmin', admin));
  insert into kentos.notification (user_id, kind, title, body)
  select a.user_id, 'professional.registered', 'Yeni meslek mensubu kaydı',
         p_name || ' doğrulanmayı bekliyor.'
    from kentos.platform_admin a where a.user_id is distinct from kentos.current_user_id();
  return null;
end $$;

-- The caller's own sign-in name (the server's role reads no credentials).
create function kentos.own_login() returns text
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$ select login from kentos.local_credential where user_id = kentos.current_user_id() $$;

create function kentos.change_own_password(p_old text, p_new text) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  hash text;
begin
  select password_hash into hash from kentos.local_credential where user_id = kentos.current_user_id() for update;
  if not found then
    return 'not_local';
  end if;
  if public.crypt(p_old, hash) <> hash then
    return 'wrong_password';
  end if;
  update kentos.local_credential set password_hash = public.crypt(p_new, public.gen_salt('bf', 12)), updated_at = now()
   where user_id = kentos.current_user_id();
  update kentos.auth_session set revoked_at = now() where user_id = kentos.current_user_id() and revoked_at is null;
  return null;
end $$;

-- ── An organisation's members, by its owners and admins ──────────────────

-- A new account as a member with a seat. An admin gives roles up to admin; owners (and platform administrators) any.
create function kentos.tenant_create_member(p_tenant uuid, p_id uuid, p_login text, p_password text, p_name text,
                                            p_email text, p_role text)
  returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  caller integer := kentos.role_rank(kentos.member_role(p_tenant));
  t record;
  code text;
begin
  if caller < 4 and not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  if kentos.role_rank(p_role) = 0 then
    return 'invalid';
  end if;
  if kentos.role_rank(p_role) = 5 and caller < 5 and not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  select * into t from kentos.tenant where id = p_tenant and kind = 'organization' for update;
  if not found then
    return 'not_found';
  end if;
  if (select count(*) from kentos.seat_allocation where tenant_id = p_tenant) >= t.seat_limit then
    return 'no_seat';
  end if;
  code := kentos.create_local_account(p_id, p_login, p_password, p_name, p_email, false);
  if code is not null then
    return code;
  end if;
  insert into kentos.membership (tenant_id, user_id, role, status) values (p_tenant, p_id, p_role, 'active');
  insert into kentos.seat_allocation (tenant_id, user_id) values (p_tenant, p_id);
  insert into kentos.audit_event (tenant_id, actor, action, detail)
  values (p_tenant, kentos.current_user_id(), 'member.add',
          jsonb_build_object('user', p_id, 'name', p_name, 'login', p_login, 'role', p_role));
  return null;
end $$;

-- A member's role, status and seat. The last active owner stays one; only owners change owners.
create function kentos.tenant_update_member(p_tenant uuid, p_user uuid, p_role text, p_status text, p_seat boolean)
  returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  caller integer := kentos.role_rank(kentos.member_role(p_tenant));
  admin boolean := kentos.is_platform_admin();
  m record;
  t record;
  seated boolean;
begin
  if caller < 4 and not admin then
    return 'forbidden';
  end if;
  if kentos.role_rank(p_role) = 0 or p_status not in ('active', 'disabled') then
    return 'invalid';
  end if;
  select * into t from kentos.tenant where id = p_tenant and kind = 'organization' for update;
  if not found then
    return 'not_found';
  end if;
  select * into m from kentos.membership where tenant_id = p_tenant and user_id = p_user for update;
  if not found then
    return 'not_found';
  end if;
  if (m.role = 'owner' or p_role = 'owner') and caller < 5 and not admin then
    return 'forbidden';
  end if;
  if m.role = 'owner' and m.status = 'active' and (p_role <> 'owner' or p_status <> 'active')
     and not exists (select 1 from kentos.membership o where o.tenant_id = p_tenant and o.user_id <> p_user
                        and o.role = 'owner' and o.status = 'active') then
    return 'last_owner';
  end if;
  seated := exists (select 1 from kentos.seat_allocation where tenant_id = p_tenant and user_id = p_user);
  if p_seat and not seated
     and (select count(*) from kentos.seat_allocation where tenant_id = p_tenant) >= t.seat_limit then
    return 'no_seat';
  end if;
  update kentos.membership set role = p_role, status = p_status where tenant_id = p_tenant and user_id = p_user;
  if p_seat and not seated then
    insert into kentos.seat_allocation (tenant_id, user_id) values (p_tenant, p_user);
  elsif not p_seat and seated then
    delete from kentos.seat_allocation where tenant_id = p_tenant and user_id = p_user;
  end if;
  insert into kentos.audit_event (tenant_id, actor, action, detail)
  values (p_tenant, kentos.current_user_id(), 'member.update',
          jsonb_build_object('user', p_user, 'role', p_role, 'status', p_status, 'seat', p_seat,
                             'before', jsonb_build_object('role', m.role, 'status', m.status, 'seat', seated)));
  return null;
end $$;

-- ── What the server's role may do ────────────────────────────────────────

revoke all on function kentos.platform_log(text, text, uuid, text, jsonb) from public;
revoke all on function kentos.create_local_account(uuid, text, text, text, text, boolean) from public;
revoke all on function kentos.is_platform_admin() from public;
revoke all on function kentos.member_role(uuid) from public;
revoke all on function kentos.professional_visible(uuid) from public;
revoke all on function kentos.dossier_side(uuid) from public;
revoke all on function kentos.notify_dossier(uuid, text, text, text, text) from public;
revoke all on function kentos.dossier_assign_number(uuid) from public;
revoke all on function kentos.platform_create_account(uuid, text, text, text, text) from public;
revoke all on function kentos.platform_create_tenant(uuid, text, text, text, text, text, integer, uuid) from public;
revoke all on function kentos.platform_update_tenant(uuid, text, text, text, text, integer, text) from public;
revoke all on function kentos.platform_set_user_status(uuid, text) from public;
revoke all on function kentos.platform_reset_password(uuid, text) from public;
revoke all on function kentos.platform_review_professional(uuid, text, text) from public;
revoke all on function kentos.platform_users(text, integer) from public;
revoke all on function kentos.platform_set_signup(boolean) from public;
revoke all on function kentos.platform_tenants() from public;
revoke all on function kentos.platform_summary() from public;
revoke all on function kentos.register_professional(uuid, text, text, text, text, text, text, text, text, text)
  from public;
revoke all on function kentos.change_own_password(text, text) from public;
revoke all on function kentos.own_login() from public;
revoke all on function kentos.tenant_create_member(uuid, uuid, text, text, text, text, text) from public;
revoke all on function kentos.tenant_update_member(uuid, uuid, text, text, boolean) from public;

grant execute on function kentos.is_platform_admin(), kentos.member_role(uuid), kentos.role_rank(text),
  kentos.professional_visible(uuid), kentos.dossier_side(uuid), kentos.notify_dossier(uuid, text, text, text, text),
  kentos.dossier_assign_number(uuid), kentos.platform_create_account(uuid, text, text, text, text),
  kentos.platform_create_tenant(uuid, text, text, text, text, text, integer, uuid),
  kentos.platform_update_tenant(uuid, text, text, text, text, integer, text),
  kentos.platform_set_user_status(uuid, text), kentos.platform_reset_password(uuid, text),
  kentos.platform_review_professional(uuid, text, text), kentos.platform_users(text, integer),
  kentos.platform_set_signup(boolean),
  kentos.platform_tenants(), kentos.platform_summary(),
  kentos.register_professional(uuid, text, text, text, text, text, text, text, text, text),
  kentos.change_own_password(text, text), kentos.own_login(),
  kentos.tenant_create_member(uuid, uuid, text, text, text, text, text),
  kentos.tenant_update_member(uuid, uuid, text, text, boolean)
  to kentos_cad_app;

grant select on kentos.platform_setting, kentos.platform_event, kentos.professional, kentos.tenant_portal
  to kentos_cad_app;
grant update (office_name, phone, province, listed) on kentos.professional to kentos_cad_app;
grant insert, update on kentos.tenant_portal to kentos_cad_app;
grant select, insert, update, delete on kentos.dossier, kentos.dossier_file to kentos_cad_app;
grant select, insert on kentos.dossier_message, kentos.dossier_event to kentos_cad_app;
grant select on kentos.notification to kentos_cad_app;
grant update (read_at) on kentos.notification to kentos_cad_app;
