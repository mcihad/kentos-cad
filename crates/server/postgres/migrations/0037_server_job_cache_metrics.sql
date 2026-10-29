-- KentOS CAD: what the result cache of server jobs gains, as numbers for the operator (docs/adr/0744 §1; plan/gis.md
-- CBS-39).
--
-- * `kentos.job_cache_metrics`: for each job kind that finished in the last 24 hours with a cache key, how many were
--   built and how many took an earlier artifact, their average times from start to finish, and the bytes taken. No
--   job's content, no organisation.

create function kentos.job_cache_metrics()
  returns table (kind text, built bigint, taken bigint, built_seconds double precision,
                 taken_seconds double precision, taken_bytes bigint)
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select s.kind,
         count(*) filter (where s.cached_from is null),
         count(*) filter (where s.cached_from is not null),
         avg(extract(epoch from s.finished_at - s.started_at)) filter (where s.cached_from is null)::double precision,
         avg(extract(epoch from s.finished_at - s.started_at)) filter (where s.cached_from is not null)::double precision,
         coalesce(sum(s.artifact_size) filter (where s.cached_from is not null), 0)::bigint
    from kentos.server_job s
   where s.state = 'succeeded' and s.cache_key is not null and s.finished_at > now() - interval '24 hours'
   group by s.kind
   order by s.kind
$$;

revoke all on function kentos.job_cache_metrics() from public;
grant execute on function kentos.job_cache_metrics() to kentos_cad_app;
