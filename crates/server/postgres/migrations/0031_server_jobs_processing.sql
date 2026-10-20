-- KentOS CAD: an İşlemler tool as a durable server job, and the worker's progress words (docs/adr/0559; plan/gis.md
-- CBS-39).
--
-- * A third kind, `project.processing`: an İşlemler tool run by a worker on the project at a pinned revision; its result
--   is a commit to the project (made in the same transaction as the job's finish) or a GeoPackage artifact.
-- * `progress_text`: what the worker says it is doing while the job runs, a few words, written with the progress at
--   every beat and read by polling the job.
-- * `kentos.beat_job` takes the words too; the old four-argument function goes (its only caller is this server's worker).

alter table kentos.server_job drop constraint server_job_kind_check;
alter table kentos.server_job add constraint server_job_kind_check
  check (kind in ('project.export', 'project.checkpoint', 'project.processing'));

alter table kentos.server_job add column progress_text text check (progress_text is null or length(progress_text) <= 200);

drop function kentos.beat_job(uuid, bigint, real, integer);

-- A running job's beat: its lease renewed and its progress (and the words for it) written, only for the claim holding
-- `p_fence`. The answer: 'run' (go on), 'cancel' (asked to stop: the job is cancelled now), 'lost' (the lease is
-- someone else's or the job is over).
create function kentos.beat_job(p_id uuid, p_fence bigint, p_progress real, p_lease integer, p_text text)
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
         progress = greatest(0, least(1, coalesce(p_progress, s.progress))),
         progress_text = coalesce(left(p_text, 200), s.progress_text)
   where s.id = p_id;
  return 'run';
end $$;

revoke all on function kentos.beat_job(uuid, bigint, real, integer, text) from public;
grant execute on function kentos.beat_job(uuid, bigint, real, integer, text) to kentos_cad_app;
