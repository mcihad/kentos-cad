import type { AppContext } from '../context';

/**
 * Cloud commands (menus, the status bar's server menu, the command line).
 * Anything that needs an account opens the sign-in first and goes on after it.
 */
export interface CloudHooks {
  signIn(then?: () => void): void;
  projects(mode: 'open' | 'upload'): void;
  conflicts(): void;
  /** Rename or delete the open cloud project (the projects list offers both for any project). */
  rename(): void;
  remove(): void;
}

export function registerCloudCommands(ctx: AppContext, hooks: CloudHooks): void {
  const cloud = ctx.cloud;
  const C = 'Bulut';
  const signedIn = () => cloud.auth.value === 'signedIn';
  const reachable = () => ctx.server.state.value === 'online';
  const needAccount = (next: () => void) => () => (signedIn() ? next() : hooks.signIn(next));
  const watch = [cloud.auth, ctx.server.state];
  // The open project, while it still exists, and whether this account may do `capability` in it.
  const openMay = (capability: string) => {
    const p = cloud.project.value;
    return !!p && cloud.sync.value?.state.value !== 'deleted' && cloud.can(p.tenantId, capability) && reachable();
  };
  ctx.commands.registerAll([
    {
      id: 'cloud.signIn',
      title: 'Buluta giriş…',
      category: C,
      icon: 'signIn',
      description: 'KentOS sunucusunda hesabınızla oturum açar (yerel hesap ya da kurumunuzun OpenID girişi).',
      aliases: ['GIRIS', 'LOGIN'],
      run: () => hooks.signIn(),
      isEnabled: () => !signedIn() && reachable(),
      watch,
    },
    {
      id: 'cloud.signOut',
      title: 'Bulut oturumunu kapat',
      category: C,
      icon: 'signOut',
      description: 'Oturumu kapatır. Açık bulut projesinin gönderilemeyen değişiklikleri bu cihazda saklanır.',
      aliases: ['CIKIS', 'LOGOUT'],
      run: () =>
        void cloud.signOut().then(
          () => ctx.log.info('Bulut oturumu kapatıldı.'),
          (e: Error) => ctx.log.error(`Oturum kapatılamadı: ${e.message}`),
        ),
      isEnabled: () => signedIn(),
      watch,
    },
    {
      id: 'cloud.open',
      title: 'Bulut projesi aç…',
      category: C,
      icon: 'cloud',
      description: 'Kurumunuzun bulut projelerinden birini açar. Açık projedeki değişiklikler kendiliğinden kaydedilir.',
      aliases: ['BULUTAC', 'CLOUDOPEN'],
      run: needAccount(() => hooks.projects('open')),
      isEnabled: () => reachable(),
      watch,
    },
    {
      id: 'cloud.upload',
      title: 'Buluta yükle…',
      category: C,
      icon: 'cloudUpload',
      description: 'Açık çizimi kurumunuzda yeni bir bulut projesi yapar; sonra her değişiklik kendiliğinden kaydedilir.',
      aliases: ['BULUTAYUKLE', 'UPLOAD'],
      run: needAccount(() => hooks.projects('upload')),
      isEnabled: () => reachable(),
      watch,
    },
    {
      id: 'cloud.rename',
      title: 'Bulut projesini yeniden adlandır…',
      short: 'Yeniden adlandır',
      category: C,
      icon: 'edit',
      description: 'Açık bulut projesinin adını kurumdaki herkes için değiştirir (project.edit yetkisi gerekir). Başka bir projeyi Bulut projesi aç listesinden yeniden adlandırın.',
      aliases: ['BULUTAD', 'RENAME'],
      run: () => hooks.rename(),
      isEnabled: () => openMay('project.edit'),
      watch: [cloud.project, cloud.sync, cloud.me, ctx.server.state],
    },
    {
      id: 'cloud.delete',
      title: 'Bulut projesini sil…',
      short: 'Projeyi sil',
      category: C,
      icon: 'trash',
      description:
        'Açık bulut projesini kurumdaki herkes için siler (project.delete yetkisi, yönetici). Nesneler sunucuda saklanır; yanlışlıkla silineni sunucu yöneticisi geri getirebilir.',
      aliases: ['BULUTSIL'],
      run: () => hooks.remove(),
      isEnabled: () => openMay('project.delete'),
      watch: [cloud.project, cloud.sync, cloud.me, ctx.server.state],
    },
    {
      id: 'cloud.conflicts',
      title: 'Kayıt çakışmalarını çöz…',
      short: 'Çakışmaları çöz',
      category: C,
      icon: 'conflict',
      description: 'Başkasının daha önce kaydettiği nesneler için sunucudakini alır ya da sizinkini kaydeder.',
      run: () => hooks.conflicts(),
      isEnabled: () => !!cloud.sync.value?.conflicts.value.length,
      watch: [cloud.sync],
    },
  ]);
}
