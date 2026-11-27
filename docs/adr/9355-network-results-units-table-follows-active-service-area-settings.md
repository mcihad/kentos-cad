# ADR 9355: Ağ sonuçlarının birimleri, Tablo'nun etkin katmanı izlemesi ve Hizmet alanı ayarlarının panelde düzenlenmesi

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, ağ turu
  (`gis-eksik-27-11-2026.md`, T33 P2, T34 P2, T35 P3). Sahibin sözü: “Gis eksik 27 11 dosyasındaki yeni maddeleri
  tamamla”.
- **İlgili:**
  - [ADR 0209](0209-network-analysis.md) (ağ analizi), [ADR 9332](9332-network-tools-see-the-network.md) ve
    [ADR 9333](9333-service-area-reach-and-buffer-side.md) (araçların ayarları, hizmet alanının çıktıları);
  - [ADR 9334](9334-network-analyses-in-processing-history.md) (ağ analizleri işlem geçmişinde);
  - [ADR 9129](9129-data-layers-in-source-status-and-table-picker.md) (tablo ve çizim hedefi), [ADR 9132](9132-table-conditions-by-layer.md)
    (tablonun koşulları katman başına).

## Bağlam

- **T33:**
  - 30 m'lik rota 50 km/sa'te 2,16 saniye sürer; komut geçmişi “Süre 0.0 dk” dedi.
  - Rota tablosu birimsiz `0.04` ve `30.000` gösterdi; Ulaşılan yollar'ın `0–10` aralığının birimi yoktu.
  - Kısa süre ile sıfır ayırt edilemiyordu.
- **T34:**
  - Katman ağacında Ulaşılan yollar etkinken Tablo önceki katmanda kaldı (Katman ▾ ile seçilmişti).
  - Tablo katmanı elle değişince önceki katmanın “Göster: Seçililer”i taşındı; tek sonuç satırı görünmedi.
- **T35:**
  - Hizmet alanının Aralıklar, Kenar payı ve Çizgiler satırları sağ panelde “sabit”ti.
  - Değiştirmek için komut satırına ve Diğer menüsüne dönmek gerekiyordu.

## Araştırma

