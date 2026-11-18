-- Municipal fees and charges (docs/adr/9003; plan BUL-22): a municipality's yearly tariff as data, and the
-- assessments (tahakkuk) it makes on dossiers.
--
-- * A tariff (kentos.fee_schedule) is a year's fees and charges: the law's fees at the amounts decided for the
--   municipality's group (2464 m.95, m.96/A) and the charges of its council's tariff (m.97); its inputs, rate tables and items live in
--   one JSON document (`definition`, the contracts' FeeDefinition), so a new fee is a new item, not a migration. A
--   municipality has at most one active tariff for a day; activating one retires the one it replaces.
-- * An assessment (kentos.dossier_fee) is the tariff applied to one dossier: its inputs, the lines it made (each with
--   what it rests on) and its total in kuruş, as issued, then paid with the receipt's number or cancelled with a
--   reason. It keeps the tariff's name, so a later tariff never changes what was issued.
-- * Both sides of the dossier see its assessments; the municipality's editors and above issue them and record
--   payments, its project managers and above cancel them (the application's rules); its admins write the tariffs.

create table kentos.fee_schedule (
  tenant_id uuid not null references kentos.tenant (id) on delete cascade,
  id uuid not null,
  name text not null check (length(name) between 3 and 120),
  year integer not null check (year between 2000 and 2200),
  valid_from date not null,
  valid_to date check (valid_to is null or valid_to >= valid_from),
  state text not null default 'draft' check (state in ('draft', 'active', 'retired')),
  decision text check (decision is null or length(decision) <= 300),
  definition jsonb not null check (jsonb_typeof(definition) = 'object'),
  created_by uuid not null references kentos.app_user (id),
  updated_by uuid references kentos.app_user (id) on delete set null,
  updated_by_name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, id)
);
create unique index fee_schedule_one_active on kentos.fee_schedule (tenant_id) where state = 'active';

alter table kentos.fee_schedule enable row level security;
-- Its members read it; its admins and owners write it.
create policy fee_schedule_read on kentos.fee_schedule for select using (kentos.member_role(tenant_id) is not null);
create policy fee_schedule_write on kentos.fee_schedule for all
  using (kentos.role_rank(kentos.member_role(tenant_id)) >= 4)
  with check (kentos.role_rank(kentos.member_role(tenant_id)) >= 4);

create table kentos.dossier_fee (
  id uuid primary key,
  dossier_id uuid not null references kentos.dossier (id) on delete cascade,
  tenant_id uuid not null references kentos.tenant (id) on delete cascade,
  schedule_id uuid not null,
  schedule_name text not null,
  inputs jsonb not null default '{}'::jsonb check (jsonb_typeof(inputs) = 'object'),
  lines jsonb not null check (jsonb_typeof(lines) = 'array'),
  total bigint not null check (total >= 0),
  payment_note text,
  state text not null default 'issued' check (state in ('issued', 'paid', 'cancelled')),
  due_on date,
  issued_by uuid not null references kentos.app_user (id),
  issued_by_name text not null,
  issued_at timestamptz not null default now(),
  paid_at date,
  receipt_no text check (receipt_no is null or length(receipt_no) between 1 and 60),
  paid_recorded_by_name text,
  cancel_reason text check (cancel_reason is null or length(cancel_reason) between 3 and 1000),
  cancelled_at timestamptz,
  check ((state = 'paid') = (receipt_no is not null and paid_at is not null)),
  check ((state = 'cancelled') = (cancel_reason is not null))
);
create index dossier_fee_dossier on kentos.dossier_fee (dossier_id, issued_at);

alter table kentos.dossier_fee enable row level security;
-- Both sides of the dossier read it; the municipality's side writes it (the roles are the application's check).
create policy dossier_fee_read on kentos.dossier_fee for select using (kentos.dossier_side(dossier_id) is not null);
create policy dossier_fee_write on kentos.dossier_fee for insert
  with check (kentos.dossier_side(dossier_id) = 'municipality' and kentos.role_rank(kentos.member_role(tenant_id)) >= 2);
create policy dossier_fee_change on kentos.dossier_fee for update
  using (kentos.dossier_side(dossier_id) = 'municipality' and kentos.role_rank(kentos.member_role(tenant_id)) >= 2)
  with check (kentos.dossier_side(dossier_id) = 'municipality');

grant select, insert, update, delete on kentos.fee_schedule to kentos_cad_app;
grant select, insert, update on kentos.dossier_fee to kentos_cad_app;
