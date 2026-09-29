import type { Vec2 } from '../model/geometry';

/**
 * The blocks in the app (docs/adr/0144 §6): the window that names a block
 * Blok oluştur has picked, and what the session keeps for it. The window is
 * loaded when first opened (ui/blocks/BlockDefineDialog.ts).
 */
export interface BlockService {
  /** Opens the window that names a new block of the objects `uids` with the base point `base`. */
  define(base: Vec2, uids: readonly string[]): void;
  /** “Seçilenleri blokla değiştir”: the window's last choice, for as long as the app lives. */
  replace: boolean;
}

/** The service; `open` loads and opens the window. */
export function createBlocks(open: (base: Vec2, uids: readonly string[]) => void): BlockService {
  return { define: open, replace: true };
}
