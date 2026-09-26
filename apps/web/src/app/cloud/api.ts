import type { ApiError } from '../../contracts/generated/ApiError';
import type { AuthConfig } from '../../contracts/generated/AuthConfig';
import type { CatalogSort } from '../../contracts/generated/CatalogSort';
import type { CatalogView } from '../../contracts/generated/CatalogView';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { CommitResult } from '../../contracts/generated/CommitResult';
import type { EventPage } from '../../contracts/generated/EventPage';
import type { FeatureConflict } from '../../contracts/generated/FeatureConflict';
import type { FeaturePage } from '../../contracts/generated/FeaturePage';
import type { FileRevisions } from '../../contracts/generated/FileRevisions';
import type { FileUpload } from '../../contracts/generated/FileUpload';
import type { FileUploadBegin } from '../../contracts/generated/FileUploadBegin';
import type { InvitationAccept } from '../../contracts/generated/InvitationAccept';
import type { InvitationAccepted } from '../../contracts/generated/InvitationAccepted';
import type { Me } from '../../contracts/generated/Me';
import type { ProjectAccessChange } from '../../contracts/generated/ProjectAccessChange';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectCheckpoints } from '../../contracts/generated/ProjectCheckpoints';
import type { ProjectCreate } from '../../contracts/generated/ProjectCreate';
import type { ProjectDetails } from '../../contracts/generated/ProjectDetails';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import type { ProjectInvitations } from '../../contracts/generated/ProjectInvitations';
import type { ProjectList } from '../../contracts/generated/ProjectList';
import type { ProjectPage } from '../../contracts/generated/ProjectPage';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import type { ShareCandidates } from '../../contracts/generated/ShareCandidates';
import { DOWNLOAD_WAITS_MS, downloadFile } from './download';

/**
 * The cloud API client (Faz B, docs/adr/0006–0007). The session is an
 * HttpOnly cookie the page never sees; every request carries
 * `x-kentos-client: web`, which the server requires of browser requests
 * that change something (cross-site request forgery). Failures, network
 * ones included, become `ApiFailure` with the server's Turkish message.
 *
 * Files (docs/adr/0031, 0033, 0034): an upload's bytes go in one `PUT`
 * whose progress is heard (`XMLHttpRequest`: `fetch` cannot report what it
 * sent); a `.kcad` comes back as bytes, read in parts with progress, with
 * what its headers say (SHA-256, revision, event cursor, file name). A
 * download gives up only when no byte arrived for half a minute, however
 * long the whole file takes.
 */

export class ApiFailure extends Error {
  /** HTTP status; 0 when the server could not be reached. */
  readonly status: number;
  /** The server's stable code (`conflict`, `forbidden` …) or `network`. */
  readonly code: string;
  readonly conflicts: FeatureConflict[];
  readonly requestId?: string;
  /** The field the error is about, when the server knows it (`name`, `tags[2]`, `q`; TODOS.md ARCH-07). */
  readonly path?: string;
  /** The project's data revision now, as decimal text, when the error depends on it (a conflict). */
  readonly revision?: string;
  /** The server says the same request may go again unchanged. */
  readonly retryable: boolean;
  /** Seconds the server asks to wait before that (a rate limit). */
  readonly retryAfter?: number;

  constructor(status: number, body: Partial<ApiError>, fallback: string) {
    super(body.message || fallback);
    this.status = status;
    this.code = body.error ?? (status === 0 ? 'network' : 'http');
    this.conflicts = body.conflicts ?? [];
    this.requestId = body.requestId;
    this.path = body.path;
    this.revision = body.revision;
    this.retryable = body.retryable === true;
    this.retryAfter = typeof body.retryAfter === 'number' && body.retryAfter >= 0 ? body.retryAfter : undefined;
  }

  /** The project was deleted (410, it is in the trash): nothing more can be read from it or saved to it. */
  get deleted(): boolean {
    return this.code === 'project_deleted';
  }

  /** The project is archived (409): it can be read, not changed, until it is unarchived (docs/adr/0028). */
  get archived(): boolean {
    return this.code === 'project_archived';
  }

  /**
   * Not there for this account (404): it never existed, or its access was
   * taken away. The server answers both alike (docs/adr/0015).
   */
  get notFound(): boolean {
    return this.code === 'not_found';
  }

