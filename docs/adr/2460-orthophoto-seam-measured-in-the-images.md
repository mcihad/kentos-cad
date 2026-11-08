# 2460 — İki ortofotonun kaymasının görüntülerin kendisinden ölçülmesi (dikiş ölçümü)

Tarih: 2026-11-08. plan/map-cad.md MAP-12, üçüncü tur (MAP-20'den sonra). Durum: taslak. Plan satırı: “Ortofoto …
mozaik ve iki zamanlı karşılaştırmada kayma ölçülebilsin.”

İlgili:
- [ADR 1480](1480-orthophoto-accuracy-check.md), ortofoto doğruluk denetimi;
- [ADR 1780](1780-two-orthophotos-compared.md), iki ortofotonun noktalarla karşılaştırılması. “Sonra”sındaki “mozaik
  dikiş hattı boyunca kayıklığın örtüşme bandında kendiliğinden ölçülmesi” ve “noktaların görüntüde kendiliğinden
  eşlenmesi (görüntü korelasyonu)” maddelerini bu ADR örtüşme için kapatır.

## Dayanak

- **BÖHHBÜY m. 67(6)** (ADR 1480'in okuması): mozaikteki ortofotolar arasındaki kayıklık en çok ±2·YÖA. Sınır kural
  paketinin `mosaic` katsayısıdır (ADR 1780).
- **Debella-Gilo ve Kääb (2011)**, “Sub-pixel precision image matching for measuring surface displacements on mass
  movements using normalized cross-correlation”, *Remote Sensing of Environment* 115(1), 130–142
  ([ESA poster](https://earth.esa.int/eogateway/documents/20142/3016001/EO-Summer-School-5-Poster-Sub-pixel-Precision-Image-Matching-for-Displacement-Measurement-of-Mass-Movements-Using-Normalised-Cross-Correlation.pdf);
  özet 8 Kasım 2026'da okundu):
  - kayma normalize çapraz korelasyonla ölçülür;
  - piksel altı için iki yol vardır: eşlemeden önce görüntüyü ara değerlemek, ya da eşlemeden sonra korelasyon
    yüzeyinin tepesini aramak. İkincisinin yolları çift kübik ara değer, parabol ve Gauss uydurmadır;
  - en iyisi görüntünün çift kübik ara değeridir. Parabol ve Gauss daha az doğrudur.
- **KentOS bugün** (ADR 1780): iki görüntü yalnız kullanıcının ikisinde de okuduğu aynı adlı noktalarla
  karşılaştırılıyordu. Dikiş boyunca ya da örtüşmede kendiliğinden ölçüm yoktu.

## Karar

1. **`seam` girdisi** (`cad.orthophoto.compare`, isteğe bağlı): `step` (m), `window` (tek, 5–255 piksel), `search`
   (1–64 piksel), `correlation` (kullanıcının en az korelasyonu; yoksa her yer sayılır), `folder` (göreli dosya için
   çizimin klasörü).
2. **Yerler:** örtüşmede `step` metrede bir yer. Örtüşmenin içinden yarım pencere ve arama kadar uzaktadırlar.
   Sıra kuzeyden güneye, batıdan doğuya. Yer, birinci görüntünün içine düştüğü pikselin ortasıdır.
3. **Eşleme:**
   - birinci görüntünün `window` × `window` parçası ikincide aranır, `search` piksel her yöne. Gri değer ilk üç
     bandın ortalamasıdır;
   - arama, yere en yakın merkezli ikinci görüntü pikselinden başlar;
   - ölçü sıfır ortalamalı normalize çapraz korelasyondur;
   - tepe (eşitlerden ilki, önce satırlar) iki yönde ayrı ayrı, komşularından geçen parabolle piksel altına
     inceltilir. Debella-Gilo ve Kääb'e göre bu yol görüntü ara değerinden az doğrudur; bu dilimin seçimidir
     (Sonra'ya bakın).
4. **Kayma:** ikincideki eşin yeri − yer (m), noktalardaki gibi “ikinci − birinci”.
5. **Yer durumu:**
   - `flat`: parçalardan birinin dokusu yoktur, korelasyon tanımsızdır;
   - `edge`: tepe aramanın kenarındadır, asıl tepe dışarıda olabilir. Kayma yazılmaz;
   - `low`: korelasyon istenen sınırın altındadır. Kayması yazılır ama sayılmaz;
   - `ok`.
6. **Cevapta `seam`:** her yer (yer, kayma, korelasyon, durum, sınırı aşar mı), durum sayıları, `ok` yerlerin kayma
   istatistiği (ortalaması sistematik kaymadır) ve hepsinin sınırın içinde olup olmadığı. Not satırı da ölçümü anlatır.
7. **Sınırlar:**
   - iki görüntünün pikselleri aynı boyda ve kuzeye bakan (dönüksüz) olmalı; yoksa ret (`seam`). Yeniden örnekleme
     sonraya kalır;
   - örtüşme yoksa ya da pencere ve arama için darsa ret;
   - görüntü dosyası yoksa `file_not_found`.
8. **Masaüstü** (Ortofoto karşılaştırması):
   - **Dikiş ölçümü** satırı: “Görüntülerden ölç”, adım, pencere ve arama, en az korelasyon. İlk değerler 10 m, 21 ve
     5 pikseldir; korelasyon boştur. Bunlar yöntem ayarıdır, yönetmelik değeri değildir;
   - özette yer sayıları, ortalama ve en büyük kayma, sınır;
   - **Noktalar | Dikiş** tablosu: yerler (Y, X, dx, dy, kayma, korelasyon, durum);
   - noktası olmayan çizimde okuma katmanları katmanların kendisinden seçilir.
9. **Kapsam işareti:** ADR 1780'deki gibi **CAD + CBS** (raster bağlam sekmesi).

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/orthophoto_compare_cases.py`, KentOS kodu yok.
  - Üç sentetik GeoTIFF burada bayt bayt yazılır: 8 bit gri, sıkıştırmasız, tek şerit, 200 × 120 piksel, 10 cm.
  - Sahne analitik bir dokudur, beş sinüsün toplamı.
  - Görüntüler:
    - ikinci görüntü sahneyi bilinen bir kaymayla gösterir: (3,4; −2,1) cm, yani (0,34; −0,21) piksel. Üçüncüsü
      2,7 piksel doğuya kaydırır;
    - birincide dokusuz bir alan, ikincide değişen bir alan vardır.
  - Ölçüm ADR'nin kurallarıyla yeniden yazılmıştır (toplamlar satır sırasıyla).
  - **Bağımsız doğru bilinen kaymadır:** penceresi temiz dokuda kalan her `ok` yerin kayması gerçeğe 0,05 piksel
    içindedir. En büyük hata 0,027 pikseldir (2,7 piksellik kaymada 0,023). Ortalama (3,26; −2,19) cm.
  - Yeni durumlar:
    - korelasyon sınırıyla: 36 `ok`, 25 `low`, 2 `flat`;
    - sınırsız: değişen alan da sayılır;
    - 2,7 piksel: sınırı (20 cm) aşar;
    - dar arama: her tepe kenarda;
    - 7 ret.
  - 40 durum (29'dan). Eski durumlar aynıdır. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/orthophoto_seam.rs`): iki durumun çizimi `.kcad` olarak açılır, görüntü yolları
  mutlaktır. Yerler, durumlar ve sayılar durumlarınkidir.
- **Masaüstü** (`ortho_compare_window_tests::dikis_olcumu_gives_the_shared_case_s_places`): Dikiş ölçümü durumun
  yerlerini verir; çizime bir şey yazılmaz. Çapraz masaüstü denetimleri geçer. Resimler
  `.run/gui-evidence/dikis-olcumu-2460/`.

## Sonra

- Görüntü ara değeriyle (çift kübik) piksel altı: Debella-Gilo ve Kääb'e göre parabolden doğru.
- Farklı piksel boyunda ya da dönük görüntüler: birinin öbürünün ızgarasına yeniden örneklenmesi.
- Çizilmiş bir dikiş hattı boyunca ölçüm (bugün örtüşmenin bütünü).
- Noktasız (yalnız dikiş) karşılaştırmada okuma katmanlarının isteğe bağlı olması.
- Yer başına korelasyonun çizimde gösterimi; büyük görüntülerde iş olarak koşma (ilerleme, durdurma; ADR 2440 gibi).
- Görüntüden türeyen nesnenin kaynağa izi (MAP-01'in sahip kararını bekler).
