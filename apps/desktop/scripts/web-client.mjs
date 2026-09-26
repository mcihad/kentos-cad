// Another editor for the desktop's cloud live run (apps/desktop/src/cloud/live_run.rs,
// docs/adr/0041): the web's HTTP calls (`x-kentos-client: web`, as
// apps/web/src/app/cloud/api.ts sends them and apps/web/scripts/e2e/cloud.mjs
// plays its second editor), signed in with a seeded development account.
//
//   node apps/desktop/scripts/web-client.mjs whoami <login>
//   node apps/desktop/scripts/web-client.mjs move-point <login> <tenant> <project> <id> <dx>
//   node apps/desktop/scripts/web-client.mjs save-file <login> <tenant> <project>
//
// The server is KENTOS_LIVE_API; the password KENTOS_DEV_PASSWORD, which is
// never printed. Each command prints one JSON line.
const [, , command, login, ...args] = process.argv;
const base = process.env.KENTOS_LIVE_API;
const password = process.env.KENTOS_DEV_PASSWORD;
if (!base || !password) throw new Error('KENTOS_LIVE_API ve KENTOS_DEV_PASSWORD gerekli.');

let cookie = '';
async function call(method, path, body, raw) {
  const headers = { 'x-kentos-client': 'web', accept: 'application/json' };
  if (cookie) headers.cookie = cookie;
  if (raw) headers['content-type'] = 'application/octet-stream';
  else if (body !== undefined) headers['content-type'] = 'application/json';
  const res = await fetch(`${base}${path}`, { method, headers, body: raw ?? (body === undefined ? undefined : JSON.stringify(body)) });
  const set = res.headers.getSetCookie?.()[0];
  if (set) cookie = set.split(';')[0];
  const type = res.headers.get('content-type') ?? '';
  const out = type.includes('json') ? await res.json() : Buffer.from(await res.arrayBuffer());
  if (!res.ok) throw new Error(`${method} ${path}: ${res.status} ${JSON.stringify(out.message ?? out)}`);
  return out;
}

function envelope(tenantId, projectId, commandName, input, expectedVersions) {
  return {
    commandName,
    version: 1,
    tenantId,
    projectId,
    requestId: `web-${crypto.randomUUID()}`,
    idempotencyKey: crypto.randomUUID(),
    expectedVersions,
    input,
  };
}

const me = await call('POST', '/v1/auth/login', { login, password });

if (command === 'whoami') {
  console.log(JSON.stringify({ id: me.user.id, name: me.user.displayName }));
} else if (command === 'move-point') {
  const [tenant, project, id, dx] = args;
  const page = await call('GET', `/v1/tenants/${tenant}/projects/${project}/features?ids=${id}`);
  const record = page.features[0];
  record.entity.p.x += Number(dx);
  const result = await call(
    'POST',
    `/v1/tenants/${tenant}/projects/${project}/commands`,
    envelope(tenant, project, 'project.changes', { features: [{ op: 'update', id, entity: record.entity }] }, { [id]: record.version }),
  );
  console.log(JSON.stringify({ version: result.versions[id], x: record.entity.p.x }));
} else if (command === 'save-file') {
  const [tenant, project] = args;
  const list = await call('GET', `/v1/tenants/${tenant}/projects/${project}/files`);
  const current = list.current;
  const listed = list.revisions.find((r) => r.revision === current);
  const bytes = await call('GET', `/v1/tenants/${tenant}/projects/${project}/files/${current}`);
  const upload = await call('POST', `/v1/tenants/${tenant}/projects/${project}/uploads`, { size: bytes.length, sha256: listed.sha256 });
  await call('PUT', `/v1/tenants/${tenant}/projects/${project}/uploads/${upload.id}`, undefined, bytes);
  const committed = await call(
    'POST',
    `/v1/tenants/${tenant}/projects/${project}/commands`,
    envelope(tenant, project, 'project.file.commit', { uploadId: upload.id }, { '@file': current }),
  );
  console.log(JSON.stringify({ revision: committed.revision }));
} else {
  throw new Error(`bilinmeyen komut: ${command}`);
}
