import type { ConnectionSecret } from '../../contracts/generated/ConnectionSecret';
import type { Entity } from '../../contracts/generated/Entity';
import type { FeatureFeed } from '../../contracts/generated/FeatureFeed';
import type { ReportItem } from '../../contracts/generated/ReportItem';
import type { ServiceConnection } from '../../contracts/generated/ServiceConnection';
import type { ServiceLayer } from '../../contracts/generated/ServiceLayer';

/**
 * The messages between the page's service hub (render/serviceHub.ts) and its services worker (io/services/worker.ts),
 * docs/adr/0208 §3, §8, §9. The worker resolves a service (its style, TileJSON, session or token), fetches and cuts its
 * picture tiles, builds its vector tiles at a zoom step and places their labels; the page asks for the tiles a frame
 * shows and draws them.
 */

/** A service as the page asks the worker to draw it. */
export interface Register {
  type: 'register';
  key: string;
  service: ServiceLayer;
  connection: ServiceConnection | null;
  secret: ConnectionSecret | null;
  /** The project's system and datum choices in the core's JSON; none for a project without one. */
  ours: string | null;
  choices: string;
  projectSrid: number;
  /** The project's system is its own definition (never the same as a service's). */
  custom: boolean;
  /** The drawing's anchor (the vector tiles' batches are from it). */
  origin: [number, number];
  /** The proxy may be used (signed in). */
  proxy: boolean;
  hidpi: boolean;
}

export type ToWorker =
  | Register
  /** Only these services stay. */
  | { type: 'keep'; keys: string[] }
  /** A picture tile, or a vector tile built at `step` (a quarter zoom). */
  | { type: 'tile'; key: string; level: number; col: number; row: number; step: number | null }
  /** Tiles no longer asked for: their requests dropped. */
  | { type: 'drop'; key: string; tiles: [number, number, number][] }
  /** Labels of these built tiles placed on a screen. */
  | { type: 'place'; seq: number; ids: number[]; x0: number; y0: number; pxPerUnit: number; width: number; height: number }
  /** Built tiles let go: their labels too. */
  | { type: 'unlabel'; ids: number[] }
  /** Google's credits for a view that rested. */
  | { type: 'credit'; key: string; zoom: number; south: number; west: number; north: number; east: number }
  /** The signed-in state changed. */
  | { type: 'proxy'; on: boolean }
  /**
   * Yeniden yükle and Önbelleği temizle: the service forgotten, read again when it registers next; with `fresh` each of
   * its answers is asked once past the browser's cache (`cache: 'reload'`), which replaces what the cache kept.
   */
  | { type: 'forget'; key: string; fresh: boolean };

/** A service resolved: what the page needs to draw it. */
export interface Ready {
  type: 'ready';
  key: string;
  srid: number;
  /** A vector service's tile template from its style or TileJSON. */
  template: string | null;
  /** A Google session's tile side (0: none). */
  tileSize: number;
  /** The credits' text and links. */
  credit: { text: string; links: [string, string][] };
  vector: boolean;
  /** What a style uses that KentOS does not draw. */
  notes: string[];
}

export type FromWorker =
  | Ready
  | { type: 'failed'; key: string; message: string }
  /** A picture tile cut into slots (`bytes`: the slots one after another), or empty (nothing there). */
  | { type: 'picture'; key: string; level: number; col: number; row: number; across: number; down: number; bytes: Uint8Array | null }
  /** A vector tile built: the style engine's batches (`json`, `data`) and its labels' id. */
  | { type: 'vector'; key: string; level: number; col: number; row: number; step: number; id: number; json: string; data: Float32Array }
  /** A tile that failed (tried again later). */
  | { type: 'tileFailed'; key: string; level: number; col: number; row: number; message: string; quiet: boolean }
  | { type: 'placed'; seq: number; json: string }
  | { type: 'credit'; key: string; text: string };

/**
 * Servisten veri al's take (docs/adr/0208 §10), asked of a worker of its own (io/services/feedWorker.ts): a feed's
 * objects page by page, each request with the connection's proof, moved into the project's system. The page stops a
 * take by ending its worker.
 */
export interface TakeAsk {
  type: 'take';
  feed: FeatureFeed;
  /** The area in the feed's system, `[x₁, y₁, x₂, y₂]`; none: everything. */
  area: [number, number, number, number] | null;
  most: number;
  /** A WFS gives its objects as GeoJSON. */
  geojson: boolean;
  connection: ServiceConnection | null;
  secret: ConnectionSecret | null;
  proxy: boolean;
  /** The feed's and the project's systems in the core's JSON and the datum choices; none when they are the same. */
  move: { from: string; to: string; choices: string } | null;
}

export type TakeAnswer =
  | { type: 'progress'; taken: number }
  | { type: 'taken'; entities: Entity[]; dropped: number; matched: number | null; capped: boolean; skipped: ReportItem[] }
  | { type: 'failed'; message: string };
