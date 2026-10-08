/**
 * The annotation tools' heights (docs/adr/0205 §2): a tool's Yükseklik typed in this drawing, else the project's height
 * of its kind (`settings.annotationMm`). What was typed is the session's: another drawing opened (`replaceWith`'s
 * reset) forgets it, so a new drawing's annotations take its own project's heights. The desktop keeps the same in its
 * tool memory (`kentos_interaction::tool::Memory::heights`).
 */
import type { AnnotationKind } from '../model/annotationScale';
import type { CadDocument } from '../model/document';

const typed = new WeakMap<CadDocument, Map<AnnotationKind, number>>();

function of(doc: CadDocument): Map<AnnotationKind, number> {
  let kept = typed.get(doc);
  if (!kept) {
    const mine = new Map<AnnotationKind, number>();
    kept = mine;
    typed.set(doc, mine);
    // The document lives as long as the app: its one listener with it.
    doc.events.on('reset', () => mine.clear());
  }
  return kept;
}

/** A new annotation's height of `kind` on paper, mm: what its tool was given in this drawing, else the project's. */
export function annotationHeightMm(ctx: { readonly doc: CadDocument }, kind: AnnotationKind): number {
  return of(ctx.doc).get(kind) ?? ctx.doc.settings.annotationMm(kind);
}

/** The same in the drawing at the plot scale, metres (`mm / 1000 × scale`, the expression every tool writes with). */
export function annotationHeight(ctx: { readonly doc: CadDocument }, kind: AnnotationKind): number {
  return (annotationHeightMm(ctx, kind) / 1000) * ctx.doc.settings.plotScale.value;
}

/** A tool's Yükseklik typed in this drawing; null gives the kind back to the project's height. */
export function setAnnotationHeightMm(ctx: { readonly doc: CadDocument }, kind: AnnotationKind, mm: number | null): void {
  if (mm === null) of(ctx.doc).delete(kind);
  else of(ctx.doc).set(kind, mm);
}

/** What a tool's Yükseklik was typed as in this drawing; undefined while it is the project's. */
export function typedAnnotationHeightMm(ctx: { readonly doc: CadDocument }, kind: AnnotationKind): number | undefined {
  return of(ctx.doc).get(kind);
}
