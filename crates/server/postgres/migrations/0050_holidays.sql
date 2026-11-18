-- The holidays a legal deadline is extended over (docs/adr/9000 §19; plan BUL-16). A deadline whose last day falls on
-- a day the offices do not work runs to the first working day after it.
--
-- * The fixed holidays of 2429 sayılı Kanun (1 January, 23 April, 1 May, 19 May, 15 July, 30 August, 29 October) and
--   the weekend are the application's calendar; they never change from year to year.
-- * The religious holidays (Ramazan and Kurban bayramı) move with the lunar calendar: they are this table, which the
--   platform's administrators keep (a year is added when its dates are announced). A half day (the arife, the day
--   before a bayram) is a working day for a deadline: the offices work its morning.
-- * Everyone reads it; only the platform's administrators write it (holiday_set, holiday_remove), and each change
--   is in the platform's record.

create table kentos.holiday (
  day date primary key,
  name text not null check (length(name) between 2 and 80),
  half_day boolean not null default false,
  created_at timestamptz not null default now()
);

alter table kentos.holiday enable row level security;
create policy holiday_read on kentos.holiday for select using (true);
grant select on kentos.holiday to kentos_cad_app;

-- Written only through these, which check the caller and keep the platform's record of the change.
create function kentos.holiday_set(p_day date, p_name text, p_half_day boolean) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  insert into kentos.holiday (day, name, half_day) values (p_day, p_name, p_half_day)
    on conflict (day) do update set name = excluded.name, half_day = excluded.half_day;
  perform kentos.platform_log('holiday_set', 'setting', null, p_day::text || ' ' || p_name,
                              jsonb_build_object('day', p_day, 'halfDay', p_half_day));
  return null;
end $$;

create function kentos.holiday_remove(p_day date) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  gone text;
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  delete from kentos.holiday where day = p_day returning name into gone;
  if gone is null then
    return 'not_found';
  end if;
  perform kentos.platform_log('holiday_removed', 'setting', null, p_day::text || ' ' || gone,
                              jsonb_build_object('day', p_day));
  return null;
end $$;

revoke all on function kentos.holiday_set(date, text, boolean) from public;
revoke all on function kentos.holiday_remove(date) from public;
grant execute on function kentos.holiday_set(date, text, boolean), kentos.holiday_remove(date) to kentos_cad_app;

-- 2026 and 2027 as the Diyanet İşleri Başkanlığı announced them.
insert into kentos.holiday (day, name, half_day) values
  ('2026-03-19', 'Ramazan Bayramı arifesi', true),
  ('2026-03-20', 'Ramazan Bayramı 1. gün', false),
  ('2026-03-21', 'Ramazan Bayramı 2. gün', false),
  ('2026-03-22', 'Ramazan Bayramı 3. gün', false),
  ('2026-05-26', 'Kurban Bayramı arifesi', true),
  ('2026-05-27', 'Kurban Bayramı 1. gün', false),
  ('2026-05-28', 'Kurban Bayramı 2. gün', false),
  ('2026-05-29', 'Kurban Bayramı 3. gün', false),
  ('2026-05-30', 'Kurban Bayramı 4. gün', false),
  ('2027-03-08', 'Ramazan Bayramı arifesi', true),
  ('2027-03-09', 'Ramazan Bayramı 1. gün', false),
  ('2027-03-10', 'Ramazan Bayramı 2. gün', false),
  ('2027-03-11', 'Ramazan Bayramı 3. gün', false),
  ('2027-05-15', 'Kurban Bayramı arifesi', true),
  ('2027-05-16', 'Kurban Bayramı 1. gün', false),
  ('2027-05-17', 'Kurban Bayramı 2. gün', false),
  ('2027-05-18', 'Kurban Bayramı 3. gün', false),
  ('2027-05-19', 'Kurban Bayramı 4. gün', false);
