# ADR 0163: Kenet ekleri

- **Durum:** kabul edildi (2026-10-02). `HYB-09`. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** TODOS.md `HYB-09` (ilgili: `SET-06`), ADR 0029 (seçim ve kenet), ADR 0059 (tek seferlik kenet), ADR 0080 (ızgara ve çizim yardımcıları), ADR 0085 (nesne izleme), ADR 0023 (tipli ayarlar), ADR 0025 (`.kcad` v2); [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); Netcad Nokta Yakalama Araçları (Karelaj, Eksene Dik), AutoCAD OSNAP (Extension, Parallel, Geometric Center) ve SNAP, ArcGIS Pro snapping (layer snapping, snap to sketch), QGIS snapping (per-layer, scale-dependent, self-snapping, custom grid).

## Bağlam

KentOS'ta dokuz kenet türü var: uç, orta, merkez, nokta, çeyrek, kesişim, dik, teğet, en yakın (ortak deponun `store::snap`'i, iki platformda aynı). Türler Uygulama ayarları › Kenetleme'den açılıp kapanır, F3 kenedin tamamını açar ve kapatır, sağ tıkta tek seferlik kenet vardır (ADR 0059), nesne izleme kenette durarak nokta alır (ADR 0085).

Öbür programlarda olup KentOS'ta olmayanlar:

- **Uzantı** (AutoCAD Extension): bir çizginin ya da yayın ucunda durulunca devamı boyunca nokta.
- **Paralel** (AutoCAD Parallel): bir kenarda durulunca, son noktadan o kenara paralel doğrultuda nokta.
- **Ağırlık merkezi** (AutoCAD Geometric Center, QGIS centroid): kapalı alanın ağırlık merkezi.
- **Karelaj** (Netcad Karelaj, AutoCAD SNAP, QGIS custom grid): imleç verilen yatay ve düşey aralıktaki ızgaranın düğümlerinde gezer.
- **Katman başına kenet** (ArcGIS layer snapping, QGIS advanced configuration): bir katmanın nesnelerine kenetlenmemek ya da yalnız bazı türlerle kenetlenmek.
- **Ölçek aralığında kenet** (QGIS scale-dependent snapping): uzak ölçekte kenetin kendiliğinden kapanması.
- **Çizilmekte olan nesneye kenet** (QGIS self-snapping, ArcGIS snap to sketch): çizilen yolun önceki köşelerine ve kenarlarına kenet.

Netcad'in Eksene Dik'i (yakalanan noktanın yalnız yatay ya da düşey bileşeni) KentOS'ta Orto ile nesne izlemenin birlikteliğidir; ayrı bir tür değildir.

## Karar

### 1. Dört yeni tür

Türler bit sırasıyla eklenir (`SnapKind::ALL` dokuzdan on üçe). Var olanların bitleri değişmez.

| Tür | Ad | Ağırlık | İşaret |
|---|---|---|---|
| `centroid` | Ağırlık merkezi | 1,08 (merkezinki) | içinde nokta olan küçük karo |
| `extension` | Uzantı | 1,3 | kesikli çizgi üstünde artı |
| `parallel` | Paralel | 1,3 | iki eğik paralel çizgi |
| `grid` | Karelaj | yalnız başka tür yokken | küçük artı |

- **Ağırlık merkezi:** kapalı alanın (deliklerden ve parçalardan arınmış, yaylarıyla) ve kapalı çoklu çizginin alan ağırlık merkezi. Kutusu imlecin kenet kutusuna değen alanlar adaydır; merkez kenet yarıçapındaysa kenetlenir. Daire ve elipsin merkezi Merkez türündedir.
- **Uzantı:** alınmış bir uç noktanın (§2) kenarının devamı: düz kenarda uçtan dışarı giden ışın, yayda yayın çemberi (uçtan dışarı, yayın yönünde). İmleç bir uzantının kenet yarıçapındaysa uzantıdaki en yakın noktaya; iki uzantı kesişiyorsa kesişimine (kesişimin ağırlığıyla) kenetlenir.
- **Paralel:** komut bir son nokta veriyorsa (dik ve teğetin kaynağı), son noktadan alınmış bir kenara (§2) paralel doğru. İmleç bu doğrunun kenet yarıçapındaysa doğru üstündeki izdüşümüne kenetlenir.
- **Karelaj:** düğümler projenin koordinatlarında `snap.gridX` ve `snap.gridY` metrenin katlarıdır (yuvarlak koordinatlar; yerel sıfır yoktur). Başka bir tür kenet yarıçapında değilse (en yakın dahil) imleç en yakın düğüme gider; nerede olursa olsun, Netcad'in Karelaj'ı ve AutoCAD'in SNAP'i gibi. Varsayılan aralıklar 1 m.

