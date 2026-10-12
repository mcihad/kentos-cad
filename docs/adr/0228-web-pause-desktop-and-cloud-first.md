# ADR 0228: Web katmanına ara; masaüstü ve bulut önce, mobil ve web için modüler düzen

- **Durum:** kabul edildi (2026-10-12). Sahibin kararı (12 Ekim): “bundan sonra web katmanına devam etmeyeceğiz masaüstü ve gerektiğinde
  bulut tarafı tamamen bittikten sonra web tarafını geliştireceğiz”, “ortak hesaplama tipleri vesaire aynı şekilde devam edecek”,
  “ileride görüntüleme için falan mobil uygulama da geliştireceğiz onun için kod organizasyon düzgün olmalı”, “web şu anda bize zaman
  kaybettirdiği için ve bizim de bu işi belli bir plan dahilinde bitirebilmemiz için web tarafına ara veriyoruz”, “modüler olarak sistemi
  geliştirmeye aynı şekilde devam edeceğiz”, “bulut ve masaüstü ile devam edeceğiz ama ileride mobil ve web ekleyecekmiş gibi”.
- **Bağlam belgesi:** TODOS.md başındaki not ve §0.2 “Web'e ertelenenler”; CLAUDE.md §0, §10.1; docs/MADDE-TARIFI.md; ADR 0010
  (bağımlılık yönü), 0016 ve 0017 (masaüstü kabuğu), 0155 (web'de yalnız şerit).

## Bağlam

25 Eylül'den (ADR 0016, 0017) beri her madde iki platformda yapıldı: ortak Rust çekirdeği, üstünde web (TypeScript/DOM) ve masaüstü
(Rust/Iced). Her maddede arayüz iki kez yazıldı, iki platformun resimleri çekildi, izler iki oynatıcıda koştu, web'in e2e'si ve envanteri
yenilendi. Masaüstünün komut kataloğu (komutların adı, ikonu, açıklaması, kısayolları ve şeritteki yeri) da web'den üretilen envanterden
(`docs/inventory/web.json`) okunuyor; masaüstüne yeni komut eklemek için önce web'in kaydına eklemek gerekiyordu.

Sahip ürünü bir plan içinde bitirmek istiyor; web bu aşamada zaman kaybettiriyor. Masaüstü ve bulut önce bitecek, web sonra gelecek.
İleride bir mobil uygulama (önce görüntüleme) da yapılacak; kod, mobil ve web sonradan eklenecekmiş gibi modüler kalmalı.

## Karar

### 1. Web'in durduğu yer

- **Web katmanının son hâli `1aad00d0`'dır** (`main`, 12 Ekim 2026: “fix(gis): İfadeler yerel projenin çizim biriminde — ADR 0165'in eki”).
  Bu commit'ten sonra web'e yeni özellik yazılmaz.
- Web katmanı şunlardır:
  - `apps/web/` (TypeScript uygulaması, betikleri, testleri, e2e ve resim betikleri);
  - WASM bağlayıcılarının yeni işlevleri (`crates/wasm/*`, aşağıda §3);
  - `docs/inventory/web.{json,md}`: web'den üretilir; `1aad00d0`'daki hâliyle donar, `pnpm inventory` koşmaz.
- Web'in denetimleri (`pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e*`, `pnpm e2e:interaction`, `shots.mjs`, `pnpm inventory`)
  madde takımında koşmaz.
- Web dururken ortak bir değişiklik web'i bozarsa (derlenmez ya da testi düşer) şimdi düzeltilmez. Bozulma §6'daki listeye yazılır,
  geri dönüşte ilk iş olarak düzeltilir.
- Yalnız sahibin açıkça istediği web işi yapılır.

### 2. Masaüstü ve bulut önce

- Maddeler masaüstünde (`apps/desktop`, `crates/native/*`) ve gerektiğinde bulutta (`kentosd`, `crates/server/*`), Python'da ve MCP'de
  yapılır.
- Web'e, masaüstü ve bulut tamamen bittiğinde dönülür. Ne zaman bittiğini sahip söyler.

