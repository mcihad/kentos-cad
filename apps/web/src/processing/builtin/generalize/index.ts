import type { ProcessingTool } from '../../types';
import { aggregate } from './aggregate';
import { building } from './building';
import { coverageSimplify } from './coverage';
import { eliminate } from './eliminate';
import { dropParts } from './parts';
import { smooth } from './smooth';
import { square } from './square';

/** Genelleştirme (docs/adr/0221): smoothing, squaring, buildings, aggregation, parts, elimination, coverages. */
export const GENERALIZE_TOOLS: readonly ProcessingTool[] = [smooth, square, building, aggregate, dropParts, eliminate, coverageSimplify];
