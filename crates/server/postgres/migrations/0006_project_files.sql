-- KentOS CAD: projects kept as files (docs/adr/0031; TODOS.md SYNC-02..06, SYNC-16).
--
-- * A project is kept either object by object in PostGIS ('database', every
--   project until now) or as a file ('file'): a sequence of immutable,
--   verified `.kcad` v2 revisions (ADR 0025). The mode is chosen when the
--   project is made and does not change silently (TODOS.md §10.1).
-- * The bytes are in the server's object store (ADR 0012), not here: a row
--   holds the object's key, size and SHA-256.
-- * Saving a revision is two steps. An upload (kentos.project_upload) takes
--   the bytes to a temporary object and verifies them (size, hash, a
--   readable KCAD v2); committing it (the product command
--   `project.file.commit`) checks the access again, compares the expected
--   revision, moves the object to its final key and adds the revision. An
--   upload nobody commits is removed after a day.
-- * project.file_revision is the newest committed revision (null before the
--   first); a commit based on an older one is a conflict.

alter table kentos.project
  add column storage text not null default 'database' check (storage in ('database', 'file')),
  add column file_revision bigint check (file_revision is null or file_revision > 0),
  add constraint project_file_revision_storage check (file_revision is null or storage = 'file');

-- The mode is the project's for good: moving between them is an explicit
-- command of its own later ("PostGIS'e aktar", "dosyaya göm"), never an update.
create function kentos.keep_storage() returns trigger
  language plpgsql set search_path = pg_catalog, pg_temp
  as $$
begin
  if new.storage is distinct from old.storage then
    raise exception 'a project''s storage mode does not change (% → %)', old.storage, new.storage;
  end if;
  return new;
end $$;
create trigger project_keep_storage before update of storage on kentos.project
  for each row execute function kentos.keep_storage();

create table kentos.project_upload (
  tenant_id uuid not null,
  project_id uuid not null,
  id uuid not null,
  created_by uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  -- What the client said it would send; the bytes must match both.
  size bigint not null check (size > 0),
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  -- Set once the bytes arrived and were verified.
  received_at timestamptz,
  blob_key text not null,
  primary key (tenant_id, project_id, id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);
create index project_upload_created on kentos.project_upload (created_at);

create table kentos.project_file_revision (
  tenant_id uuid not null,
  project_id uuid not null,
  revision bigint not null check (revision > 0),
  size bigint not null check (size > 0),
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  blob_key text not null,
  created_by uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  -- The command that committed it (its idempotency key), for the audit and the log.
  request_id text,
  primary key (tenant_id, project_id, revision),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);

-- Both are the project's own rows: seen and written only with the project in
-- scope, while the current user has a role in it (as the objects are).
alter table kentos.project_upload enable row level security;
create policy upload_project on kentos.project_upload
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

alter table kentos.project_file_revision enable row level security;
create policy file_revision_project on kentos.project_file_revision
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

-- The server's cleanup: removes, of every tenant, at most p_batch uploads
-- made before p_before that were never committed, and returns their object
-- keys for the server to remove from the store. The app role cannot see
-- other projects' uploads, so this runs with the owner's rights; it removes
-- nothing else.
create function kentos.expire_uploads(p_before timestamptz, p_batch integer)
  returns table (blob_key text)
  language sql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
    delete from kentos.project_upload u
     where (u.tenant_id, u.project_id, u.id) in (
       select q.tenant_id, q.project_id, q.id from kentos.project_upload q
        where q.created_at < p_before
        order by q.created_at
        limit greatest(p_batch, 1)
        for update skip locked)
    returning u.blob_key;
  $$;
revoke all on function kentos.expire_uploads(timestamptz, integer) from public;

-- Which of these projects still exist (for removing the objects of projects
-- removed for good); the answer names only the ones that do, of any tenant.
create function kentos.existing_projects(p_ids uuid[])
  returns table (id uuid)
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$ select q.id from kentos.project q where q.id = any (p_ids) $$;
revoke all on function kentos.existing_projects(uuid[]) from public;

grant execute on function kentos.expire_uploads(timestamptz, integer) to kentos_cad_app;
grant execute on function kentos.existing_projects(uuid[]) to kentos_cad_app;
grant select, insert, update, delete on kentos.project_upload to kentos_cad_app;
grant select, insert on kentos.project_file_revision to kentos_cad_app;
