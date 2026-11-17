-- KentOS CAD: an extension's audit rule as a durable server job (docs/adr/8057 §42).
--
-- * One more kind: `project.audit`. The worker takes a published release of the organisation's extension catalog (else
--   the public one's), runs its rule on the project through the job runner, as the desktop runs it, and keeps the
--   report as the job's result; nothing is written to the project and there is no artifact.
-- * A second identical request (the same release and rule, the project in the same state, the same reach and the same
--   software) answers from the cache: the cache's key and the job it was taken from are the existing columns.

alter table kentos.server_job drop constraint server_job_kind_check;
alter table kentos.server_job add constraint server_job_kind_check
  check (kind in ('project.export', 'project.checkpoint', 'project.processing', 'project.import', 'project.atlas',
                  'project.audit'));
