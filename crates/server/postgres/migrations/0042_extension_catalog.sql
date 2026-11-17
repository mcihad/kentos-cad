-- KentOS CAD: the extension catalog's persistent store (docs/adr/8059 §11, §13).
--
-- * A release is one row: its place (the catalog and the extension's version), its publisher, author and state, its
--   content (package, lock and SBOM digests, the capabilities asked for), the flow's own record (`record`, the
--   `Release` JSON the server reads back through the flow's checks) and how many moves it made (`moves`). The columns
--   are the record's: a row whose columns say other than its record is refused. A catalog is `public` (no reference),
--   `tenant` (the organisation's id) or `personal` (the account's id).
-- * A release's identity never changes, and its content changes only while it is a draft or sent back: a trigger
--   refuses the rest, as the flow does (`content_frozen`). Its moves only grow: one kept is never changed.
-- * Every move is also a row of `extension_release_move`, added in the same transaction as the record; the server's
--   role may not change or remove one. A move is written over the record it was made on only:
--   `update … where moves = <as read>`; when no row changes another move came first.
-- * A package's bytes are in the object store under its content digest, once; its row is never changed either.
-- * Grants, publishers and the publishers a catalog trusts are the server administrator's (`kentosd
--   extension-catalog`); the server's role reads them. An organisation's members come from its memberships.
-- * Row-level security as projects': an organisation's releases only with that organisation in scope, an account's
--   own only with that account, the public catalog's to every transaction. The server sets the scope from the catalog
--   a call is about; who may see which release is the flow's (`Release::visible_to`).

create table kentos.extension_release (
  id uuid primary key,
  catalog_kind text not null check (catalog_kind in ('public', 'tenant', 'personal')),
  catalog_ref text not null,
  extension_id text not null check (length(extension_id) between 1 and 200),
  version text not null check (length(version) between 1 and 100),
  publisher text not null,
  author text not null,
  state text not null check (state in ('draft', 'submitted', 'inReview', 'changesRequested', 'approved', 'published',
                                       'suspended', 'withdrawn', 'archived')),
  package_digest text not null check (package_digest ~ '^sha256:[0-9a-f]{64}$'),
  lock_digest text,
  sbom_digest text,
  capabilities jsonb not null,
  record jsonb not null,
  moves integer not null check (moves >= 1),
  created_at timestamptz not null default now(),
  changed_at timestamptz not null default now(),
  unique (catalog_kind, catalog_ref, extension_id, version),
  check ((catalog_kind = 'public') = (catalog_ref = '')),
  check (catalog_kind = 'public' or catalog_ref ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'),
  check (record ->> 'extensionId' = extension_id and record ->> 'version' = version
         and record ->> 'publisher' = publisher and record ->> 'author' = author and record ->> 'state' = state
         and record -> 'content' ->> 'package' = package_digest
         and (record -> 'content' ->> 'lock') is not distinct from lock_digest
         and (record -> 'content' ->> 'sbom') is not distinct from sbom_digest
         and record -> 'content' -> 'capabilities' = capabilities
         and jsonb_typeof(record -> 'audit') = 'array' and jsonb_array_length(record -> 'audit') = moves)
);

-- One per move, in the record's order (`seq` from 0); `at` is the record's: milliseconds since 1970-01-01 UTC.
create table kentos.extension_release_move (
  release_id uuid not null references kentos.extension_release (id),
  seq integer not null check (seq >= 0),
  at bigint not null check (at >= 0),
  account text not null,
  via text not null check (via in ('window', 'program')),
  action text not null,
  from_state text,
  to_state text not null,
  content text not null,
  gate text,
  reason text,
  primary key (release_id, seq)
);

-- A package's bytes: `object_key` in the object store, `sha256` of those bytes (the backup checks it), `digest` the
-- package's content digest (every file's path, size and SHA-256) the releases name.
create table kentos.extension_package (
  digest text primary key check (digest ~ '^sha256:[0-9a-f]{64}$'),
  size bigint not null check (size > 0),
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  object_key text not null unique check (object_key = 'extension-packages/sha256-' || substr(digest, 8)),
  created_at timestamptz not null default now()
);

-- A publisher: an extension's owner in the catalogs, by the id its manifests name (`publisherId`).
create table kentos.extension_publisher (
  id text primary key check (id ~ '^[a-z0-9-]{2,100}$'),
  name text not null check (length(name) between 1 and 200),
  verified_domain text,
  created_at timestamptz not null default now()
);

create table kentos.extension_publisher_member (
  publisher_id text not null references kentos.extension_publisher (id) on delete cascade,
  account uuid not null references kentos.app_user (id) on delete cascade,
  joined_at timestamptz not null default now(),
  primary key (publisher_id, account)
);
create index extension_publisher_member_account on kentos.extension_publisher_member (account);

-- A catalog role: in one catalog, or in `every` one (the catalog's own services, the scanner). One's own catalog has
-- no grants: its owner holds its roles and nobody else is in it but the services.
create table kentos.extension_catalog_grant (
  catalog_kind text not null check (catalog_kind in ('every', 'public', 'tenant')),
  catalog_ref text not null,
  account uuid not null references kentos.app_user (id) on delete cascade,
  role text not null check (role in ('publisher', 'reviewer', 'catalogAdmin', 'scanner')),
  granted_at timestamptz not null default now(),
  primary key (catalog_kind, catalog_ref, account, role),
  check ((catalog_kind = 'tenant') = (catalog_ref <> ''))
);
create index extension_catalog_grant_account on kentos.extension_catalog_grant (account);

-- A publisher a catalog trusts: its code takes the review's fast path when nothing new is asked (D12).
create table kentos.extension_catalog_trust (
  catalog_kind text not null check (catalog_kind in ('public', 'tenant')),
  catalog_ref text not null,
  publisher_id text not null references kentos.extension_publisher (id) on delete cascade,
  trusted_at timestamptz not null default now(),
  primary key (catalog_kind, catalog_ref, publisher_id),
  check ((catalog_kind = 'tenant') = (catalog_ref <> ''))
);

-- What a release keeps: its identity, its content once sent, every move it made.
create function kentos.extension_release_kept() returns trigger
  language plpgsql set search_path = pg_catalog, pg_temp
  as $$
begin
  if (new.id, new.catalog_kind, new.catalog_ref, new.extension_id, new.version, new.publisher, new.author,
      new.created_at)
     is distinct from (old.id, old.catalog_kind, old.catalog_ref, old.extension_id, old.version, old.publisher,
                       old.author, old.created_at) then
    raise exception 'extension_release: a release''s identity does not change';
  end if;
  if old.state not in ('draft', 'changesRequested')
     and (new.package_digest, new.lock_digest, new.sbom_digest, new.capabilities)
         is distinct from (old.package_digest, old.lock_digest, old.sbom_digest, old.capabilities) then
    raise exception 'extension_release: content_frozen: the content of a release sent for review does not change';
  end if;
  if new.moves < old.moves then
    raise exception 'extension_release: a release''s moves are not taken away';
  end if;
  for i in 0 .. old.moves - 1 loop
    if (new.record -> 'audit' -> i) is distinct from (old.record -> 'audit' -> i) then
      raise exception 'extension_release: move % of the record changed', i;
    end if;
  end loop;
  new.changed_at := now();
  return new;
end $$;
create trigger extension_release_kept before update on kentos.extension_release
  for each row execute function kentos.extension_release_kept();

-- An organisation's id by its short name, for the scope of its catalog (the tenant row is visible to its members only;
-- a catalog's scanner and the audit worker are none).
create function kentos.extension_catalog_tenant(p_slug text) returns uuid
  language sql stable security definer set search_path = pg_catalog, pg_temp
  as $$ select t.id from kentos.tenant t where t.slug = p_slug $$;

alter table kentos.extension_release enable row level security;
create policy extension_release_catalog on kentos.extension_release
  using (catalog_kind = 'public'
         or (catalog_kind = 'tenant' and catalog_ref = kentos.current_tenant()::text)
         or (catalog_kind = 'personal' and catalog_ref = kentos.current_user_id()::text))
  with check (catalog_kind = 'public'
              or (catalog_kind = 'tenant' and catalog_ref = kentos.current_tenant()::text)
              or (catalog_kind = 'personal' and catalog_ref = kentos.current_user_id()::text));

alter table kentos.extension_release_move enable row level security;
create policy extension_release_move_release on kentos.extension_release_move
  using (exists (select 1 from kentos.extension_release r where r.id = release_id))
  with check (exists (select 1 from kentos.extension_release r where r.id = release_id));

revoke all on function kentos.extension_catalog_tenant(text) from public;
grant execute on function kentos.extension_catalog_tenant(text) to kentos_cad_app;
grant select, insert, update on kentos.extension_release to kentos_cad_app;
grant select, insert on kentos.extension_release_move, kentos.extension_package to kentos_cad_app;
grant select on kentos.extension_publisher, kentos.extension_publisher_member, kentos.extension_catalog_grant,
  kentos.extension_catalog_trust to kentos_cad_app;
