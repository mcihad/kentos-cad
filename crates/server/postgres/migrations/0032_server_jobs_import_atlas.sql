-- KentOS CAD: a file import and an atlas as durable server jobs (docs/adr/0561; plan/gis.md CBS-39).
--
-- * Two more kinds: `project.import` (a file the user staged is taken into the project by the worker, the same import
--   commands as the desktop's, committed in the job's finish like a processing job's result) and `project.atlas` (the
--   atlas of a sheet written as one PDF, the job's artifact).
-- * `kentos.job_input`: a file staged for a job (the import's file): its declared size and SHA-256, its object's key and
--   when its bytes arrived. It belongs to the user who opened it and to one project; a job names it, a retry of the job
--   reads it again.
-- * `kentos.expire_job_inputs` and `kentos.existing_job_inputs`: the retention of staged files (rows and objects) and the
--   store cleanup's question of which objects some row holds.

alter table kentos.server_job drop constraint server_job_kind_check;
alter table kentos.server_job add constraint server_job_kind_check
  check (kind in ('project.export', 'project.checkpoint', 'project.processing', 'project.import', 'project.atlas'));

create table kentos.job_input (
  tenant_id uuid not null,
  project_id uuid not null,
  id uuid not null,
  created_by uuid not null references kentos.app_user (id),
  name text not null check (length(name) between 1 and 200),
  size bigint not null check (size > 0),
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  blob_key text not null,
  created_at timestamptz not null default now(),
  received_at timestamptz,
  primary key (tenant_id, project_id, id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade
);
create index job_input_age on kentos.job_input (created_at);

alter table kentos.job_input enable row level security;
create policy job_input_project on kentos.job_input
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));
grant select, insert, update, delete on kentos.job_input to kentos_cad_app;

-- Removes up to `p_batch` staged files opened before `p_before`; one row per file, with its object's key.
create function kentos.expire_job_inputs(p_before timestamptz, p_batch integer)
  returns table (blob_key text)
  language sql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
  with old as (
    select i.tenant_id, i.project_id, i.id from kentos.job_input i
     where i.created_at < p_before
     order by i.created_at
     limit greatest(1, p_batch)
     for update skip locked
  ), gone as (
    delete from kentos.job_input i using old
     where i.tenant_id = old.tenant_id and i.project_id = old.project_id and i.id = old.id
    returning i.blob_key
  )
  select gone.blob_key from gone
$$;

-- Which of the given object keys a staged file's row holds.
create function kentos.existing_job_inputs(p_keys text[])
  returns table (blob_key text)
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select i.blob_key from kentos.job_input i where i.blob_key = any (p_keys)
$$;

revoke all on function kentos.expire_job_inputs(timestamptz, integer) from public;
revoke all on function kentos.existing_job_inputs(text[]) from public;
grant execute on function kentos.expire_job_inputs(timestamptz, integer) to kentos_cad_app;
grant execute on function kentos.existing_job_inputs(text[]) to kentos_cad_app;