Varsayılanlar: yeni dört tür kapalıdır. Bugünkü çizim, siz açmadıkça değişmez.

### 2. Uzantı ve Paralel'in alınması

Nesne izlemenin (ADR 0085) beklemesi kullanılır. Kenette ya da kenarda 350 ms durmak:

- **Uç noktada** (uç kenedi): nokta izleme için alınır. Uzantı açıksa ucun kenarı da (doğrultusu ya da çemberi) alınır.
- **Kenarda** (bir düz kenarın iç kısmı, uç ya da orta değil): Paralel açıksa kenarın doğrultusu alınır.

Alınanlar izleme noktalarıyla bir listededir: en çok 3, en eskisi önce gider; aynısında yeniden durmak bırakır. Komut değişince gider. İzleme kapalıyken de Uzantı ve Paralel kendi alınanlarıyla çalışır; izlemenin hizaları (yatay, düşey, kutupsal) yalnız İzleme açıkken çizilir.

İşaretler, nesne izlemeninkiler gibi kenet renginde:

- alınmış ucun uzantısı kesikli çizgi;
- alınmış kenarın üstünde küçük paralel işareti;
- kilitlenen paralel doğru kesikli;
- imlecin yanında “Uzantı 12.063 m” ya da “Paralel”.

### 3. Çizilmekte olan nesneye kenet

Çalışan aracın şimdiye kadarki noktaları ve parçaları (yol araçları, Çizgi zinciri, Eğri, Revizyon bulutu…) açık türlerle kenet adayıdır: uç (köşeler), orta, kesişim (öbür nesnelerle ve kendi parçalarıyla), dik, teğet, en yakın. İmlece giden parça aday değildir.

- **Ayar:** `snap.self` (açık). Kapatılınca yalnız çizimdeki nesneler adaydır.
- Araç taslağını oturuma bir şekil olarak verir (`Tool::draft_path` / `draftPath`).

### 4. Katman başına kenet

Katman düğümü (`LayerNode`) yeni bir alan alır: `snap`.

- **Yoksa:** katman genel türlerle kenetlenir (bugünkü davranış).
- **`{ "off": true }`:** katmanın nesnelerine kenetlenilmez; seçme ve kenet dışı her şey sürer.
- **`{ "kinds": [...] }`:** katmanın nesnelerine yalnız bu türlerle kenetlenilir; genel türlerle kesişimi kullanılır (genelde kapalı bir tür katmanda da kapalıdır).

Kesişim kenedi iki nesneden birinin katmanında kesişim türü açıksa çalışır.

