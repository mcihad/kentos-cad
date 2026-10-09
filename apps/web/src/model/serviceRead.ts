/**
 * A drawing's map services read from the contract's JSON form (docs/adr/0208 §2, §10): a layer's service and source
 * and the project's connections, field by field (a field the contract does not have stops the open, as the KCAD
 * readers' `unknown_field` does), then each whole by the contract's rules (`serviceRules.ts`). The reader that calls
 * these says where (`where`) and how to stop (`fail`).
 */
import type { AuthKind } from '../contracts/generated/AuthKind';
import type { FeatureFeed } from '../contracts/generated/FeatureFeed';
import type { FeedKind } from '../contracts/generated/FeedKind';
import type { ServiceConnection } from '../contracts/generated/ServiceConnection';
import type { ServiceKind } from '../contracts/generated/ServiceKind';
import type { ServiceLayer } from '../contracts/generated/ServiceLayer';
import type { TileGrid } from '../contracts/generated/TileGrid';
import { AUTH_KINDS, connectionsProblem, FEED_KINDS, feedProblem, SERVICE_KINDS, serviceProblem } from './serviceRules';

type Fail = (where: string, what: string) => never;
type Obj = Record<string, unknown>;

const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v);

/** The readers of one place: each says what the value must be. */
function readers(fail: Fail) {
  const text = (v: unknown, w: string): string => (typeof v === 'string' ? v : fail(w, 'metin olmalı'));
  const whole = (v: unknown, w: string, most = 0xffffffff): number =>
    typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= most ? v : fail(w, `0 ile ${most} arasında bir tam sayı olmalı`);
  const number = (v: unknown, w: string): number => (typeof v === 'number' && Number.isFinite(v) ? v : fail(w, 'sonlu bir sayı olmalı'));
  const flag = (v: unknown, w: string): boolean => (typeof v === 'boolean' ? v : fail(w, 'doğru/yanlış olmalı'));
  const texts = (v: unknown, w: string): string[] => (Array.isArray(v) ? v.map((x, i) => text(x, `${w} › ${i + 1}`)) : fail(w, 'metin listesi olmalı'));
  const known = (o: Obj, keys: readonly string[], w: string) => {
    const unknown = Object.keys(o).find((k) => !keys.includes(k));
    if (unknown !== undefined) fail(`${w} › ${unknown}`, 'bilinmeyen alan');
  };
  const oneOf = <T extends string>(v: unknown, values: readonly T[], w: string): T =>
    values.includes(v as T) ? (v as T) : fail(w, `şunlardan biri olmalı: ${values.join(', ')}`);
  return { text, whole, number, flag, texts, known, oneOf };
}

const SERVICE_KEYS = [
  'kind', 'url', 'layers', 'style', 'format', 'srid', 'grid', 'matrixSet', 'template', 'tileSize', 'minZoom', 'maxZoom',
  'subdomains', 'yFlip', 'transparent', 'version', 'params', 'dynamic', 'attribution', 'opacity', 'connection', 'preset',
  'bbox',
] as const;
const MATRIX_KEYS = ['id', 'resolution', 'x0', 'y0', 'tileWidth', 'tileHeight', 'matrixWidth', 'matrixHeight'] as const;
const FEED_KEYS = ['kind', 'url', 'name', 'srid', 'filter', 'bbox', 'limit', 'version', 'key', 'connection', 'fetched'] as const;
const CONNECTION_KEYS = ['id', 'name', 'origin', 'auth', 'names', 'tokenUrl', 'scope'] as const;

/** A layer's map service (`type`: the node's), its fields then its rules. */
export function readService(v: unknown, type: 'group' | 'layer', w: string, fail: Fail): ServiceLayer {
  if (type === 'group') return fail(w, 'grubun servisi olmaz; servis yalnız katmanındır');
  if (!isObj(v)) return fail(w, 'nesne olmalı');
  const r = readers(fail);
  r.known(v, SERVICE_KEYS, w);
  const s: ServiceLayer = { kind: r.oneOf<ServiceKind>(v.kind, SERVICE_KINDS, `${w} › tür`), url: r.text(v.url, `${w} › adres`) };
  for (const k of ['style', 'format', 'matrixSet', 'template', 'version', 'attribution', 'connection', 'preset'] as const)
    if (v[k] !== undefined) s[k] = r.text(v[k], `${w} › ${k}`);
  for (const k of ['srid', 'tileSize', 'minZoom', 'maxZoom'] as const) if (v[k] !== undefined) s[k] = r.whole(v[k], `${w} › ${k}`);
  for (const k of ['layers', 'subdomains'] as const) if (v[k] !== undefined) s[k] = r.texts(v[k], `${w} › ${k}`);
  for (const k of ['yFlip', 'transparent', 'dynamic'] as const) if (v[k] !== undefined) s[k] = r.flag(v[k], `${w} › ${k}`);
  if (v.opacity !== undefined) s.opacity = r.number(v.opacity, `${w} › opacity`);
  if (v.params !== undefined) {
    if (!Array.isArray(v.params)) fail(`${w} › params`, 'liste olmalı');
    s.params = (v.params as unknown[]).map((p, i) => {
      const pw = `${w} › params › ${i + 1}`;
      if (!isObj(p)) return fail(pw, 'nesne olmalı');
      r.known(p, ['name', 'value'], pw);
      return { name: r.text(p.name, `${pw} › name`), value: r.text(p.value, `${pw} › value`) };
    });
  }
  if (v.grid !== undefined) s.grid = readGrid(v.grid, `${w} › grid`, fail);
  if (v.bbox !== undefined) {
    if (!Array.isArray(v.bbox) || v.bbox.length !== 4) fail(`${w} › bbox`, 'dört sayı olmalı');
    const b = (v.bbox as unknown[]).map((x, i) => r.number(x, `${w} › bbox › ${i + 1}`));
    s.bbox = [b[0], b[1], b[2], b[3]];
  }
  const problem = serviceProblem(s);
  return problem ? fail(w, problem) : s;
}