### 3. Ortak çekirdek ve sözleşmeler aynen sürer

- **Hesap tek yerdedir:** `crates/shared/*` platformdan bağımsız kalır (Iced, DOM, ağ ve SQL bilmez). Her kural ve hesap KentOS kodu
  kullanmadan yazılmış bağımsız Python başvurusuyla denetlenir (`--check`).
- **Ortak fixture'lar** (`fixtures/*/v1`) platformdan bağımsız biçimde yazılmaya devam eder; web geri geldiğinde aynı dosyaları okur.
- **Ortak etkileşim izleri** (`fixtures/interaction/v1`) aynı biçimde yazılır ve masaüstünde oynatılır; web'in oynatıcısı geri dönüşte koşar.
- **Sözleşmeler** (`crates/shared/contracts`) TS türlerini ve komut kataloğunu üretmeye devam eder. `cargo test -p kentos-contracts`'ın
  yazdığı `apps/web/src/contracts/generated/` web'in elle yazılan kodu değil, sözleşmenin çıktısıdır; commit'e girer.
- **WASM bağlayıcıları** (`crates/wasm/*`) Rust çalışma alanının parçası olarak derlenir ve clippy'den geçer.
  - Yeni çekirdek işlevlerine bağlayıcı yazılmaz; geri dönüşte yazılır.
  - Çekirdeğin değişen bir imzası bağlayıcıyı bozarsa yalnız derlenmesi sağlanır.
- **`.kcad`:** kodek (`crates/shared/kcad`), spesifikasyon ve bağımsız Python okuyucu ve yazıcısı aynen sürer. `FORMATS_VERSION`'ın web
  kopyası (`apps/web/src/io/version.ts`) ve web'in `io/kcad.ts`, `io/columns.ts`, `model/snapshot.ts`'i değiştirilmez; artan şema ve sürüm
  §6'daki listeye yazılır.

### 4. Mobil ve web için modüler düzen

Kod, mobil ve web yarın eklenecekmiş gibi düzenlenir:

- **Platformdan bağımsız hesap:** `crates/shared/*`.
- **Arayüzden bağımsız uygulama katmanı:** `crates/native/*` (belge `domain`, ürün komutları `application`, araç oturumu `interaction`,
  işlemler `processing`, proje `project`, stil `style`, başsız sunucu `headless`, Python, MCP, bulut istemcisi `cloud`). Bunlar Iced, wgpu ve
  pencere sistemi bilmez; ileride bir mobil kabuk ve başsız yollar da bunları kullanır.
- **Çizim hattı:** `crates/render/wgpu` Iced bilmez. wgpu Metal, Vulkan ve GLES ile mobilde de çalışır.
- **Iced yalnız** KentOS UI'da (`crates/ui`) ve kabuklardadır (`apps/desktop`, `apps/ui-showcase`, paftanın arayüzü). Masaüstü kabuğu ince
  kalır: sunum olmayan iş (kural, hesap, durum makinesi, biçimlendirme, katalog) `crates/native/*`'a ya da `crates/shared/*`'a yazılır,
  `apps/desktop/src`'ye yazılmaz.
- **Bağımlılık yönü denetimi** (`pnpm arch:deps`, `scripts/arch/deps.mjs`, ADR 0010) bunu zaten zorlar: `crates/shared` ve
  `crates/native/*` için Iced, wgpu, winit ve naga yasaktır. Yeni crate de aynı kurallarla eklenir; denetim madde takımında koşar
  (`pnpm rust:test`).
- **Arayüz kataloğu:** komutların adı, kısa adı, ikonu, açıklaması, takma adları, kısayolları ve şeritteki yeri bugün web'in envanterinden
  okunur. Web dururken yeni komutların kaydı platformdan bağımsız bir ek dosyadadır: `catalog/v1/additions.json` (biçimi envanterin alt
  kümesi: `commands`, `icons` ve türe göre şerit yerleri).
  - Masaüstü dosyayı envanterle birlikte okur ve kurallarını bir testle denetler (kimlik tekliği, envanterle çakışmama, ikonların ve
    şerit yerlerinin varlığı).
  - Web ve mobil geri geldiğinde aynı dosyayı okur.
  - İlk tüketicisi `GIS-40`'tır; dosya ve okuyucusu o maddenin ilk adımıdır.
