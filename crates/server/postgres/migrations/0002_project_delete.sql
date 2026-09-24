-- KentOS CAD: deleting a cloud project (docs/adr/0006-data-layer.md,
-- "2026-09-24: bulut projesini silme").
--
-- Deleting is soft: the project row is marked (deleted_at, deleted_by) and
-- from then on hidden from lists and refused for opening and writing, but
-- nothing of it is removed. Its objects, command log, audit and events stay,
-- so an accidental deletion can be undone by the operator (`kentosd project
-- restore`). Marking is an update, which the server's role may already make
-- under row-level security; it still has no right to remove a project row.

alter table kentos.project
  add column deleted_at timestamptz,
  add column deleted_by uuid references kentos.app_user (id),
  add constraint project_deleted_by check ((deleted_at is null) = (deleted_by is null));
