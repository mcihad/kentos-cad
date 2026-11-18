-- Notifications that leave the portal (docs/adr/9000 §17; plan BUL-11): an e-mail or an SMS beside the bell.
--
-- * The platform's administrator turns each channel on; until then nothing leaves. Each person chooses for
--   themselves: e-mail on by default (to a verified address only: an unverified one is whoever typed it), SMS off until
--   they give a number and turn it on.
-- * Every notification goes through kentos.notification, so one trigger there puts its deliveries in an outbox; the
--   server sends them (its own sendmail, the institution's gateway by a signed webhook, or a folder) and tries again
--   with growing pauses. A delivery that keeps failing is given up, and says why.
-- * The person sees what was sent to them; the server's sender reads and marks the outbox through two functions, never
--   the table's rows of other people.

alter table kentos.platform_setting
  add column email_notifications boolean not null default false,
  add column sms_notifications boolean not null default false;

create table kentos.notification_preference (
  user_id uuid primary key references kentos.app_user (id) on delete cascade,
  email boolean not null default true,
  sms boolean not null default false,
  -- International form without spaces (+905321234567) or a national one (05321234567).
  phone text check (phone is null or phone ~ '^\+?[0-9]{10,15}$'),
  updated_at timestamptz not null default now()
);
alter table kentos.notification_preference enable row level security;
create policy notification_preference_own on kentos.notification_preference
  using (user_id = kentos.current_user_id()) with check (user_id = kentos.current_user_id());

create table kentos.notification_delivery (
  id uuid primary key default gen_random_uuid(),
  notification_id uuid not null references kentos.notification (id) on delete cascade,
  user_id uuid not null references kentos.app_user (id) on delete cascade,
  channel text not null check (channel in ('email', 'sms')),
  address text not null,
  state text not null default 'pending' check (state in ('pending', 'sent', 'failed')),
  attempts integer not null default 0,
  last_error text check (last_error is null or length(last_error) <= 500),
  next_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  sent_at timestamptz
);
create index notification_delivery_due on kentos.notification_delivery (next_at) where state = 'pending';
create index notification_delivery_user on kentos.notification_delivery (user_id, created_at desc);
alter table kentos.notification_delivery enable row level security;
create policy notification_delivery_own on kentos.notification_delivery
  using (user_id = kentos.current_user_id());

-- A notification's deliveries, as the platform's channels and the recipient's choices allow.
create function kentos.enqueue_delivery() returns trigger
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  s record;
  p record;
begin
  select email_notifications, sms_notifications into s from kentos.platform_setting;
  if not (s.email_notifications or s.sms_notifications) then
    return new;
  end if;
  select u.status, u.email, u.email_verified, coalesce(np.email, true) as email_on, coalesce(np.sms, false) as sms_on,
         np.phone
    into p
    from kentos.app_user u left join kentos.notification_preference np on np.user_id = u.id
   where u.id = new.user_id;
  if not found or p.status <> 'active' then
    return new;
  end if;
  if s.email_notifications and p.email_on and p.email is not null and p.email_verified then
    insert into kentos.notification_delivery (notification_id, user_id, channel, address)
    values (new.id, new.user_id, 'email', p.email);
  end if;
  if s.sms_notifications and p.sms_on and p.phone is not null then
    insert into kentos.notification_delivery (notification_id, user_id, channel, address)
    values (new.id, new.user_id, 'sms', p.phone);
  end if;
  return new;
end $$;
create trigger notification_enqueue after insert on kentos.notification
  for each row execute function kentos.enqueue_delivery();

-- The server's sender takes at most p_batch due deliveries of its channels, each leased for p_lease seconds (another
-- sender skips them meanwhile), with what to say and where the notification opens.
create function kentos.take_deliveries(p_channels text[], p_batch integer, p_lease integer)
  returns table (id uuid, channel text, address text, attempts integer, recipient text, title text, body text,
                 dossier_id uuid, tenant_slug text, created_at timestamptz)
  language sql security definer set search_path = pg_catalog, pg_temp
  as $$
  with due as (
    select d.id from kentos.notification_delivery d
     where d.state = 'pending' and d.next_at <= now() and d.channel = any (p_channels)
     order by d.next_at
     limit p_batch
     for update skip locked
  ), leased as (
    update kentos.notification_delivery d
       set attempts = d.attempts + 1, next_at = now() + make_interval(secs => p_lease)
      from due where d.id = due.id
    returning d.*
  )
  select l.id, l.channel, l.address, l.attempts, u.display_name, n.title, n.body, n.dossier_id, t.slug, n.at
    from leased l
    join kentos.notification n on n.id = l.notification_id
    join kentos.app_user u on u.id = l.user_id
    left join kentos.tenant t on t.id = n.tenant_id
$$;

-- What became of a delivery: sent, or failed (tried again after p_retry seconds, given up when p_retry is null).
create function kentos.finish_delivery(p_id uuid, p_sent boolean, p_error text, p_retry integer) returns void
  language sql security definer set search_path = pg_catalog, pg_temp
  as $$
  update kentos.notification_delivery
     set state = case when p_sent then 'sent' when p_retry is null then 'failed' else 'pending' end,
         sent_at = case when p_sent then now() else sent_at end,
         last_error = case when p_sent then null else left(p_error, 500) end,
         next_at = case when p_sent or p_retry is null then next_at else now() + make_interval(secs => p_retry) end
   where id = p_id and state = 'pending'
$$;

-- Turning a channel on or off is the platform administrator's, and an event of the platform's.
create function kentos.platform_set_notifications(p_email boolean, p_sms boolean) returns text
  language plpgsql security definer set search_path = pg_catalog, pg_temp
  as $$
declare
  before record;
begin
  if not kentos.is_platform_admin() then
    return 'forbidden';
  end if;
  select email_notifications, sms_notifications into before from kentos.platform_setting;
  if before.email_notifications = p_email and before.sms_notifications = p_sms then
    return null;
  end if;
  update kentos.platform_setting
     set email_notifications = p_email, sms_notifications = p_sms, updated_at = now(),
         updated_by = kentos.current_user_id();
  perform kentos.platform_log('setting.notifications', 'setting', null, 'Bildirim kanalları',
    jsonb_build_object('email', p_email, 'sms', p_sms));
  return null;
end $$;

revoke all on function kentos.take_deliveries(text[], integer, integer) from public;
revoke all on function kentos.finish_delivery(uuid, boolean, text, integer) from public;
revoke all on function kentos.platform_set_notifications(boolean, boolean) from public;
grant execute on function kentos.take_deliveries(text[], integer, integer) to kentos_cad_app;
grant execute on function kentos.finish_delivery(uuid, boolean, text, integer) to kentos_cad_app;
grant execute on function kentos.platform_set_notifications(boolean, boolean) to kentos_cad_app;
grant select, insert, update on kentos.notification_preference to kentos_cad_app;
grant select on kentos.notification_delivery to kentos_cad_app;
