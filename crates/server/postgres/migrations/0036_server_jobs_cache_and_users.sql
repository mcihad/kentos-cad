-- KentOS CAD: the result cache of server jobs, a user's share of the queue and a fresh run (docs/adr/0739; plan/gis.md
-- CBS-39).
--
-- * `cache_key`: the SHA-256 of what a succeeded export's, atlas's or file-writing tool's artifact was built from: its
--   kind, the input that changes the result, the project's data revision and event cursor, the reach it was cut by, and
--   for the atlas and the tool also the user who asked and the day (they may read `@` values). A later job of the same
--   project with the same key takes that artifact (a second key of the same object) instead of building it again.
-- * `cached_from`: the job whose artifact a job took.
-- * `fresh`: the asker wants the job built again, whatever was built before.
-- * `kentos.job_room`: whether the organisation and the user have a place for one more queued or running job; under the
--   organisation's lock, as `kentos.job_slot` (whose lock it takes, so the two never race).

alter table kentos.server_job add column cache_key text check (cache_key is null or cache_key ~ '^[0-9a-f]{64}$');
alter table kentos.server_job add column cached_from uuid;
alter table kentos.server_job add column fresh boolean not null default false;
alter table kentos.server_job add constraint server_job_cache_succeeded check (cache_key is null or state = 'succeeded');
create index server_job_cache on kentos.server_job (tenant_id, project_id, cache_key, finished_at desc)
  where cache_key is not null;

-- 'user' when `p_user` has `p_user_limit` queued and running jobs in `p_tenant`, else 'tenant' when the organisation has
-- `p_tenant_limit` of them, null when there is room for one more. The user's share is said first: when both are full,
-- only one of the user's own jobs ending makes room for them.
create function kentos.job_room(p_tenant uuid, p_user uuid, p_tenant_limit integer, p_user_limit integer)
  returns text
  language plpgsql volatile security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  tenant_active bigint;
  user_active bigint;
begin
  perform pg_advisory_xact_lock(hashtextextended('kentos.job_slot:' || p_tenant::text, 0));
  select count(*), count(*) filter (where s.created_by = p_user) into tenant_active, user_active
    from kentos.server_job s
   where s.tenant_id = p_tenant and s.state in ('queued', 'running');
  if user_active >= p_user_limit then
    return 'user';
  end if;
  if tenant_active >= p_tenant_limit then
    return 'tenant';
  end if;
  return null;
end $$;

revoke all on function kentos.job_room(uuid, uuid, integer, integer) from public;
grant execute on function kentos.job_room(uuid, uuid, integer, integer) to kentos_cad_app;
