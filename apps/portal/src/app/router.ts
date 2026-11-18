// The portal's addresses (docs/adr/9000 §8): `#/kurum/ornek-belediyesi/basvurular?durum=…`. A hash keeps every page
// reachable from a link without the server knowing the portal's pages.

export type Params = Record<string, string>;

export type Match<T> = { value: T; params: Params; query: URLSearchParams };

export class Routes<T> {
  private readonly table: { parts: string[]; value: T }[] = [];

  add(pattern: string, value: T): this {
    this.table.push({ parts: pattern.split('/').filter(Boolean), value });
    return this;
  }

  /** The first pattern the address matches; `:name` parts are its parameters. */
  match(hash: string): Match<T> | null {
    const [path, search = ''] = hash.replace(/^#\/?/, '').split('?');
    const parts = path.split('/').filter(Boolean).map(decodeURIComponent);
    for (const route of this.table) {
      if (route.parts.length !== parts.length) continue;
      const params: Params = {};
      const ok = route.parts.every((p, i) => {
        if (p.startsWith(':')) {
          params[p.slice(1)] = parts[i];
          return true;
        }
        return p === parts[i];
      });
      if (ok) return { value: route.value, params, query: new URLSearchParams(search) };
    }
    return null;
  }
}

export function go(hash: string): void {
  if (location.hash === hash) window.dispatchEvent(new HashChangeEvent('hashchange'));
  else location.hash = hash;
}

/** The current address with `query` replaced, for filters kept in the address. */
export function withQuery(query: Record<string, string | null>): string {
  const [path, search = ''] = location.hash.split('?');
  const q = new URLSearchParams(search);
  for (const [k, v] of Object.entries(query)) {
    if (v === null || v === '') q.delete(k);
    else q.set(k, v);
  }
  const s = q.toString();
  return s ? `${path}?${s}` : path;
}
