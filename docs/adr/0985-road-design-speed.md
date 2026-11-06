# 0985 — Aksın proje hızı ve kurbun kendi hızı (V)

Tarih: 2026-11-06. Durum: kabul (Y79). Kapsam ROAD-SEC-11'in some bilgisi (V satırı), ROAD-ALIGN-11'in hız kaynağı
(kısmen). ADR 0983 ve 0984'ün §Açık'ındaki "V yazılmıyor: projede tasarım hızı yok".

## Sorun

KGM'nin some kutusu her kurbun proje hızını yazar (`V = 80`). KentOS'ta hız yalnız bazı sorgu komutlarının
parametresiydi (`road.alignments.check`, dever ve genişleme önerileri, görüş mesafesi); projede saklanmıyordu. Bu yüzden
kutu V'yi yazamıyordu.

## Araştırma (6 Kasım 2026)

### Türkiye mevzuatı ve KGM

- **KGM 2019 şartnamesi** (hâlâ geçerli; 2025 ve 2026 birim fiyat listeleri ona bağlı, ADR 0984), s. 5:
  - "Yatay kurblar proje hızına göre seçilmelidir."
  - "Maksimum deverler proje hızı, kurb yarıçapı, kavşak yerleri … dikkate alınarak … tespit edilmelidir."
  - Proje sonu raporu: "Yol proje geometrik standartları: Projede kabul edilen kriterlerin sebep ve
    sonuçları."
- **KGM örnekleri:**
  - 2019 EK-10'da S1 `V=110`; aynı yolun EK-11'indeki kavşak kurbu S11 `V=50` (`S=KAVŞAK DEVERİ`). **Hız kurptan
    kurba değişebiliyor.**
  - Haziran 2024 örnek paftalarında her kutu `V = 80`.
- **Karayolu Tasarım El Kitabı (2005, 2022):** yol sınıfı ve araziye göre proje hızı tablosu bunlardadır. 2022 baskısı
  ücretli KGM yayınıdır ve okunamadı. Bu yüzden KentOS hız önermez, sınıf tablosu kurmaz. İkincil bir ders notu
  (Bartın Ü.) art arda kesimlerde hız farkının 10–15 km/sa'i aşmamasını önerir; kaynağı doğrulanamadığı için kural
  olarak alınmadı.
- **Karayolları Trafik Yönetmeliği md. 100** (2918 sayılı Kanun md. 51): yasal hız sınırları proje hızı değildir.
  İkincil kaynaklara göre otomobilde bölünmüş yolda 110, otoyolda 120 (KGM ilanıyla 130, YİD otoyolunda 140) km/sa.
  Resmî metin bu araştırmada okunamadı. En yüksek sınırın 140 km/sa olması, aşağıdaki akıl sınırının gerekçesidir.

### Yabancı yazılımlar

- **Civil 3D:** aksın "Design Speeds" tablosu: istasyonlardan başlayan hızlar. Tasarım denetimleri ve dever hesabı
  bunları okur.
- **OpenRoads:** aksa "Design Standard" ve hız atanır; kurp başına geçersiz kılınabilir.
- **LandXML 1.2:** `Alignment` altında `DesignSpeed` (staStart, speed).

## Karar

- **Sözleşme** (`kentos_contracts`):
  - `AlignmentDef.design_speed` (`designSpeed`, km/sa): yolun proje hızı.
  - `AlignmentPi.speed`: o PI'nın kurbunun kendi hızı (KGM'nin V=110 / V=50 örneği).
  - İkisi de isteğe bağlıdır. 0'dan büyük, en çok 200 km/sa (`MAX_DESIGN_SPEED`; bir akıl sınırıdır, standart değil:
    tasarım tabloları 130'da biter, yasal sınır en çok 140).
  - Paralel aksın kendi hızı olmaz, kaynağınınkini kullanır (`invalid_alignment`).
- **Revizyon artmaz:** hız geometri değildir. `road.alignments.edit` `set` `designSpeed` (0 kaldırır) ya da PI'ların
  `speed`'ini değiştirdiğinde aksın revizyonu aynı kalır. Bağlı boykesit, dever ve koridor "eskidi" olmaz.
