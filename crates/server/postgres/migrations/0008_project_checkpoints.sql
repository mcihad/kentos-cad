-- KentOS CAD: checkpoints, named points of a project's history
-- (docs/adr/0034; TODOS.md SYNC-11, CLOUD-07).
--
-- * A database project's checkpoint is its snapshot of one moment
--   (docs/adr/0033) kept in the object store; the row holds the object's
--   key, its size, SHA-256 and object count, and the data revision it shows.
-- * A file project's checkpoint names one of its revisions (docs/adr/0031):
--   the row repeats the revision's number, size, SHA-256 and object count;
--   nothing is copied, and revisions go only with their project.
-- * A snapshot's object is written before its row. One whose row was never
--   committed belongs to no checkpoint; the server's cleanup removes it
--   (kentos.existing_checkpoints).

create table kentos.project_checkpoint (
  tenant_id uuid not null,
  project_id uuid not null,
  id uuid not null,
  name text not null check (length(name) between 1 and 120),
  note text check (note is null or length(note) <= 2000),
  kind text not null check (kind in ('snapshot', 'revision')),
  -- The data revision a snapshot shows, or the file revision a file project's checkpoint names.
  revision bigint not null check (revision >= 0),
  size bigint not null check (size >= 0),
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  objects bigint check (objects is null or objects >= 0),
  -- A snapshot's own object; a named revision's object is the revision's.
  blob_key text,
  created_by uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  request_id text,
  primary key (tenant_id, project_id, id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade,
  check ((kind = 'snapshot') = (blob_key is not null))
);
create index project_checkpoint_created on kentos.project_checkpoint (tenant_id, project_id, created_at desc);

-- The project's own rows: seen and written only with the project in scope,
-- while the current user has a role in it (as its objects and revisions are).
alter table kentos.project_checkpoint enable row level security;
create policy checkpoint_project on kentos.project_checkpoint
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

-- Which of these checkpoints exist (for removing the objects of snapshots
-- whose row was never committed); the answer names only the ones that do.
create function kentos.existing_checkpoints(p_ids uuid[])
  returns table (id uuid)
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$ select c.id from kentos.project_checkpoint c where c.id = any (p_ids) $$;
revoke all on function kentos.existing_checkpoints(uuid[]) from public;

grant execute on function kentos.existing_checkpoints(uuid[]) to kentos_cad_app;
grant select, insert, delete on kentos.project_checkpoint to kentos_cad_app;
