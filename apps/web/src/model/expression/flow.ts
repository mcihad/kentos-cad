import { op } from '../../wasm/core';
import type { ExprDiagnostic, ExprField } from './builder';

/**
 * The expression's flow (docs/adr/0101), from the core
 * (crates/shared/expression/src/editor/flow): the builder's second view of
 * the same text, as nodes. The text is the flow's state: tree 0 is the
 * result's expression, the others are nodes put down and not connected yet,
 * each with its place; `?` is an input with nothing connected. `exprFlow`
 * gives the nodes, laid out; `exprFlowEdit` applies a change and gives the
 * new texts (canonical). The desktop calls the same Rust;
 * fixtures/expression/v2/flow.json pins both.
 */

/** A tree of the flow: its text and, for one not connected to the result, where its root stands. */
export interface FlowTree {
  readonly text: string;
  readonly at?: readonly [number, number];
}

export type FlowType = 'any' | 'number' | 'text' | 'bool';
export type FlowNodeKind = 'result' | 'field' | 'variable' | 'number' | 'text' | 'constant' | 'function' | 'operator' | 'keyword';

export interface FlowPort {
  /** As the signature names it: sayı, basamak; a, b; eğer 1. */
  readonly name: string;
  readonly type: FlowType;
  readonly optional: boolean;
  /** The node connected to it. */
  readonly from?: string;
  /** How a value of another type is read. */
  readonly note?: string;
  readonly removable: boolean;
  /** Its centre below the node's top edge. */
  readonly y: number;
}

export interface FlowNode {
  /** `r` for the result; else the tree and the ports from its root: 0, 0.1, 2.0.1. */
  readonly id: string;
  readonly kind: FlowNodeKind;
  readonly title: string;
  /** The key of its help: func:yuvarla, op:=, field:Ada, var:alan. */
  readonly key?: string;
  readonly type: FlowType;
  readonly ports: readonly FlowPort[];
  /** Whether an input can be added. */
  readonly grows: boolean;
  /** `değil` before içinde, arasında, gibi, benzer or after boş: whether it is there. */
  readonly negated?: boolean;
  /** Its own expression (the host previews it when `whole`). */
  readonly text: string;
  readonly whole: boolean;
  readonly error?: string;
  readonly warnings: readonly string[];
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
}

export interface Flow {
  readonly nodes: readonly FlowNode[];
  /** Each tree's text as the flow writes it. */
  readonly texts: readonly string[];
  /** The result's text does not read: the language's error. */
  readonly error?: ExprDiagnostic;
  /** Left, top, right, bottom. */
  readonly bounds: readonly [number, number, number, number];
  /** A node's rows: the title, an input, the value. */
  readonly head: number;
  readonly row: number;
  readonly value: number;
}

export type FlowEdit =
  | { readonly op: 'add'; readonly key: string; readonly at: readonly [number, number] }
  | { readonly op: 'connect'; readonly from: string; readonly to: string; readonly port: number }
  | { readonly op: 'disconnect'; readonly to: string; readonly port: number }
  | { readonly op: 'remove'; readonly node: string }
  | { readonly op: 'setNumber'; readonly node: string; readonly value: number }
  | { readonly op: 'setText'; readonly node: string; readonly value: string }
  | { readonly op: 'setBool'; readonly node: string; readonly value: boolean }
  | { readonly op: 'setNull'; readonly node: string }
  | { readonly op: 'setField'; readonly node: string; readonly name: string }
  | { readonly op: 'setVariable'; readonly node: string; readonly name: string }
  | { readonly op: 'setOperator'; readonly node: string; readonly symbol: string }
  | { readonly op: 'setFunction'; readonly node: string; readonly name: string }
  | { readonly op: 'setNegated'; readonly node: string; readonly value: boolean }
  | { readonly op: 'setFold'; readonly node: string; readonly value: boolean }
  | { readonly op: 'addPort'; readonly node: string }
  | { readonly op: 'removePort'; readonly node: string; readonly port: number }
  | { readonly op: 'move'; readonly tree: number; readonly at: readonly [number, number] };

export interface FlowEdited {
  readonly trees: readonly FlowTree[];
  /** The node to select after the change. */
  readonly focus?: string;
}

type Fields = readonly ExprField[];

export const exprFlow = op<(trees: readonly FlowTree[], fields: Fields) => Flow>('exprFlow');
/** Throws (the core's Turkish message) when the change cannot be made. */
export const exprFlowEdit = op<(trees: readonly FlowTree[], change: FlowEdit, fields: Fields) => FlowEdited>('exprFlowEdit');
