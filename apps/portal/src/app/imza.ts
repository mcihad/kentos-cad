// KentOS İmza on this computer (docs/adr/9002, 9004): the local signing service the browser hands a server-signed
// request to. It listens on 127.0.0.1 only; its address is this browser's setting (Hesabım › E-imza), 32147 by
// default. The PIN never passes through the page: the service's own window asks for it (its native mode).

const KEY = 'kentos.imza.base';
export const DEFAULT_BASE = 'http://127.0.0.1:32147';

/** The service's address as this browser keeps it. */
export function imzaBase(): string {
  try {
    return localStorage.getItem(KEY) || DEFAULT_BASE;
  } catch {
    return DEFAULT_BASE;
  }
}

export function setImzaBase(base: string | null): void {
  try {
    if (base && base.trim() && base.trim() !== DEFAULT_BASE) localStorage.setItem(KEY, base.trim().replace(/\/+$/, ''));
    else localStorage.removeItem(KEY);
  } catch {
    // A browser that keeps nothing uses the default.
  }
}

/** A refusal of the service, as it words it. */
export class ImzaError extends Error {
  readonly code: string | null;

  constructor(message: string, code: string | null = null) {
    super(message);
    this.code = code;
  }
}

async function call<T>(method: 'GET' | 'POST', path: string, body?: unknown): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${imzaBase()}${path}`, {
      method,
      headers: body === undefined ? {} : { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ImzaError('KentOS İmza bu bilgisayarda çalışmıyor ya da bu sayfaya izin vermiyor. Hesabım › E-imza bölümünde kurulumu ve adresi denetleyin.', 'unreachable');
  }
  const text = await res.text();
  const data = text ? (JSON.parse(text) as Record<string, unknown>) : {};
  if (!res.ok) {
    throw new ImzaError(String(data.userMessage ?? data.technicalMessage ?? `KentOS İmza ${res.status} döndü.`), (data.code as string | undefined) ?? null);
  }
  return data as T;
}

export type ImzaHealth = { status: string; service: string; version: string; interactiveAgent: boolean; devices: number };
export type ImzaInfo = { pinEntry: 'NATIVE' | 'CLIENT'; insecurePinEntry: boolean; trustConfigured: boolean; pkcs11Loaded: boolean; mockToken?: boolean };
export type ImzaBatch = { id: string; transactionId: string; status: string; lastError: { userMessage?: string; code?: string } | null };

export const imza = {
  health: () => call<ImzaHealth>('GET', '/api/v1/health'),
  info: () => call<ImzaInfo>('GET', '/api/v1/info'),
  create: (signedRequest: string) => call<ImzaBatch>('POST', '/api/v1/signing/batches', { signedRequest }),
  open: (id: string) => call<ImzaBatch>('POST', `/api/v1/signing/batches/${id}/open`),
  // In the native mode the card, the certificate and the PIN are chosen in the service's own window.
  sign: (id: string) => call<ImzaBatch>('POST', `/api/v1/signing/batches/${id}/sign`, {}),
  get: (id: string) => call<ImzaBatch>('GET', `/api/v1/signing/batches/${id}`),
  result: (id: string) => call<unknown>('GET', `/api/v1/signing/batches/${id}/result`),
  acknowledge: (id: string, accepted: boolean, reason: string | null) =>
    call<ImzaBatch>('POST', `/api/v1/signing/batches/${id}/acknowledge`, { accepted, reason }),
  cancel: (id: string) => call<ImzaBatch>('POST', `/api/v1/signing/batches/${id}/cancel`),
};
