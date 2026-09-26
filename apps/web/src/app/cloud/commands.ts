import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { AppContext } from '../context';

/** A project to select in the open dialog (`cloud.open` with args). */
const isPick = (v: unknown): v is { tenantId: string; projectId: string } =>
  !!v && typeof (v as { tenantId?: unknown }).tenantId === 'string' && typeof (v as { projectId?: unknown }).projectId === 'string';

/**
 * Cloud commands (menus, the status bar's server menu, the command line).
 * Anything that needs an account opens the sign-in first and goes on after it.
 */
export interface CloudHooks {
  signIn(then?: () => void): void;
  /** `pick`: a project to select in the list (the application menu's recent projects). */
  projects(mode: 'open' | 'upload', pick?: { tenantId: string; projectId: string }): void;
  conflicts(): void;
  /** Rename, delete or share the open cloud project (the projects list offers these for any project). */
  rename(): void;
  remove(): void;
  share(): void;
}

export function registerCloudCommands(ctx: AppContext, hooks: CloudHooks): void {
  const cloud = ctx.cloud;
  const C = 'Bulut';
  const signedIn = () => cloud.auth.value === 'signedIn';
  const reachable = () => ctx.server.state.value === 'online';
  const needAccount =
    (next: (args?: unknown) => void) =>
    (args?: unknown) =>
      signedIn() ? next(args) : hooks.signIn(() => next(args));
  const watch = [cloud.auth, ctx.server.state];
  // The open project, while it exists for this account, and whether this account may do `permission` in it
  // (`writes`: it changes the project, which an archived one refuses, docs/adr/0028).
  const openMay = (permission: ProjectPermission, writes = false) => {
    const p = cloud.project.value;
    const state = cloud.sync.value?.state.value;
    return !!p && state !== 'deleted' && state !== 'revoked' && !(writes && p.state === 'archived') && cloud.may(permission) && reachable();
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
      description:
        'Bulut projeleriniz: son kullanılanlar, favoriler, projelerim, kurum projeleri, benimle paylaşılanlar, arşivlenmişler ve çöp kutusu; sunucuda aranır ve sıralanır. Seçilen projeyi açar, paylaşır, bilgilerini değiştirir, kopyalar, arşivler ya da çöpe taşır. Açık projedeki değişiklikler kendiliğinden kaydedilir.',
      aliases: ['BULUTAC', 'CLOUDOPEN'],
      run: needAccount((pick) => hooks.projects('open', isPick(pick) ? pick : undefined)),
      isEnabled: () => reachable(),
      watch,
    },
    {
      id: 'cloud.upload',
      title: 'Buluta yükle…',
      category: C,
      icon: 'cloudUpload',
      description:
        'Açık çizimi kişisel alanınızda ya da kurumunuzda yeni bir bulut projesi yapar; proje sizindir, başkaları paylaşımla eklenir. Sonra her değişiklik kendiliğinden kaydedilir.',
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
      description:
        'Açık bulut projesinin adını projeye erişen herkes için değiştirir (project.edit yetkisi gerekir). Başka bir projeyi Bulut projesi aç listesinden yeniden adlandırın.',
      aliases: ['BULUTAD', 'RENAME'],
      run: () => hooks.rename(),
      isEnabled: () => openMay('project.edit', true),
      watch: [cloud.project, cloud.sync, cloud.me, ctx.server.state],
    },
    {
      id: 'cloud.delete',
      title: 'Bulut projesini çöp kutusuna taşı…',
      short: 'Çöpe taşı',
      category: C,
      icon: 'trash',
      description:
        'Açık bulut projesini erişen herkes için çöp kutusuna taşır (project.delete yetkisi: proje sahibi ya da kurum yöneticisi). Hiçbir şey silinmez: saklama süresi dolana kadar Bulut projeleri → Çöp kutusu’ndan geri yüklenebilir, sonra kalıcı olarak silinir.',
      aliases: ['BULUTSIL'],
      run: () => hooks.remove(),
      isEnabled: () => openMay('project.delete'),
      watch: [cloud.project, cloud.sync, cloud.me, ctx.server.state],
    },
    {
      id: 'cloud.share',
      title: 'Bulut projesini paylaş…',
      short: 'Paylaş',
      category: C,
      icon: 'share',
      description:
        'Açık bulut projesine kimin hangi rolle erişebildiğini gösterir; kişi ekler, rolünü değiştirir ya da erişimini kaldırır (project.share yetkisi: proje sahibi ya da yöneticisi). Paylaşım alıcıya veritabanı yetkisi vermez. Başka bir projeyi Bulut projesi aç listesinden paylaşın.',
      aliases: ['PAYLAS', 'SHARE'],
      run: () => hooks.share(),
      isEnabled: () => openMay('project.share'),
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
