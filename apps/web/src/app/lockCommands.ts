import type { Command } from '../core/commands';
import { clearLocks, hasLocks, keepLocks, lockReference, lockTravel, NO_LOCK_REFERENCE, NO_TRAVEL, type LockAsk } from '../tools/locks';
import type { AppContext } from './context';

const C = 'Çizim yardımcıları';

/**
 * The digitizing locks' commands (docs/adr/0166 §6). They work on the running command without ending it, as the point
 * calculator does, and only while it waits for a point after another. Uzunluk, Açı and Sapma open the value card under
 * their own name; the number typed there locks that.
 */
export function lockCommands(ctx: AppContext): Command[] {
  const s = ctx.settings;
  const enabled = () => lockReference(ctx) !== null;
  const why = () => (enabled() ? null : 'Kilitler, bir komut ilk noktasından sonra yeni nokta beklerken kullanılır.');
  // The running command's points change its prompt: what the commands depend on follows it.
  const watch = [ctx.tools.activeId, ctx.tools.prompt, s.locks];
  const ask = (kind: LockAsk) => () => {
    if (!enabled()) return void ctx.log.warn(NO_LOCK_REFERENCE);
    if (kind === 'deflection' && !lockTravel(ctx)) return void ctx.log.warn(NO_TRAVEL);
    s.lockAsk.set(kind);
  };
  return [
    {
      id: 'draft.lock.length',
      title: 'Uzunluk kilidi…',
      short: 'Uzunluk',
      category: C,
      icon: 'lockLength',
      description: 'Sonraki noktanın son noktadan uzaklığını kilitler: değeri imleç yanındaki kutuya yazın, doğrultuyu imleç seçer. Değer kutusunda bir sayıdan sonra Tab da kilitler.',
      run: ask('length'),
      isEnabled: enabled,
      whyDisabled: why,
      watch,
    },
    {
      id: 'draft.lock.angle',
      title: 'Açı kilidi…',
      short: 'Açı',
      category: C,
      icon: 'lockAngle',
      description:
        'Sonraki noktanın doğrultusunu kilitler: CAD projesinde doğudan saat yönünün tersine açı, CBS projesinde kuzeyden saat yönünde semt, projenin açı biriminde. Değer kutusuna <45 yazmak da kilitler.',
      run: ask('angle'),
      isEnabled: enabled,
      whyDisabled: why,
      watch,
    },
    {
      id: 'draft.lock.deflection',
      title: 'Sapma kilidi…',
      short: 'Sapma',
      category: C,
      icon: 'lockDeflection',
      description: 'Sonraki kenarın önceki kenarın doğrultusundan dönüşünü kilitler: CAD projesinde sola (saat yönünün tersine), CBS projesinde sağa (saat yönünde) artıdır.',
      run: ask('deflection'),
      isEnabled: () => enabled() && lockTravel(ctx) !== null,
      whyDisabled: () => why() ?? (lockTravel(ctx) ? null : NO_TRAVEL),
      watch,
    },
    {
      id: 'draft.lock.parallel',
      title: 'Nesneye paralel',
      short: 'Paralel',
      category: C,
      icon: 'lockParallel',
      description: 'Sonraki noktanın doğrultusunu tıklanan kenara ya da yayın o yerdeki teğetine paralel kilitler; hangi yöne gideceğini imleç seçer.',
      run: () => void ctx.tools.pickLockEdge('parallel'),
      isEnabled: enabled,
      whyDisabled: why,
      watch,
    },
    {
      id: 'draft.lock.perpendicular',
      title: 'Nesneye dik',
      short: 'Dik',
      category: C,
      icon: 'lockPerpendicular',
      description: 'Sonraki noktanın doğrultusunu tıklanan kenara ya da yayın o yerdeki teğetine dik kilitler; hangi yöne gideceğini imleç seçer.',
      run: () => void ctx.tools.pickLockEdge('perpendicular'),
      isEnabled: enabled,
      whyDisabled: why,
      watch,
    },
    {
      id: 'draft.lock.keep',
      title: 'Kilitler kalıcı',
      short: 'Kalıcı',
      category: C,
      icon: 'lockKeep',
      description: 'Açıkken kilitler nokta konunca kalkmaz; komut bitene dek sonraki noktalarda da durur.',
      run: () => keepLocks(ctx, !s.locks.value.keep),
      isChecked: () => s.locks.value.keep,
      isEnabled: enabled,
      whyDisabled: why,
      watch,
    },
    {
      id: 'draft.lock.clear',
      title: 'Kilitleri kaldır',
      short: 'Kaldır',
      category: C,
      icon: 'unlock',
      description: 'Sonraki noktayı tutan uzunluk ve doğrultu kilitlerini kaldırır; Esc de önce kilitleri kaldırır.',
      run: () => void clearLocks(ctx),
      isEnabled: () => enabled() && hasLocks(s.locks.value),
      watch,
    },
  ];
}
