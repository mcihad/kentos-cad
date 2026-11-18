-- An office's shared dossiers (docs/adr/9000 §18; plan BUL-14): an application or a transfer a professional works on
-- with the colleagues of their office (an organisation of category 'office').
--
-- * The dossier keeps its responsible professional (`professional_id`: the one who signs, whose chamber record the
--   municipality checks); `office_id` shares it with the office. Its members are then on the professional's side as
--   well: a viewer reads, an editor prepares files and writes, a project manager and above takes the steps (the
--   application layer's flow rules); the responsible professional does everything as before.
-- * Only the responsible professional shares a dossier with an office they are an editor of, or takes it back; an
--   application may be opened on an office's behalf from the start.
-- * The office's colleagues (editors and above) are told of the dossier as the professional is.

alter table kentos.dossier add column office_id uuid references kentos.tenant (id) on delete set null;
create index dossier_office on kentos.dossier (office_id, updated_at desc) where office_id is not null;

-- The caller's role in an office they may use now (member_role, of an organisation of category 'office'), or null.
create function kentos.office_role(p_tenant uuid) returns text
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select kentos.member_role(p_tenant)
   where exists (select 1 from kentos.tenant t where t.id = p_tenant and t.category = 'office')
$$;

create or replace function kentos.dossier_side(p_dossier uuid) returns text
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select case when (d.professional_id = kentos.current_user_id()
                    or (d.office_id is not null and kentos.office_role(d.office_id) is not null))
                   and not (d.direction = 'transfer' and d.status = 'draft') then 'professional'
              when kentos.member_role(d.tenant_id) is not null
                   and not (d.direction = 'application' and d.status = 'draft') then 'municipality' end
    from kentos.dossier d where d.id = p_dossier
$$;

-- A professional is also seen by the colleagues of an office they share a dossier with: the office works on the
-- dossier under the professional's name and check (only a verified professional's side takes steps).
create or replace function kentos.professional_visible(p_user uuid) returns boolean
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select p_user = kentos.current_user_id()
      or kentos.is_platform_admin()
      or (exists (select 1 from kentos.professional p
                   where p.user_id = p_user and p.status = 'verified' and p.listed)
          and exists (select 1 from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
                       where m.user_id = kentos.current_user_id() and m.status = 'active'
                         and t.status = 'active' and t.category = 'municipality'))
      or exists (select 1 from kentos.dossier d
                  where d.professional_id = p_user and kentos.member_role(d.tenant_id) is not null
                    and not (d.direction = 'application' and d.status = 'draft'))
      or exists (select 1 from kentos.dossier d
                  where d.professional_id = p_user and d.office_id is not null
                    and kentos.office_role(d.office_id) is not null)
$$;

-- The office a dossier is shared with, by name, for whoever sees the dossier (the municipality sees which office).
create function kentos.dossier_office_name(p_dossier uuid) returns text
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select t.name from kentos.dossier d join kentos.tenant t on t.id = d.office_id
   where d.id = p_dossier and kentos.dossier_side(p_dossier) is not null
$$;

drop policy dossier_open_application on kentos.dossier;
create policy dossier_open_application on kentos.dossier for insert
  with check (direction = 'application' and status = 'draft' and professional_id = kentos.current_user_id()
              and created_by = kentos.current_user_id()
              and (office_id is null or kentos.role_rank(kentos.office_role(office_id)) >= 2));

create or replace function kentos.notify_dossier(p_dossier uuid, p_audience text, p_kind text, p_title text,
                                                  p_body text)
  returns integer
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  d record;
  n integer := 0;
