import type { CallSet } from './harness';
import { P0 } from './sets/p0-basics';
import { P1 } from './sets/p1-primitives';
import { P2 } from './sets/p2-curves';
import { P3 } from './sets/p3-offset-annotation';
import { P4 } from './sets/p4-overlay';
import { P5 } from './sets/p5-entities';
import { P6 } from './sets/p6-path-editing';
import { P7 } from './sets/p7-editing';
import { P8 } from './sets/p8-triangulate';
import { R1_PREDICATES } from './sets/r1-predicates';
import { S4 } from './sets/s4-processing';
import { S5_INPUT } from './sets/s5-input';
import { S5_TOOLS } from './sets/s5-tools';

/** Every call set, in the order the core was ported (docs/adr/0008). */
export const SETS: CallSet[] = [P0, P1, P2, P3, P4, P5, P6, P7, P8, S4, S5_INPUT, S5_TOOLS, R1_PREDICATES];
