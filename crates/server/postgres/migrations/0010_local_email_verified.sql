-- KentOS CAD: a local account's e-mail is set by an administrator, so it
-- counts as verified (docs/adr/0035). The database keeps that rule for
-- every path that writes a local account, instead of each path setting it.

create function kentos.local_email_verified() returns trigger
  language plpgsql set search_path = pg_catalog, pg_temp
  as $$
begin
  if new.issuer = 'kentos:local' then
    new.email_verified := new.email is not null;
  end if;
  return new;
end $$;

create trigger app_user_local_email before insert or update on kentos.app_user
  for each row execute function kentos.local_email_verified();
