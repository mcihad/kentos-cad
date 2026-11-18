-- Notifications as they happen (docs/adr/9000 §23; plan BUL-17): a notification written or read signals its
-- recipient's id on the channel `kentos_notification`. A server listening on it pushes the new unread count to that
-- person's open pages and programs (Server-Sent Events) instead of their asking every minute. The signal carries
-- the id alone: what it means is read again under the person's own rights.

create function kentos.notification_signal() returns trigger
  language plpgsql set search_path = pg_catalog, pg_temp
  as $$
begin
  perform pg_notify('kentos_notification', new.user_id::text);
  return null;
end $$;

create trigger notification_signal after insert or update of read_at on kentos.notification
  for each row execute function kentos.notification_signal();