  /**
   * Worth retrying unchanged: the server says so (a rate limit, the
   * database briefly away), or no answer came from it at all — no
   * connection, a timeout, a gateway in front of it.
   */
  get transient(): boolean {
    return this.retryable || this.status === 0 || this.status === 408 || this.status === 502 || this.status === 503 || this.status === 504;
  }
}

/** How far a transfer is: bytes so far, of all (0 when the total is not known). */
export type Transfer = (done: number, total: number) => void;

/** A `.kcad` file as the server sent it, with what its headers said. */
export interface KcadDownload {
  bytes: Uint8Array<ArrayBuffer>;
  /** The server's SHA-256 of the bytes (`ETag`), lowercase hex; null when it gave none. */
  sha256: string | null;
  /** `X-Kentos-Revision`: the file revision, or the data revision of a snapshot. */
  revision: string | null;
  /** `X-Kentos-Event-Cursor`: the event cursor of a snapshot's moment. */
  cursor: string | null;
  /** The file name the server suggests. */
  fileName: string | null;
}

/** Sends an upload's bytes (`PUT`); the browser's is `XMLHttpRequest`, tests pass their own. */
export type BytesSender = (url: string, bytes: Uint8Array, headers: Record<string, string>, progress: Transfer, signal?: AbortSignal) => Promise<{ status: number; text: string }>;

/** One page of a view of the account's catalog (`GET /v1/me/catalog`, docs/adr/0028). */
export interface CatalogRequest {
  view: CatalogView;
  /** The workspace of the `organization` view; for the others, keeps the list to it. */
  tenant?: string;
  /** Words that must all be in the name, description or tags. */
  q?: string;
  type?: ProjectType;
  sort?: CatalogSort;
  limit?: number;
  /** The `next` of the page before. */
  after?: string;
}

/** The query of a catalog request, without its empty parts. */
export function catalogQuery(r: CatalogRequest): string {
  const q = new URLSearchParams({ view: r.view });
  if (r.tenant) q.set('tenant', r.tenant);
  if (r.q?.trim()) q.set('q', r.q.trim());
  if (r.type) q.set('type', r.type);
  if (r.sort) q.set('sort', r.sort);
  if (r.limit) q.set('limit', String(r.limit));
  if (r.after) q.set('after', r.after);
  return q.toString();
}

