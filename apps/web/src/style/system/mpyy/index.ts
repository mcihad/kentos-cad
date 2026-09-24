import type { LibraryCategory, LibraryItem } from '../../../model/style';
import { LEVEL_CATEGORIES, type Sheet } from './dsl';
import { PICTOGRAMS } from './pictograms';

/**
 * The MPYY system library: every legend section of every plan level. Each
 * file under ortak/, uip/, nip/, cdp/ and msp/ exports its sections
 * (`Sheet`s, or arrays of them); they are picked up here, so adding a file
 * needs no edit elsewhere. Section order comes from each sheet's `order`,
 * item order is the legend's.
 */

const modules = import.meta.glob<Record<string, unknown>>(['./ortak/*.ts', './uip/*.ts', './nip/*.ts', './cdp/*.ts', './msp/*.ts', '!./**/*.test.ts'], { eager: true });

const isSheet = (v: unknown): v is Sheet => !!v && typeof v === 'object' && Array.isArray((v as Sheet).items) && !!(v as Sheet).category;

const SHEETS: readonly Sheet[] = Object.keys(modules)
  .sort()
  .flatMap((path) => Object.values(modules[path]).flatMap((v) => (Array.isArray(v) ? v.filter(isSheet) : isSheet(v) ? [v] : [])));

export const MPYY_ITEMS: readonly LibraryItem[] = [...SHEETS.flatMap((s) => s.items), ...PICTOGRAMS];
export const MPYY_CATEGORIES: readonly LibraryCategory[] = [...LEVEL_CATEGORIES, ...SHEETS.map((s) => s.category), { path: ['MPYY', 'Piktogramlar'], order: 99, description: 'Gösterimlerdeki çizimler (SVG)' }];
