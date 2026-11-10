# 4680 — Ayırma çapında sığmayan numaranın kılavuzla dışarıda yazılması

Tarih: 2026-11-10. plan/map-cad.md MAP-11, dokuzuncu tur. Durum: taslak. ADR 4280'in “Sonra”sındaki “Sığmayan numaranın
parçanın dışına kılavuz çizgiyle yazılması”.

İlgili:
- [ADR 4280](4280-separation-number-fitted-in-its-part.md): dış parçanın numarası parçaya sığar (`numberHeight`);
- [ADR 3480](3480-separation-sheet-piece-numbers-drawn.md): dış parçanın numarası çizimde (`label_at`);
- ADR 0146: kılavuzun düzeni (iniş, not).

## Dayanak

- **ArcGIS Pro**, [Place labels outside polygons when they won't fit inside](https://doc.esri.com/en/arcgis-pro/latest/help/mapping/text/place-labels-outside-polygons-when-they-won-t-fit-inside.html)
  (10 Kasım 2026'da okundu):
  - içine sığmayan alan etiketi önce alanın dışında, yatay ve sınırından etiket aralığı kadar uzakta denenir;
  - olmazsa varsayılandan dışa doğru, etiket konana ya da en büyük aralığa varılana dek denenir;
  - isteğe bağlı bir aralık ve tercih edilen dış bölge verilebilir.
  - Sayfa kılavuz çizgisini anlatmaz. Kılavuz KentOS'un mevcut kılavuz nesnesidir (ADR 0146): ucu, çizgisi, inişi ve
    notu.
- **Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik** (ADR 3080, 3480): m. 13(3), dışarıda kalan her parça parsel
  numarasıyla tescil edilir. Numaranın çapta nereye yazılacağını söylemez.
- **KentOS bugün** (ADR 4280): sığmayan numara onda bir adımlarla 0,1'e dek küçülüyordu; hiçbiri sığmazsa yatay ve en
  küçük payda, `fits` false. Dar parçada numara ya okunmaz küçüklükte ya da parçadan taşkındı.

## Karar

1. **`cad.readjustment.sheets`'e `numberLeader`** (`numberHeight` ile; sürüm aynı, ek alan):
   - yalnız tam boy denenir: yatay, sonra en uzun düz kenar boyunca (ADR 4280'in dönüşleri ve yeri);
   - sığarsa içeride yazıdır, ADR 4280'deki gibi;
   - sığmazsa küçültülmez: tam boyda, yatay, bir **kılavuzla** dışarıda yazılır.
2. **Kılavuz** (KentOS'un kuralı; ArcGIS'in dışarıda yatay yerleştirmesi gibi):
   - ucu parçanın etiket yeridir (`label_at`; parçanın içinde);
   - çizgisi aynı yükseklikte doğuya, o çizginin parçanın sınırını son kestiği yerden bir yazı boyu öteye uzanır. İniş
     oradan başlar;
   - iniş 2 boy, not 0,5 boy ötede, soldan hizalı (ADR 0146'nın düzeni);
   - kılavuz parçanın katmanında, parçanın öznitelikleriyle (`Parsel`, `Kısım`, `ParselNo`) yazılır.
3. **Cevap:** etiketin `leader`'ı kılavuzun ucudur. `at` inişin başladığı yer, `height` tam boy, `rotation` 0, `share` 1,
   `fits` false.
4. **Ret:** `numberHeight`'sız `numberLeader` `invalid_input` (`numberLeader`): “Kılavuz, numaranın yazısıyla olur:
   numaranın yazı boyunu (numberHeight) da verin.”
5. **Masaüstü** (Düzenleme penceresi › Ayırma çapı): Numara yazısının altında **Sığmayan numara** anahtarı (“Küçültme;
   parçanın dışına kılavuzla yaz”). Açıkken numara yazısının açıklaması: “tam boyda; yatay ya da en uzun kenarı boyunca
   sığmazsa dışarıda, kılavuzla”.
6. **Kapsam işareti:** ADR 4280 gibi **Harita ve CBS**. Çap taslaktır; yazının yeri resmî bir kural değildir.

## Sınırlar

- Tercih edilen dış bölge doğudur. Dışarıdaki notun öbür parsellerle ya da yazılarla çakışması denetlenmez, başka bir yöne
  geçilmez (ArcGIS'in dışa doğru denemesi yok).
- Yaylı parçada da çizgi yataydır; yayın teğeti boyunca yerleştirme yoktur.
- Kılavuzun okunur en küçük boyu için resmî değer okunmadı.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/readjustment_sheets_cases.py`; KentOS kodu yok; yazı tipinin ilerlemeleri
  dosyadan), 3 yeni durum (46):
  - 6 mm ve kılavuzla: 101/10'un 4 m'lik iki parçasının numarası tam boyda sığmaz, kılavuzla dışarıda (uç parçanın
    ortasında, iniş sınırdan 6 m doğuda); öbürleri tam boyda içeride;
  - 2 mm ve kılavuzla: hepsi tam boyda içeride, kılavuz yok;
  - ret.

  Eski 43 durum aynı. Rust ve Python geçer. Bozma denemesi: iniş yarım boyda başlayınca koşucu yakalar.
- **Python** (`python/tests/test_separation_number_leader.py`), testin kendi okumasıyla (fontTools):
  - 6 m'lik numara 4 m'lik parçaya yatay da dik de sığmaz;
  - kılavuzun ucu parçada, inişi parçanın doğu kenarından bir boy öte, not parçanın dışında;
  - kılavuz olmadan aynı numaralar içeride küçülür;
  - ret.

  ADR 4280'in Python testi değişmeden geçer.
- **MCP** (`crates/native/mcp/tests/separation_number_leader.rs`): etiketler durumunki, iki kılavuz ucu; ret yolu.
- **Masaüstü** (`readjustment_window_tests::the_numbers_too_big_go_outside_on_leaders`): 101/10'un parça katmanında iki
  kılavuz, 6 m, yatay, uçtan doğuya; küçülmüş yazı yok. Düzenleme penceresi testleri ve çapraz denetimler geçer.
  Resimler `.run/gui-evidence/numara-kilavuz-4680/`.

## Sonra

- Dışarıdaki notun çakışma denetimi ve öbür yönler; yaylı parçada yay boyunca yerleştirme.
- Tescil bildirimi (TKGM'nin biçimi beklenir); değer esaslı dağıtım (resmî metinde kural yok).
- Web (TODOS §0.2).