function readGrid(v: unknown, w: string, fail: Fail): TileGrid {
  if (!isObj(v)) return fail(w, 'nesne olmalı');
  const r = readers(fail);
  r.known(v, ['srid', 'matrices'], w);
  if (!Array.isArray(v.matrices)) fail(`${w} › matrices`, 'liste olmalı');
  return {
    srid: r.whole(v.srid, `${w} › srid`),
    matrices: (v.matrices as unknown[]).map((m, i) => {
      const mw = `${w} › ${i + 1}`;
      if (!isObj(m)) return fail(mw, 'nesne olmalı');
      r.known(m, MATRIX_KEYS, mw);
      return {
        id: r.text(m.id, `${mw} › id`),
        resolution: r.number(m.resolution, `${mw} › resolution`),
        x0: r.number(m.x0, `${mw} › x0`),
        y0: r.number(m.y0, `${mw} › y0`),
        tileWidth: r.whole(m.tileWidth, `${mw} › tileWidth`),
        tileHeight: r.whole(m.tileHeight, `${mw} › tileHeight`),
        matrixWidth: r.whole(m.matrixWidth, `${mw} › matrixWidth`, Number.MAX_SAFE_INTEGER),
        matrixHeight: r.whole(m.matrixHeight, `${mw} › matrixHeight`, Number.MAX_SAFE_INTEGER),
      };
    }),
  };
}

/** Where a layer's objects came from, its fields then its rules. */
export function readFeed(v: unknown, type: 'group' | 'layer', w: string, fail: Fail): FeatureFeed {
  if (type === 'group') return fail(w, 'grubun veri kaynağı olmaz; kaynak yalnız katmanındır');
  if (!isObj(v)) return fail(w, 'nesne olmalı');
  const r = readers(fail);
  r.known(v, FEED_KEYS, w);
  const f: FeatureFeed = { kind: r.oneOf<FeedKind>(v.kind, FEED_KINDS, `${w} › tür`), url: r.text(v.url, `${w} › adres`) };
  for (const k of ['name', 'filter', 'version', 'key', 'connection', 'fetched'] as const) if (v[k] !== undefined) f[k] = r.text(v[k], `${w} › ${k}`);
  if (v.srid !== undefined) f.srid = r.whole(v.srid, `${w} › srid`);
  if (v.limit !== undefined) f.limit = r.whole(v.limit, `${w} › limit`, Number.MAX_SAFE_INTEGER);
  if (v.bbox !== undefined) {
    if (!Array.isArray(v.bbox) || v.bbox.length !== 4) fail(`${w} › bbox`, 'dört sayı olmalı');
    const b = (v.bbox as unknown[]).map((x, i) => r.number(x, `${w} › bbox › ${i + 1}`));
    f.bbox = [b[0], b[1], b[2], b[3]];
  }
  const problem = feedProblem(f);
  return problem ? fail(w, problem) : f;
}

/** The project's connections, each its fields, then the list by its rules. */
export function readConnections(v: unknown, w: string, fail: Fail): ServiceConnection[] {
  if (!Array.isArray(v)) return fail(w, 'liste olmalı');
  const r = readers(fail);
  const list = v.map((c, i): ServiceConnection => {
    const cw = `${w} › ${i + 1}`;
    if (!isObj(c)) return fail(cw, 'nesne olmalı');
    r.known(c, CONNECTION_KEYS, cw);
    const out: ServiceConnection = {
      id: r.text(c.id, `${cw} › id`),
      name: r.text(c.name, `${cw} › ad`),
      origin: r.text(c.origin, `${cw} › köken`),
      auth: r.oneOf<AuthKind>(c.auth, AUTH_KINDS, `${cw} › doğrulama`),
    };
    if (c.names !== undefined) out.names = r.texts(c.names, `${cw} › names`);
    if (c.tokenUrl !== undefined) out.tokenUrl = r.text(c.tokenUrl, `${cw} › tokenUrl`);
    if (c.scope !== undefined) out.scope = r.text(c.scope, `${cw} › scope`);
    return out;
  });
  const problem = connectionsProblem(list);
  return problem ? fail(w, problem) : list;
}
