# 3260 — Dikiş ölçümünde çift kübik piksel altı ve yaylı dikiş hattı

Tarih: 2026-11-09. plan/map-cad.md MAP-12, beşinci tur. Durum: taslak. ADR 2460 ve 2860'ın “Sonra”sındaki “görüntü ara
değeriyle (çift kübik) piksel altı” ve “yaylı dikiş hattı (yay boyunca uzaklık)”.

İlgili:
- [ADR 1780](1780-two-orthophotos-compared.md): iki ortofotonun karşılaştırılması;
- [ADR 2460](2460-orthophoto-seam-measured-in-the-images.md): dikiş ölçümü (NCC, parabolle piksel altı);
- [ADR 2860](2860-orthophoto-seam-along-a-drawn-line.md): çizilmiş dikiş hattı boyunca ölçüm.

## Dayanak

- **Debella-Gilo ve Kääb 2011**, *Sub-pixel precision image matching for measuring surface displacements on mass
  movements using normalized cross-correlation* (Remote Sensing of Environment 115(1), 130–142; ADR 2460'ın okuması):
  - piksel altı için iki yol karşılaştırılır: korelasyon yüzeyine eğri (parabol, Gauss) uydurmak ve görüntünün
    kendisini ara değerle yeniden örneklemek;
  - görüntünün çift kübik ara değeri daha doğrudur. Küçük pencerede fark büyür.
- **Keys 1981**, *Cubic Convolution Interpolation for Digital Image Processing* (IEEE Transactions on Acoustics, Speech,
  and Signal Processing 29(6), 1153–1160):
  - kübik evrişim çekirdeği 4 × 4 komşuyla çalışır;
  - a = −1/2 seçimi örnekleme aralığına göre üçüncü mertebeden yakınsar;
  - çekirdek tam piksellerde 1 ve 0'dır, yani örnekleri aynen geçirir.
- **Yaylı çoklu çizgi** (DXF LWPOLYLINE'ın bulge'ı; [ezdxf belgesi](https://ezdxf.readthedocs.io/en/stable/dxfentities/lwpolyline.html),
  9 Kasım 2026'da okundu; KentOS'un `geom::bulge`'ı):
  - bir kenarın yayı bulge = tan(θ/4) ile verilir, θ iç açı, artı saat yönünün tersi;
  - son köşenin bulge'ı yalnız kapalı çizgide anlamlıdır (kapanan kenar).
- **KentOS bugün:**
  - piksel altı yalnız parabolleydi (ADR 2460);
  - yaylı dikiş hattı reddediliyordu (ADR 2860 §4).

## Karar

1. **`seam.subpixel`** (`cad.orthophoto.compare`; ek alan, komut sürümü değişmez):
   - `parabola`: verilmezse de. Tepe ve iki komşusundan geçen parabol (ADR 2460);
   - `bicubic`: ikinci görüntünün kendisi Keys'in kübik evrişimiyle (a = −1/2) piksel kesirlerinde yeniden örneklenir.
2. **Çift kübik arama** (tam pikseldeki tepe bulunduktan sonra; kenardaki tepe yine `edge`):
   - örnek, yerin çevresindeki 4 × 4 pikseldir: her satır sütun ağırlıklarıyla toplanır, sonra satırlar satır
     ağırlıklarıyla;
   - korelasyonun en yükseği önce tepenin yarım piksel içinde 1/8 piksellik ızgarada aranır, sonra bulunanın 1/8
     pikseli içinde 1/64 piksellik ızgarada. Satırlar, sonra sütunlar; eşitlerin ilki kalır;
   - ızgarada (0, 0) vardır ve çekirdek tam pikselde örneği aynen verir. Bu yüzden sonuç tam pikseldeki tepeden
     kötü olamaz;
   - söylenen korelasyon tam pikseldeki tepeninkidir; durumlar (`ok`, `low`) iki yolda aynı kalır.
   - Okunan ikinci görüntü parçası kübiğin erimi için bir piksel büyür. Örtüşme payı (ADR 2460) bunu zaten içerir.
3. **Not:** “… (piksel altı, görüntünün çift kübik ara değeriyle).” ya da “(piksel altı, parabolle).”
4. **Yaylı dikiş hattı** (`seam.line`):
   - çoklu çizginin yaylı kenarları da okunur. Kenarın uzunluğu yay boyudur (yarıçap × iç açı);
   - k'ıncı yer hattın başından k × adım uzaklıktadır. Yayda, kenarın başından o yay boyu kadar dönülmüş noktadır;
   - kapalı çizginin kapanan kenarının yayı son köşenin bulge'ıdır;
   - ADR 2860'ın “yaylı” reddi kalktı. Öbür retler aynıdır.
5. **Masaüstü** (Ortofoto karşılaştırması): **Piksel altı** satırında Parabol / Çift kübik seçimi. Seçili yaylı
   çoklu çizgi Seçili çizgiyi al ile alınır.
6. **Kapsam işareti:** ADR 1780'deki gibi **CAD + CBS** (raster bağlam sekmesi).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/orthophoto_compare_cases.py`, KentOS kodu yok):
  - kübik evrişim, iki aşamalı ızgara, yay boyu ve yaydaki nokta başvuruda yeniden yazıldı;
  - 48 durum (45'ten; yaylı hat reddi ölçüme döndü, dört durum eklendi). Eski durumlar aynı. Rust ve Python geçer.
- **Bilinen kaymaya karşı** (ADR 2460'ın sentetik görüntüleri, B'nin kayması 0,34 ve −0,21 piksel; temiz yerler):

  | Durum | Yer | En büyük sapma (piksel) |
  |---|---|---|
  | Parabol, 11 piksel pencere, 2 m adım | 20 | 0,108 |
  | Çift kübik, aynı yerler | 20 | 0,019 |
  | Çift kübik, 2,7 piksellik kayma (C) | 8 | 0,013 |
  | Parabol, 21 piksel pencere (ADR 2460) | 10 | 0,027 |

  Küçük pencerede parabol 0,05 piksel sınırını aşar, çift kübik aşmaz (başvuru bunu sağlar).
- **Yaylı hat:** ADR 2860'ın hattının ilk kenarı 0,2 bulge'la yay. Hat uzar, yerler yay boyunca; 3'ten çok yer sayılır.
- **MCP** (`crates/native/mcp/tests/orthophoto_seam.rs`): yaylı hat ve çift kübik durumları da sunucudan aynı
  (10⁻⁹ içinde).
- **Masaüstü** (`ortho_compare_window_tests.rs`):
  - Çift kübik durumun cevabı ve notu; Parabol'e dönünce parabol durumunun cevabı;
  - yaylı hat seçilip alınınca durumun cevabı;
  - resimler `.run/gui-evidence/dikis-cift-kubik-3260/` (çift kübik açık geniş, yaylı hat koyu dar).

  Çapraz denetimler geçer.

## Sonra

- Farklı piksel boyunda ya da dönük görüntülerde birini öbürünün ızgarasına yeniden örnekleme (ADR 2460 Sonra);
  kübik evrişim bunun da çekirdeği olabilir.
- Görüntüden türeyen nesnenin kaynağa izi (MAP-01 kararı); mozaiğin dikiş çokgenlerinden hatlar.
- Web (TODOS §0.2).
