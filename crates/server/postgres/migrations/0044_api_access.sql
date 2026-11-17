-- Programs signing in to the cloud (docs/adr/9000 §12): the desktop, a mobile app or an AI agent.
--
-- A program signs in with a local account and gets its session as a bearer token instead of a cookie. The session is
-- the browser's kind of session, in the same table, with its kind, the program's name and the device's label, so the
-- account sees and ends its sessions one by one. An account also makes personal access tokens for agents and scripts:
-- named, read-only or read-write, expiring, revocable. As with sessions, only a token's SHA-256 is kept.

alter table kentos.auth_session
  add column id uuid not null default gen_random_uuid(),
  add column kind text not null default 'cookie' check (kind in ('cookie', 'bearer')),
  add column client text check (client is null or client ~ '^[a-z][a-z0-9-]{1,30}$'),
  add column label text check (label is null or length(label) between 1 and 80);
create unique index auth_session_id on kentos.auth_session (id);

create table kentos.api_token (
  id uuid primary key,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  name text not null check (length(name) between 1 and 80),
  token_hash bytea not null unique,
  -- The token's first characters, shown to recognise it by.
  prefix text not null,
  scope text not null check (scope in ('read', 'write')),
  created_at timestamptz not null default now(),
  expires_at timestamptz not null,
  last_used_at timestamptz,
  revoked_at timestamptz
);
create index api_token_user on kentos.api_token (user_id, created_at desc);

alter table kentos.api_token enable row level security;
create policy api_token_own on kentos.api_token
  using (user_id = kentos.current_user_id()) with check (user_id = kentos.current_user_id());

-- A token's account and scope, before anyone is signed in (as check_local_login): an active account and a token
-- neither revoked nor expired. Its last use is written at most once a minute.
create function kentos.api_token_user(p_token text) returns table (account uuid, token_scope text)
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  t record;
begin
  select a.id, a.user_id, a.scope, a.last_used_at into t
    from kentos.api_token a join kentos.app_user u on u.id = a.user_id
   where a.token_hash = public.digest(p_token, 'sha256') and a.revoked_at is null and a.expires_at > now()
     and u.status = 'active';
  if not found then
    return;
  end if;
  if t.last_used_at is null or t.last_used_at < now() - interval '1 minute' then
    update kentos.api_token set last_used_at = now() where kentos.api_token.id = t.id;
  end if;
  account := t.user_id;
  token_scope := t.scope;
  return next;
end $$;

revoke all on function kentos.api_token_user(text) from public;
grant execute on function kentos.api_token_user(text) to kentos_cad_app;
grant select, insert, update on kentos.api_token to kentos_cad_app;