/** What the rest of the app needs from the server (tests pass a fake). */
export interface CloudApi {
  authConfig(): Promise<AuthConfig>;
  me(): Promise<Me>;
  login(login: string, password: string): Promise<Me>;
  logout(): Promise<void>;
  projects(tenant: string): Promise<ProjectList>;
  /** “Projelerim”: the account's own projects and the ones shared with it, in every workspace. */
  myProjects(): Promise<ProjectList>;
  createProject(tenant: string, input: ProjectCreate, idempotencyKey: string): Promise<ProjectInfo>;
  project(tenant: string, project: string): Promise<ProjectInfo>;
  /** Moves a project to the trash for everyone (the `DELETE` route; the app sends `project.trash` through `lifecycle`). */
  deleteProject(tenant: string, project: string): Promise<void>;
  features(tenant: string, project: string, after: string | null, limit: number, signal?: AbortSignal): Promise<FeaturePage>;
  featuresById(tenant: string, project: string, ids: readonly string[]): Promise<FeaturePage>;
  command(envelope: CommandEnvelope): Promise<CommitResult>;
  events(tenant: string, project: string, after: string): Promise<EventPage>;
  /** Who may use a project and why (needs `project.share`). */
  access(tenant: string, project: string): Promise<ProjectAccessList>;
  /** People the project can be shared with whose name or e-mail holds every word of `query` (needs `project.share`). */
  candidates(tenant: string, project: string, query: string, signal?: AbortSignal): Promise<ShareCandidates>;
  /** `project.share` or `project.access.revoke`: the same command route, their own answer. */
  accessCommand(envelope: CommandEnvelope): Promise<ProjectAccessChange>;
  /** A project's invitations, newest first: the waiting ones and those of the last 30 days (`project.share`, docs/adr/0035). */
  invitations(tenant: string, project: string, signal?: AbortSignal): Promise<ProjectInvitations>;
  /** The signed-in account takes the invitation of a link's token (`POST /v1/invitations/accept`). */
  acceptInvitation(token: string): Promise<InvitationAccepted>;
  /** One page of a view of the account's catalog, searched and counted on the server. */
  catalog(request: CatalogRequest, signal?: AbortSignal): Promise<ProjectPage>;
  /** A project's catalog entry with its object and layer counts and extent (410 in the trash). */
  details(tenant: string, project: string, signal?: AbortSignal): Promise<ProjectDetails>;
  /**
   * A product command other than `project.changes` on the project the
   * envelope names (the catalog's, `project.file.commit`, `project.import`,
   * the checkpoints'); the answer is the command's own output.
   */
  lifecycle<T>(envelope: CommandEnvelope): Promise<T>;
  /** Opens an upload of a file of this size and SHA-256 (`POST …/uploads`, docs/adr/0031). */
  beginUpload(tenant: string, project: string, begin: FileUploadBegin): Promise<FileUpload>;
  /**
   * Sends an upload's bytes; the answer is the upload once the server
   * verified them. With `offset`, one part of the file going on from the
   * bytes that arrived (`?offset=N`, at most 32 MiB; docs/adr/0045): the
   * answer says how many arrived (`receivedBytes`), and the last part's is
   * the verified upload.
   */
  sendUpload(tenant: string, project: string, upload: string, bytes: Uint8Array, progress?: Transfer, signal?: AbortSignal, offset?: number): Promise<FileUpload>;
  /** One of the caller's own uploads as it stands: whether its bytes arrived (`GET …/uploads/{id}`, docs/adr/0040). */
  uploadState(tenant: string, project: string, upload: string, signal?: AbortSignal): Promise<FileUpload>;
  /** A file project's revisions, newest first (`GET …/files`). */
  fileRevisions(tenant: string, project: string, signal?: AbortSignal): Promise<FileRevisions>;
  /** One revision's bytes (`GET …/files/{n}`). */
  fileRevision(tenant: string, project: string, revision: string, progress?: Transfer, signal?: AbortSignal): Promise<KcadDownload>;
  /** A database project as one `.kcad` of one moment (`GET …/snapshot`, docs/adr/0033). */
  snapshot(tenant: string, project: string, progress?: Transfer, signal?: AbortSignal): Promise<KcadDownload>;
  /** A project's checkpoints, newest first (`GET …/checkpoints`, docs/adr/0034). */
  checkpoints(tenant: string, project: string, signal?: AbortSignal): Promise<ProjectCheckpoints>;
  /** A checkpoint's file (`GET …/checkpoints/{id}`). */
  checkpointFile(tenant: string, project: string, checkpoint: string, progress?: Transfer, signal?: AbortSignal): Promise<KcadDownload>;
}

const TIMEOUT_MS = 30_000;
/** An upload's `PUT` may take as long as the server allows it (docs/adr/0031: ten minutes). */
const UPLOAD_TIMEOUT_MS = 10 * 60_000;

/** A failure from a body the server sent as text (JSON when it is the server's own). */
export function failureOf(status: number, text: string): ApiFailure {
  let body: Partial<ApiError> = {};
  try {
    body = (JSON.parse(text) ?? {}) as Partial<ApiError>;
  } catch {
    // A proxy's page, or nothing: the status says enough.
  }
  return new ApiFailure(status, body, `Sunucu ${status} yanıtı verdi.`);
}

/** The browser's upload: `XMLHttpRequest`, whose upload reports its progress. */
export const xhrSender: BytesSender = (url, bytes, headers, progress, signal) =>
  new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open('PUT', url);
    xhr.withCredentials = true;
    xhr.timeout = UPLOAD_TIMEOUT_MS;
    for (const [k, v] of Object.entries(headers)) xhr.setRequestHeader(k, v);
    xhr.upload.onprogress = (e) => progress(e.loaded, e.lengthComputable ? e.total : bytes.byteLength);
    const stop = () => xhr.abort();
    signal?.addEventListener('abort', stop);
    const done = () => signal?.removeEventListener('abort', stop);
    xhr.onload = () => {
      done();
      resolve({ status: xhr.status, text: xhr.responseText });
    };
    xhr.onerror = xhr.ontimeout = () => {
      done();
      reject(new ApiFailure(0, { error: 'network' }, 'Sunucuya ulaşılamadı; dosyanın gönderimi yarıda kaldı.'));
    };
    xhr.onabort = () => {
      done();
      reject(new ApiFailure(499, { error: 'aborted' }, 'Gönderim durduruldu.'));
    };
    xhr.send(bytes as Uint8Array<ArrayBuffer>);
  });

/** The quoted value of a header, unquoted (`"abc"` → `abc`). */

