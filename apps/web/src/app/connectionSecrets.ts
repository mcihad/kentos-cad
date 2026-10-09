import type { ConnectionSecret } from '../contracts/generated/ConnectionSecret';
import { Signal } from '../core/signal';

/**
 * The connections' secrets on this device (docs/adr/0208 §12; the desktop's `services/secrets.rs`): an API key, a
 * user's password, a token, a client's secret. They are never written into a drawing (the project names a
 * connection, its origin and how it proves itself); they live in this browser's storage under
 * `kentos.connections.v1`, each under its connection's origin and id, so two projects naming the same connection use
 * the one secret. A stored value that does not read is set aside under `kentos.connections.v1#unreadable-<time>`,
 * never written over.
 */

export const SECRETS_KEY = 'kentos.connections.v1';
const FORMAT = 'kentos.connection-secrets';

/** One secret under its connection's origin. */
export interface SecretEntry {
  readonly origin: string;
  readonly secret: ConnectionSecret;
}

/** Where the secrets are kept: the browser's local storage in the app, memory in tests. */
export interface SecretsStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function readEntries(storage: SecretsStorage | null): SecretEntry[] {
  if (!storage) return [];
  let raw: string | null = null;
  try {
    raw = storage.getItem(SECRETS_KEY);
  } catch {
    return [];
  }
  if (raw === null) return [];
  try {
    const v = JSON.parse(raw) as { format?: unknown; entries?: unknown };
    if (v.format !== FORMAT || !Array.isArray(v.entries)) throw new Error('format');
    return (v.entries as SecretEntry[]).filter((e) => typeof e?.origin === 'string' && typeof e?.secret?.id === 'string');
  } catch {
    try {
      storage.setItem(`${SECRETS_KEY}#unreadable-${Date.now()}`, raw);
    } catch {
      // Storage full: the unreadable value stays where it was.
    }
    return [];
  }
}

export class ConnectionSecrets {
  private readonly storage: SecretsStorage | null;
  private entries: SecretEntry[];
  /** Bumped on each change: a service that failed for want of a secret tries again. */
  readonly version = new Signal(0);

  constructor(storage: SecretsStorage | null) {
    this.storage = storage;
    this.entries = readEntries(storage);
  }

  /** The secret of connection `id` at `origin`. */
  get(origin: string, id: string): ConnectionSecret | null {
    return this.entries.find((e) => e.origin === origin && e.secret.id === id)?.secret ?? null;
  }

  /** Keeps `secret` for its connection at `origin`, replacing the one there was; why not when it cannot be kept. */
  put(origin: string, secret: ConnectionSecret): string | null {
    this.entries = [...this.entries.filter((e) => !(e.origin === origin && e.secret.id === secret.id)), { origin, secret: structuredClone(secret) }];
    return this.write();
  }

  /** Forgets the secret of connection `id` at `origin`. */
  remove(origin: string, id: string): string | null {
    const before = this.entries.length;
    this.entries = this.entries.filter((e) => !(e.origin === origin && e.secret.id === id));
    return this.entries.length === before ? null : this.write();
  }

  private write(): string | null {
    this.version.update((v) => v + 1);
    if (!this.storage) return null;
    try {
      this.storage.setItem(SECRETS_KEY, JSON.stringify({ format: FORMAT, version: 1, entries: this.entries }));
      return null;
    } catch (e) {
      return `Bağlantı bilgileri bu tarayıcıda saklanamadı (${e instanceof Error ? e.message : String(e)}); yalnız bu oturumda kullanılacak.`;
    }
  }
}

/** The app's secrets in this browser's local storage (none where it is closed to the page). */
export function browserSecrets(): ConnectionSecrets {
  let storage: SecretsStorage | null = null;
  try {
    storage = globalThis.localStorage ?? null;
  } catch {
    storage = null;
  }
  return new ConnectionSecrets(storage);
}
