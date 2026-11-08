# 2240 — İmar planı denetiminde yola giren karar (`roadWidth`)

Tarih: 2026-11-08. plan/map-cad.md MAP-09, üçüncü tur. Durum: taslak. Plan satırı: “İmarı sembol çiziminden tipli ve
denetlenebilir plan modeline taşı.”

İlgili:

- [ADR 1420](1420-zoning-plan-typed-check.md): İmar planı denetimi, `imar-plani` profili. Yol aksının alanları
  YolKimlik, Tür ve Genişlik (m)'dir;
- [ADR 2020](2020-zoning-road-continuity-and-upper-plan.md): kopuk yol ve üst plan dışına taşma. Bu ADR onun
  “Sonra”sındaki “yol genişliğinin karar alanıyla tutarlılığı” maddesini kapatır. Parselle tutarlılık sonraya kalır.

## Dayanak

- **Planlı Alanlar İmar Yönetmeliği** ([mevzuat.gov.tr](https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=23722&MevzuatTertip=5),
  8 Kasım 2026'da okundu):
  - Madde 9'un başlığı “Yol genişliklerine göre bina kat adetleri”dir;
  - 9(1)(b)'ye göre bu maddede kullanılan yol genişliği, parselin ön cephesindeki yolun planda belirtilen
    genişliğidir.

  Yazılan genişlik böylece yapılaşma hakkını belirleyen bir değerdir. Çizimde yolun iki yanındaki kararlar o
  genişliğe girmemelidir.
- **ArcGIS Pro, poligon topoloji kuralı “Must Not Overlap With”**
  ([kurallar](https://doc.esri.com/en/arcgis-pro/latest/help/editing/geodatabase-topology-rules-for-polygon-features.html),
  8 Kasım 2026'da okundu):
  - birinci sınıfın poligonları ikinci sınıfınkilerle çakışmamalıdır;
  - çakıştıkları yerde poligon hatası oluşur.

  KentOS'un topoloji çekirdeğinde aynı kural vardır: “… ile çakışmamalı” (ADR 0202).
- **KentOS bugün** (ADR 1420, 2020):
  - yolun Genişlik alanı vardı ama hiçbir şey onu okumuyordu;
  - 12 m yazılıp 10 m'lik aralığa çizilen yol bulunmuyordu.

## Karar

1. **`roadWidth`:** Yollar katmanı çizimdeyse, Genişlik'i 0'dan büyük bir sayı olan her yol için aşağıdaki denetim
   yapılır.
   - **Şerit:** aksın iki yanında genişliğin yarısı kadardır. Çekirdeğin sol ve sağ tek yanlı tamponları birleştirilir:
     uçlar düzdür, dönüşün dış yanı yuvarlaktır. Yol, aksının bittiği yerde biter. Kavşakta aks öbür yolun aksında
     biter, köşedeki adaya taşmaz.
   - **Yolun planı:** yolun en uzun kenarının ortasını (eğrisi üstünde; eşitlerde ilki) içine alan en alt kademeli
     plandır. Bunun nedeni: nazım planın kendi kararı uygulama planının yollarını da kaplar; o, uygulama planının yolunun
     işi değildir (ADR 1420 §2'deki gibi plan plan).
   - **Denetlenen kararlar:** yolun planının, kullanımı yol olmayan kararlarıdır. Yol kararı yolun alanıdır.
   - **Kural:** şerit bu kararlarla çakışmamalıdır. Toleranstan dar parça sayılmaz (ADR 0202 §2). Bulgu yolu ve kararı
     adlandırır, alanı ölçülen parçaların toplamıdır.
   - İleti: “Y1” yolunun 12 m genişliği içine Konut alanı kararı “A1” giriyor.
   - Genişliği yazılmamış yol ya da hiçbir planın içinde olmayan yol denetlenmez.
2. **Sıra:** `roadWidth` `roadGap`'ten sonra gelir (18 tür). Komut sürümü değişmez: yeni tür yalnız genişliği yazılmış
   yolda görünür, eski çizimlerin bulguları aynı kalır.
3. **Masaüstü** (İmar planı denetimi): **Yola giren karar** Alan grubundadır. Nesne sütunu kararı ve yolu adlandırır;
   Seç ve göster ikisini de seçer.
4. **Kapsam işareti:** ADR 1420'deki gibi **Harita + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/zoning_check_cases.py`, KentOS kodu yok.
  - Şerit eksene paralel yollar için dikdörtgendir. Parçalar ADR 1420'nin koordinat sıkıştırmasıyla tam hesaplanır.
  - Dönen yolda hiçbir karar dönüşün yuvarlak köşesine yaklaşmaz; başvuru bunu kendisi doğrular.
  - Durum sayısı 34'ten 40'a çıktı. Çizim: dört ada, aralarında iki yol, yolların alanları yol kararı, nazım planın
    kararı hepsinin üstünde. Yeni durumlar:
    - yollar yazılan genişlikte: bulgu yok (yol kararları ve nazım planın kararı sayılmaz);
    - 12 m yazılan yol 10 m'lik aralıkta: dört karar, her biri 45 m²;
    - genişliği yazılmamış yol: bulgu yok;
    - dönen yol, iki kolunda da: üç karar (45, 69 ve 25 m²);
    - 3,125 cm'lik şerit 1 cm toleransla sayılır, 5 cm toleransla sayılmaz.
  - Rust ve Python geçer. Python koşucusu eski kodla tam olarak üç bulgulu durumu düşürdü.
- **MCP** (`crates/native/mcp/tests/zoning_road_width.rs`): üç durumun çizimi `.kcad` olarak açılır. Bulguların türleri,
  nesneleri, alanları ve iletileri durumunkilerle aynıdır.
- **Masaüstü** (`zoning_window_tests.rs`, `the_decisions_inside_a_road_s_width`):
  - dört bulgu Alan grubundadır;
  - Seç ve göster yolu ve dört kararı seçer.

  Çapraz masaüstü denetimleri 137 geçer. Resimler `.run/gui-evidence/imar-yol-genisligi-2240/`.

## Sonra

- Yol genişliğinin kadastro parseliyle tutarlılığı (yola giden parsel parçası; MAP-11 ile).
- Planda çizilenden dar yazılmış yol: şeridin iki yanında karar olmayan fazla alan.
- Resmî PlanGML şeması, plan değişikliği taslağı, sembolün karardan seçilmesi (ADR 2020 Sonra).