/** The file name of a `Content-Disposition`: its UTF-8 form when it has one. */
export function dispositionName(v: string | null): string | null {
  if (!v) return null;
  const utf8 = /filename\*=UTF-8''([^;]+)/i.exec(v);
  if (utf8) {
    try {
      return decodeURIComponent(utf8[1]);
    } catch {
      // Not percent-encoding after all: the plain name below.
    }
  }
  return /filename="([^"]*)"/i.exec(v)?.[1] ?? null;
}

export class HttpCloudApi implements CloudApi {
  private readonly fetcher: typeof fetch;
  private readonly sender: BytesSender;
  /** Waits between the tries of a download cut short (tests set shorter ones). */
  downloadWaits: readonly number[] = DOWNLOAD_WAITS_MS;

  constructor(fetcher: typeof fetch = (...a) => fetch(...a), sender: BytesSender = xhrSender) {
    this.fetcher = fetcher;
    this.sender = sender;
  }

  private async call<T>(method: string, path: string, body?: unknown, headers: Record<string, string> = {}, signal?: AbortSignal): Promise<T> {
    const abort = new AbortController();
    const timer = setTimeout(() => abort.abort(), TIMEOUT_MS);
    const onAbort = () => abort.abort();
    signal?.addEventListener('abort', onAbort);
    let res: Response;
    try {
      res = await this.fetcher(path, {
        method,
        credentials: 'same-origin',
        cache: 'no-store',
        headers: { accept: 'application/json', 'x-kentos-client': 'web', ...(body === undefined ? {} : { 'content-type': 'application/json' }), ...headers },
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: abort.signal,
      });
    } catch {
      throw new ApiFailure(signal?.aborted ? 499 : 0, { error: signal?.aborted ? 'aborted' : 'network' }, signal?.aborted ? 'İstek iptal edildi.' : 'Sunucuya ulaşılamadı.');
    } finally {
      clearTimeout(timer);
      signal?.removeEventListener('abort', onAbort);
    }
    if (res.status === 204) return undefined as T;
    let data: unknown;
    try {
      data = await res.json();
    } catch {
      throw new ApiFailure(res.status, {}, `Sunucu ${res.status} yanıtı verdi.`);
    }
    if (!res.ok) throw new ApiFailure(res.status, (data ?? {}) as Partial<ApiError>, `Sunucu ${res.status} yanıtı verdi.`);
    return data as T;
  }

  private base(tenant: string, project?: string): string {
    const t = `/v1/tenants/${encodeURIComponent(tenant)}/projects`;
    return project ? `${t}/${encodeURIComponent(project)}` : t;
  }

