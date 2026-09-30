import type { AppContext } from './context';

/**
 * The text commands (docs/adr/0145 §6): Bul ve değiştir, from Düzen, Giriş › Açıklama and the command line. The
 * window is loaded when first opened; the desktop's is `text.findReplace` in apps/desktop/src/find_replace.rs.
 */
export function registerTextCommands(ctx: AppContext, open: { findReplace: () => void }): void {
  ctx.commands.register({
    id: 'text.findReplace',
    title: 'Bul ve değiştir',
    category: 'Düzen',
    icon: 'findReplace',
    aliases: ['BUL', 'FIND', 'BULDEGISTIR'],
    description: 'Yazılarda arar ve değiştirir: joker (*), büyük küçük harf ve tam sözcük seçenekleri, seçimde ya da bütün çizimde; eşleşmeler listelenir, seçilenler ya da hepsi tek adımda değişir.',
    run: () => open.findReplace(),
  });
}
