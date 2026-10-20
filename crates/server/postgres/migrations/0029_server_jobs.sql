-- KentOS CAD: durable server jobs (docs/adr/0542; plan/gis.md CBS-39).
--
-- * A job is asked for by a user in a project: its kind, its input (kept as given, with its SHA-256; it never changes)
--   and the project's data revision when it was asked. A retried request with the same request id gets the same job.
-- * A worker claims the oldest queued job across tenants (`kentos.claim_job`, security definer: the worker sees no
--   project's rows otherwise). Claiming gives the job a lease and a fence: the fence goes up by one at every claim, and
--   only the claim holding the current fence may beat (`kentos.beat_job`) or finish it (`kentos.finish_job`). A worker
--   that stops beating loses the lease; the next claim takes the job again until its attempts run out.
-- * A job runs at most `timeout_seconds` from its first start; past it, it is failed and its worker told to stop.
-- * Cancelling marks the job; a queued job is cancelled at once, a running one at its worker's next beat.
-- * The result is staged: the worker writes its artifact to the object store first, then finishes the job as the user
--   who asked for it, under the project's row-level security, with the access asked again. A finish that comes too
--   late (lost lease, cancelled, timed out) changes nothing; the server's cleanup removes its object.

create table kentos.server_job (
  tenant_id uuid not null,
  project_id uuid not null,
  id uuid not null,
  kind text not null check (kind in ('project.export')),
  input jsonb not null,
  input_sha256 text not null check (input_sha256 ~ '^[0-9a-f]{64}$'),
  -- The project's data revision when the job was asked for.
  base_revision bigint not null check (base_revision >= 0),
  created_by uuid not null references kentos.app_user (id),
  created_at timestamptz not null default now(),
  request_id text not null check (length(request_id) between 1 and 200),
  state text not null default 'queued' check (state in ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
  cancel_requested boolean not null default false,
  attempts integer not null default 0 check (attempts >= 0),
  max_attempts integer not null default 3 check (max_attempts between 1 and 10),
  timeout_seconds integer not null default 600 check (timeout_seconds between 1 and 86400),
  lease_owner uuid,
  lease_until timestamptz,
  fence bigint not null default 0 check (fence >= 0),
  progress real not null default 0 check (progress between 0 and 1),
  started_at timestamptz,
  finished_at timestamptz,
  -- A succeeded job's answer and its artifact in the object store.
  result jsonb,
  artifact_key text,
  artifact_size bigint check (artifact_size is null or artifact_size >= 0),
  artifact_sha256 text check (artifact_sha256 is null or artifact_sha256 ~ '^[0-9a-f]{64}$'),
  error_code text,
  error_message text check (error_message is null or length(error_message) <= 2000),
  primary key (tenant_id, project_id, id),
  foreign key (tenant_id, project_id) references kentos.project (tenant_id, id) on delete cascade,
  unique (tenant_id, project_id, created_by, request_id),
  check ((state = 'running') = (lease_owner is not null)),
  check ((state = 'failed') = (error_code is not null)),
  check (artifact_key is null or state = 'succeeded')
);
create index server_job_queue on kentos.server_job (created_at) where state = 'queued';
create index server_job_running on kentos.server_job (lease_until) where state = 'running';
create index server_job_project on kentos.server_job (tenant_id, project_id, created_at desc);

-- The project's own rows: seen and asked for only with the project in scope, while the current user has a role in it.
alter table kentos.server_job enable row level security;
create policy server_job_project on kentos.server_job
  using (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()))
  with check (tenant_id = kentos.current_tenant() and project_id = kentos.current_project() and (select kentos.project_readable()));

-- Takes back what lost its lease (queued again, or failed when its attempts are used up) and fails what ran past its
-- time; then claims the oldest queued job of the given kinds for `p_owner`, for `p_lease` seconds.
create function kentos.claim_job(p_owner uuid, p_lease integer, p_kinds text[])
  returns table (tenant_id uuid, project_id uuid, id uuid, kind text, input jsonb, base_revision bigint,
                 created_by uuid, fence bigint, attempts integer)
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  j kentos.server_job%rowtype;
begin
  if p_owner is null or p_lease is null or p_lease < 1 or p_lease > 3600 then
    raise exception 'claim_job: owner and a lease of 1 to 3600 seconds';
  end if;
  update kentos.server_job s
     set state = 'failed', lease_owner = null, lease_until = null, finished_at = now(),
         error_code = 'timeout', error_message = 'İş süresini aştı; durduruldu.'
   where s.state = 'running' and s.started_at + make_interval(secs => s.timeout_seconds) < now();
  update kentos.server_job s
     set state = case when s.attempts >= s.max_attempts then 'failed' else 'queued' end,
         lease_owner = null, lease_until = null,
         finished_at = case when s.attempts >= s.max_attempts then now() end,
         error_code = case when s.attempts >= s.max_attempts then 'lease_lost' end,
         error_message = case when s.attempts >= s.max_attempts
                              then 'İşi çalıştıran kayboldu ve deneme hakkı bitti.' end
   where s.state = 'running' and s.lease_until < now();
  select * into j from kentos.server_job s
   where s.state = 'queued' and not s.cancel_requested and s.kind = any (p_kinds)
   order by s.created_at, s.id
   for update skip locked
   limit 1;
  if not found then
    return;
  end if;
  update kentos.server_job s
     set state = 'running', lease_owner = p_owner, lease_until = now() + make_interval(secs => p_lease),
         fence = s.fence + 1, attempts = s.attempts + 1, started_at = coalesce(s.started_at, now())
   where s.tenant_id = j.tenant_id and s.project_id = j.project_id and s.id = j.id;
  return query
    select s.tenant_id, s.project_id, s.id, s.kind, s.input, s.base_revision, s.created_by, s.fence, s.attempts
      from kentos.server_job s
     where s.tenant_id = j.tenant_id and s.project_id = j.project_id and s.id = j.id;
end $$;

-- A running job's beat: its lease renewed and its progress written, only for the claim holding `p_fence`. The answer:
-- 'run' (go on), 'cancel' (asked to stop: the job is cancelled now), 'lost' (the lease is someone else's or the job is
-- over).
create function kentos.beat_job(p_id uuid, p_fence bigint, p_progress real, p_lease integer)
  returns text
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  j kentos.server_job%rowtype;
begin
  select * into j from kentos.server_job s where s.id = p_id for update;
  if not found or j.state <> 'running' or j.fence <> p_fence or j.lease_until < now() then
    return 'lost';
  end if;
  if j.cancel_requested then
    update kentos.server_job s
       set state = 'cancelled', lease_owner = null, lease_until = null, finished_at = now()
     where s.id = p_id;
    return 'cancel';
  end if;
  if j.started_at + make_interval(secs => j.timeout_seconds) < now() then
    update kentos.server_job s
       set state = 'failed', lease_owner = null, lease_until = null, finished_at = now(),
           error_code = 'timeout', error_message = 'İş süresini aştı; durduruldu.'
     where s.id = p_id;
    return 'lost';
  end if;
  update kentos.server_job s
     set lease_until = now() + make_interval(secs => greatest(1, least(p_lease, 3600))),
         progress = greatest(0, least(1, coalesce(p_progress, s.progress)))
   where s.id = p_id;
  return 'run';
end $$;

-- A running job finished by the claim holding `p_fence`: succeeded with its result and artifact, or failed with its
-- code and message. False when the claim no longer holds the job (nothing changed).
create function kentos.finish_job(p_id uuid, p_fence bigint, p_ok boolean, p_result jsonb, p_key text, p_size bigint,
                                  p_sha256 text, p_code text, p_message text)
  returns boolean
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  update kentos.server_job s
     set state = case when p_ok then 'succeeded' else 'failed' end,
         lease_owner = null, lease_until = null, finished_at = now(), progress = case when p_ok then 1 else s.progress end,
         result = case when p_ok then p_result end,
         artifact_key = case when p_ok then p_key end,
         artifact_size = case when p_ok then p_size end,
         artifact_sha256 = case when p_ok then p_sha256 end,
         error_code = case when p_ok then null else coalesce(p_code, 'failed') end,
         error_message = case when p_ok then null else left(p_message, 2000) end
   where s.id = p_id and s.state = 'running' and s.fence = p_fence and s.lease_until >= now();
  return found;
end $$;

-- Which of these artifact keys a job still holds (for removing the objects of jobs whose finish never came).
create function kentos.existing_job_artifacts(p_keys text[])
  returns table (artifact_key text)
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$ select s.artifact_key from kentos.server_job s where s.artifact_key = any (p_keys) $$;

revoke all on function kentos.claim_job(uuid, integer, text[]) from public;
revoke all on function kentos.beat_job(uuid, bigint, real, integer) from public;
revoke all on function kentos.finish_job(uuid, bigint, boolean, jsonb, text, bigint, text, text, text) from public;
revoke all on function kentos.existing_job_artifacts(text[]) from public;
grant execute on function kentos.claim_job(uuid, integer, text[]) to kentos_cad_app;
grant execute on function kentos.beat_job(uuid, bigint, real, integer) to kentos_cad_app;
grant execute on function kentos.finish_job(uuid, bigint, boolean, jsonb, text, bigint, text, text, text) to kentos_cad_app;
grant execute on function kentos.existing_job_artifacts(text[]) to kentos_cad_app;
grant select, insert, update on kentos.server_job to kentos_cad_app;