begin
  if kentos.dossier_side(p_dossier) is null then
    raise exception 'notify_dossier: the caller is not a party to the dossier';
  end if;
  select * into d from kentos.dossier where id = p_dossier;
  if p_audience = 'professional' then
    -- The responsible professional (in their own space), and the office's editors and above (in the office's).
    with audience as (
      select d.professional_id as user_id, null::uuid as tenant_id
      union
      select m.user_id, d.office_id from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
       where d.office_id is not null and m.tenant_id = d.office_id and m.status = 'active' and t.status = 'active'
         and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
         and kentos.role_rank(m.role) >= 2 and m.user_id <> d.professional_id
    )
    insert into kentos.notification (user_id, kind, dossier_id, tenant_id, title, body)
    select a.user_id, p_kind, p_dossier, a.tenant_id, p_title, coalesce(p_body, '')
      from audience a where a.user_id is distinct from kentos.current_user_id();
    get diagnostics n = row_count;
  elsif p_audience = 'municipality' then
    with usable as (
      select m.user_id from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
       where m.tenant_id = d.tenant_id and m.status = 'active' and t.status = 'active'
         and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
         and kentos.role_rank(m.role) >= 2
    ), audience as (
      select user_id from usable where user_id = d.assignee_id
      union
      select user_id from usable where not exists (select 1 from usable u where u.user_id = d.assignee_id)
    )
    insert into kentos.notification (user_id, kind, dossier_id, tenant_id, title, body)
    select a.user_id, p_kind, p_dossier, d.tenant_id, p_title, coalesce(p_body, '')
      from audience a where a.user_id is distinct from kentos.current_user_id();
    get diagnostics n = row_count;
  else
    raise exception 'notify_dossier: unknown audience %', p_audience;
  end if;
  return n;
end $$;

revoke all on function kentos.office_role(uuid) from public;
revoke all on function kentos.dossier_office_name(uuid) from public;
grant execute on function kentos.office_role(uuid), kentos.dossier_office_name(uuid) to kentos_cad_app;

-- An application is sent by its professional, or by a project manager (and above) of the office it is shared with.
create or replace function kentos.dossier_assign_number(p_dossier uuid) returns integer
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  d record;
  y integer := extract(year from now())::integer;
  given integer;
begin
  select * into d from kentos.dossier where id = p_dossier for update;
  if not found then
    raise exception 'dossier_assign_number: no dossier %', p_dossier;
  end if;
  if not ((d.direction = 'application'
           and (d.professional_id = kentos.current_user_id()
                or (d.office_id is not null and kentos.role_rank(kentos.office_role(d.office_id)) >= 3)))
          or (d.direction = 'transfer' and kentos.role_rank(kentos.member_role(d.tenant_id)) >= 3)) then
    raise exception 'dossier_assign_number: the caller does not send this dossier';
  end if;
  if d.number is not null then
    return d.number;
  end if;
  perform pg_advisory_xact_lock(hashtextextended('kentos.dossier_number:' || d.tenant_id::text || ':' || y, 0));
  select coalesce(max(number), 0) + 1 into given from kentos.dossier where tenant_id = d.tenant_id and year = y;
  update kentos.dossier set year = y, number = given where id = p_dossier;
  return given;
end $$;

-- The office's colleagues change the dossier as the professional does (the flow rules say which step); its manager
-- may delete its draft. Only the professional's own check lets the sharing itself change (the application refuses
-- anyone else).
drop policy dossier_change on kentos.dossier;
create policy dossier_change on kentos.dossier for update
  using (kentos.dossier_side(id) is not null)
  with check (professional_id = kentos.current_user_id() or kentos.member_role(tenant_id) is not null
              or (office_id is not null and kentos.office_role(office_id) is not null));
drop policy dossier_drop_draft on kentos.dossier;
create policy dossier_drop_draft on kentos.dossier for delete
  using (status = 'draft'
         and ((direction = 'application'
               and (professional_id = kentos.current_user_id()
                    or (office_id is not null and kentos.role_rank(kentos.office_role(office_id)) >= 3)))
              or (direction = 'transfer' and kentos.role_rank(kentos.member_role(tenant_id)) >= 3)));
