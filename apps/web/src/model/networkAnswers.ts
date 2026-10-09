import type { EntityGeometry } from './entities';
import type { Vec2 } from './geometry';

/**
 * The network's answers (docs/adr/0209 §4–§9) as the core writes them (crates/shared/geometry-core/src/ops/network/
 * session.rs): places, routes, service areas, closest facilities, traces and the check. A field the core has no value
 * for is left out (`facility` of merged facilities, a problem's `value`). A point that finds no network is said by
 * its index; the tools and the İşlemler tools say it in words (`refusalWords`).
 */

/** A way along the network: its points and one bulge a segment (arcs exact). */
export interface NetworkLine {
  readonly pts: readonly (readonly [number, number])[];
  readonly bulges: readonly number[];
}

/** Where a point sits on the network. */
export interface NetworkPlace {
  readonly piece: number;
  readonly offset: number;
  readonly x: number;
  readonly y: number;
  /** How far the point was from it. */
  readonly d: number;
}

/** The places that may find no network, as the answers name them. */
export type PlaceError = 'stopNotFound' | 'barrierNotFound' | 'facilityNotFound' | 'originNotFound' | 'targetNotFound' | 'startNotFound';

export type NetworkRoute =
  | {
      readonly order: readonly number[];
      readonly cost: number;
      /** Each cost along the route (Uzunluk first); null where a part cannot be travelled with it. */
      readonly totals: readonly (number | null)[];
      readonly line: NetworkLine;
      readonly legs: readonly number[];
    }
  | { readonly error: PlaceError; readonly at: number }
  | { readonly error: 'tooFew' | 'tooMany' | 'noOrder' }
  | { readonly error: 'unreachable'; readonly between: readonly [number, number] };

export interface NetworkAreaLine {
  /** The facility (index); absent when the facilities are merged. */
  readonly facility?: number | null;
  readonly band: number;
  readonly line: NetworkLine;
}

export interface NetworkArea {
  readonly facility?: number | null;
  readonly band: number;
  /** A polygon (its parts and holes); absent for a band nothing reached. */
  readonly shape?: EntityGeometry | null;
}

export type NetworkServiceArea =
  | { readonly lines: readonly NetworkAreaLine[]; readonly areas: readonly NetworkArea[] }
  | { readonly error: PlaceError; readonly at: number }
  | { readonly error: 'breaks' };

export interface NetworkFound {
  readonly target: number;
  readonly cost: number;
  /** Each cost along the way (asked with its way); absent without. */
  readonly totals?: readonly (number | null)[] | null;
  readonly line?: NetworkLine | null;
}

/** For each origin its targets, cheapest first. */
export type NetworkNearest = readonly (readonly NetworkFound[])[] | { readonly error: PlaceError; readonly at: number };

export type NetworkTrace =
  | {
      /** The objects reached, by id. */
      readonly objects: readonly number[];
      /** The valves to close (Yalıtım). */
      readonly valves: readonly { readonly id: number; readonly x: number; readonly y: number }[];
      /** The objects no source feeds once those valves are closed. */
      readonly unfed: readonly number[];
      readonly length: number;
      readonly unfedLength: number;
      readonly lines: readonly NetworkLine[];
      readonly unfedLines: readonly NetworkLine[];
    }
  | { readonly error: PlaceError; readonly at: number };

export type NetworkProblemKind = 'detached' | 'nearMiss' | 'crossing' | 'offNetwork' | 'short' | 'unread';

export interface NetworkProblem {
  readonly kind: NetworkProblemKind;
  readonly x: number;
  readonly y: number;
  readonly ids: readonly number[];
  readonly value?: number | null;
  /** The cost an unread value is of (its place in the network's costs); absent for the direction. */
  readonly cost?: number | null;
}

export interface NetworkCheck {
  readonly nodes: number;
  readonly pieces: number;
  readonly length: number;
  /** Each connected part: its pieces and length, the largest first. */
  readonly parts: readonly (readonly [number, number])[];
  readonly deadEnds: number;
  readonly counts: readonly { readonly kind: NetworkProblemKind; readonly count: number }[];
  readonly problems: readonly NetworkProblem[];
}

/** A problem's kind in Denetle's words (docs/adr/0209 §9). */
export const NETWORK_PROBLEM_LABELS: Record<NetworkProblemKind, string> = {
  detached: 'Kopuk parça',
  nearMiss: 'Yakın ama bağlı değil',
  crossing: 'Bağlanmayan kesişim',
  offNetwork: 'Ağın dışında düğüm',
  short: 'Sıfır uzunluklu parça',
  unread: 'Okunamayan değer',
};

/** What a place that found no network was, in words. */
const PLACE_WORDS: Record<PlaceError, string> = {
  stopNotFound: 'durak',
  barrierNotFound: 'engel',
  facilityNotFound: 'tesis',
  originNotFound: 'başlangıç',
  targetNotFound: 'varış',
  startNotFound: 'başlangıç',
};

/** Whether an answer is a refusal. */
export const isRefusal = (a: unknown): a is { error: string } => !!a && typeof a === 'object' && !Array.isArray(a) && 'error' in a;

/** A refusal in words (`within`: the search distance as written, “20 piksel” in the tools). */
export function refusalWords(a: { error: string; at?: number; between?: readonly [number, number] }, within: string): string {
  if (a.error in PLACE_WORDS) return `${(a.at ?? 0) + 1}. ${PLACE_WORDS[a.error as PlaceError]} ağa ${within} içinde değil.`;
  switch (a.error) {
    case 'tooFew':
      return 'En az iki durak gerekir.';
    case 'tooMany':
      return 'Sıra en çok 12 durakla iyileştirilir; durakları azaltın ya da Sıra’yı kapatın.';
    case 'noOrder':
      return 'Durakları her sırayla gezen bir yol yok; bazı duraklar birbirine ulaşamıyor.';
    case 'unreachable':
      return `${(a.between?.[0] ?? 0) + 1}. ve ${(a.between?.[1] ?? 0) + 1}. duraklar arasında yol yok (kapalı kenar, tek yön ya da kopuk ağ).`;
    case 'breaks':
      return 'Aralıklar artan, sıfırdan büyük sayılar olmalı.';
    default:
      return a.error;
  }
}

/** A way's points as the drawing's. */
export const linePoints = (l: NetworkLine): Vec2[] => l.pts.map(([x, y]) => ({ x, y }));

/** A way as a polyline's geometry: its points, and its bulges when it has an arc. */
export function lineGeometry(l: NetworkLine): EntityGeometry {
  const arcs = l.bulges.some((b) => b !== 0);
  return { kind: 'polyline', pts: linePoints(l), ...(arcs && { bulges: [...l.bulges] }) } as unknown as EntityGeometry;
}
