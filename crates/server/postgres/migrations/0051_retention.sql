-- How long personal data is kept, and its periodic destruction (docs/adr/9000 §21; plan BUL-18): 6698 sayılı KVKK
-- m.7 and the regulation on deleting, destroying and anonymising personal data (Yönetmelik m.11: periodic destruction
-- at intervals of at most six months; m.7: the records of destruction kept at least three years).
--
-- * A dossier is an administrative record the municipality keeps (its archive rules, not this), so a closed one is
--   not deleted: after `archive_after_days` it moves to the archive, out of the working lists.
-- * What has served its purpose is destroyed: a draft never sent and untouched for `draft_keep_days` (with its files;
--   the store's keys come back for the server to remove), notifications older than `notification_keep_days`, and the
--   technical leftovers older than `technical_keep_days` (e-mail / SMS deliveries done, signing requests that left no
--   signature).
-- * Each run is a record: when, by schedule or by hand (and whose), what it did and under which periods. Kept.

alter table kentos.platform_setting
  add column archive_after_days integer not null default 365 check (archive_after_days between 30 and 3650),
  add column draft_keep_days integer not null default 180 check (draft_keep_days between 30 and 3650),
  add column notification_keep_days integer not null default 180 check (notification_keep_days between 30 and 3650),
  add column technical_keep_days integer not null default 90 check (technical_keep_days between 7 and 3650);

alter table kentos.dossier add column archived_at timestamptz;
create index dossier_archived on kentos.dossier (tenant_id, archived_at) where archived_at is not null;

create table kentos.retention_run (
  id bigint generated always as identity primary key,
  started_at timestamptz not null default now(),
  trigger text not null check (trigger in ('schedule', 'manual')),
  actor_id uuid references kentos.app_user (id) on delete set null,
  actor_name text,
  archived integer not null default 0,
  drafts_deleted integer not null default 0,
  files_deleted integer not null default 0,
  notifications_deleted integer not null default 0,
  deliveries_deleted integer not null default 0,
  signing_requests_deleted integer not null default 0,
  settings jsonb not null
);
create index retention_run_at on kentos.retention_run (started_at desc);

alter table kentos.retention_run enable row level security;
create policy retention_run_read on kentos.retention_run for select using (kentos.is_platform_admin());
grant select on kentos.retention_run to kentos_cad_app;

-- The periods, set by a platform administrator; in the platform's record.
create function kentos.platform_set_retention(p_archive integer, p_drafts integer, p_notifications integer,
                                               p_technical integer) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  if p_archive not between 30 and 3650 or p_drafts not between 30 and 3650 or p_notifications not between 30 and 3650
     or p_technical not between 7 and 3650 then
    return 'invalid';
  end if;
  update kentos.platform_setting
     set archive_after_days = p_archive, draft_keep_days = p_drafts, notification_keep_days = p_notifications,
         technical_keep_days = p_technical, updated_at = now(), updated_by = kentos.current_user_id();
  perform kentos.platform_log('setting.retention', 'setting', null, 'Kişisel verilerin saklanması',
    jsonb_build_object('archiveAfterDays', p_archive, 'draftKeepDays', p_drafts,
                       'notificationKeepDays', p_notifications, 'technicalKeepDays', p_technical));
  return null;
end $$;

-- One run of the periodic destruction: by the server's schedule, or by a platform administrator's hand. The keys of
-- the destroyed drafts' files come back (the store is the server's to clean).
create function kentos.retention_sweep(p_trigger text) returns table (blob_key text)
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  s kentos.platform_setting%rowtype;
  n_archived integer;
  n_drafts integer;
  n_files integer;
  n_notes integer;
  n_deliveries integer;
  n_requests integer;
begin
  if p_trigger not in ('schedule', 'manual') then
    raise exception 'retention_sweep: unknown trigger %', p_trigger;
  end if;
  if p_trigger = 'manual' and not kentos.is_platform_admin() then
    raise exception 'retention_sweep: only a platform administrator runs it by hand';
  end if;
  select * into s from kentos.platform_setting;

  update kentos.dossier set archived_at = now()
   where archived_at is null
     and status in ('approved', 'rejected', 'withdrawn', 'declined', 'completed', 'cancelled')
     and updated_at < now() - make_interval(days => s.archive_after_days);
  get diagnostics n_archived = row_count;

  return query
    select f.blob_key from kentos.dossier_file f join kentos.dossier d on d.id = f.dossier_id
     where d.status = 'draft' and d.number is null
       and d.updated_at < now() - make_interval(days => s.draft_keep_days);
  get diagnostics n_files = row_count;
  delete from kentos.dossier d
   where d.status = 'draft' and d.number is null
     and d.updated_at < now() - make_interval(days => s.draft_keep_days);
  get diagnostics n_drafts = row_count;

  delete from kentos.notification where at < now() - make_interval(days => s.notification_keep_days);
  get diagnostics n_notes = row_count;
  delete from kentos.notification_delivery
   where state <> 'pending' and created_at < now() - make_interval(days => s.technical_keep_days);
  get diagnostics n_deliveries = row_count;
  delete from kentos.signing_request r
   where r.created_at < now() - make_interval(days => s.technical_keep_days)
     and not exists (select 1 from kentos.dossier_signature g where g.request_id = r.id);
  get diagnostics n_requests = row_count;

  insert into kentos.retention_run (trigger, actor_id, actor_name, archived, drafts_deleted, files_deleted,
                                    notifications_deleted, deliveries_deleted, signing_requests_deleted, settings)
  values (p_trigger,
          case when p_trigger = 'manual' then kentos.current_user_id() end,
          case when p_trigger = 'manual' then
            (select display_name from kentos.app_user where id = kentos.current_user_id()) end,
          n_archived, n_drafts, n_files, n_notes, n_deliveries, n_requests,
          jsonb_build_object('archiveAfterDays', s.archive_after_days, 'draftKeepDays', s.draft_keep_days,
                             'notificationKeepDays', s.notification_keep_days,
                             'technicalKeepDays', s.technical_keep_days));
end $$;

revoke all on function kentos.platform_set_retention(integer, integer, integer, integer) from public;
revoke all on function kentos.retention_sweep(text) from public;
grant execute on function kentos.platform_set_retention(integer, integer, integer, integer),
  kentos.retention_sweep(text) to kentos_cad_app;
