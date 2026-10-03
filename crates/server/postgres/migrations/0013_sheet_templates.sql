-- KentOS CAD: the sheet template library in the cloud (docs/sheet/design.md §13).
--
-- A template is a person's: its owner's, kept in their personal space
-- (tenant_id; libraries of an organisation come in phase 2). Its content is
-- the template file (`kentos.sheet.template/1`, crates/shared/sheet), kept
-- once per revision and never changed; the template's row carries the
-- newest revision's metadata.
--
-- * Reading: the owner and the people it is shared with (viewer, editor).
--   Writing a new revision: the owner and editors. Sharing and deleting: the
--   owner. `kentos.sheet_template_role` is the one place that works out the
--   role, and row-level security follows it: a mistake in the server's code
--   opens nobody's template.
-- * Sharing goes only to people of an organisation the owner belongs to
--   (docs/adr/0024: no open search, no counting of accounts); the server
--   checks it, as it checks everything else, and records each change in the
--   audit.
-- * A retried command gets the answer stored the first time
--   (`sheet_template_command`, the person's own command log).
-- * What a change tells the owner's and the grantees' devices is an event
--   for each of them (`sheet_template_event`), read after a cursor like a
--   project's events; it names the template and the revision, never content
--   or people.
--
-- If `main` has taken this number by the time the branch is merged, the
-- file is renamed to the first free one: nothing else refers to it
-- (docs/sheet/integration.md §4).

create table kentos.sheet_template (
  id uuid primary key,
  tenant_id uuid not null references kentos.tenant (id) on delete cascade,
  owner_user_id uuid not null references kentos.app_user (id),
  name text not null check (length(name) between 1 and 120),
  description text not null default '',
  category text not null default '',
  tags text[] not null default '{}',
  -- "a3-landscape", the recommended paper first.
  papers text[] not null default '{}',
  workspaces text[] not null default '{}',
  project_types text[] not null default '{}',
  revision integer not null check (revision > 0),
  sha256 bytea not null check (length(sha256) = 32),
  size integer not null check (size between 1 and 8388608),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  -- Deleted by its owner: gone from every list; the revisions stay for the audit.
  deleted_at timestamptz
);
create index sheet_template_owner on kentos.sheet_template (owner_user_id) where deleted_at is null;

-- One row per revision, never changed.
create table kentos.sheet_template_revision (
  template_id uuid not null references kentos.sheet_template (id) on delete cascade,
  revision integer not null check (revision > 0),
  content bytea not null,
  sha256 bytea not null check (length(sha256) = 32),
  size integer not null check (size between 1 and 8388608),
  author_user_id uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  primary key (template_id, revision)
);

-- A person a template is shared with (ownership is not given in phase 1).
create table kentos.sheet_template_grant (
  template_id uuid not null references kentos.sheet_template (id) on delete cascade,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  role text not null check (role in ('viewer', 'editor')),
  granted_by uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (template_id, user_id)
);
create index sheet_template_grant_user on kentos.sheet_template_grant (user_id);

-- A person's template commands by idempotency key: a retry gets the stored answer.
create table kentos.sheet_template_command (
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  idempotency_key text not null check (length(idempotency_key) between 8 and 200),
  command_name text not null,
  request_hash bytea not null,
  response jsonb not null,
  created_at timestamptz not null default now(),
  primary key (user_id, idempotency_key)
);

-- What changed, for each person who should hear it: the owner, the
-- grantees, and the person a share was just taken from.
create table kentos.sheet_template_event (
  seq bigint generated always as identity primary key,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  template_id uuid not null,
  revision integer not null,
  kind text not null check (kind in ('created', 'updated', 'deleted', 'shared', 'unshared')),
  actor uuid,
  request_id text,
  created_at timestamptz not null default now()
);
create index sheet_template_event_user on kentos.sheet_template_event (user_id, seq);

-- ── The role ─────────────────────────────────────────────────────────────

-- The current user's role in a template: 'owner', the grant's role, or
-- null. It reads no template row (the owner is given), so the template's
-- own policy can call it; it answers nothing about anyone but the current user.
create function kentos.sheet_template_role(p_template uuid, p_owner uuid) returns text
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  granted text;
begin
  if me is null or p_template is null then
    return null;
  end if;
  if p_owner = me then
    return 'owner';
  end if;
  select g.role into granted from kentos.sheet_template_grant g where g.template_id = p_template and g.user_id = me;
  return granted;
end $$;

-- Who should hear of a change to a template: its owner and everyone it is
-- shared with. Asked by someone with a role in it (an editor does not see the
-- other grants: the list of people is the owner's).
create function kentos.sheet_template_audience(p_template uuid) returns setof uuid
  language plpgsql stable security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  t_owner uuid;
begin
  select owner_user_id into t_owner from kentos.sheet_template where id = p_template;
  if t_owner is null or kentos.sheet_template_role(p_template, t_owner) is null then
    return;
  end if;
  return query select t_owner union select g.user_id from kentos.sheet_template_grant g where g.template_id = p_template;
end $$;

-- An audit record of a template change, in the audit of the template's
-- space (no project). Only someone with a role in the template writes one,
-- and only as themselves.
create function kentos.sheet_template_audit(p_template uuid, p_action text, p_request text, p_revision integer, p_detail jsonb)
  returns void
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  me constant uuid := kentos.current_user_id();
  t_tenant uuid;
  t_owner uuid;
begin
  select tenant_id, owner_user_id into t_tenant, t_owner from kentos.sheet_template where id = p_template;
  if t_tenant is null or me is null or kentos.sheet_template_role(p_template, t_owner) is null then
    raise exception 'no role in sheet template %', p_template;
  end if;
  insert into kentos.audit_event (tenant_id, project_id, actor, action, request_id, data_revision, detail)
  values (t_tenant, null, me, p_action, p_request, p_revision, p_detail);
end $$;

-- ── Row-level security ───────────────────────────────────────────────────

alter table kentos.sheet_template enable row level security;
create policy sheet_template_visible on kentos.sheet_template for select
  using (kentos.sheet_template_role(id, owner_user_id) is not null);
-- A new template is its creator's, in their own personal space.
create policy sheet_template_insert on kentos.sheet_template for insert
  with check (owner_user_id = kentos.current_user_id()
              and tenant_id = kentos.current_tenant()
              and exists (select 1 from kentos.tenant t where t.id = tenant_id and t.owner_user_id = kentos.current_user_id()));
create policy sheet_template_update on kentos.sheet_template for update
  using (kentos.sheet_template_role(id, owner_user_id) in ('owner', 'editor'))
  with check (kentos.sheet_template_role(id, owner_user_id) in ('owner', 'editor'));

alter table kentos.sheet_template_revision enable row level security;
create policy sheet_template_revision_visible on kentos.sheet_template_revision for select
  using (exists (select 1 from kentos.sheet_template t where t.id = template_id));
create policy sheet_template_revision_insert on kentos.sheet_template_revision for insert
  with check (author_user_id = kentos.current_user_id()
              and exists (select 1 from kentos.sheet_template t
                           where t.id = template_id and kentos.sheet_template_role(t.id, t.owner_user_id) in ('owner', 'editor')));

alter table kentos.sheet_template_grant enable row level security;
-- One's own grants (the "shared with me" list starts from them).
create policy sheet_template_grant_own on kentos.sheet_template_grant for select
  using (user_id = kentos.current_user_id());
-- The grants of one's own templates; only the owner gives and takes them.
create policy sheet_template_grant_owner on kentos.sheet_template_grant
  using (exists (select 1 from kentos.sheet_template t where t.id = template_id and t.owner_user_id = kentos.current_user_id()))
  with check (granted_by = kentos.current_user_id()
              and exists (select 1 from kentos.sheet_template t where t.id = template_id and t.owner_user_id = kentos.current_user_id()));

alter table kentos.sheet_template_command enable row level security;
create policy sheet_template_command_own on kentos.sheet_template_command
  using (user_id = kentos.current_user_id())
  with check (user_id = kentos.current_user_id());

alter table kentos.sheet_template_event enable row level security;
create policy sheet_template_event_own on kentos.sheet_template_event for select
  using (user_id = kentos.current_user_id());
-- Written by whoever changed a template they have a role in, for the people who should hear it.
create policy sheet_template_event_write on kentos.sheet_template_event for insert
  with check (actor = kentos.current_user_id()
              and exists (select 1 from kentos.sheet_template t where t.id = template_id));

-- A template's owner and the people it is shared with see each other's names.
create policy app_user_sheet_template on kentos.app_user for select
  using (exists (select 1 from kentos.sheet_template_grant g join kentos.sheet_template t on t.id = g.template_id
                  where (t.owner_user_id = kentos.current_user_id() and g.user_id = app_user.id)
                     or (g.user_id = kentos.current_user_id() and t.owner_user_id = app_user.id)));

-- ── What the server's role may do ────────────────────────────────────────

revoke all on function kentos.sheet_template_role(uuid, uuid) from public;
revoke all on function kentos.sheet_template_audit(uuid, text, text, integer, jsonb) from public;
revoke all on function kentos.sheet_template_audience(uuid) from public;
grant execute on function kentos.sheet_template_audience(uuid) to kentos_cad_app;
grant execute on function kentos.sheet_template_role(uuid, uuid) to kentos_cad_app;
grant execute on function kentos.sheet_template_audit(uuid, text, text, integer, jsonb) to kentos_cad_app;
grant select, insert, update on kentos.sheet_template to kentos_cad_app;
grant select, insert on kentos.sheet_template_revision to kentos_cad_app;
grant select, insert, update, delete on kentos.sheet_template_grant to kentos_cad_app;
grant select, insert on kentos.sheet_template_command to kentos_cad_app;
grant select, insert on kentos.sheet_template_event to kentos_cad_app;
