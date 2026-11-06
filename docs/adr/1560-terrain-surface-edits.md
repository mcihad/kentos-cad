# 1560 — Arazi yüzeyinin düzenlemeleri: nokta ekle, kot değiştir, kenar ekle/çevir, üçgen sil (`.kcad` şema 196)

Tarih: 2026-11-07. plan/map-cad.md MAP-14, ikinci dilim. Alt madde: “Nokta ekle/sil/kot değiştir, kenar değiştir,
üçgen dışla, sınır/kırık hat düzenle … işlemleri geçmişten yeniden üretilebilsin”. Durum: taslak. İlgili ADR'ler:
- [ADR 0900](0900-road-terrain-surface.md): yüzey tarifi ve Delaunay;
- [ADR 0916](0916-road-surface-analysis-and-quality.md): `excluded`, nokta dışlama;
- [ADR 1520](1520-map-project-terrain-surfaces.md): Harita'da yüzey.

## Dayanak

- **Civil 3D:** yüzeyin tanımı (Definition) sıralı bir işlem listesidir: Boundaries, Breaklines, Point Groups, Edits.
  - Edits altında Add Line, Delete Line, Swap Edge, Add Point, Delete Point, Modify Point… bulunur.
  - Yüzey her yeniden kurulduğunda işlemler sırayla uygulanır.
  - Bir işlem listeden silinerek ya da kapatılarak geri alınır.

  Kaynaklar:
  - [About the Surface Definition](https://help.autodesk.com/cloudhelp/2021/ENG/Civil3D-UserGuide/files/GUID-EB41E9FE-56DC-4469-8280-2C2C8B7F1B1B.htm);
  - [About Breaklines](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-FCCAF28A-506B-4773-9706-DD1F1F8CC933.htm).
- **BricsCAD TIN:** aynı düzen; tanımlar düzenlemeye açıktır
  ([TIN Definitions for editing](https://help.bricsys.com/en-us/document/bricscad/civil-tools/tin-surfaces/tin-definitions/tin-definitions-for-editing)).
- **BÖHHBÜY:** m. 63(1) otomatik ve yarı otomatik eş yükseklik çiziminde arazinin morfolojik yapısının dikkate
  alınmasını ister. Üçgenlemenin yanlış kenarını düzeltmek (kenar çevirmek, morfolojiye uyan kenarı zorlamak) bunun
  aracıdır. m. 45(1)(c) desen ve karakteristik noktaların ölçülmesini ister; sonradan nokta eklemek bu eksiği
  belgeleyerek kapatır.
- **KentOS bugün:** yüzey tarifi kaynaklardan kurulur (nokta ve kırık hat katmanları, sınır, boşluklar, en uzun kenar,
  tolerans). Onarımın nokta dışlaması vardır (`excluded`). Üçgenler kaydedilmez. Elle düzenleme yoktu.

## Karar

1. **Tarifin düzenlemeleri** (`TerrainSurfaceDef.edits`, sıralı, en çok 10 000):
   - `addPoint {x,y,z}`: hiçbir kaydın olmadığı yere nokta.
   - `setHeight {x,y,z}`: o yerdeki bütün nokta kayıtlarının ve kırık hat köşelerinin kotu.
   - `addEdge {ax,ay,bx,by}`: iki köşe arasına zorunlu TIN kenarı. Kenar çevir, iki üçgenin öbür köşegenidir.
   - `removeTriangle {x,y}`: o yerdeki üçgen çıkar; masaüstü yerine üçgenin ağırlık merkezini yazar.

   Düzenlemeler üçgen kaydetmez. Yüzey her kurulduğunda kaynaklarından sonra sırayla uygulanır. Sınır, boşluk ve
   kırık hat kaynakların kendisidir; düzenlemeleri kaynak nesnelerin düzenlenmesidir.
2. **Uygulama sırası ve kuralları** (`kentos_road::sources::gather` ve `surface::build`):
   - Nokta ekleme ve kot değiştirme kaynak toplanırken, yazıldıkları sırayla uygulanır: eklenen noktanın kotu sonraki
     bir `setHeight` ile değişir.
   - Kenarlar kırık hatlardan ve halkalardan (sınır, boşluk; oturtulan köşeleri dahil) sonra konur. Ucu köşe olmayan ya
     da önceki bir parçayı (kırık hat, halka, önceki kenar) kesen kenar konmaz. Kesişme kesin işaretlerle
     (`orient2d`) bulunur.
   - Üçgen silme kenar yönteminden sonradır: o yerde tutulan üçgen yoksa yapılmaz. Sorguda silinen üçgen
     “dışarıda”dır.
3. **Yapılamayan düzenleme silinmez, söylenir.**
   - Rapor ve uyarılarda `left` şunu gösterir: `{id: "edits[k]", role: "edits", why}`.
   - Nedenler: `occupied`, `no_point`, `no_corner`, `crosses`, `no_triangle`.
   - Kaynak değişince yeniden denenir.
   - Figürlere `removed` (çıkan üçgen sayısı) eklenir. Düzenlemesiz yüzeyde alan ve özet yazılmaz, böylece
     bugünkü özetler aynı kalır.
4. **Özet ve kabul:** düzenlemeler özetin girdisindedir. Düzenlenen yüzey, Kabul et ile kabul edilene dek “eskidi”
   görünür (`road.terrains.edit set` özeti korur).
5. **`.kcad` şema 196:** 195 ve öncesi `edits`'i `unknown_field` sayar. Bağımsız okuyucu (`tools/kcad/kcad.py`) ve
   başvuru yazıcısı da aynı kuralı uygular.
6. **Masaüstü: Arazi yüzeyleri › Düzenlemeler.**
   - **Bölme:** sağ bölme “Kaynaklar | Düzenlemeler” olarak ikiye ayrıldı.
   - **Araçlar:** Nokta ekle, Kot değiştir, Kenar çevir, Üçgen sil. İkonların seçenekleri sahibine gönderildi; B, A,
     A, C seçildi.
   - **Seçim:** her araç pencereyi çekip çizimde bir yer ister (`PickPoint`). Kenet ve yazılan koordinat çalışır;
     Esc vazgeçer.
   - **Kot (m):** Nokta ekle'de boş bırakılırsa yüzeyin o yerdeki kotu alınır.
   - **Kenar çevir:** iki üçgenin dörtgeni dışbükey değilse ya da kenar kırık hat, halka ya da eklenmiş bir kenarsa
     reddedilir; neden söylenir.
   - **Tablo:** Sıra, İşlem, Yer, Kot, Durum (yapıldı / yapılamadı). Altında her yapılamayanın nedeni bir satırdır.
   - **Geri alma:** Sonuncuyu kaldır, Düzenlemeleri temizle. Kaydet tarifi yazar.
   - **Çizim:** silinen üçgenler tehlike renginde, hafif dolgulu gösterilir.
7. **Kapsam:** Harita ve Yol (ADR 1520). Python ve MCP düzenlemeleri tarifin parçası olarak `road.terrains.edit set`
   ile verir.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/road_terrain_command_cases.py` bağımsız referanstır: Shewchuk'un Triangle'ı
  (`triangle` paketi), kesirlerle kesişme ve içerilme. KentOS kodu kullanılmaz.
  - `road.terrains.edit` 17 yeni durum içerir:
    - her işlem;
    - dolu yere nokta, noktasız yerde kot, kırık hattın köşesinin kotu;
    - sırayla ekle-sonra-kot;
    - kenar: köşeden, sınıra oturtulan köşeden, kırık hattı kesen, sınırı kesen, ucu köşe olmayan, önceki kenarı
      kesen;
    - üçgen sil: içeride, dışarıda, aynı yer iki kez;
    - dördü birlikte;
    - iki ucu aynı yerde olan kenarın reddi.
  - `road.terrains.query` düzenlenmiş yüzeyde kotu sınar: eklenen noktada, silinen üçgende, yanında.

  Rust ve Python (`.kcad` gidiş-dönüşüyle) geçer. Öbür arazi dosyaları değişmedi.
- **`.kcad`:**
  - örnek `surface-edits.json/.kcad`;
  - bozuk `broken/surface-edits-in-schema-195.kcad`, `broken/surface-edits-edge-of-no-length.kcad`;
  - `schema-version-197.kcad`.

  Rust ve bağımsız okuyucu aynı sonucu verir.
- **Masaüstü:**
  - `terrain_edits::tests`: her araç ve reddi; Kenar çevir'in köşegeni kurulan yüzeyde kenardır.
  - `terrain_window_tests`:
    - araç seçimi, çizimden seçim, Esc, yüzey dışı seçim;
    - Sonuncuyu kaldır;
    - Kaydet; kurulan yüzeyde kotlar ve çıkan üçgen.

    Resimlerde seçim aracın kendisiyle yapılır: komut satırına Y,X yazılır.

  Resimler `.run/gui-evidence/yuzey-duzenle-1560/` altındadır.

## Sonra

- Sınır ve kırık hattın pencereden düzenlenmesi (bugün kaynak nesnenin düzenlenmesi), yüzey birleştir.
- Bölgesel yeniden kurma ve bağlı eğri/profil/hacmin yüzey revizyonuna bağlanması.
- Düzenlemeyi kapatma (silmeden), düzenleme listesinde sıra değiştirme.
