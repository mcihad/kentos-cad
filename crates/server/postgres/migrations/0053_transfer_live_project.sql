-- A transfer worked on in the municipality's live project instead of a copy (docs/adr/9000 §24; plan BUL-15; the
-- guests of docs/adr/0035): the municipality's project managers link one of its cloud projects to the transfer; the
-- professional who takes the job on gets an editor's grant on it — a guest's when they are not a member of the
-- municipality (and only while it takes guests) — and loses it when the job closes (completed, cancelled, declined).
--
-- * The grant is the dossier's only when it made one: a professional who already had a grant on the project keeps
--   theirs and keeps it after the job (`live_granted` false).
-- * Each grant and revocation is in the project's audit record and is a `project.access` event for its open
--   connections, as a share is (ADR 0024).

alter table kentos.dossier
  add column live_project_id uuid,
  add column live_granted boolean not null default false,
  add constraint dossier_live_project foreign key (tenant_id, live_project_id)
    references kentos.project (tenant_id, id) on delete set null (live_project_id),
  add constraint dossier_live_transfer check (live_project_id is null or direction = 'transfer');

-- The project's audit record and its open connections' `project.access` event, as sharing writes them.
create function kentos.dossier_live_announce(p_tenant uuid, p_project uuid, p_action text, p_request text,
                                             p_detail jsonb) returns void
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  rev bigint;
begin
  select data_revision into rev from kentos.project where tenant_id = p_tenant and id = p_project;
  insert into kentos.audit_event (tenant_id, project_id, actor, action, request_id, data_revision, detail)
  values (p_tenant, p_project, kentos.current_user_id(), p_action, p_request, rev, p_detail);
  insert into kentos.outbox_event (tenant_id, project_id, data_revision, kind, payload)
  values (p_tenant, p_project, rev, 'project.access',
          jsonb_build_object('seq', '', 'dataRevision', rev::text, 'kind', 'project.access',
                             'actor', kentos.current_user_id()::text, 'requestId', p_request,
                             'features', '[]'::jsonb, 'meta', false));
end $$;

-- Gives the transfer's professional the live project's grant (p_on) or takes the dossier's grant back.
create function kentos.dossier_live_access(p_dossier uuid, p_on boolean) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  d record;
  ws record;
  member_ok boolean;
  held text;
begin
  select * into d from kentos.dossier where id = p_dossier;
  if not found or kentos.dossier_side(p_dossier) is null then
    return 'forbidden';
  end if;
  if d.live_project_id is null then
    return null;
  end if;
  if not p_on then
    if d.live_granted then
      delete from kentos.project_grant
       where tenant_id = d.tenant_id and project_id = d.live_project_id and user_id = d.professional_id;
      update kentos.dossier set live_granted = false where id = p_dossier;
      perform kentos.dossier_live_announce(d.tenant_id, d.live_project_id, 'dossier.live_project.revoke',
                                           p_dossier::text, jsonb_build_object('userId', d.professional_id));
    end if;
    return null;
  end if;
  if d.live_granted then
    return null;
  end if;
  select g.role into held from kentos.project_grant g
   where g.tenant_id = d.tenant_id and g.project_id = d.live_project_id and g.user_id = d.professional_id
     and (g.expires_at is null or g.expires_at > now());
  if held is not null then
    -- Their own grant stays theirs.
    return null;
  end if;
  select t.status, t.allow_guests into ws from kentos.tenant t where t.id = d.tenant_id;
  select m.status = 'active'
           and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
    into member_ok
    from kentos.membership m where m.tenant_id = d.tenant_id and m.user_id = d.professional_id;
  if member_ok is false then
    -- A member whose membership or seat is gone gains nothing from a grant (ADR 0035).
    return 'inactive';
  end if;
  if member_ok is null and not ws.allow_guests then
    return 'guests_off';
  end if;
  insert into kentos.project_grant (tenant_id, project_id, user_id, role, granted_by, guest)
  values (d.tenant_id, d.live_project_id, d.professional_id, 'editor', d.created_by, member_ok is null)
  on conflict on constraint project_grant_pkey do update set
    role = 'editor', guest = excluded.guest, granted_by = excluded.granted_by, expires_at = null, updated_at = now();
  update kentos.dossier set live_granted = true where id = p_dossier;
  perform kentos.dossier_live_announce(d.tenant_id, d.live_project_id, 'dossier.live_project.grant', p_dossier::text,
                                       jsonb_build_object('userId', d.professional_id, 'role', 'editor',
                                                          'guest', member_ok is null));
  return null;
end $$;

-- Links one of the municipality's cloud projects to its transfer (null unlinks), its project managers and above,
-- while the transfer is open; a job already taken on gets its grant now.
create function kentos.dossier_set_live_project(p_dossier uuid, p_project uuid) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  d record;
  code text;
begin
  select * into d from kentos.dossier where id = p_dossier for update;
  if not found or kentos.dossier_side(p_dossier) is null then
    return 'not_found';
  end if;
  if d.direction <> 'transfer' or kentos.role_rank(kentos.member_role(d.tenant_id)) < 3 then
    return 'forbidden';
  end if;
  if d.status in ('completed', 'cancelled', 'declined') then
    return 'invalid';
  end if;
  if p_project is not null and not exists (select 1 from kentos.project p
                                            where p.tenant_id = d.tenant_id and p.id = p_project
                                              and p.deleted_at is null) then
    return 'not_found';
  end if;
  if d.live_project_id is not distinct from p_project then
    return null;
  end if;
  code := kentos.dossier_live_access(p_dossier, false);
  update kentos.dossier set live_project_id = p_project, live_granted = false where id = p_dossier;
  if p_project is not null and d.status in ('accepted', 'revision_requested', 'delivered') then
    code := kentos.dossier_live_access(p_dossier, true);
  end if;
  return code;
end $$;

-- The linked project as the dossier's two sides see it (the professional cannot read the municipality's projects).
create function kentos.dossier_live_project(p_dossier uuid)
  returns table (tenant_id uuid, project_id uuid, name text, granted boolean)
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$
  select d.tenant_id, d.live_project_id, p.name, d.live_granted
    from kentos.dossier d join kentos.project p on p.tenant_id = d.tenant_id and p.id = d.live_project_id
   where d.id = p_dossier and kentos.dossier_side(p_dossier) is not null
$$;

revoke all on function kentos.dossier_live_announce(uuid, uuid, text, text, jsonb) from public;
revoke all on function kentos.dossier_live_access(uuid, boolean) from public;
revoke all on function kentos.dossier_set_live_project(uuid, uuid) from public;
revoke all on function kentos.dossier_live_project(uuid) from public;
grant execute on function kentos.dossier_live_access(uuid, boolean), kentos.dossier_set_live_project(uuid, uuid),
  kentos.dossier_live_project(uuid) to kentos_cad_app;
