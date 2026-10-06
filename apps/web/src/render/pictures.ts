/**
 * The drawing's pictures on the web's GPU (docs/adr/0192 §3), for both
 * backends, twin of crates/render/wgpu/src/styled/pictures.rs: each picture
 * its own texture, premultiplied, with its mip levels so that a photograph
 * seen small stays smooth, at most `MAX_SIDE` pixels a side (a larger one
 * is drawn smaller first). A picture the page cannot give (still decoding,
 * a linked file the browser cannot read, an image that does not decode)
 * draws light grey.
 */

/** The longest side a picture is drawn with, in pixels. */
export const MAX_SIDE = 4096;

/** Light grey, a picture's stand-in (opaque). */
export const GREY: readonly [number, number, number, number] = [200, 200, 200, 255];

/** A picture ready for a texture: no side longer than `most`. */
export interface Fitted {
  source: TexImageSource;
  width: number;
  height: number;
}

/** The picture fitted to `most` pixels a side (the device's limit and `MAX_SIDE`). */
export function fitted(img: HTMLImageElement, most: number): Fitted {
  const w = Math.max(1, img.naturalWidth);
  const h = Math.max(1, img.naturalHeight);
  const limit = Math.min(MAX_SIDE, most);
  if (w <= limit && h <= limit) return { source: img, width: w, height: h };
  const k = limit / Math.max(w, h);
  const c = document.createElement('canvas');
  c.width = Math.max(1, Math.floor(w * k));
  c.height = Math.max(1, Math.floor(h * k));
  c.getContext('2d')!.drawImage(img, 0, 0, c.width, c.height);
  return { source: c, width: c.width, height: c.height };
}

/** How many levels a picture of `w` × `h` has: halved (rounded down, as the GPU sizes them) down to one pixel. */
export function levelCount(w: number, h: number): number {
  let n = 1;
  while (w > 1 || h > 1) {
    w = Math.max(1, Math.floor(w / 2));
    h = Math.max(1, Math.floor(h / 2));
    n++;
  }
  return n;
}

/** Level `i`'s size of a picture of `w` × `h`, as the GPU sizes it. */
export function levelSize(w: number, h: number, i: number): [number, number] {
  return [Math.max(1, Math.floor(w / 2 ** i)), Math.max(1, Math.floor(h / 2 ** i))];
}

/** Level `i` of a fitted picture, drawn by the canvas at its size (WebGPU makes no levels itself). */
export function levelCanvas(p: Fitted, i: number): HTMLCanvasElement {
  const [w, h] = levelSize(p.width, p.height, i);
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  const g = c.getContext('2d')!;
  g.imageSmoothingQuality = 'high';
  g.drawImage(p.source as CanvasImageSource, 0, 0, w, h);
  return c;
}
