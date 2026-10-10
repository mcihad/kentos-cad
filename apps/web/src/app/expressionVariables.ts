import { documentVariables, type ExpressionVariable } from '../model/projectVariables';
import type { AppContext } from './context';

/**
 * The open drawing's `@` values (docs/adr/0214 §2.3): its own variables,
 * then the built-in ones with this device's clock and the signed-in person
 * (none when no one is). Every expression field of the app asks here when
 * its builder opens and when it runs, so a value is never older than the
 * evaluation that reads it.
 */
export const appVariables = (ctx: Pick<AppContext, 'doc' | 'cloud'>): readonly ExpressionVariable[] =>
  documentVariables(ctx.doc, new Date(), ctx.cloud.me.value?.user.displayName ?? '');
