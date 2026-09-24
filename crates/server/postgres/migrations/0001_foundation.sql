-- KentOS CAD, Faz B foundation (docs/adr/0006-data-layer.md, 0007-authentication.md).
--
-- Runs as the owner role (kentos_cad_owner). The server connects as
-- kentos_cad_app: not the owner, no BYPASSRLS, so every tenant-bound table
-- below is filtered by row-level security. The server sets the scope at the
-- start of each transaction with set_config('app.tenant_id' / 'app.user_id',
-- …, true); outside such a transaction nothing tenant-bound is visible.
-- Extensions (postgis, pgcrypto) are installed by `kentosd db-setup`.

create schema kentos;
revoke all on schema kentos from public;
grant usage on schema kentos to kentos_cad_app;

create function kentos.current_tenant() returns uuid
  language sql stable
  as $$ select nullif(current_setting('app.tenant_id', true), '')::uuid $$;

create function kentos.current_user_id() returns uuid
  language sql stable
  as $$ select nullif(current_setting('app.user_id', true), '')::uuid $$;

-- ── Accounts: global, one person may belong to several tenants ───────────

-- Identity is (issuer, subject), never the e-mail address (CLAUDE.md §16).
-- Local accounts use issuer 'kentos:local' and their own id as subject.
create table kentos.app_user (
  id uuid primary key,
  issuer text not null,
  subject text not null,
  display_name text not null,
  email text,
  status text not null default 'active' check (status in ('active', 'disabled')),
  created_at timestamptz not null default now(),
  unique (issuer, subject)
);

-- Password hashes are bcrypt from pgcrypto's crypt() (ADR 0007).
create table kentos.local_credential (
  user_id uuid primary key references kentos.app_user (id) on delete cascade,
  login text not null,
  password_hash text not null,
  updated_at timestamptz not null default now()
);
create unique index local_credential_login on kentos.local_credential (lower(login));

-- Only the SHA-256 of a session token is stored; the token lives in the browser's HttpOnly cookie.
create table kentos.auth_session (
  token_hash bytea primary key,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  method text not null check (method in ('local', 'oidc')),
  created_at timestamptz not null default now(),
  expires_at timestamptz not null,
  last_seen_at timestamptz not null default now(),
  revoked_at timestamptz
);
create index auth_session_user on kentos.auth_session (user_id);

-- An OpenID login in progress: state, nonce and the PKCE verifier, for ten minutes.
create table kentos.oidc_login (
  state text primary key,
  nonce text not null,
  code_verifier text not null,
  return_to text not null,
  expires_at timestamptz not null
);

-- ── Tenants, membership and seats ────────────────────────────────────────

create table kentos.tenant (
  id uuid primary key,
  slug text not null unique check (slug ~ '^[a-z0-9][a-z0-9-]{1,62}$'),
  name text not null,
  status text not null default 'active' check (status in ('active', 'suspended')),
  seat_limit integer not null check (seat_limit >= 0),
  created_at timestamptz not null default now()
);

create table kentos.membership (
  tenant_id uuid not null references kentos.tenant (id) on delete cascade,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  role text not null check (role in ('owner', 'admin', 'project_manager', 'editor', 'viewer')),
  status text not null default 'active' check (status in ('invited', 'active', 'disabled')),
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id)
);
create index membership_user on kentos.membership (user_id);

-- A membership is not a seat: the tenant admin allocates seats up to the tenant's limit.
create table kentos.seat_allocation (
  tenant_id uuid not null,
  user_id uuid not null,
  allocated_at timestamptz not null default now(),
  primary key (tenant_id, user_id),
  foreign key (tenant_id, user_id) references kentos.membership (tenant_id, user_id) on delete cascade
);

-- ── Projects and features ────────────────────────────────────────────────

-- The layer tree, settings and project styles are contract JSON (crates/shared/contracts),
-- versioned together by meta_version. data_revision counts every committed change.
create table kentos.project (
  tenant_id uuid not null references kentos.tenant (id) on delete cascade,
  id uuid not null,
  name text not null,
  srid integer not null,
  settings jsonb not null,
  layers jsonb not null,
  active_layer text not null,
  origin_x double precision not null,
  origin_y double precision not null,
  home_view jsonb,
  styles jsonb not null,
  meta_version bigint not null default 1,
  data_revision bigint not null default 0,
  created_by uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, id)
);

-- source_kind 'geom': the PostGIS geometry is the source (point, line, straight
-- polyline and polygon; written and read as binary EWKB, bit-exact).
-- source_kind 'cad': cad_definition is the source and geom its linear projection
-- (projection_version says how it was made); geom is null where no finite
-- geometry exists (construction lines and rays).
create table kentos.feature (
  tenant_id uuid not null,
  project_id uuid not null,
  id uuid not null,
  layer_id text not null,
  version bigint not null default 1 check (version > 0),
  kind text not null check (kind in ('point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline',
                                     'xline', 'ray', 'text', 'dimension', 'hatch')),
  source_kind text not null check (source_kind in ('geom', 'cad')),
  srid integer not null,
  geom geometry,
  cad_definition jsonb,
  properties jsonb not null default '{}'::jsonb,
  label text,
  color text,
  symbol text,
  projection_version integer not null,
  created_by uuid not null,
  updated_by uuid not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, project_id, id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade,
  check (geom is null or st_srid(geom) = srid),
  check ((source_kind = 'geom' and geom is not null and cad_definition is null)
      or (source_kind = 'cad' and cad_definition is not null))
);
create index feature_geom on kentos.feature using gist (geom);
create index feature_layer on kentos.feature (tenant_id, project_id, layer_id);