- **Mobil uygulama** (önce görüntüleme) kendi kabuğuyla (`apps/mobile` gibi) aynı crate'lerin üstüne kurulur. Arayüz yığını o zaman
  sahibin kararıyla seçilir.

### 5. Madde tarifi değişir

[docs/MADDE-TARIFI.md](../MADDE-TARIFI.md) buna göre güncellendi. Özetle bir madde şunlardan oluşur:

- ADR: “Web: ertelendi (ADR 0228)” satırıyla ve web'in geri dönüşte ne gerektirdiğiyle.
- Ortak çekirdek ve bağımsız başvuru.
- Masaüstü; gerekiyorsa bulut, Python ve MCP.
- Ortak izler, masaüstünde oynatılır.
- Masaüstünün resimleri (1440×900 ve 1100×650, açık ve koyu tema).
- Release süreleri.
- §6'daki listeye maddenin satırı.
- Tek commit, takım, push ve rapor.

**Takım:**

- `pnpm rust:test` (bağımlılık yönü dahil), `pnpm rust:test:desktop`.
- Ürün komutu ya da Python değiştiyse `pnpm py:test`.
- Dokunulan bağımsız başvurular; `.kcad` değiştiyse `kcad_v2_reference.py --check`.
- Masaüstü ikilisi.

Web komutları koşmaz.

### 6. Geri dönüşün planı: Web'e ertelenenler

TODOS.md §0.2 “Web'e ertelenenler” geri dönüşün planıdır. Ara verildikten sonra yalnız masaüstünde biten her madde bu listeye bir satır
yazar. Satır web'de gerekenleri sayar:

- WASM bağlayıcısı ve TS sarmalayıcısı,
- arayüz (araç, pencere, şerit, ikon),
- oynatılacak izler,
- resimler,
- envanter.

Web'i bozduğu bilinen ortak değişiklikler (şema, sürüm, imza, fixture) de aynı listeye yazılır.

Geri dönüşte sıra şudur:

1. `1aad00d0`'daki web'den başlanır.
2. Web'in takımı (`pnpm typecheck`, `pnpm test`, `pnpm e2e:interaction`) koşar; bozulanlar düzeltilir.
3. Liste sırayla kapatılır.
4. Katalog ek dosyası web'in kaydına alınır, envanter yeniden üretilir.
5. Madde tarifinin iki platform kuralı geri gelir.

## Uygulama

- **Bu karar (yalnız belgeler):**
  - bu ADR;
  - CLAUDE.md §0 (en üstte karar), §1, §2 (web komutlarının notu), §8.1, §9.4, §10.1 (çalışma düzeni ve sıradaki madde);
  - TODOS.md: baştaki not, §0.1, §0.2 “Web'e ertelenenler”;
  - docs/MADDE-TARIFI.md;
  - docs/DEVIR.md.
- **Sıradaki teknik adım:** `GIS-40` (adres arama, ADR 0229), yalnız masaüstünde. İlk adımı §4'teki katalog ek dosyası ve masaüstünün
  okuyucusudur. (Devir notunda GIS-40'a ayrılan 0228 bu karara verildi; GIS-40 0229'u alır.)

## Doğrulama

- Yalnız belgeler değişti; kod değişmediği için takım bu commit'te koşmadı.
- Son tam takım `1aad00d0`'da koştu ve geçti:
  - `pnpm rust:test`;
  - `pnpm rust:test:desktop` (1336 test);
  - web'in `typecheck`, Vitest (5305 test), `build` ve `e2e:interaction` (146 iz × 3 varyant);
  - `pnpm py:test` (61 test);
  - envanter.
- Commit kimliği (`git log -1 1aad00d0`) ve bağlantılar denetlendi.
