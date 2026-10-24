# Hassas kapı/pencere yerleştirme — gerçek pencere kaydı

25 Ekim 2026 · ADR 0633 · `codex/architecture-2d-workflows`.

Bu kayıt yerleştirme diliminin kanıtıdır. ARCH-OPEN-01'in tüm kabulü değildir.
Uygulama yerel derlemedir; ayrılmış test ayar/veri dizinleri ve sentetik çizim
kullanıldı. Kullanıcının açık çizimlerinde işlem yapılmadı. Görüntüler gerçek
masaüstü penceresinden alındı ve gözle incelendi; yapay ekran veya çizilmiş
mockup değildir. FPS/titreme kabulü çıkarılmaz.

## Derleme ve görüntü ayrımı

- İlk gözlem `03`: Yerleştirme alanı mesafe etkin olduğu hâlde Serbest diyordu,
  büyük yazıda referans etiketi kesiliyordu. Bunlar düzeltildi.
- Ara derleme `07–13`: ortala/mesafe önceliği, menteşe yerleşimi, undo/redo,
  iki pervaz, örnekten alma. `10` eski tür eninin sabit gösterimini yakaladı;
  bu yanıltıcı alan son derlemede iki pervaz yönteminden kaldırıldı.
- **Son GUI derlemesi `14–26`:** SHA-256
  `3777b17710d2e7fae7435c1879c6f343dd8678570822424e0e553dcfa0f0481f`.
  Bundan sonra yalnız fixture/test/belge değişti; üretim kodu değişmedi.
- Geniş pencere 2880×1720 fiziksel piksel (1440×828 uygulama iç alanı,
  1440×860 dış pencere; macOS başlığı dahil). Dar pencere 2200×1364 fiziksel
  piksel (1100×650 uygulama iç alanı, 1100×682 dış pencere).
  **Geniş gerçek pencere 1440×900 iç alan değildir.** Plandaki tam boyut
  matrisi henüz kapanmadı. Görsel ölçüler görüntü boyutlarından okunmuştur.

## Gerçek etkileşimler

| Görüntü | Gözlenen sonuç |
|---|---|
| `01` | Üç host duvar, iki mevcut açıklık; beş başlangıç nesnesi. |
| `07` | Eski 2 m uzaklığa rağmen Ortala merkezde 4,5/4,5 m net pay gösterir. Ara derlemenin sabit alanı son sürümde gizlidir. |
| `08–09` | Sondan 2 m, menteşe Baş, en 1 m → merkez 8,5 m. Tek Undo yeni kapıyı kaldırır; Redo geri getirir. |
| `10–11` | Ters duvarda iki pervaz eni 1,389 m; ikinci tık tek pencere. Yeni yarım açıklık tek Onayla ile kapanır. |
| `12–13` | Mevcut 1100 mm kapıdan tür/en alınır; alma işlemi yazmaz. Sonraki yerleştirme yeni kapı yazar. |
| `14` | Kaydedilmiş sekiz nesne yeniden açılır. |
| `15–16` | Yaylı hostta iki pervaz: yol boyunca 3,925 m en, net pay ve host vurgusu. İkinci tık tek ilişkili pencere, toplam dokuz nesne. |
| `17` | Koyu tema 18 px: Ölçü noktası adı sığar; çakışma metni tüm açıklamayla satırlanır. |
| `18–19` | Seçilen yeni kapı eni panelde Enter ile 1,1 → 1,2 m olur; duvar boşluğu da değişir. Tek Undo 1,1 m'ye döner; kaydedilir. |
| `20–21` | Koyu tema 13 px, dar pencere: yeni alanlar okunur; 1,5 m yazılan taslak Escape ile 0,9 m eski araç değerine döner. |
| `22` | Açık tema 13 px, dar pencere: tekerlek yakınlaştırması sonrası ters host üzerinde 1,2 m önizleme, iki uç payı ve vurgu. Belgeye yazılmaz. |
| `24–25` | Son sürümde İki pervaz menüsünde tek Onayla/bitir. Yarım açıklık kapatılır, araç sonlanır, nesne sayısı dokuz kalır. |
| `26` | Kaydırma sürüklemesi denendi; kamera hareketi doğrulanamadı. Başarılı kaydırma kanıtı değildir. |

