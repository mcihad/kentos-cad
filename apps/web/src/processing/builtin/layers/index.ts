import type { ProcessingTool } from '../../types';
import { deleteDuplicates } from './dedupe';
import { extractSelected } from './extract';
import { mergeLayers } from './merge';
import { orderByExpression } from './orderBy';
import { splitByAttribute } from './split';

/** Katman işlemleri (docs/adr/0217): split, merge, extract, delete duplicates, order. */
export const LAYER_TOOLS: readonly ProcessingTool[] = [splitByAttribute, mergeLayers, extractSelected, deleteDuplicates, orderByExpression];