- **QGIS, Ağ analizi:** Service area ve Shortest path çıktıları maliyeti sayı olarak yazar. Süre saniye ya da saat
  olarak, hız birimiyle seçilir. Hizmet alanında ağ parçaları ve sınır düğümleri ayrı çıktılardır; her biri ayrı
  istenir ([Network analysis](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/networkanalysis.html#service-area-from-point)).
- **QGIS, Öznitelik tablosu:** tablo açıldığı katmana bağlıdır. Katman ağacında başka katman seçilince açık tablo
  değişmez; yeni tablo etkin katmanınkidir. Göster süzgeci (Show All / Selected) her tablonun kendisinindir
  ([Attribute table](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/attribute_table.html)).
- **KentOS kararı:**
  - Sonuç sütunu adında birimini taşır; kısa süre saniyeyle okunur.
  - Şeritteki Tablo etkin katmanı açar. Katman ▾ ile seçilen katman sabittir; bunu söyler, geri dönüş yolunu sunar.
  - Hizmet alanının ayarları panelde düzenlenir; komut satırıyla aynı belleğe yazılır.

## Karar

### 1. Birimler (T33)

- **Süre sözü:** bir dakikanın altındaki süre saniyeyle söylenir.
  - “Süre 2.16 sn”, gerçek sıfır “0 sn”, değer yoksa “—”; bir dakika ve üstü önceki gibi “Süre 1.4 dk”.
  - Komut satırı, komut geçmişi ve rota parçaları aynı sözü kullanır (`Base::minutes_text`).
- **Sütun adları:** sonucun maliyet sütunu adında birimini taşır (`NetworkDef::cost_column`, ortak sözleşme):

  | Maliyet | Sütun | Değer |
  |---|---|---|
  | Uzunluk | “Uzunluk (m)” | metre, projenin ondalığıyla |
  | Hız maliyeti (Süre) | “Süre (sn)” | saniye, iki ondalık (dakika × 60) |
  | Alan maliyeti, birimli | “Ücret (TL)” | olduğu gibi, iki ondalık |
  | Alan maliyeti, birimsiz | adı | olduğu gibi |

- **Hizmet alanı:** Başlangıç, Bitiş ve Aralık aralıkların biriminde yazılır (`NetworkDef::breaks_unit`).
  - Uzunlukta “Bitiş (m)”, hızda “Aralık (dk)”: aralıklar dakikayla verilir.
- **Nerede:**
  - Etkileşimli En kısa yol ve Hizmet alanı.
  - İşlemler'in En yakın tesis, Maliyet matrisi ve Hizmet alanları araçları; tablo çıktılarının sütunları da.
  - Bağımsız referans üreticisi (`scripts/fixtures/network_processing_cases.py`) aynı kuralla yazar.

### 2. Tablo ve etkin katman (T34)

- **Şeritteki Tablo** (`data.featureTable`) etkin katmanın tablosunu açar.
  - Katman ▾ ile sabitlenen katman bırakılır.
  - Çizimde seçili nesne varsa tablo onların katmanını gösterir; seçimi izleme kuralı değişmedi.
  - Katmanı kendisi veren yollar (Tabloda aç, sınıfın tablosu, seçim sonucunun katmanı) tabloyu açtıktan sonra kendi
    katmanını sabitler.
- **Sabit işareti:**
  - Tablo Katman ▾ ile etkin olmayan bir katmana sabitken çubukta kilitli “Sabit” çipi ve “Etkin katmana geç” durur.
  - İpucu iki katmanı adıyla söyler.
  - Etkin katmana geç tabloyu etkin katmana götürür; tablo sonra etkin katmanı izler.
  - Çizim ve veri katmanının çubuğunda aynıdır.
- **Göster katmanın:** Göster (Tümü, Seçililer, Görünümdekiler) Ara, İfade süzgeci ve sıra gibi katman başına tutulur
  (ADR 9132). İlk kez gelinen katman bütün satırlarını gösterir; kendi seçimi o katmana dönünce geri gelir.

### 3. Hizmet alanının ayarları (T35)

```text
Etkin aracın ayarları
Ağ              qa_yollar ağı (sabit)
Maliyet         Uzunluk, metre (sabit)
Aralıklar (m)   [35            ]
Kenar payı      [50.000 m      ]
Yaklaşık alan   (●)
Ulaşılan yollar (●)
Çıktı           Çıktılar: “Yaklaşık hizmet alanı” (Başlangıç (m), Bitiş (m)): ulaşılan yolların 50.000 m
                çevresi, yaklaşık; “Ulaşılan yollar” (Aralık (m)): ağın ulaşılan parçaları.
```

- **Aralıklar ve Kenar payı:**
  - Panelde yazılır, Enter ya da alandan çıkış uygular.
  - Komut satırının kuralıyla denetlenir; ardındaki birim gösterilenin aynısıdır.
  - Yanlış değer söylenir, değer önceki gibi kalır.
- **Çıktı anahtarları:**
  - Yaklaşık alan (öntanımlı açık) ve Ulaşılan yollar (komut satırındaki Çizgiler (Ç) ile aynı ayar).
  - Yalnız Ulaşılan yollar açıkken tek çizgi katmanı yazılır: “Hizmet alanı: 3 ulaşılan yol parçası yazıldı.”
  - Son açık çıktı kapatılmaz; söylenir. Ç ile çizgiler kapanırken alan da kapalıysa alan açılır.
- **Çıktı satırı:** çalıştırmadan önce yazılacak katmanları, sütunlarını ve birimlerini söyler.
- **Geçmiş:** çalıştırmanın değerleri `areas`'ı da taşır; Yeniden aç anahtarı geri getirir.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Ağ sonuçlarının birimleri, kısa süre saniyeyle | yok: Mimari'de ağ araçları yoktur | yok: ağ araçları CBS'nindir (ADR 9353) | yok: aynı | var |
| Tablo'nun etkin katmanı açması, Sabit işareti, Göster katman başına | yok: Tablo sekmesi CBS ve Harita'dadır | var | var | var |
| Hizmet alanının panelde düzenlenen ayarları | yok: ağ araçları yoktur | yok: aynı | yok: aynı | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | `cad.processing.run` ile İşlemler'in ağ araçları (En yakın tesis, Maliyet matrisi, Hizmet alanları) aynı birimli sütunları yazar: kural ortak sözleşmededir (`NetworkDef::cost_column`, `column_value`, `breaks_unit`). |
| MCP | var | Aynı gerekçe. |
| Yapay zekâ ajanı | var | Aynı gerekçe. |
| Eklenti | yok | İşlem aracını çalıştırmak eklentiye kapalıdır (ADR 9310). Etkileşimli araçlar ve tablo arayüzün işidir. |

Tablo ve Hizmet alanının paneli arayüzün işidir; kanallar tabloyu okumaz, aracı değerleriyle çalıştırır.

## Açık

- **Eski çıktılar:** önceki sürümle yazılmış “Süre” ve “Uzunluk” sütunları olduğu gibi kalır; taşıma yok (uygulama
  geliştirmede).
- **Saat:** çok uzun süreler de saniye ya da dakikadır; saate geçiş yok.
- **Komut satırında alan anahtarı:** Yaklaşık alan yalnız paneldedir; komut satırında harfi yoktur.
- **Gerçek pencere:** bu dilimin akışları fare ve klavyeyle denendi (Doğrulama); `scripts/usage/real/`'ın Linux koşusu yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **CBS paketi** (`cargo test -p kentos-desktop-cbs`, 66 geçti):
  - `networks::units_tests::a_30_m_route_says_its_time_in_seconds_and_its_columns_carry_their_units`: A → C 30 m,
    komut satırı ve geçmiş “Uzunluk 30.000 m, Süre 2.16 sn”; Rota'da “Uzunluk (m)” 30.000, “Süre (sn)” 2.16.
  - `networks::units_tests::the_pane_writes_the_breaks_the_margin_and_which_outputs_are_made`:
    - panelden “35 m” ve kenar payı 20; yanlış “30 20” söylendi, değer kaldı;
    - yalnız Ulaşılan yollar: çıktı satırı onu söylüyor, son çıktı kapatılmıyor;
    - 3 parça “Aralık (m)” 0–35 ile yazıldı, alan yazılmadı.
  - `service_area_tests` ve `analysis_history_tests` yeni satırlarla geçti.
- **Masaüstü:** `features::follow_tests::tablo_opens_the_active_layer_and_seciler_stays_with_its_layer` geçti:
  - Tablo `bakim`'i açıyor; Katman ▾ `mahalleler`'i sabitliyor, “Sabit” iki adı söylüyor;
  - Seçililer `bakim`'e taşınmıyor, `mahalleler`'e dönünce geri geliyor;
  - Etkin katmana geç ve şeritteki Tablo etkin katmana götürüyor; etkin katman değişince tablo izliyor.
- **Masaüstü, ortak sınamalar:** `-- processing:: features:: form_scenes module_tests ribbon_tests history::text catalog
  settings`: 405 geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır (render birleşmesinin bağları).
- **İşlemler:** `cargo test -p kentos-processing --test cases` 38 geçti. Ağ durumları üreticiyle yeniden yazıldı
  (`network_processing_cases.py`, bağımsız referans; `--check` yeşil). Değişen yalnız sütun adları ve dakikanın saniyesi.
- **Programlar:** `cargo test -p kentos-cad` 37 geçti. `network-route` izi “Süre 40.32 sn” ve “Süre (sn)” 72.38
  bekliyor; `network-service-area` “Aralık (dk)”, “Başlangıç (dk)”, “Bitiş (dk)” bekliyor.
- **Resim** (`.run/gui-evidence/kullanim-9355/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-rota-birimler`: Rota tablosu “Süre (sn) 2.16”, “Uzunluk (m) 30.000”;
  - `02-hizmet-ayarlari`: düzenlenebilir Aralıklar (m), Kenar payı ve iki anahtar;
  - `03-tablo-sabit`: “Sabit” ve “Etkin katmana geç”, “Çizim hedefi: bakim”.
- **Gerçek pencere** (27 Kasım 2026, macOS; sistem imleci ve klavye olaylarıyla, yalıtılmış kullanıcı dizininde, sınama
  GeoPackage'ı üzerinde; resimler `.run/real-9353/shots/`, Git'e eklenmez):
  - 30 m'lik rota: komut satırı ve geçmiş “Süre 2.15 sn”; Rota tablosu “Süre (sn) 2.15”, “Uzunluk (m) 29.878”.
  - Hizmet alanı: panelde Aralıklar (m) “35”, Yaklaşık alan kapalı, Ulaşılan yollar açık; komut satırı aynı ayarı
    gösterdi; yalnız “Ulaşılan yollar” yazıldı, sütunu “Aralık (m)” 0–35.
  - Tablo: şeritteki Tablo etkin katmanı açtı; Katman ▾ ile `bakim`'de “Sabit” ve Etkin katmana geç; Etkin katmana geç
    Taslak'a döndü.
