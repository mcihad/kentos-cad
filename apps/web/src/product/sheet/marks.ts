/**
 * How the paper writes a value not given yet (docs/sheet/design.md §7):
 * “‹ad?›”, angle quotes every drawing face has (the core's `missing`,
 * crates/shared/sheet/src/expr.rs). The engine does not hand the mark out
 * (`EngineInfo` has no field for it), so this is the one place the web
 * writes it; the texts that explain it quote it from here.
 */
export const missingMark = (name: string): string => `‹${name}?›`;
