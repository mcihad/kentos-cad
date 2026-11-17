-- E-signing a dossier's files (docs/adr/9004; plan BUL-21): the requests the server issued for the signing service
-- and the signatures it verified.
--
-- * A request (kentos.signing_request) is the payload the server signed (its own record of the files and the digests
--   of the bytes it keeps): the delivery is checked against this, never against what the client sends back. Only the
--   person who asked for it delivers or cancels it; a delivered request keeps its first verdict.
-- * A signature (kentos.dossier_signature) is one verified signature on one file: the signer as the server read the
--   certificate, the digest signed, the signature and the certificate as delivered (for a later chain and revocation
--   check), and the file's sha256 at the time, so a later change of the file shows.
-- * Both sides of the dossier read its signatures; a request is its requester's.

create table kentos.signing_request (
  id uuid primary key,
  batch_id uuid not null,
  dossier_id uuid not null references kentos.dossier (id) on delete cascade,
  requested_by uuid not null references kentos.app_user (id) on delete cascade,
  requested_by_name text not null,
  side text not null check (side in ('municipality', 'professional')),
  client_type text not null check (client_type in ('WEB', 'DESKTOP', 'MOBILE')),
  payload jsonb not null check (jsonb_typeof(payload) = 'object'),
  state text not null default 'issued'
    check (state in ('issued', 'completed', 'partially', 'failed', 'cancelled', 'expired')),
  created_at timestamptz not null default now(),
  expires_at timestamptz not null,
  delivered_at timestamptz,
  verdict jsonb
);
create index signing_request_dossier on kentos.signing_request (dossier_id, created_at desc);

alter table kentos.signing_request enable row level security;
create policy signing_request_own on kentos.signing_request for all
  using (requested_by = kentos.current_user_id())
  with check (requested_by = kentos.current_user_id() and kentos.dossier_side(dossier_id) is not null);

create table kentos.dossier_signature (
  id uuid primary key,
  dossier_id uuid not null references kentos.dossier (id) on delete cascade,
  file_id uuid not null references kentos.dossier_file (id) on delete cascade,
  request_id uuid not null references kentos.signing_request (id) on delete cascade,
  side text not null check (side in ('municipality', 'professional')),
  signer_user_id uuid references kentos.app_user (id) on delete set null,
  signer_name text not null,
  subject text not null,
  issuer text not null,
  serial_number text not null,
  fingerprint_sha256 text not null check (fingerprint_sha256 ~ '^[0-9A-F]{64}$'),
  key_algorithm text not null,
  signature_algorithm text not null,
  digest_algorithm text not null,
  digest text not null check (digest ~ '^[0-9a-f]{64,128}$'),
  file_sha256 text not null check (file_sha256 ~ '^[0-9a-f]{64}$'),
  signature_value text not null,
  certificate_der text not null,
  signed_at timestamptz not null,
  verified_at timestamptz not null default now()
);
create index dossier_signature_dossier on kentos.dossier_signature (dossier_id, verified_at);
create index dossier_signature_file on kentos.dossier_signature (file_id);

alter table kentos.dossier_signature enable row level security;
create policy dossier_signature_read on kentos.dossier_signature for select
  using (kentos.dossier_side(dossier_id) is not null);
create policy dossier_signature_add on kentos.dossier_signature for insert
  with check (signer_user_id = kentos.current_user_id() and kentos.dossier_side(dossier_id) is not null);

grant select, insert, update on kentos.signing_request to kentos_cad_app;
grant select, insert on kentos.dossier_signature to kentos_cad_app;
