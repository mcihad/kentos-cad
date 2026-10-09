import type { NetworkDef } from '../../contracts/generated/NetworkDef';
import type { Entity } from '../../model/entities';

/**
 * Messages between the page and the network worker (docs/adr/0209 §12). The page sends a network's objects once per
 * change of them (`build`); questions name the network and are answered in the order they were asked. Answers carry
 * what the tools draw ready-made: paths as `flags, n, x0, y0, …` (flags 0 open, 1 closed; tools/preview.ts
 * `strokePaths`), so the page only draws. Everything is structured-clone data; the numbers of the way to the cursor are
 * transferred.
 */

/** `core`: the compiled geometry core, sent with a new worker's first message (src/wasm/core.ts). */
export type NetworkRequest =
  | {
      type: 'build';
      id: number;
      network: string;
      def: NetworkDef;
      /** The network's layers' objects, each layer's in document order. */
      entities: readonly Entity[];
      /** Layer id → name, for `$katman` in the expressions. */
      layers: readonly (readonly [string, string])[];
      core?: WebAssembly.Module;
    }
  | { type: 'ask'; id: number; network: string; question: NetworkQuestion; core?: WebAssembly.Module }
  | { type: 'drop'; network: string };

export type XY = readonly [number, number];

/** The ways a route's stops may be put in a better order (docs/adr/0209 §5). */
export type NetworkReorder = 'none' | 'keepFirst' | 'keepFirstLast';
/** The traces (docs/adr/0209 §8). */
export type NetworkTraceKind = 'connected' | 'downstream' | 'upstream' | 'isolation';

export type NetworkQuestion =
  | { kind: 'locate'; at: XY; reach: number }
  | { kind: 'route'; stops: readonly XY[]; barriers: readonly XY[]; reach: number; cost: number; reorder: NetworkReorder }
  | { kind: 'treeFrom'; at: XY; reach: number; cost: number; barriers: readonly XY[] }
  | { kind: 'pathTo'; at: XY; reach: number }
  | {
      kind: 'area';
      facilities: readonly XY[];
      breaks: readonly number[];
      reach: number;
      cost: number;
      toward: boolean;
      separate: boolean;
      barriers: readonly XY[];
      trim: number;
      rings: boolean;
      areas: boolean;
    }
  | {
      kind: 'nearest';
      origins: readonly XY[];
      targets: readonly XY[];
      reach: number;
      /** Below 0: every target. */
      k: number;
      /** NaN: none. */
      cutoff: number;
      cost: number;
      reverse: boolean;
      barriers: readonly XY[];
      paths: boolean;
    }
  | { kind: 'trace'; starts: readonly XY[]; barriers: readonly XY[]; reach: number; trace: NetworkTraceKind }
  | { kind: 'check' };

/** What building a network said (the core's summary, with the expressions that did not compile). */
export interface NetworkSummary {
  readonly nodes: number;
  readonly pieces: number;
  readonly length: number;
  /** Objects of the network's layers not taken, with their kinds. */
  readonly skipped: readonly { readonly id: number; readonly kind: string }[];
  readonly short: number;
  readonly offNetwork: number;
  readonly unread: number;
  readonly problems: readonly string[];
  /** Milliseconds the worker took to build it. */
  readonly ms: number;
}

export type NetworkReply =
  | { type: 'built'; id: number; summary: NetworkSummary }
  /** The core's answer (JSON) and the paths it draws as. */
  | { type: 'answer'; id: number; json: string; paths: Float64Array; fills: Float64Array }
  | { type: 'error'; id: number; message: string };
