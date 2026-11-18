// The portal's HTTP client (docs/adr/9000 §8): JSON in and out, the session cookie, the header that says the request
// comes from the app itself (the server refuses changes without it, docs/adr/0007), and the server's error as an
// exception that carries its code, its Turkish message and the field it is about.

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly path: string | null;

  constructor(status: number, code: string, message: string, path: string | null) {
    super(message);
    this.status = status;
    this.code = code;
    this.path = path;
  }
}

const HEADERS = { 'x-kentos-client': 'web' } as const;

async function failure(res: Response): Promise<ApiError> {
  let body: { error?: string; message?: string; path?: string } = {};
  try {
    body = (await res.json()) as typeof body;
  } catch {
    // Not the server's error document (a proxy's page): said by the status below.
  }
  const message =
    body.message ??
    (res.status === 503 ? 'Sunucuya ulaşılamıyor; biraz sonra yeniden deneyin.' : `İstek yerine getirilemedi (HTTP ${res.status}).`);
  return new ApiError(res.status, body.error ?? 'http', message, body.path ?? null);
}

/** A JSON request; `undefined` for an answer without a body (204). */
export async function api<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method,
    credentials: 'same-origin',
    headers: body === undefined ? HEADERS : { ...HEADERS, 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) throw await failure(res);
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

export const get = <T>(path: string) => api<T>('GET', path);
export const post = <T>(path: string, body?: unknown) => api<T>('POST', path, body ?? {});
export const put = <T>(path: string, body: unknown) => api<T>('PUT', path, body);
export const patch = <T>(path: string, body: unknown) => api<T>('PATCH', path, body);
export const del = <T>(path: string) => api<T>('DELETE', path);

/** A file's bytes as the request's body; its name and kind in the address and the content type. */
export async function upload<T>(path: string, file: Blob): Promise<T> {
  const res = await fetch(path, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { ...HEADERS, 'content-type': file.type || 'application/octet-stream' },
    body: file,
  });
  if (!res.ok) throw await failure(res);
  return (await res.json()) as T;
}

/** One part of a file (`PUT`, `application/octet-stream`); a connection that breaks is an error of status 0. */
export async function putBytes<T>(path: string, bytes: Blob): Promise<T> {
  let res: Response;
  try {
    res = await fetch(path, {
      method: 'PUT',
      credentials: 'same-origin',
      headers: { ...HEADERS, 'content-type': 'application/octet-stream' },
      body: bytes,
    });
  } catch {
    throw new ApiError(0, 'network', 'Bağlantı kesildi; yükleme kaldığı yerden sürecek.', null);
  }
  if (!res.ok) throw await failure(res);
  return (await res.json()) as T;
}

/** A query string of the defined values. */
export function query(params: Record<string, string | number | boolean | null | undefined>): string {
  const pairs = Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== '');
  if (!pairs.length) return '';
  return `?${pairs.map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`).join('&')}`;
}
