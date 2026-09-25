import type { CommandResult } from '../contracts/generated/CommandResult';
import type { CadDocument } from '../model/document';

/**
 * Product commands (docs/adr/0013, 0022; TODOS.md CMD-04..07): typed,
 * versioned operations from the catalog that Rust generates
 * (`contracts/generated/commandCatalog.json`). They are not the interface's
 * commands (`core/commands.ts`: open a window, pick a tool); a tool may end
 * in one, as the closed-area tool ends in `cad.polygon.create`.
 *
 * The web's handlers are TypeScript over `CadDocument`; the desktop's are
 * Rust over its own document (`crates/native/application`). Neither calls
 * the other: the contract types and the shared cases in
 * `fixtures/commands/v1` hold them together.
 */

/**
 * What a command runs against (CMD-06). Locally: the open document and
 * nothing else — no tenant, no actor, nothing to authorize. A cloud host
 * adds the actor and rights from a verified session, never from the input;
 * the server's `CommandEnvelope` stays its wire form.
 */
export interface ExecutionContext {
  /** The only way a command changes anything; validate and plan only read it. */
  readonly doc: CadDocument;
}

/**
 * One command's handler, in the flow of CMD-04: validate → plan → execute.
 * Every mode answers with a `CommandResult` (CMD-05). Everything the
 * interface knows implicitly (the active layer, the current colour) is in
 * the input (CMD-07); a handler never reads interface state.
 */
export interface ProductCommand<Input, Output, Plan> {
  /** The catalog's id (`cad.polygon.create`) and version. */
  readonly id: string;
  readonly version: number;
  /** Checks the input against the document; writes nothing (not the revision, dirty flag or history). */
  validate(cx: ExecutionContext, input: Input): CommandResult<null>;
  /** What execute would write now and the revision to expect for exactly that; writes nothing. */
  plan(cx: ExecutionContext, input: Input): CommandResult<Plan>;
  /**
   * Checks again, refuses with `conflict` when the input's expected revision
   * is not the document's, and writes one undo step through the document's
   * own edits.
   */
  execute(cx: ExecutionContext, input: Input): CommandResult<Output>;
}