Proje verisidir: `.kcad` şema 10, katman düğümünde isteğe bağlı `snap`. Eski okuyucu alanı bilmez; şema 10 dosyası şemayı bilmeyen okuyucuda açılmaz (ADR 0025'in kuralı). Bulutta katman ağacıyla gider.

Arayüz, iki platformda:

- **Katmanlar panelinde** gözün ve kilidin yanında mıknatıs: tık aç ve kapat; kapalıyken soluk, türleri seçilmişse noktalı.
- **Sağ tık › Kenet ▸:** Açık, Kapalı ve türlerin işaretleri. Türlerden biri seçilince katmanın kendi listesi olur; Genel türler listeyi kaldırır.
- **Katman ve grup:** grubun düğmesi içindekilerin hepsine yazar.

### 5. Ölçek aralığında kenet

`snap.scaleMin` ve `snap.scaleMax`: görünüm ölçeğinin paydası (1:N), 0 sınırsız. Görünüm aralığın dışındayken kenet kapalı sayılır: işaret çıkmaz, tek seferlik kenet de çalışmaz. Durum çubuğundaki Kenet hücresi o sırada soluk, ipucunda “ölçek aralığının dışında (1:N)”.

Varsayılan 0–0, yani bugünkü gibi her ölçekte. Katman başına ölçek aralığı kapsam dışıdır (§8).

### 6. Kenet hücresinin menüsü

Durum çubuğundaki Kenet hücresinin sağ tık menüsü, iki platformda:

- on üç tür, işaretleriyle;
- Karelaj aralığı…;
- Çizilmekte olan nesneye;
- Kenet ayarları… (Uygulama ayarları › Kenetleme).

Her türün komutu vardır (`draft.snap.<tür>`, komut satırında ve Araçlar › Çizim yardımcıları'nda). Tek seferlik kenet menüsü (ADR 0059) yeni dört türü de gösterir.

### 7. Ortak çekirdek

`store::snap` genişler: `snap(p, tol, kinds, from, extras)`. `extras` şunları taşır:

- alınmış uzantılar (uç ve doğrultu ya da çember);
- alınmış paralel doğrultular;
- çizilmekte olan yol;
- karelaj aralıkları;
- katman maskeleri (katman başına tür kümesi).

Ağırlık merkezi `geom::region`'ın alanıyla aynı yoldan hesaplanır: yaylar daire diliminin ağırlık merkeziyle.

Bağımsız başvuru `scripts/fixtures/snap_cases.py`, KentOS kodu olmadan:

- ağırlık merkezi: düz kenarlarda kesin kesirlerle, yaylarda 50 basamaklı `mpmath` ile; delikli ve parçalı alanlar;
- uzantı ve paralel: kesin kesirler;
- karelaj düğümleri;
- türlerin önceliği;
- katman maskeleri;
- çizilmekte olan yol.

Web WASM'dan (`PickIndex.snap`), masaüstü yerli çağırır. İki platform aynı sonucu verir.

### 8. Kapsam dışı

- **Katman başına ölçek aralığı:** genel aralık yeter; gerekirse ayrı karar.
- **Görünür ızgaranın karelaja uyması:** F7'nin ızgarası yakınlığa göre değişir; karelaj aralığını çizmek ayrı iştir.
- **Eksene Dik:** Orto ve nesne izleme bunu yapar (§Bağlam).
- **Görünür kesişim** (AutoCAD Apparent Intersection) ve **ekleme noktası** (Insertion): ayrı karar.

### 9. İş sırası

1. Çekirdek: dört tür, çizilmekte olan yol, katman maskeleri, `extras`; WASM; bağımsız başvuru ve ortak durumlar.
2. Ayarlar ve Kenet hücresinin menüsü iki platformda: türlerin komutları, karelaj aralığı, `snap.self`, ölçek aralığı; araçların taslağı; işaretler; ortak iz; resimler.
3. Uzantı ve Paralel'in alınması iki platformda (bekleme, işaretler, kilit); ortak iz; resimler.
4. Katman başına kenet: `.kcad` şema 10 (spesifikasyon, bağımsız Python yazıcısı ve okuyucusu, kodek), Katmanlar panelinin mıknatısı ve menüsü iki platformda, bulutta katman ağacı; ortak iz; resimler.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- KentOS'un kenedi AutoCAD'in, Netcad'in, ArcGIS'in ve QGIS'in kenet eklerini karşılar.
- Varsayılanlar bugünkü davranıştır; yeni türler açılınca devreye girer.
- Katman başına kenet proje verisidir; `.kcad` şema 10 ister.

## Doğrulama

- **Çekirdek:** bağımsız başvuruya göre iki platformda.
- **Arayüz:** ortak izlerle iki platformda; resimler iki temada, 1440×900 ve 1100×650.
