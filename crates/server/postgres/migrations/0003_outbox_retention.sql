-- KentOS CAD: how long the event log is kept (docs/adr/0006-data-layer.md,
-- "2026-09-24: olay günlüğünün budanması").
--
-- `kentosd serve` removes outbox events older than the retention window in
-- small batches. A client that asks for events after a cursor older than
-- what is left must reopen the project instead (resyncRequired), so the
-- server remembers, per project, the newest event it removed: that tells a
-- quiet project (nothing after the cursor) from one whose events are gone.

create table kentos.outbox_horizon (
  tenant_id uuid not null,
  project_id uuid not null,
  -- Every event of the project with seq <= pruned_through may be gone.
  pruned_through bigint not null,
  primary key (tenant_id, project_id)
);

alter table kentos.outbox_horizon enable row level security;
create policy outbox_horizon_tenant on kentos.outbox_horizon for select
  using (tenant_id = kentos.current_tenant());

-- The oldest events first, without reading the whole log.
create index outbox_age on kentos.outbox_event (created_at);

-- Removes at most p_batch events older than p_keep (across tenants: the
-- server role cannot see or delete them itself) and records each project's
-- horizon in the same statement. Refuses a window under an hour, so the
-- server role can never empty a log that live clients still read from.
create function kentos.prune_outbox(p_keep interval, p_batch integer) returns bigint
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  removed bigint;
begin
  if p_keep < interval '1 hour' then
    raise exception 'outbox events are kept at least an hour (asked: %)', p_keep;
  end if;
  with gone as (
    delete from kentos.outbox_event
     where seq in (select seq from kentos.outbox_event
                    where created_at < now() - p_keep
                    order by created_at
                    limit greatest(p_batch, 1)
                    for update skip locked)
    returning tenant_id, project_id, seq
  ), per_project as (
    select tenant_id, project_id, max(seq) as through, count(*) as n
      from gone group by tenant_id, project_id
  ), marked as (
    insert into kentos.outbox_horizon as h (tenant_id, project_id, pruned_through)
    select tenant_id, project_id, through from per_project
    on conflict (tenant_id, project_id)
      do update set pruned_through = greatest(h.pruned_through, excluded.pruned_through)
  )
  select coalesce(sum(n), 0) into removed from per_project;
  return removed;
end $$;

revoke all on function kentos.prune_outbox(interval, integer) from public;
grant execute on function kentos.prune_outbox(interval, integer) to kentos_cad_app;
grant select on kentos.outbox_horizon to kentos_cad_app;
