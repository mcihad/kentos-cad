import { op } from '../../wasm/core';
import type { ExprValue } from './expressionLib';

/**
 * The expression builder's language services, from the core
 * (crates/shared/expression/src/editor, docs/adr/0100 §5): the tokens'
 * classes for the colours, the error and warnings with their spans,
 * completion at the cursor, the call's signature, the matching
 * parenthesis, the builder's tree and each entry's help, a field's values,
 * how an entry goes into the text and how a value is previewed. The
 * desktop's builder calls the same Rust; fixtures/expression/v2/builder.json
 * pins both. Positions are UTF-16 units from 0, as a text field counts.
 */

export type ExprFieldType = 'text' | 'number' | 'bool' | 'date';
export type ExprFieldSource = 'attribute' | 'user' | 'builtin';

/** A field of the objects an expression runs on, as the host knows it. */
export interface ExprField {
  readonly name: string;
  readonly type: ExprFieldType;
  /** Today's text attributes are `attribute` (the default). */
  readonly source?: ExprFieldSource;
  readonly description?: string;
}

export type ExprTokenClass = 'number' | 'text' | 'field' | 'variable' | 'function' | 'keyword' | 'constant' | 'operator' | 'paren' | 'comma' | 'unknown';

export interface ExprToken {
  readonly class: ExprTokenClass;
  readonly start: number;
  readonly end: number;
}

/** What is wrong or doubtful and the span to underline (`start === end` at the end of the text). */
export interface ExprDiagnostic {
  readonly start: number;
  readonly end: number;
  readonly message: string;
  /** "8. karakterde: …" */
  readonly text: string;
}

export interface ExprCheck {
  readonly error?: ExprDiagnostic;
  readonly warnings: readonly ExprDiagnostic[];
}

export type ExprItemKind = 'field' | 'variable' | 'function' | 'operator' | 'keyword';

/** An entry of the completion list or of the tree. */
export interface ExprItem {
  readonly kind: ExprItemKind;
  readonly label: string;
  readonly detail: string;
  readonly insert: string;
  /** Where the cursor goes in `insert`. */
  readonly caret: number;
  /** The key of its help (`exprHelp`). */
  readonly key: string;
  /** The other name completion found it by (`round` for yuvarla). */
  readonly alias?: string;
}

/** The entries that can stand at the cursor and the span they replace. */
export interface ExprCompletion {
  readonly start: number;
  readonly end: number;
  readonly items: readonly ExprItem[];
}

export interface ExprSignatureArg {
  readonly name: string;
  readonly description: string;
  readonly optional: boolean;
  /** Where it is in the signature's text. */
  readonly start: number;
  readonly end: number;
}

/** The call the cursor is in and the argument being written (absent past the last). */
export interface ExprSignature {
  readonly key: string;
  readonly name: string;
  readonly signature: string;
  readonly description: string;
  readonly args: readonly ExprSignatureArg[];
  readonly active?: number;
  readonly open: number;
}

/** A parenthesis beside the cursor and its pair (absent when it has none). */
export interface ExprBracket {
  readonly at: number;
  readonly partner?: number;
}

export interface ExprHelp {
  readonly key: string;
  readonly kind: ExprItemKind;
  readonly title: string;
  readonly group: string;
  readonly signature: string;
  readonly description: string;
  readonly args: readonly { readonly name: string; readonly description: string; readonly optional: boolean }[];
  readonly examples: readonly { readonly expression: string; readonly result: string }[];
  readonly aliases: readonly string[];
  /** A field's type and source. */
  readonly type?: ExprFieldType;
  readonly source?: ExprFieldSource;
}

/** A group of the builder's tree. */
export interface ExprSection {
  readonly group: string;
  readonly title: string;
  readonly items: readonly ExprItem[];
}

export interface ExprValueItem {
  readonly text: string;
  readonly insert: string;
}

export interface ExprPlaced {
  readonly text: string;
  readonly caret: number;
}

type Fields = readonly ExprField[];

export const exprTokens = op<(source: string) => ExprToken[]>('exprTokens');
export const exprCheck = op<(source: string, fields: Fields) => ExprCheck>('exprCheck');
/** Without `explicit` only where a name is being written; with it (Ctrl+Space) everywhere. */
export const exprComplete = op<(source: string, cursor: number, fields: Fields, explicit: boolean) => ExprCompletion | null>('exprComplete');
export const exprSignature = op<(source: string, cursor: number) => ExprSignature | null>('exprSignature');
export const exprBracket = op<(source: string, cursor: number) => ExprBracket | null>('exprBracket');
export const exprHelp = op<(key: string, fields: Fields) => ExprHelp | null>('exprHelp');
export const exprHelpAt = op<(source: string, cursor: number, fields: Fields) => ExprHelp | null>('exprHelpAt');
/** The tree's groups in order; `query` keeps the entries whose names hold it. */
export const exprBuilderCatalog = op<(fields: Fields, query: string) => ExprSection[]>('exprBuilderCatalog');
/** The text after placing an entry over `start..end`: a function wraps the selection, an operator keeps single spaces. */
export const exprPlace = op<(source: string, start: number, end: number, kind: ExprItemKind, insert: string, caret: number) => ExprPlaced>('exprPlace');
/** A field's distinct values in the builder's order, each with how the expression writes it. */
export const exprValues = op<(values: readonly string[], type: ExprFieldType) => ExprValueItem[]>('exprValues');
/** A value as the preview shows it: 'text', 12.5, doğru, boş. */
export const exprPreview = op<(value: ExprValue) => string>('exprPreview');
