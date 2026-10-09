import type { AppContext } from './context';

/**
 * Ağ analizi's commands (docs/adr/0209 §10): Ağlar opens the networks' window, which loads on first use (CLAUDE.md
 * §20); the tools are the catalog's (tools/catalog.ts: En kısa yol, Hizmet alanı, Şebeke izleme).
 */
export function registerNetworkCommands(ctx: AppContext): void {
  ctx.commands.registerAll([
    {
      id: 'network.manage',
      title: 'Ağlar…',
      short: 'Ağlar',
      category: 'Analiz',
      icon: 'networks',
      description:
        'Projenin yol ve şebeke ağlarını tanımlar: kenar ve düğüm katmanları (süzgeçleriyle), bağlanma kuralı ve toleransı, yön, süre ve maliyetler, kapalı kenarlar. Denetle ağın düğüm, parça ve uzunluğunu, kopuk parçaları ve sorunları gösterir.',
      aliases: ['AGLAR', 'AĞLAR', 'AGTANIMI', 'NETWORKS'],
      run: () =>
        void import('../ui/networks/NetworksDialog').then(
          (m) => m.openNetworksDialog(ctx),
          (e: Error) => ctx.log.error(`Pencere yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`),
        ),
    },
  ]);
}