- **KCAD şema 193:** `designSpeed` ve `speed`. 192 ve öncesi `unknown_field` der
  (`broken/road-speed-in-schema-192.kcad`). Örnek `road-speed.json/.kcad`; çok yeni sürüm dosyası
  `schema-version-194.kcad`. Bağımsız Python okuyucusu (`tools/kcad/kcad.py`) ve başvuru yazıcısı da 193'ü bilir.
- **Some kutusu:** başlığın altında `V = 80`. Değer PI'nın hızı, yoksa aksınkidir; hiçbiri yoksa satır yazılmaz. Tam
  km/sa ondalıksız, değilse bir ondalık.
- **Güzergâh penceresi:** Ad ve Başlangıç km'nin yanında "Proje hızı (km/sa)" alanı; PI tablosunda Ove'den önce
  "V (km/sa)" sütunu (uçlarda "—").
- **Kutunun maskesi:** kutudaki yazılar maskelidir. Altındaki ölçüm noktası başlığa karışıyordu ("S○1"). Maske kutunun
  genişliğine yayıldığında (paragraf `box_width`) harita yazıları çizgilerin üstüne çizdiği için kutunun yatay
  çizgilerini de örttü. Bu yüzden maske yazının kendisi kadardır.

## Ölçüm

- **Komut durumları** (`scripts/fixtures/road_alignment_command_cases.py`, KentOS kodu yok):
  - oluşturma: proje hızı 80 + kurbun 50'si, hız 0, kurbun 250'si ve paralel aksın hızı retleri;
  - düzenleme: hız yazılır, kurbun hızı yazılır (ikisinde de revizyon 1 kalır, elemanlar "kept"), 0 ile kalkar.
  - Bütün `road.alignments.` durumları geçti.
- **KCAD:** Python okuyucusu 1646 dosyayı tutarlı buldu. Rust yazıcısı `road-speed.kcad`'ı başvuru yazıcısıyla bayt
  bayt aynı yazar. KCAD testleri 242 geçti (geçerli örnek 223 → 224).
- **Uygulama testi:** hızsız kutuda V yok; aksın 80'i `V = 80`; kurbun 62,5'i `V = 62.5` (aksınkini geçer).
- **Pencere testi:** proje hızı ve V yazılıp kaydedilir, revizyon değişmez. Alan boşaltılınca aksın hızı kalkar,
  kurbunki kalır.
- **Testler:** masaüstü 1853 (`RUST_MIN_STACK=67108864`), uygulama 519 + 137, komutlar, sözleşmeler, KCAD, yol ve plot
  3449. Python SDK 545 testten 544'ü geçti. Biri main'den gelen `file.package`'ın bağımsız Python koşucusu yok (ADR
  1110'un işi), yolla ilgisi yok.
- **Resimler** `.run/gui-evidence/yol-y79/`:
  - Güzergâh açık/geniş ve koyu/dar: alan ve sütun, taşma yok;
  - KGM paftası ve PDF'i: kutuda `V = 80`.
  - İlk resimde ölçüm noktası başlığa karışıyordu; maskelendi. İkinci ve üçüncüde geniş maske çizgileri örttü; yazı
    maskesine dönüldü.

## Açık

- ~~**Denetimlerin hızı aksından okuması:**~~ Y80, ADR 0986. `road.alignments.check`, dever ve genişleme önerileri, görüş mesafesi hızı hâlâ
  parametre olarak alır (ROAD-ALIGN-11).
- **Km aralığıyla hız** (Civil 3D'nin Design Speeds tablosu): bugün aks + kurp. LandXML `DesignSpeed` Y82'de okunur ve
  yazılır (ADR 0988).
- **Kutunun tamamını örten maske:** kısa bir satırın yanındaki nokta görünür. Doğru çözüm alan maskesi (wipeout), ama
  haritanın kopyasına nesne yazan `NewObject` maske taşımıyor.
- **Proje hızı önerisi** (yol sınıfı, arazi): El Kitabı'nın tablosu okunamadı.
