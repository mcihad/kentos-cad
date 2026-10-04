# ADR 0170: Derleme ve test süreleri

- **Durum:** kabul edildi (2026-10-04)
- **Bağlam:** sahibin isteği (4 Ekim): “Testler build süreleri çok uzun, biraz iyileştirmeler yapsak iyi olur.” İlgili: ADR 0001
  (çalışma alanı; tek ağır süreç), ADR 0008 (WASM sınırı), CLAUDE.md §2.

## Bağlam

Bir değişiklikten sonra testlerin ve uygulamanın yeniden derlenmesi çalışma gününün büyük kısmını alıyordu. Ölçülen durum
(4 Ekim, 8 çekirdek, 15 GB bellek, `jobs = 4`):

- `target/debug` 130 GB'tı. Geliştirme profili tam DWARF hata ayıklama bilgisi yazıyordu; masaüstünün test ikilisi tek
  başına yaklaşık 985 MB'tı. Her yeniden derleme bu dosyaları yeniden bağlayıp diske yazıyordu.
- Çalışma alanında yaklaşık 180 entegrasyon test dosyası vardı ve Cargo her birini ayrı bir ikili olarak derleyip
  bağlıyordu (interaction 52, geometry-core 27, sunucu uygulaması 20, formats 14, kcad 10 …). Ortak bir crate değişince
  hepsi yeniden bağlanıyordu.
- WASM paketleri (geometri çekirdeği, biçimler, DXF, NCZ, SVG, pafta) her derlemede gönderilen profille, tam LTO ve tek kod
  birimiyle derleniyordu. Bir çekirdek değişikliği altı paketi bu profille yeniden derletiyordu, geliştirme sunucusu ve
  testler için de.

## Karar

1. **Geliştirme profili** (`Cargo.toml`): çalışma alanının crate'leri yalnız satır tablolarıyla derlenir
   (`debug = "line-tables-only"`); panik izi dosya ve satırı yine gösterir. Bağımlılıklar hata ayıklama bilgisiz derlenir.
2. **Entegrasyon testleri crate başına tek ikilidir:** `tests/all/main.rs` her eski test dosyasını bir modül olarak
   bildirir, ortak yardımcı `tests/all/common.rs`'dir. Yalnız `--test <ad>` ile ayrıca çalıştırılan ve belgelenen testler
   (ölçümler, yazıcıların örnek çıktısını yeniden yazanlar, GPU testleri) `tests/`'te kendi dosyalarında kalır.
   Birleştirilenler: interaction, geometry-core, formats, kcad, uygulama komutları, masaüstü stili, belge, pafta,
   sözleşmeler. Birleştirilmeyenler: sunucu uygulaması (veritabanı testleri tek süreçte daha çok bağlantı açardı),
   wgpu çizim hattı (aynı süreçte birlikte açılan wgpu örnekleri Vulkan yükleyicisini çökertmişti) ve ifade dili (ayrı
   testleri `reference` modülünü paylaşır).
3. **WASM'ın geliştirme profili** (`wasm-dev`): gönderilen `wasm` profilinden türer, ama bütün program LTO'su yoktur ve kod 16
   birimde üretilir. Geliştirme sunucusu, Vitest, e2e ve `pnpm wasm` bunu kullanır. `pnpm build` ve perf betikleri
   `scripts/wasm/ensure.mjs --release` ile gönderilen `wasm` profilini kullanır. Profil paketin özetine girer; biri öbürünün
   yerine kullanılmaz. Kod aynı Rust'tır ve kayan nokta anlamı aynıdır (Rust işlemleri birleştirmez ya da yeniden
   sıralamaz); yalnız crate'ler arası satır içi açma değişir.
4. Derleme `jobs = 4`'te kalır (makine tarayıcı ve Vite ile paylaşılıyor). 6 iş artımlı derlemelerde ölçülebilir bir kazanç
   vermedi (Doğrulama); sürenin çoğu büyük crate'in seri derlenmesi ve bağlamadır.

## Sonuçlar

Ölçümler “Doğrulama”dadır. Yeni bir entegrasyon testi `tests/all/`'a modül olarak eklenir, ayrı dosya olarak değil.
Taşınan dosyaların depodaki bütün anılışları yeni yollarına çevrildi (kod yorumları, ADR'ler, fixture notları).

## Doğrulama

Aynı makinede, aynı derleyiciyle, `jobs = 4`, `nice -n 10`. Eski düzen HEAD'in ayrı bir çalışma ağacında, kendi hedef
klasörüyle; yeni düzen ana ağaçta. Her adım gerçek bir düzenlemedir (dosyaya bir özel işlev eklenir), süre derlemenindir;
sonra düzenleme geri alınır ve yeniden derlenir, sonraki adım sıcak başlar (`scripts/perf/rebuild-times.sh`).

| Düzenleme → derleme | Eski | Yeni |
|---|---|---|
| geometry-core → interaction testleri | 73 sn | 9 sn |
| formats → formats testleri | 13 sn | 2 sn |
| interaction → masaüstü testleri | 52 sn | 35 sn |
| masaüstü → masaüstü testleri | 13 sn | 10 sn |
| masaüstü → masaüstü uygulaması | 15 sn | 8 sn |

- Sıfırdan derleme: eski düzende formats, interaction ve masaüstü testleriyle uygulama 259 sn; yeni düzende bütün çalışma
  alanının testleri 130 sn, masaüstü crate'lerinin testleri 119 sn, uygulama 36 sn.
- `target/debug`: 130 GB'tan (birikmiş eski çıktılarla) bütün çalışma alanı derliyken 16 GB'a; masaüstünün test ikilisi
  985 MB'tan 287 MB'a.
- WASM paketleri (gönderilen `wasm` ile `wasm-dev`, her biri kendi bloğunda): formats düzenlemesinden sonra (3 paket) 28 sn
  ile 12 sn; geometry-core düzenlemesinden sonra (6 paket) 128 sn ile 60 sn.
- 4 ile 6 iş, aynı düzenlemelerle ikişer kez: interaction → masaüstü testleri 12 ve 21 sn ile 26 ve 18 sn; geometry-core →
  interaction testleri 6 ve 4 sn ile 6 ve 2 sn. Fark ölçüm dalgalanmasının içindedir.
- Tam paket (biçim, başvurular, tip denetimi, Rust ve clippy, masaüstü, web testleri, Python, etkileşim izleri, envanter,
  tarayıcı duman testi) yeni düzende geçti, 811 sn; taşınan testlerin hepsi tek ikililerinde çalıştı.
