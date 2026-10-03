import type { CloudOrganization } from '../../contracts/generated/sheet/CloudOrganization';
import type { TemplateRole } from '../../contracts/generated/sheet/TemplateRole';
import { isObj, keptKey, TEMPLATES, type KeyValue } from './store';

/**
 * The user's templates on this device (docs/sheet/design.md §12, §13), in
 * IndexedDB `kentos.sheets.v1` `templates` by the template's id: the ones
 * saved here (“Şablon olarak kaydet”) and the cached copies of the
 * account's cloud library (its own, those shared with it and those of the
 * organisations it is an active member of), each with its place in that
 * library. The envelope is this code's; the template
 * inside is the engine's JSON, read and checked by the engine only. A
 * record this code cannot read is kept aside before anything replaces it.
 */

export const TEMPLATE_FORMAT = 'kentos.sheets.deviceTemplate';
export const TEMPLATE_VERSION = 1;

/**
 * A template's place in an account's cloud library (design §13): what the
 * sync plan reads (`LocalTemplate`) and what the gallery's badges say. A
 * copy of the cloud's is this device's cache of its last downloaded
 * revision (it is used offline); one waiting to be uploaded has revision 0.
 */
export interface TemplateCloudState {
  /** The account whose library it is in (another account signed in on this browser neither sees nor syncs it). */
  readonly account: string;
  /** The cloud revision this copy is based on; 0: not uploaded yet (“Buluta eşitle” asked for it). */
  readonly revision: number;
  /** Changed here since that revision. */
  readonly changed: boolean;
  /** Deleted here: kept until the cloud hears of it. */
  readonly deleted?: boolean;
  /**
   * The account's role in it: its own (`owner`), or shared with it; in an organisation's library the
   * one who published it (`owner`), the organisation's owner and administrators (`admin`), every other
   * member (`viewer`).
   */
  readonly role: TemplateRole;
  /** The organisation whose library it is in: its commands go to that organisation's route; none: the account's personal space. */
  readonly organization?: CloudOrganization;
  /** The template it was published from (“Kuruma yayımla…”), when it was. */
  readonly publishedFrom?: string;
  /** Who owns it, for one shared with the account. */
  readonly ownerName?: string;
  /** Its owner shared it with someone. */
  readonly shared?: boolean;
  /** A copy a conflict kept (“… (bu cihazdaki kopya)”): the template it was a copy of. */
  readonly conflictOf?: string;
  /** The ids it had on this device before the cloud gave it its own (sheets made from it name the old one). */
  readonly formerIds?: readonly string[];
  /** Its revision on this device when the cloud gave it its id (the cloud's numbering starts again at 1). */
  readonly formerRevision?: number;
}

/** A template saved on this device (“Şablon olarak kaydet”), or the cached copy of one of the account's cloud library (design §13). */
export interface StoredTemplate {
  readonly format: typeof TEMPLATE_FORMAT;
  readonly version: typeof TEMPLATE_VERSION;
  readonly id: string;
  readonly saved: number;
  /** The engine's `Template` (kentos.sheet.template/1) with its assets' bytes; read and checked by the engine only. */
  readonly template: unknown;
  /** Its place in a cloud library; none: on this device only. */
  readonly cloud?: TemplateCloudState;
}

export type ReadTemplate = { readonly status: 'ok'; readonly record: StoredTemplate } | { readonly status: 'unreadable'; readonly key: string; readonly reason: string };

const ROLES: readonly TemplateRole[] = ['owner', 'admin', 'editor', 'viewer'];
const text = (v: unknown): string | undefined => (typeof v === 'string' ? v : undefined);

/** The cloud state of a stored record; one this code cannot read leaves the template on this device only (its content is kept). */
function readCloud(raw: unknown): TemplateCloudState | undefined {
  if (!isObj(raw) || typeof raw.account !== 'string' || !raw.account || typeof raw.revision !== 'number' || typeof raw.changed !== 'boolean') return undefined;
  const role = ROLES.find((r) => r === raw.role);
  if (!role) return undefined;
  const former = Array.isArray(raw.formerIds) ? raw.formerIds.filter((x): x is string => typeof x === 'string') : [];
  const o = raw.organization;
  const organization = isObj(o) && typeof o.tenantId === 'string' && o.tenantId && typeof o.name === 'string' ? { tenantId: o.tenantId, name: o.name } : undefined;
  return {
    account: raw.account,
    revision: raw.revision,
    changed: raw.changed,
    role,
    ...(raw.deleted === true && { deleted: true }),
    ...(text(raw.ownerName) && { ownerName: text(raw.ownerName) }),
    ...(raw.shared === true && { shared: true }),
    ...(text(raw.conflictOf) && { conflictOf: text(raw.conflictOf) }),
    ...(former.length && { formerIds: former }),
    ...(former.length && typeof raw.formerRevision === 'number' && { formerRevision: raw.formerRevision }),
    ...(organization && { organization }),
    ...(text(raw.publishedFrom) && { publishedFrom: text(raw.publishedFrom) }),
  };
}

export function readStoredTemplate(raw: unknown, key: string): ReadTemplate {
  if (!isObj(raw) || raw.format !== TEMPLATE_FORMAT) return { status: 'unreadable', key, reason: 'kayıt bir pafta şablonu değil' };
  if (raw.version !== TEMPLATE_VERSION) return { status: 'unreadable', key, reason: `kayıt bu sürümün bilmediği ${String(raw.version)}. biçimde` };
  if (raw.id !== key || typeof raw.saved !== 'number' || !isObj(raw.template)) return { status: 'unreadable', key, reason: 'kaydın kimliği, zamanı ya da içeriği okunamadı' };
  const cloud = readCloud(raw.cloud);
  return { status: 'ok', record: { format: TEMPLATE_FORMAT, version: TEMPLATE_VERSION, id: key, saved: raw.saved, template: raw.template, ...(cloud && { cloud }) } };
}

/** The user's templates on this device. */
export class DeviceTemplateStore {
  private readonly kv: KeyValue;
  private readonly now: () => number;

  constructor(kv: KeyValue, now: () => number = Date.now) {
    this.kv = kv;
    this.now = now;
  }

  /** Every template on this device; one that cannot be read is listed as such (never dropped from the list). */
  async list(): Promise<ReadTemplate[]> {
    const keys = (await this.kv.keys(TEMPLATES)).filter((k) => !k.includes('#okunamadi-'));
    return Promise.all(keys.map(async (k) => readStoredTemplate(await this.kv.get(TEMPLATES, k), k)));
  }

  /** One template, or null when there is none under the id. */
  async get(id: string): Promise<ReadTemplate | null> {
    const raw = await this.kv.get(TEMPLATES, id);
    return raw === null || raw === undefined ? null : readStoredTemplate(raw, id);
  }

  async save(id: string, template: unknown, cloud?: TemplateCloudState): Promise<StoredTemplate> {
    const stored = await this.kv.get(TEMPLATES, id);
    if (stored !== null && stored !== undefined && readStoredTemplate(stored, id).status === 'unreadable') await this.kv.put(TEMPLATES, keptKey(id, this.now()), stored);
    const record: StoredTemplate = { format: TEMPLATE_FORMAT, version: TEMPLATE_VERSION, id, saved: this.now(), template, ...(cloud && { cloud }) };
    await this.kv.put(TEMPLATES, id, record);
    return record;
  }

  remove(id: string): Promise<void> {
    return this.kv.delete(TEMPLATES, id);
  }
}
