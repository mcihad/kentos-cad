// Hesabım › E-imza (docs/adr/9004): whether KentOS İmza runs on this computer, where this browser reaches it, and
// what to write into its settings so that it trusts this server's requests (the server's public key, its names and
// this portal's address as an allowed origin).
import { signing } from '../api/portal';
import { DEFAULT_BASE, imza, imzaBase, setImzaBase } from '../app/imza';
import { h } from '../lib/dom';
import { icon } from '../ui/icons';
import { badge, button, facts, notice, panel } from '../ui/kit';
import { toast } from '../ui/toast';

export function esignPanel(): HTMLElement {
  const status = h('div', { class: 'stack', style: 'gap: 10px' }, h('div', { class: 'sign-step' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), 'KentOS İmza aranıyor…'));
  const setup = h('div', { class: 'stack', style: 'gap: 10px' });
  const address = h('input', { class: 'input', value: imzaBase(), 'aria-label': 'KentOS İmza adresi', spellcheck: false });

  const check = async () => {
    try {
      const [health, info] = await Promise.all([imza.health(), imza.info()]);
      status.replaceChildren(
        h('div', { class: 'party' }, badge('success', 'Çalışıyor'), h('span', null, `KentOS İmza ${health.version}`)),
        facts([
          ['PIN penceresi', health.interactiveAgent ? 'Açık' : 'Kapalı: KentOS İmza uygulamasını başlatın'],
          ['Takılı kart', String(health.devices)],
          ['PIN girişi', info.pinEntry === 'NATIVE' ? 'KentOS İmza penceresinde (PIN tarayıcıya gelmez)' : 'Tarayıcıda (geliştirme kipi, güvensiz)'],
          ['Sunucu anahtarı', info.trustConfigured ? 'Yazılmış' : 'Yazılmamış: aşağıdaki satırları ekleyin'],
          ...(info.mockToken ? ([['Kart', 'Deneme kartı (gerçek değil)']] as [string, string][]) : []),
        ]),
        ...(info.insecurePinEntry ? [notice('warning', 'PIN tarayıcıdan geçiyor.', 'Bu kip yalnız geliştirme içindir; KENTOS_IMZA_PIN_ENTRY ayarını kaldırın.')] : []),
      );
    } catch (e) {
      status.replaceChildren(
        h('div', { class: 'party' }, badge('danger', 'Ulaşılamadı'), h('span', { class: 'muted' }, imzaBase())),
        h('div', { class: 'field-hint' }, e instanceof Error ? e.message : String(e)),
      );
    }
  };

  const loadSetup = async () => {
    try {
      const t = await signing.trust();
      const lines = [
        `KENTOS_IMZA_TRUST_KEY=${t.publicKey}`,
        `KENTOS_IMZA_TRUST_ISSUER=${t.issuer}`,
        `KENTOS_IMZA_TRUST_AUDIENCE=${t.audience}`,
        `KENTOS_IMZA_ORIGINS=${location.origin}`,
      ].join('\n');
      const pre = h('pre', { class: 'code-sample' }, lines);
      setup.replaceChildren(
        h('p', { class: 'panel-text' }, 'KentOS İmza yalnız bu sunucunun imzaladığı istekleri kabul eder ve yalnız izin verilen sayfalarla konuşur. Bu satırları KentOS İmza’nın ayar dosyasına yazın (Linux’ta ~/.config/kentos-cad/imza.env) ve servisi yeniden başlatın:'),
        pre,
        h(
          'div',
          null,
          button('Satırları kopyala', {
            kind: 'ghost',
            icon: 'copy',
            onClick: async () => {
              await navigator.clipboard.writeText(lines).catch(() => undefined);
              toast('Ayar satırları kopyalandı.');
            },
          }),
        ),
      );
    } catch (e) {
      setup.replaceChildren(notice('info', 'Bu sunucuda e-imza açık değil.', e instanceof Error ? e.message : String(e)));
    }
  };

  const save = button('Kaydet', {
    kind: 'secondary',
    onClick: () => {
      setImzaBase(address.value);
      address.value = imzaBase();
      toast('Adres bu tarayıcıda saklandı.');
      void check();
    },
  });
  void check();
  void loadSetup();
  return panel(
    'E-imza',
    h(
      'div',
      { class: 'stack' },
      h('p', { class: 'panel-text' }, 'Dosyalardaki belgeleri nitelikli elektronik sertifikanızla imzalamak için bilgisayarınızda KentOS İmza çalışmalı. Kart ve PIN yalnız onun penceresinde istenir.'),
      status,
      h('div', { class: 'field' }, h('label', { class: 'field-label' }, 'KentOS İmza adresi (bu tarayıcı)'), h('div', { class: 'fee-range' }, address, save), h('div', { class: 'field-hint' }, `Varsayılan ${DEFAULT_BASE}; yalnız bu bilgisayardaki servis.`)),
      h('div', { class: 'with-icon' }, icon('key', 16), h('strong', null, 'Kurulum')),
      setup,
    ),
  );
}
