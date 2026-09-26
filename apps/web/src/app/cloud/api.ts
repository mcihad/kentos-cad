import type { ApiError } from '../../contracts/generated/ApiError';
import type { AuthConfig } from '../../contracts/generated/AuthConfig';
import type { CatalogSort } from '../../contracts/generated/CatalogSort';
import type { CatalogView } from '../../contracts/generated/CatalogView';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { CommitResult } from '../../contracts/generated/CommitResult';
import type { EventPage } from '../../contracts/generated/EventPage';
import type { FeatureConflict } from '../../contracts/generated/FeatureConflict';
import type { FeaturePage } from '../../contracts/generated/FeaturePage';
import type { Me } from '../../contracts/generated/Me';
import type { ProjectAccessChange } from '../../contracts/generated/ProjectAccessChange';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectCreate } from '../../contracts/generated/ProjectCreate';
import type { ProjectDetails } from '../../contracts/generated/ProjectDetails';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import type { ProjectList } from '../../contracts/generated/ProjectList';
import type { ProjectPage } from '../../contracts/generated/ProjectPage';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import type { ShareCandidates } from '../../contracts/generated/ShareCandidates';

/**
 * The cloud API client (Faz B, docs/adr/0006–0007). The session is an
 * HttpOnly cookie the page never sees; every request carries
 * `x-kentos-client: web`, which the server requires of browser requests
 * that change something (cross-site request forgery). Failures, network
 * ones included, become `ApiFailure` with the server's Turkish message.
 */

export class ApiFailure extends Error {
  /** HTTP status; 0 when the server could not be reached. */
  readonly status: number;
  /** The server's stable code (`conflict`, `forbidden` …) or `network`. */
  readonly code: string;
  readonly conflicts: FeatureConflict[];
  readonly requestId?: string;

  constructor(status: number, body: Partial<ApiError>, fallback: string) {
    super(body.message || fallback);
    this.status = status;
    this.code = body.error ?? (status === 0 ? 'network' : 'http');
    this.conflicts = body.conflicts ?? [];
    this.requestId = body.requestId;
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

  /** Worth retrying unchanged: no answer, a timeout, or the server/database briefly away. */
  get transient(): boolean {
    return this.status === 0 || this.status === 408 || this.status === 502 || this.status === 503 || this.status === 504;
  }
}

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
  /** One page of a view of the account's catalog, searched and counted on the server. */
  catalog(request: CatalogRequest, signal?: AbortSignal): Promise<ProjectPage>;
  /** A project's catalog entry with its object and layer counts and extent (410 in the trash). */
  details(tenant: string, project: string, signal?: AbortSignal): Promise<ProjectDetails>;
  /**
   * A catalog or lifecycle command (`project.rename` … `project.favorite`,
   * docs/adr/0028) on the project the envelope names; the answer is the command's own output.
   */
  lifecycle<T>(envelope: CommandEnvelope): Promise<T>;
}

const TIMEOUT_MS = 30_000;

export class HttpCloudApi implements CloudApi {
  private readonly fetcher: typeof fetch;

  constructor(fetcher: typeof fetch = (...a) => fetch(...a)) {
    this.fetcher = fetcher;
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
  catalog(request: CatalogRequest, signal?: AbortSignal) {
    return this.call<ProjectPage>('GET', `/v1/me/catalog?${catalogQuery(request)}`, undefined, {}, signal);
  }
  details(tenant: string, project: string, signal?: AbortSignal) {
    return this.call<ProjectDetails>('GET', `${this.base(tenant, project)}/details`, undefined, {}, signal);
  }
  lifecycle<T>(envelope: CommandEnvelope) {
    return this.call<T>('POST', `${this.base(envelope.tenantId, envelope.projectId)}/commands`, envelope);
  }
}
