# 2640 — İmar planı denetiminde yola giren parsel

Tarih: 2026-11-08. plan/map-cad.md MAP-09, dördüncü tur. Durum: taslak. Plan satırı: “Alan sınırı, karar çelişkisi,
eksik plan notu, kopuk yol devamlılığı ve üst/alt plan tutarsızlığı sahnede bulunabilsin”; ADR 2240'ın “Sonra”sındaki
“yolun parselle tutarlılığı”.

İlgili:
- [ADR 2240](2240-zoning-decision-inside-road-width.md): yola giren karar (yolun şeridi, yolun planı);
- [ADR 1400](1400-parcel-area-check-and-identity.md): parsel katmanı ve parsel kimliği (Ada/Parsel);
- [ADR 1420](1420-zoning-plan-typed-check.md): İmar planı denetimi.

## Dayanak

- **Planlı Alanlar İmar Yönetmeliği m. 9(1)(b)** (ADR 2240'ın okuması): yolun genişliği planda yazılandır.
- **ArcGIS Pro, “Must Not Overlap With”** (ADR 2240'ın okuması): bir katmanın nesneleri öbür katmanınkilerle
  çakışmamalı; çakışan kısım bulgudur.
- **3194 sayılı İmar Kanunu md. 18** (arazi ve arsa düzenlemesi): resmî metin mevzuat.gov.tr'den okunamadı. İkincil
  kaynaklara göre (Anayasa Mahkemesi norm denetimi kararları, hukuk yazıları) yol, meydan, park gibi umumi hizmet
  alanları düzenleme ortaklık payıyla parsellerden ayrılır. KentOS bundan bir oran ya da işlem çıkarmaz; yalnız planın
  yoluna giren parsel kısmını gösterir.
- **KentOS bugün:** `roadWidth` yolun şeridini yolun planının kararlarıyla karşılaştırıyordu; parsel katmanındaki
  parseller (Parsel alan denetiminin okuduğu `parsel` katmanı) hiç bakılmıyordu: yolun genişliği içinde kalan parsel
  kısmı görünmüyordu.

## Karar

1. **Yeni bulgu `roadParcel`** (`cad.zoning.check`): genişliği yazılmış yolun şeridi (ADR 2240'taki gibi aksın iki
   yanında yarı genişlik, uçlar düz; yolun planı olan yollar) parsel katmanındaki bir alanla (`PARCEL_LAYER`, hangi planda
   olursa olsun) çakışmamalı. Çakışan kısmın toleranstan geniş parçalarının alanı `area`'dır; bulgu yolu ve parseli
   anar: ““Y1” yolunun 12 m genişliği içine “101/1” parseli giriyor.”. Parsel Ada/Parsel'iyle anılır, ikisi de yoksa
   “kimliksiz”.
2. Her yolun bulgusu ayrıdır: iki yolun şeridine giren parselin iki bulgusu vardır. Bulgular `roadWidth`'ten sonra gelir.
3. Bu bir denetim bulgusudur; yasal bir sonuç (terk, kamulaştırma, düzenleme ortaklık payı) iddia edilmez. Kadastro
   parseli yola girebilir; uygulama sonrası imar parselinde girmemesi beklenir. Ayrımı kullanıcı katmanıyla yapar.
4. **Masaüstü** (İmar planı denetimi): “Yola giren parsel”, Alan grubunda; Nesne sütunu parseli Ada/Parsel'iyle anar; Seç
   ve göster yolu ve parselleri seçer.
5. **Kapsam işareti:** ADR 2240'taki gibi **Harita + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/zoning_check_cases.py`, KentOS kodu yok (eksene paralel dikdörtgenler, parçalar
  koordinat sıkıştırmasıyla tam).
  - Parseller yolun kıyısında: bulgu yok; Y1 12 m: üç parsel 20, 25, 45 m²; iki yol 12 m: bir parsel iki yola giriyor,
    beş bulgu; 3,125 cm'lik şerit 1 cm toleransla sayılır, 5 cm'le sayılmaz; kimliksiz parsel.
  - 46 durum (40'tan); var olan durumlar değişmedi. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/zoning_road_parcel.rs`): çizim `.kcad` olarak açılır, üç parsel bulgusu durumunkiler.
- **Masaüstü** (`zoning_window_tests.rs`): alanlar ve cümle durumunkiler, Alan grubu, Seç ve göster sekiz nesneyi seçer.
  Çapraz denetimler geçer. Resimler `.run/gui-evidence/imar-yol-parsel-2640/`.

## Sonra

- Parselin yola giren kısmının sahnede taranması; parsel bazında toplam (birden çok yoldan).
- Plan değişikliği taslağı, sembolün karardan seçilmesi, resmî şema ve eşleme (MAP-09'un öbür maddeleri).
- Web (TODOS §0.2).
