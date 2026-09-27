# ADR 0078: Bulut veritabanı projesinde düzenleyici katman ağacını değiştiremez

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §16, §21.3; ADR 0015 (proje erişimi), 0024 (açık projede rol değişikliği), 0041–0044 (masaüstünün bulutu), 0072 ve 0076 (katman silme ve ekleme)
- **Kaynak:** web ajanının `b19ed6f`'si:
  - `ui/layers/treeRights.ts` (`treeLocked`);
  - `app/commands.ts`;
  - `ui/layers/LayersPanel.ts`.
  Kararı sahibin genel yönüyle web ajanıyla birlikte verdik.

## Bağlam

- Proje düzeyindeki Düzenleyici rolü nesne yazar (`FeatureWrite`), projenin bilgisini ve katman ağacını değiştiremez (`Edit` yok, `access.rs`).
- İki platformda da Düzenleyici katman ekleyebiliyor, adlandırabiliyor ve silebiliyordu. Ağaç cihazda kalıyor ve “Katman ve proje bilgisi değişiklikleriniz yalnız bu cihazda kalıyor” deniyordu.
- Asıl sorun yeni katmandaki nesnelerdi, hiç kaydedilemiyorlardı. Sunucu ağacında olmayan katmandaki nesneyi reddeder (“‘layer-12’ katmanı projede yok.”). Bu ret yeniden denenmez; gönderilmemiş öbür değişiklikler de arkasında bekler.

## Karar

- **Kural:** açık proje veritabanı projesiyse ve hesabın `project.edit` izni yoksa, katman ağacının değişiklikleri başladıkları yerde reddedilir. Arşivlenmiş proje de izin yok sayılır.
- **İleti** web'inkidir: “Bu projede katman ağacını değiştirme yetkiniz yok (project.edit); proje sahibinden ya da yöneticisinden isteyin.”
- **Kapalı düğmeler:** Yeni katman ve Yeni grup kapalıdır; şeritte ve Katmanlar panelinin başlığında ipuçları nedeni söyler (web'in `whyDisabled`'ı). Kısayolla ya da komut satırından çalıştırılırlarsa ileti uyarı olarak söylenir.
- **Satırın menüsü:** Yanına/İçine yeni katman, Yeniden adlandır ve Sil açık kalır; seçilince iletiyi söyler, hiçbir şey değiştirmez (Sil'in etkin katmandaki reddi gibi). Ağaçta F2 ve Delete de öyle.
- **Değişmeyenler:** göz, kilit, açma ve kapama, etkin katman ve katman stili kullanıcının kendi ayarlarıdır.
- **Dosya projesi ve yerel çizim:** ağacı dosyayla kaydedildiği için değişmez.
- **Yetki değişirse:** oturum sürerken değişen rol (ADR 0024) kuralı hemen değiştirir. Kural projenin bugünkü erişiminden okunur.

## Doğrulama

- `cloud::tests::an_editor_may_not_change_the_layer_tree_of_a_database_project`: iki komutun kapalılığı ve nedeni, çalıştırılınca ileti, satırın üç işlemi, ağacın değişmemesi, gözün çalışması; dosya projesinde serbest.
- Görüntüler (`cargo test -p kentos-desktop cloud::tests::tree_locked_screens -- --ignored --nocapture`): `.run/shots/bulut-agac-kilitli-*`, Katmanlar başlığındaki Yeni katman'ın ipucu; koyu ve açık, 1440×900 ve 1100×650.
- Geçenler: `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e` (web ajanının Düzenleyici duman denetimi dahil), `pnpm inventory:check`.
