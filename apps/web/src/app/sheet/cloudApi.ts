import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { SheetTemplateAccess } from '../../contracts/generated/sheet/SheetTemplateAccess';
import type { SheetTemplateCandidates } from '../../contracts/generated/sheet/SheetTemplateCandidates';
import type { SheetTemplateChanged } from '../../contracts/generated/sheet/SheetTemplateChanged';
import type { SheetTemplateCreate } from '../../contracts/generated/sheet/SheetTemplateCreate';
import type { SheetTemplateDelete } from '../../contracts/generated/sheet/SheetTemplateDelete';
import type { SheetTemplateDetail } from '../../contracts/generated/sheet/SheetTemplateDetail';
import type { SheetTemplateEventPage } from '../../contracts/generated/sheet/SheetTemplateEventPage';
import type { SheetTemplateList } from '../../contracts/generated/sheet/SheetTemplateList';
import type { SheetTemplatePublish } from '../../contracts/generated/sheet/SheetTemplatePublish';
import type { SheetTemplateShare } from '../../contracts/generated/sheet/SheetTemplateShare';
import type { SheetTemplateUnshare } from '../../contracts/generated/sheet/SheetTemplateUnshare';
import type { SheetTemplateUpdate } from '../../contracts/generated/sheet/SheetTemplateUpdate';
import { uuidv7 } from '../../core/uuid';
import { HttpCloudApi } from '../cloud/api';

/**
 * The cloud's template library as the sheets see it (docs/sheet/design.md
 * §13; the routes and the envelope: .run/sheet-engine-ready “BULUT ŞABLON
 * KİTAPLIĞI”, “KURUM ŞABLONLARI”): reading the account's list (its own, those
 * shared with it, and the libraries of its organisations), a template, who
 * it is shared with and whom it may be shared with, the account's change
 * events (a long poll, not the socket: tasks-rust.md Sapmalar 30), and the
 * commands (`POST /v1/tenants/{alan}/commands`, the envelope's project
 * empty): a template's go to the route of the library it is in, the
 * personal space's or its organisation's (the server refuses another with
 * 422); publishing goes to the organisation's. Requests go through the app's own cloud
 * client (`HttpCloudApi`: the session cookie, `x-kentos-client: web`, the
 * server's Turkish message on a failure as `ApiFailure`); tests pass a fake.
 */

export type TemplateCommand =
  | { readonly name: 'sheet.template.create'; readonly input: SheetTemplateCreate }
  | { readonly name: 'sheet.template.update'; readonly input: SheetTemplateUpdate }
  | { readonly name: 'sheet.template.delete'; readonly input: SheetTemplateDelete }
  | { readonly name: 'sheet.template.share'; readonly input: SheetTemplateShare }
  | { readonly name: 'sheet.template.unshare'; readonly input: SheetTemplateUnshare }
  | { readonly name: 'sheet.template.publish'; readonly input: SheetTemplatePublish };

export interface TemplateCloudApi {
  /** The account's own templates and those shared with it, and its organisations' libraries, without their content. */
  list(signal?: AbortSignal): Promise<SheetTemplateList>;
  /** A template's latest revision with its content. */
  detail(id: string, signal?: AbortSignal): Promise<SheetTemplateDetail>;
  /** Who it is shared with (its owner only). */
  access(id: string, signal?: AbortSignal): Promise<SheetTemplateAccess>;
  /** People of a common organisation whose name or e-mail holds every word (its owner only; two letters at least). */
  candidates(id: string, query: string, signal?: AbortSignal): Promise<SheetTemplateCandidates>;
  /** The account's template events after `after`; `wait` seconds for one when none is new (at most 25). */
  events(after: string, wait: number, signal?: AbortSignal, limit?: number): Promise<SheetTemplateEventPage>;
  /** A command on a library's route (`tenant`: the personal space or an organisation); the same key gives the same answer again (`replayed`). */
  command(tenant: string, command: TemplateCommand, key: string): Promise<SheetTemplateChanged>;
}

/** The envelope of a template command: the library's route (the personal space or an organisation), no project, version 1 (design §13). */
export function templateEnvelope(tenant: string, command: TemplateCommand, key: string): CommandEnvelope {
  return { commandName: command.name, version: 1, tenantId: tenant, projectId: '', requestId: uuidv7(), idempotencyKey: key, expectedVersions: {}, input: command.input };
}

const q = (o: Record<string, string>) => new URLSearchParams(o).toString();

/** The library over HTTP, through the app's cloud client. */
export class HttpTemplateApi implements TemplateCloudApi {
  private readonly api: HttpCloudApi;

  constructor(api: HttpCloudApi = new HttpCloudApi()) {
    this.api = api;
  }

  list(signal?: AbortSignal) {
    return this.api.read<SheetTemplateList>('/v1/me/sheet-templates', signal);
  }
  detail(id: string, signal?: AbortSignal) {
    return this.api.read<SheetTemplateDetail>(`/v1/sheet-templates/${encodeURIComponent(id)}`, signal);
  }
  access(id: string, signal?: AbortSignal) {
    return this.api.read<SheetTemplateAccess>(`/v1/sheet-templates/${encodeURIComponent(id)}/access`, signal);
  }
  candidates(id: string, query: string, signal?: AbortSignal) {
    return this.api.read<SheetTemplateCandidates>(`/v1/sheet-templates/${encodeURIComponent(id)}/access/candidates?${q({ q: query })}`, signal);
  }
  events(after: string, wait: number, signal?: AbortSignal, limit?: number) {
    return this.api.read<SheetTemplateEventPage>(`/v1/me/sheet-templates/events?${q({ after, wait: String(wait), ...(limit ? { limit: String(limit) } : {}) })}`, signal);
  }
  command(tenant: string, command: TemplateCommand, key: string) {
    return this.api.tenantCommand<SheetTemplateChanged>(templateEnvelope(tenant, command, key));
  }
}