  authConfig() {
    return this.call<AuthConfig>('GET', '/v1/auth/config');
  }
  me() {
    return this.call<Me>('GET', '/v1/me');
  }
  login(login: string, password: string) {
    return this.call<Me>('POST', '/v1/auth/login', { login, password });
  }
  logout() {
    return this.call<void>('POST', '/v1/auth/logout');
  }
  projects(tenant: string) {
    return this.call<ProjectList>('GET', this.base(tenant));
  }
  myProjects() {
    return this.call<ProjectList>('GET', '/v1/me/projects');
  }
  createProject(tenant: string, input: ProjectCreate, idempotencyKey: string) {
    return this.call<ProjectInfo>('POST', this.base(tenant), input, { 'idempotency-key': idempotencyKey });
  }
  project(tenant: string, project: string) {
    return this.call<ProjectInfo>('GET', this.base(tenant, project));
  }
  deleteProject(tenant: string, project: string) {
    return this.call<void>('DELETE', this.base(tenant, project));
  }
  features(tenant: string, project: string, after: string | null, limit: number, signal?: AbortSignal) {
    const q = new URLSearchParams({ limit: String(limit) });
    if (after) q.set('after', after);
    return this.call<FeaturePage>('GET', `${this.base(tenant, project)}/features?${q}`, undefined, {}, signal);
  }
  featuresById(tenant: string, project: string, ids: readonly string[]) {
    return this.call<FeaturePage>('GET', `${this.base(tenant, project)}/features?ids=${ids.map(encodeURIComponent).join(',')}`);
  }
  command(envelope: CommandEnvelope) {
    return this.call<CommitResult>('POST', `${this.base(envelope.tenantId, envelope.projectId)}/commands`, envelope);
  }
  events(tenant: string, project: string, after: string) {
    return this.call<EventPage>('GET', `${this.base(tenant, project)}/events?after=${encodeURIComponent(after)}`);
  }
  access(tenant: string, project: string) {
    return this.call<ProjectAccessList>('GET', `${this.base(tenant, project)}/access`);
  }
  candidates(tenant: string, project: string, query: string, signal?: AbortSignal) {
    return this.call<ShareCandidates>('GET', `${this.base(tenant, project)}/access/candidates?${new URLSearchParams({ q: query })}`, undefined, {}, signal);
  }
  accessCommand(envelope: CommandEnvelope) {
    return this.call<ProjectAccessChange>('POST', `${this.base(envelope.tenantId, envelope.projectId)}/commands`, envelope);
  }
  invitations(tenant: string, project: string, signal?: AbortSignal) {
    return this.call<ProjectInvitations>('GET', `${this.base(tenant, project)}/invitations`, undefined, {}, signal);
  }
  acceptInvitation(token: string) {
    const input: InvitationAccept = { token };
    return this.call<InvitationAccepted>('POST', '/v1/invitations/accept', input);
  }
  catalog(request: CatalogRequest, signal?: AbortSignal) {
    return this.call<ProjectPage>('GET', `/v1/me/catalog?${catalogQuery(request)}`, undefined, {}, signal);
  }
  details(tenant: string, project: string, signal?: AbortSignal) {
    return this.call<ProjectDetails>('GET', `${this.base(tenant, project)}/details`, undefined, {}, signal);
  }
  lifecycle<T>(envelope: CommandEnvelope) {
    return this.call<T>('POST', `${this.base(envelope.tenantId, envelope.projectId)}/commands`, envelope);
  }
  beginUpload(tenant: string, project: string, begin: FileUploadBegin) {
    return this.call<FileUpload>('POST', `${this.base(tenant, project)}/uploads`, begin);
  }
  async sendUpload(tenant: string, project: string, upload: string, bytes: Uint8Array, progress: Transfer = () => {}, signal?: AbortSignal, offset?: number) {
    const url = `${this.base(tenant, project)}/uploads/${encodeURIComponent(upload)}${offset === undefined ? '' : `?offset=${offset}`}`;
    const headers = { accept: 'application/json', 'content-type': 'application/octet-stream', 'x-kentos-client': 'web' };
    const r = await this.sender(url, bytes, headers, progress, signal);
    if (r.status < 200 || r.status >= 300) throw failureOf(r.status, r.text);
    try {
      return JSON.parse(r.text) as FileUpload;
    } catch {
      throw new ApiFailure(r.status, {}, 'Sunucunun yanıtı okunamadı.');
    }
  }
  uploadState(tenant: string, project: string, upload: string, signal?: AbortSignal) {
    return this.call<FileUpload>('GET', `${this.base(tenant, project)}/uploads/${encodeURIComponent(upload)}`, undefined, {}, signal);
  }
  fileRevisions(tenant: string, project: string, signal?: AbortSignal) {
    return this.call<FileRevisions>('GET', `${this.base(tenant, project)}/files`, undefined, {}, signal);
  }
  // A revision and a checkpoint never change: a cut goes on with Range (docs/adr/0045).
  fileRevision(tenant: string, project: string, revision: string, progress?: Transfer, signal?: AbortSignal) {
    return this.download(`${this.base(tenant, project)}/files/${encodeURIComponent(revision)}`, progress, signal, true);
  }
  // The snapshot is made anew for each request: a cut starts over.
  snapshot(tenant: string, project: string, progress?: Transfer, signal?: AbortSignal) {
    return this.download(`${this.base(tenant, project)}/snapshot`, progress, signal, false);
  }
  checkpoints(tenant: string, project: string, signal?: AbortSignal) {
    return this.call<ProjectCheckpoints>('GET', `${this.base(tenant, project)}/checkpoints`, undefined, {}, signal);
  }
  checkpointFile(tenant: string, project: string, checkpoint: string, progress?: Transfer, signal?: AbortSignal) {
    return this.download(`${this.base(tenant, project)}/checkpoints/${encodeURIComponent(checkpoint)}`, progress, signal, true);
  }

  /** A file's bytes (download.ts): `resumable` ones go on from what arrived after a cut. */
  private download(path: string, progress: Transfer | undefined, signal: AbortSignal | undefined, resumable: boolean): Promise<KcadDownload> {
    return downloadFile(this.fetcher, path, { progress, signal, resumable, waits: this.downloadWaits });
  }
}
