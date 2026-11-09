# 3940 — Stok hacminde köşelerden üçgenlenmiş taban

Tarih: 2026-11-09. plan/map-cad.md MAP-15, yedinci tur. Durum: taslak. ADR 3140'ın “Sonra”sındaki “Üçgenlenmiş taban
(köşelerden üçgenlenmiş yüzey, Pix4D'nin önerdiği)”.

İlgili:
- [ADR 3140](3140-stockpile-volume-over-a-boundary-base.md): stok hacmi ve düzlem tabanları;
- [ADR 0900](0900-road-terrain-surface.md): yüzey çekirdeğinin kısıtlı Delaunay üçgenlemesi (`kentos.cdt/1`).

## Dayanak

- **Pix4D, “Menu View > Volumes > Sidebar > Objects - PIX4Dmapper”** ([belge](https://support.pix4d.com/hc/en-us/articles/208652926);
  ADR 3140'ın okuması, 9 Kasım 2026): hacim tabanlarından **Triangulated** bütün köşeleri bağlar, hacmi tabanın üstünde
  ve altında üçgenlerle hesaplar.
- **Pix4D, “Which Base Surface is recommended for the Volume Calculation?”**
  ([belge](https://support.pix4d.com/hc/en-us/articles/217114246); ADR 3140'ın okuması, 9 Kasım 2026): sınırın tamamı
  görünüyorsa, özellikle çevre düz değilse üçgenlenmiş taban önerilir.
- Belgeler üçgenlemenin kuralını (Delaunay mı, kulak kesme mi) ve dört köşe bir çemberdeyken seçilecek köşegeni söylemez.
- **KentOS bugün** (ADR 3140): beş taban vardı (en alçak, en yüksek, ortalama, eğimli düzlem, kot); hepsi tek düzlemdi.
  Eğimli zeminde, sınırı boyunca kotu değişen bir yığında taban sınırın kotlarını izlemiyordu.

## Karar

1. **Yeni taban `triangulated`** (`road.terrains.stockpile`; komut sürümü değişmez):
   - sınırın dış köşeleri, yüzeydeki kotlarıyla, dış halkanın içinde üçgenlenir. Üçgenleme yüzey çekirdeğinin kısıtlı
     Delaunay'ıdır (`kentos.cdt/1`): halka kapalı kırık hat ve sınırdır, iç bükey halkanın girintisine üçgen düşmez;
   - her üçgen kendi düzlemiyle tabandır. Kuralı belgelerde olmadığından KentOS'un seçimidir (ADR'de açık).
2. **Hesap** (`kentos_road::stockpile::measure_tin`): yüzeyin her üçgeni halkaları kırpar (ADR 3140), parça bir de taban
   üçgenleriyle kırpılır (Sutherland–Hodgman; iki üçgenin kutuları ayrıksa atlanır). Fark (yüzey − taban üçgeninin
   düzlemi) doğrusaldır; parça sıfırında bölünür, alanı ve birinci momentleriyle kesin tümlenir. Düzlem tabanların hesabı
   aynen kalır (aynı işlemler, aynı sıra).
3. **Belirsiz köşegen:** dört köşe bir çemberin üstündeyse (dikdörtgen gibi) ve aradaki kenar halkanın kenarı değilse
   üçgenleme tek değildir. Kesin `incircle` ile bulunur. KentOS çekirdeğin köşegenini alır, notta söyler: “Köşelerden
   dördü bir çemberin üstünde: tabanın üçgenlemesi tek değil. Köşegen (…)–(…) KentOS'un üçgenlemesinindir; öbür köşegen
   hacmi değiştirebilir.”
4. **Cevap:** üçgenlenmiş tabanda `plane` yoktur (tek düzlem değil); `triangles` tabanın üçgenleridir. Her üçgen üç köşe
   `[x, y, z]`, en küçük yerinden (önce x, sonra y) saat yönünün tersine; liste köşelerine göre sıralı. Öbür tabanlarda
   cevap değişmez (`plane` vardır, `triangles` yazılmaz).
5. **Retler** (`invalid_stockpile`): yüzeyin dışında kalan köşe (öbür köşe tabanlarınki gibi); köşeleri üçgenlenemeyen
   sınır (`boundary`): “Sınırın köşeleri üçgenlenemiyor: köşeler bir doğru üstünde ya da üst üste” ya da “sınırın
   kenarları birbirini kesiyor”. Bilinmeyen tabanın sözü artık triangulated'ı da sayar.
6. **Masaüstü** (Stok hacmi › Taban): **Üçgenlenmiş** seçeneği (Eğimli düzlem ile Kot arasında). Özette “Taban: köşelerden
   N üçgen, eğimleri %a–%b”; düzlemlerde eskisi gibi kot ya da eğim.
7. **Kapsam işareti:** ADR 3140'taki gibi **Harita + Yol**.

## Kanıt

- **Çekirdek** (`stockpile::tests`):
  - piramitte kare sınırın köşeleri (hepsi 100) üçgenlenince taban 100'deki yataydır, hacim 800 m³. Kare dört köşe bir
    çemberde: belirsiz köşegen bir tane;
  - eğimli zeminde (köşeleri düzlemde) beşgenin üçgenlenmiş tabanı eğimli düzlemin hacmini verir, üç üçgen, belirsiz
    yok;
  - L'nin dört üçgeni L'nin alanını örter, girintide üçgen yok;
  - kendini kesen halka üçgenlenmez (`constraints_cross`).
- **Bağımsız başvuru** (`scripts/fixtures/road_terrain_stockpile_cases.py`; Triangle ve kesirler, KentOS kodu yok):
  - taban Triangle'ın halkayı kısıtlı üçgenlemesidir (girinti yenir). Üçgenlemenin tek olduğu doğrulanır: her iç kenarın
    karşısındaki köşe çemberin kesin dışında;
  - kırpma ve tümleme kesirlerle;
  - beş yeni durum (30): beşgen ve L, yüzeyin adıyla doğrulama, yüzeyin dışındaki köşe, bir doğru üstündeki köşeler.
    Bilinmeyen taban durumunun sözü güncellendi; öbür durumlar aynı;
  - Rust ve Python geçer; taban üçgenleri Triangle'ınkiyle köşesi köşesine aynı.
- **Python** (`python/tests/test_stockpile_triangulated.py`), testin kendi hesabıyla:
  - taban üçgenleri beşgenin alanını örter, kotları köşelerinki;
  - net hacim (kazı − dolgu) eğimli düzleminkine eşittir, artı her taban üçgeninde iki tabanın tümlevleri farkı (alanı
    çarpı düzlemin ağırlık merkezindeki kotu eksi köşelerin ortalaması);
  - doğru üstündeki köşeler ret.
- **MCP** (`crates/native/mcp/tests/stockpile_triangulated.rs`): iki durumun hacimleri ve üçgenleri durumunki, düzlem yok;
  ret durumun sözüyle. ADR 3140'ın MCP testi değişmeden geçer.
- **Masaüstü** (`stockpile_window_tests::the_triangulated_base_is_the_shared_case_s`): beşgen ve L durumlarınki, üçgenler
  köşesi köşesine; özet satırı. Stok hacminin öbür testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/stok-ucgenlenmis-3940/`.

## Sonra

- Tabanın ve hacmin çizimde gösterimi; CSV (ADR 3140 Sonra).
- Belirsiz köşegende iki köşegenin hacmini birlikte vermek (aralık olarak).
- Kesit yöntemiyle hacim karşılaştırması; malzeme bölgelerinin çizime yazılması (MAP-15'in öbür maddeleri).
- Web (TODOS §0.2).
