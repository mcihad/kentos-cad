-- Files in the cloud for every client (docs/adr/9000 §13-§15): the desktop, a mobile app, a browser or an AI agent.
--
-- * An upload (kentos.upload) is the account's own until something takes it: a file goes in parts of at most 32 MiB,
--   a part cut short costs only itself, and the upload says how far it came. When the last part arrives its size and
--   SHA-256 must be the declared ones and the file is scanned; then a dossier or a project takes it by its id. An
--   upload nobody takes is removed after a day.
-- * The scan keeps out what must not reach the other side: programs and scripts under any name, a file whose bytes
--   are not what its name says, an archive with a program inside, and malware (an antivirus engine when the server has
--   one, the industry's test file always). A refused file is not kept; the platform's event log says who sent what.
--   What is kept says whether an engine scanned it ('clean') or only the type rules did ('unscanned').
-- * A project keeps attachments beside its drawing: reports, sheets, tables, photos and source files, readable by
--   whoever reads the project.

create table kentos.upload (
  id uuid primary key,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  name text not null check (length(name) between 1 and 200),
  media_type text not null check (length(media_type) between 3 and 120),
  -- What the client said it would send; the bytes must match both.
  size bigint not null check (size > 0),
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  blob_key text not null,
  created_at timestamptz not null default now(),
  -- Set once every byte arrived, matched and passed the scan.
  received_at timestamptz,
  scan text check (scan in ('clean', 'unscanned')),
  scan_note text check (scan_note is null or length(scan_note) <= 300),
  check ((received_at is null) = (scan is null))
);
create index upload_user on kentos.upload (user_id, created_at);
create index upload_created on kentos.upload (created_at);

alter table kentos.upload enable row level security;
create policy upload_own on kentos.upload
  using (user_id = kentos.current_user_id()) with check (user_id = kentos.current_user_id());

-- The server's cleanup: removes at most p_batch uploads of every account opened before p_before and returns their
-- objects' keys. It reads other accounts' rows, so it runs with the owner's rights; it removes nothing else.
create function kentos.expire_user_uploads(p_before timestamptz, p_batch integer) returns setof text
  language sql security definer set search_path = pg_catalog, pg_temp
  as $$
  delete from kentos.upload u
   where u.id in (select q.id from kentos.upload q where q.created_at < p_before order by q.created_at limit p_batch)
  returning u.blob_key
$$;
revoke all on function kentos.expire_user_uploads(timestamptz, integer) from public;
grant execute on function kentos.expire_user_uploads(timestamptz, integer) to kentos_cad_app;
grant select, insert, update, delete on kentos.upload to kentos_cad_app;

-- A dossier's files are scanned as they come, whether sent whole or by an upload.
alter table kentos.dossier_file
  add column scan text not null default 'unscanned' check (scan in ('clean', 'unscanned')),
  add column scan_note text check (scan_note is null or length(scan_note) <= 300);

-- A refused file is an event of the platform's: its administrators see who sent what and why it was refused.
alter table kentos.platform_event drop constraint platform_event_target_kind_check;
alter table kentos.platform_event add constraint platform_event_target_kind_check
  check (target_kind in ('tenant', 'user', 'professional', 'setting', 'file'));

-- Written by the request that refused the file, as its sender.
create function kentos.log_refused_file(p_name text, p_detail jsonb) returns void
  language sql security definer set search_path = pg_catalog, pg_temp
  as $$ select kentos.platform_log('file.refused', 'file', null, left(p_name, 200), p_detail) $$;
revoke all on function kentos.log_refused_file(text, jsonb) from public;
grant execute on function kentos.log_refused_file(text, jsonb) to kentos_cad_app;

create table kentos.project_attachment (
  tenant_id uuid not null,
  project_id uuid not null,
  id uuid not null,
  name text not null check (length(name) between 1 and 200),
  media_type text not null check (length(media_type) between 3 and 120),
  size bigint not null check (size > 0),
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  blob_key text not null,
  scan text not null check (scan in ('clean', 'unscanned')),
  scan_note text check (scan_note is null or length(scan_note) <= 300),
  note text check (note is null or length(note) between 1 and 2000),
  -- The drawing's revision it goes with, when it was added for one (a file project's).
  revision bigint check (revision is null or revision > 0),
  created_by uuid not null references kentos.app_user (id),
  created_by_name text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, project_id, id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);
create index project_attachment_created on kentos.project_attachment (tenant_id, project_id, created_at desc);

-- The project's own rows, as its revisions: seen and written only with the project in scope, while the current user
-- has a role in it. Who may add or remove one is the application's check (its edit permission).
alter table kentos.project_attachment enable row level security;
create policy attachment_project on kentos.project_attachment
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));
grant select, insert, update, delete on kentos.project_attachment to kentos_cad_app;
