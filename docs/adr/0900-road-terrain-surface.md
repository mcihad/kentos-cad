# ADR 0900 — Yol arazi yüzeyi: kısıtlı Delaunay ve kot sorgusu

Tarih: 2026-11-02. plan/road.md Y2 (ROAD-TIN-01…11, ROAD-SURVEY-01/03/08, ROAD-NUM-10). Durum: Y2.1 uygulanıyor.

## Dayanak

- **OpenRoads Designer:** arazi modeli grafik öğelerden kurulur: noktalar, kırık hatlar, sınır, boşluklar (void),
  boşluk içindeki adalar; *drape void*'in köşeleri alttaki yüzeyin kotunu alır. Sınır kilitli kenardır; dış kenarın
  ince ve uzun üçgenlerini silen “kenar yöntemi” (en uzun kenar ya da ince üçgen) sınırı aşmaz ve iç üçgenlere
  dokunmaz. [Kenar yöntemi](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/404806/GUID-6AFC830B-D74B-48E7-808C-AF0C081BB422.html),
  [Sınır ekle](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/404806/GUID-187C1D8E-213B-49F7-887D-1424F46B410C.html),
  [Montana DOT eğitimi](https://ftp.mdt.mt.gov/other/webdata/External/ESDC/Training/OR/Terrains.pdf).
- **Civil 3D:** kırık hat türleri standart (uçları yüzeyin noktası olur), yakınlık (en yakın var olan noktaya
  oturur), bozmayan (kenarları kestiği yerlere nokta ekler) ve duvar; kesişen kırık hatlara izin bir yüzey ayarıdır.
  [Kırık hatlar](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-FCCAF28A-506B-4773-9706-DD1F1F8CC933.htm),
  [API: standart, yakınlık, bozmayan](https://help.autodesk.com/cloudhelp/2021/ENU/Civil3D-API/files/html/a263a3a4-3b7d-e394-0ffc-63b3b3132f8a.htm).
- **Netcad:** saha noktalarından üçgen oluşturma, bozuk üçgenlerin silinmesi ve üçgen döndürme kübajın ilk
  adımıdır; Netpro 12'de arazi modelindeki düzeltme bütün projeyi dinamik günceller
  ([NETPRO](https://wiki.netcad.com.tr/display/HELP/NETPRO)).
- **Algoritma:** kısıtlı Delaunay üçgenlemesinin her kenarı ya girdi parçasıdır ya yerel Delaunay'dır; artımlı
  yöntem önce Delaunay'ı kurar, sonra parçaları ekler. [Shewchuk ve Brown](https://people.eecs.berkeley.edu/~jrs/papers/inccdtj.pdf),
  [Triangle](https://www.wias-berlin.de/people/si/course/files/Shewchuk96-Triangle.pdf). Kesin işaretli öngörüler
  ([Shewchuk](https://www.cs.cmu.edu/~quake/robust.html)) çekirdekte zaten vardır (`predicates`, ADR 0232 §4).

## Karar

1. **Ayrı crate.** Yol hesabı `crates/shared/road` (`kentos-road`); `kentos-geometry-core`'un `predicates` ve
   `geom::delaunay`'ı üstünde. DOM, Iced ve modül bilmez; Harita'nın imar planı da kullanabilir (plan/road.md
   ROAD-MOD-08). Algoritmanın kimliği `kentos.cdt/1`.
2. **Girdi.** Noktalar (x, y, z ve kaynak kimliği), kırık hatlar (köşe başına kotlu açık ya da kapalı yol), isteğe
   bağlı bir dış sınır ve boşluklar (kotsuz kapalı halkalar), kot toleransı (öntanımlı 0,001 m) ve kenar yöntemi
   (yok ya da en uzun dış kenar). Sayılar metrede, projenin düzleminde; sonlu olmayan değer, sıfır boylu parça ve
   kendini kesen halka atomik retle yerini söyler.
3. **Aynı yer.** Aynı (x, y)'deki iki kayıt kotları tolerans içindeyse tek köşe olur (ilk kaynağıyla, öbürü
   raporda); değilse `height_ambiguous` ile ret: tek değerli yüzeyde iki kot olamaz (ROAD-TIN-05). Tahminle seçilmez.
4. **Kırık hat.** Standart kırık hattır: köşeleri yüzeyin köşesi, parçaları üçgen kenarıdır. Bir parçanın tam
   üstündeki (kesin `orient2d = 0`) köşe parçayı böler; kotu parçanın oradaki kotundan toleranstan çok ayrılırsa
   `height_conflict`. İki kırık hat yalnız ortak köşelerinde buluşur; içten kesişme `breaklines_cross` ile reddedilir,
   yeri söylenir (ROAD-TIN-04; kesişimi bölme seçeneği sonraki dilim).
5. **Sınır ve boşluk** *drape* kuralıyla: köşeleri, noktalar ve kırık hatlarla kurulan yüzeyin o yerdeki kotunu
   alır; yüzeyin dışındaki köşe `boundary_outside` ile ret. Halkalar kısıt olarak eklenir; sınırın dışındaki ve
   boşlukların içindeki üçgenler atılır. Bir halka kırık hatla içten kesişirse ret (bölme sonraki dilim).
6. **Kenar yöntemi.** `maxEdge(L)`: dış kenarı L'den uzun olan dış üçgen, yeni dış kenar kalmayana dek atılır;
   kısıt kenarı (kırık hat, sınır) aşılmaz, iç üçgene dokunulmaz.
7. **Yöntem.** Köşelerin kesin Delaunay'ı (`geom::delaunay`), sonra her kısıt parçası çevirmeyle geri kazanılır
   (kesen kenarlar kuyrukta, dışbükey dörtgen çevrilir, Sloan 1993), sonra kısıt olmayan kenarlar Lawson
   çevirmeleriyle yerel Delaunay'a döner. Bütün kararlar kesin öngörülerle; inşa edilen sayı yoktur (Y2.1'de yeni
   nokta üretilmez).
8. **Sorgu.** Bir yerin kotu: yürüyerek üçgen bulunur, kot düzlemin barisantrik değeridir; sonuç kot, üçgen ve
   durum (`inside`, `outside`, `void`). Dışarıda ve boşlukta kot yoktur; dışa uzatma yapılmaz (ROAD-TIN-11).
9. **Doğrulama.** Bağımsız başvuru: Shewchuk'un Triangle'ı (`triangle` Python paketi, `pc`/`p` anahtarları), genel
   konumda üçgen kümeleri bire bir; ayrıca KentOS kodu olmadan yazılmış Python denetleyicisi: her kısıt kenar mı,
   her kısıtsız kenar kesirlerle yerel Delaunay mı, alanların toplamı alana eşit mi, kotlar düzlemde mi.

## Sonra

Y2.2 yüzeyin kalıcı kaydı ve kaynakları (`.kcad`), Y2.3 komutlar (`road.terrains.*`) ve Python/MCP, Y2.4
masaüstünde çizim, Arazi sekmesi ve proje ağacı, Y2.5 eş yükselti eğrileri (ROAD-TIN-15), eğim ve LandXML yüzeyi.
