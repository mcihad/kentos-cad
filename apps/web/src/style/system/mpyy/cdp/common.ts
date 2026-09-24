import type { PictogramName } from '../pictograms';
import { boxMarks, pictureMarks, stack, type BoxOptions, type PictureOptions } from '../nip/common';

export { captionMark, em, freeDots, gear, gridDots, pairs, speckle, stack, turned } from '../nip/common';

/**
 * Shared parts of the ÇDP (EK-1c) legend. Lengths are paper mm; symbols
 * are 5 mm (EK-1c AÇIKLAMA 4, EK-1a AÇIKLAMA 6). The frame and hatch
 * helpers are the NİP ones at the ÇDP symbol size.
 */

/** Symbol (frame) size. */
export const S = 5;
/** Caption under a frame (bold Times, font size). */
export const CAPTION = 1.9;

export const boxMarks5 = (code: string, o: BoxOptions = {}) => boxMarks(code, { size: S, captionSize: CAPTION, ...o });

/** A code in a 5 mm frame at the area's inside point. */
export const box = (code: string, o: BoxOptions = {}) => stack(boxMarks5(code, o));

export const pictureMarks5 = (name: PictogramName, caption?: string, o: PictureOptions = {}) => pictureMarks(name, caption, { size: S, captionSize: CAPTION, ...o });

/** A pictogram in a 5 mm frame at the area's inside point. */
export const picture = (name: PictogramName, caption?: string, o: PictureOptions = {}) => stack(pictureMarks5(name, caption, o));