-- A retried command with the same key returns the stored answer instead of committing twice.
create table kentos.command_log (
  tenant_id uuid not null,
  idempotency_key text not null check (length(idempotency_key) between 8 and 200),
  project_id uuid not null,
  command_name text not null,
  request_hash bytea not null,
  response jsonb not null,
  actor uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, idempotency_key)
);

create table kentos.audit_event (
  id bigint generated always as identity primary key,
  tenant_id uuid not null,
  project_id uuid,
  actor uuid,
  action text not null,
  request_id text,
  data_revision bigint,
  detail jsonb not null default '{}'::jsonb,
  at timestamptz not null default now()
);
create index audit_event_scope on kentos.audit_event (tenant_id, project_id, id);

-- Written in the same transaction as the change, read by clients after it (event cursor = seq).
create table kentos.outbox_event (
  seq bigint generated always as identity primary key,
  tenant_id uuid not null,
  project_id uuid not null,
  data_revision bigint not null,
  kind text not null,
  payload jsonb not null,
  created_at timestamptz not null default now()
);
create index outbox_scope on kentos.outbox_event (tenant_id, project_id, seq);

-- ── Row-level security ───────────────────────────────────────────────────

alter table kentos.tenant enable row level security;
create policy tenant_visible on kentos.tenant for select
  using (id = kentos.current_tenant()
         or exists (select 1 from kentos.membership m where m.tenant_id = tenant.id and m.user_id = kentos.current_user_id()));

alter table kentos.membership enable row level security;
create policy membership_visible on kentos.membership for select
  using (tenant_id = kentos.current_tenant() or user_id = kentos.current_user_id());

alter table kentos.seat_allocation enable row level security;
create policy seat_visible on kentos.seat_allocation for select
  using (tenant_id = kentos.current_tenant() or user_id = kentos.current_user_id());

alter table kentos.app_user enable row level security;
create policy app_user_visible on kentos.app_user for select
  using (id = kentos.current_user_id()
         or exists (select 1 from kentos.membership m where m.user_id = app_user.id and m.tenant_id = kentos.current_tenant()));

alter table kentos.project enable row level security;
create policy project_tenant on kentos.project
  using (tenant_id = kentos.current_tenant()) with check (tenant_id = kentos.current_tenant());

alter table kentos.feature enable row level security;
create policy feature_tenant on kentos.feature
  using (tenant_id = kentos.current_tenant()) with check (tenant_id = kentos.current_tenant());

alter table kentos.command_log enable row level security;
create policy command_log_tenant on kentos.command_log
  using (tenant_id = kentos.current_tenant()) with check (tenant_id = kentos.current_tenant());

alter table kentos.audit_event enable row level security;
create policy audit_tenant on kentos.audit_event
  using (tenant_id = kentos.current_tenant()) with check (tenant_id = kentos.current_tenant());

alter table kentos.outbox_event enable row level security;
create policy outbox_tenant on kentos.outbox_event
  using (tenant_id = kentos.current_tenant()) with check (tenant_id = kentos.current_tenant());

-- ── Sign-in before any scope exists ──────────────────────────────────────
-- Checking a password and finding an OpenID account happen before the user
-- is known, so row-level security cannot scope them. These two functions are
-- the only way in: they run as the owner, touch nothing else, and the server
-- role cannot read password hashes at all (ADR 0007).

create function kentos.check_local_login(p_login text, p_password text) returns uuid
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  cred record;
begin
  select c.user_id, c.password_hash into cred
    from kentos.local_credential c join kentos.app_user u on u.id = c.user_id
   where lower(c.login) = lower(p_login) and u.status = 'active';
  if not found then
    -- The same bcrypt work as a real check: a missing login cannot be told apart by timing.
    perform public.crypt(p_password, public.gen_salt('bf', 12));
    return null;
  end if;
  if public.crypt(p_password, cred.password_hash) = cred.password_hash then
    return cred.user_id;
  end if;
  return null;
end $$;

-- Finds or creates the account of an OpenID identity (never a local one); a disabled account gives null.
create function kentos.resolve_identity(p_new_id uuid, p_issuer text, p_subject text, p_name text, p_email text)
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
  insert into kentos.app_user (id, issuer, subject, display_name, email)
  values (p_new_id, p_issuer, p_subject, p_name, p_email)
  on conflict (issuer, subject) do update set display_name = excluded.display_name, email = excluded.email
  returning id, status into uid, st;
  return case when st = 'active' then uid end;
end $$;

revoke all on function kentos.check_local_login(text, text) from public;
revoke all on function kentos.resolve_identity(uuid, text, text, text, text) from public;
grant execute on function kentos.check_local_login(text, text) to kentos_cad_app;
grant execute on function kentos.resolve_identity(uuid, text, text, text, text) to kentos_cad_app;

-- ── What the server's role may do ────────────────────────────────────────
-- Tenants, memberships, seats and local credentials are managed by the owner
-- role (kentosd admin commands); the server only reads them.

grant select on kentos.tenant, kentos.membership, kentos.seat_allocation, kentos.app_user to kentos_cad_app;
grant select, insert, update on kentos.auth_session to kentos_cad_app;
grant select, insert, delete on kentos.oidc_login to kentos_cad_app;
grant select, insert, update on kentos.project to kentos_cad_app;
grant select, insert, update, delete on kentos.feature to kentos_cad_app;
grant select, insert on kentos.command_log, kentos.audit_event, kentos.outbox_event to kentos_cad_app;
grant execute on function kentos.current_tenant(), kentos.current_user_id() to kentos_cad_app;
-- The server checks at start that every migration of its build is applied.
grant select on public._sqlx_migrations to kentos_cad_app;