Yaylı pencerenin eni host yolu boyunca ölçülür. Mevcut sembol iki pervaz
arasında düz kiriş gösterir; bu kayıt eğri doğrama üretiminin kanıtı değildir.

## Bağımsız dosya kontrolü

`before.kcad` beş başlangıç nesnesi, `after.kcad` dokuz nesnedir.
`tools/kcad/kcad.py` ile, native/Python KentOS modülü çağrılmadan okundu.
`saved-file-check.json` beş eski UID'nin bütün alanlarının aynı kaldığını,
proje ayarlarının korunduğunu ve dört yeni açıklığın host/en/konumunu kaydeder.
Metre ve 1:100 kayıtta aynıdır. En düzenlemesi undo edildiği için örnekten
alınan kapının kayıtlı eni 1,1 m'dir.

## Otomatik doğrulama

`test-results.json` yerel günlüklerin hash'lerini ve sonuç satırlarını tutar.
Günlükler `.run/architecture-openings/` içindedir; yok sayılan testler geçmiş
sayılmaz. Çalıştırmalar farklı aşamalardadır:

- Semantik değişiklikler: commands 796; native 51 birim + 161 bütünleşme,
  22 atlanan test. İlk interaction 710; son sunum düzeltmeleriyle yeniden
  tam interaction **712** geçti (6 atlanan).
- Desktop tam **1274** geçti, 219 atlandı. Son etiket/alan gizleme/hata
  satırlama düzeltmesinden önceki tam çalıştırmadır; son kaynakta ilgili
  iki architecture_settings testi, gerçek GUI ve derleme yeniden geçti.
- Headless 64, mevcut MCP 56; yeni MCP yerleştirme testi ayrıca 1 geçti.
- Yeniden derlenmiş native modülle Python **271** test geçti. Sonrasında
  bağımsız başvuru 38 yeni durumla genişletildi; yeni toplam **79 oluşturma +
  45 düzenleme** ortak durum native ve Python'da ayrıca yeniden geçti.
  Eski 41 oluşturma ve 45 düzenleme durumu değişmeden korunmuştur.
- clippy: commands/native application/interaction/desktop/Python ve ayrıca
  MCP, tüm hedefler, `-D warnings`; ortak WASM derlemesi geçti.
- SDK üretim kontrolü, katalog testleri, 887 satırlı CAD manifesti, şerit,
  bağımlılık yönü (47 crate/61 hedef) ve fixture `--check` geçti.
- TS dışa aktarımı `.run/architecture-openings/contracts-ts` içine yönlendirildi.
  Git'teki iki TS dosyası yalnız ortak NewOpening ve yeni referans enumudur.

## Açık kabul ve bulunan takip hataları

- Gerçek Kaydır kamera hareketi bu oturumda doğrulanamadı; girdi olaylarıyla
  kamera yolunu ayrı incelemek gerekiyor.
- İlk incelemede çizim özetindeki ölçek farklı sanıldı. `25` görüntüsünün ilgili
  kısmı büyütülerek tekrar incelendi: **1:100**, durum çubuğu ve dosyayla aynı.
  `scale-summary-crop.png`, aynı görüntünün yalnız okunabilirlik için büyütülmüş
  kesitidir. Noktalı sıfır yanlış okunmuştu; uygulama ölçek hatası değildir.
- Yeni yöntemlerle üretilmiş açıklık için tutamaç + host değişimi + duvar
  kalınlığı/bölme + undo birleşik kabulü tamamlanmalı.
- Tam 1440×900 / 1100×650 × açık/koyu × 13/18 px matrisi tamamlanmalı.

Bu eksikler nedeniyle ana madde `[~]`; yerleştirme diliminin başarılı
kontrolleri bunların yerine geçirilmez.
