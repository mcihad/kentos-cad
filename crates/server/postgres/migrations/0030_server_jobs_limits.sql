-- KentOS CAD: limits, retention and measures of durable server jobs (docs/adr/0547; plan/gis.md CBS-39).
--
-- * A second kind, `project.checkpoint`: a checkpoint made as a job (no artifact; the answer is the command's).
-- * `kentos.job_slot`: whether an organisation has a place for one more queued or running job. It takes a transaction
--   lock on the organisation, so two requests at once cannot both take the last place; the caller counts and inserts
--   in the same transaction.
-- * `kentos.expire_jobs`: removes finished jobs older than a time and says their artifact keys, for the server to
--   remove the objects (a key it fails to remove is found by the store's cleanup: no job holds it any more).
-- * `kentos.job_metrics` and `kentos.job_load`: the queue as numbers for the operator, no job's content.

alter table kentos.server_job drop constraint server_job_kind_check;
alter table kentos.server_job add constraint server_job_kind_check check (kind in ('project.export', 'project.checkpoint'));

-- True when `p_tenant` has fewer than `p_limit` queued and running jobs.
create function kentos.job_slot(p_tenant uuid, p_limit integer)
  returns boolean
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  active bigint;
begin
  perform pg_advisory_xact_lock(hashtextextended('kentos.job_slot:' || p_tenant::text, 0));
  select count(*) into active from kentos.server_job s
   where s.tenant_id = p_tenant and s.state in ('queued', 'running');
  return active < p_limit;
end $$;

-- Removes up to `p_batch` finished jobs that ended before `p_before`; one row per job, with its artifact key (null
-- for a job without one).
create function kentos.expire_jobs(p_before timestamptz, p_batch integer)
  returns table (artifact_key text)
  language sql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
  with old as (
    select s.tenant_id, s.project_id, s.id from kentos.server_job s
     where s.state in ('succeeded', 'failed', 'cancelled') and s.finished_at < p_before
     order by s.finished_at
     limit greatest(1, p_batch)
     for update skip locked
  ), gone as (
    delete from kentos.server_job s using old
     where s.tenant_id = old.tenant_id and s.project_id = old.project_id and s.id = old.id
    returning s.artifact_key
  )
  select gone.artifact_key from gone
$$;

-- The queue as numbers: waiting, running, the last day's endings, and how long the oldest waiting job has waited.
create function kentos.job_metrics()
  returns table (queued bigint, running bigint, failed_24h bigint, succeeded_24h bigint, cancelled_24h bigint,
                 oldest_queued_seconds double precision)
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select count(*) filter (where s.state = 'queued'),
         count(*) filter (where s.state = 'running'),
         count(*) filter (where s.state = 'failed' and s.finished_at > now() - interval '24 hours'),
         count(*) filter (where s.state = 'succeeded' and s.finished_at > now() - interval '24 hours'),
         count(*) filter (where s.state = 'cancelled' and s.finished_at > now() - interval '24 hours'),
         extract(epoch from now() - min(s.created_at) filter (where s.state = 'queued'))::double precision
    from kentos.server_job s
$$;

-- The organisations with queued or running jobs and how many.
create function kentos.job_load()
  returns table (tenant_id uuid, queued bigint, running bigint)
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select s.tenant_id, count(*) filter (where s.state = 'queued'), count(*) filter (where s.state = 'running')
    from kentos.server_job s
   where s.state in ('queued', 'running')
   group by s.tenant_id
   order by 2 desc, 3 desc, 1
$$;

revoke all on function kentos.job_slot(uuid, integer) from public;
revoke all on function kentos.expire_jobs(timestamptz, integer) from public;
revoke all on function kentos.job_metrics() from public;
revoke all on function kentos.job_load() from public;
grant execute on function kentos.job_slot(uuid, integer) to kentos_cad_app;
grant execute on function kentos.expire_jobs(timestamptz, integer) to kentos_cad_app;
grant execute on function kentos.job_metrics() to kentos_cad_app;
grant execute on function kentos.job_load() to kentos_cad_app;
