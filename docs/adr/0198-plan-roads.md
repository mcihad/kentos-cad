# ADR 0198: Plan yolu çizimi

- **Durum:** kabul edildi (2026-10-07). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-35`'in ardından `CAD-36`; madde **[M]**: geometrisi
  geneldir, genişlik ve yarıçap değerleri imar mevzuatındandır ve sahip tarif edecektir. Sahibin 6 Ekim gecesi sözüyle (“cad-36 bitene kadar
  devam”) geometri bu ADR'nin varsayılanlarıyla yapıldı; araçlar değerleri kullanıcıdan alır, mevzuata uygun olduklarını söylemez.
  Örnek Netcad PLANET'in Çizim ve Düzenleme İşlemleri'dir.
- **Bağlam belgesi:** TODOS.md `CAD-36`; ADR 0057 (Paralel çizgi), ADR 0065 (alan işlemleri), ADR 0140 (Tüm köşeleri yuvarla), ADR 0143
  (çok parçalı alan).

## Bağlam

İmar planında yollar eksenden iki yana genişlikleriyle çizilir; kavşaklarda yol kenarları birbirinin içinde kalmamalı, ada ve kaldırım
köşeleri yuvarlatılmalı, geniş yollarda refüj (orta ayırıcı) uçlarından kapatılmalıdır. Bugün Paralel çizgi kenarları, Köşe yuvarla tek
köşeyi yapar; kavşak temizliği, toplu yuvarlatma ve refüj kapama elle yapılıyor.

## Karar

### 1. Yolun nesnesi

Yeni nesne türü yoktur. Yol **alandır**: eksenin iki yanında genişliğinin yarısı kadar koridor (Paralel çizgi'nin “Alan olarak”ı, köşelerde
uzayan kenarlar, çok keskin köşede pah; `geom::parallel::corridor_area`). Yolun türü ve genişliği öznitelikleridir: `Tür` (`Yol`, `Yaya
yolu`, `Bisiklet yolu`, `Taşıt yolu`, `Refüj`, `Yol ekseni`) ve `Genişlik` (metre, sayı metni). Alan olduğu için kavşak temizliği alanların
birleşimi, ada ve kaldırım köşeleri birleşimin iç köşeleridir; CBS'de yol alanı doğrudan katmandır.

### 2. Plan yolu

- Eksen tıklanarak (kenet çalışır) ya da yazılarak çizilir; Enter ya da sağ tık bitirir. Seçenekler:
  - **Tür (T):** Yol, Yaya yolu, Bisiklet yolu;
  - **Genişlik (G):** yolun bütün genişliği; türü başına hatırlanır;
  - **Kaldırım (K):** yalnız Yol'da; iki yanda kaldırım genişliği; 0 kaldırımsız;
  - **Refüj (R):** yalnız Yol'da; ortada refüj genişliği; 0 refüjsüz;
  - **Eksen (E):** eksen de çoklu çizgi olarak yazılır.
  Başlangıç değerleri yalnız aracın ilk değerleridir (Yol 10 m, Yaya yolu 5 m, Bisiklet yolu 2,5 m; kaldırım ve refüj yok); mevzuatın
  yol sınıflarına göre değer tablosu sahibin tarifiyle eklenecektir.
- Yazılanlar etkin katmanda, tek adımda (`cad.entities.create`, adım “Plan yolu”):
  - yol alanı: genişlik G, `Tür` türün adı, `Genişlik` G;
  - kaldırım varsa taşıt yolu alanı: genişlik G − 2K, `Tür` “Taşıt yolu”; kaldırımlar iki alanın arasıdır;
  - refüj varsa refüj: genişlik R, iki ucu yarım daireyle kapalı alan (§4), `Tür` “Refüj”;
  - Eksen açıksa eksen: `Tür` “Yol ekseni”.
- 2K ≥ G ya da R ≥ G − 2K yazılmaz, nedeni söylenir.

### 3. Kavşak temizle

- Seçili alanlar `Tür`'lerine göre öbeklenir (özniteliksizler bir öbek; Refüj ve Yol ekseni dışarıda kalır). Her öbeğin alanları
  **birleştirilir** (`geom::region::union_areas`): birbirinin içine giren kenarlar kalkar, yolların kapattığı adalar delik olur. Sonra
  birleşimin **iç köşeleri** (alanın içinin 180°'den büyük açı yaptığı köşeler: kavşaktaki ada köşeleri ve kıvrımın iç yanı) yarıçapla
  yuvarlanır: Taşıt yolu öbeğinde **Kaldırım köşesi (K)**, ötekilerde **Ada köşesi (A)**; 0 yuvarlamaz.
- Yuvarlama kuralı Tüm köşeleri yuvarla'nınkidir (ADR 0140): iki düz kenar arasındaki köşe, kenarlarına yarıçapın teğet uzaklığı
  d = R·tan(dönüş/2) sığıyorsa yuvarlanır; iki köşe aralarındaki kenara birlikte sığmıyorsa ikisi de bırakılır; yay kenarlı köşe
  bırakılır. Bırakılanlar sayılır, söylenir. Bir halkada alanın içi kenarın solundaysa sağa dönüş, sağındaysa sola dönüş iç köşedir.
- Her öbeğin ilk alanı sonucu alır (birden çok parçaysa çok parçalı alan), ötekiler silinir; tek adım (`cad.entities.edit`, adım
  “Kavşak temizle”). Tek alanlı öbek yalnız yuvarlanır: toplu yol yuvarlatma budur.

### 4. Refüj kapat

- İki çizgi (çizgi ya da tek parçalı açık çoklu çizgi; refüjün kenarları) seçilir. İkincisi, başı birincinin sonuna sonundan yakınsa
  olduğu gibi, değilse ters çevrilerek eklenir. Halka: birincinin köşeleri, sonra ikincininkiler; köşelerine göre saat yönünün tersine
  çevrilir; uçları birleştiren iki kenar **yarım daire** (kavisi 1: dışa) ya da **Uç (U): Düz** ile düz.
- Birinci çizgi alana dönüşür (öznitelikleriyle), ikincisi silinir; tek adım (`cad.entities.edit`, adım “Refüj kapat”). Plan yolu'nun
  refüjü aynı kapamayla yazılır (eksenin iki yanında R/2'lik kenarlar).

### 5. Araçlar

- **Plan yolu** (`planRoad`), **Kavşak temizle** (`roadJunctions`), **Refüj kapat** (`medianClose`): Çizim'in yeni **Yol** bölümünde (CAD
  Giriş › Çizim ▾), CBS'de Harita › Yol (Parsel'in yanında; Düzenle sekmesi bir bölüm daha almıyor). Kavşak temizle önce seçer;
  seçimden sonra sonuç kesikli önizlenir; Enter yazar.
- Ada köşesi, Kaldırım köşesi, Tür, türlerin genişlikleri, Kaldırım, Refüj, Eksen ve Uç oturum boyu hatırlanır.

## Kapsam dışı

Mevzuatın değer tablosu (sahibin tarifi), yol eğrileri (eksende yay), şerit ve yön çizgileri, kavşakta trafik adası, yay kenarlı köşelerin
yuvarlanması, ada poligonlarının ayrı aracı (Tüm köşeleri yuvarla yapar).

## Uygulama

- **Sözleşme:** `CreateOperation::PlanRoad` (“Plan yolu”), `EditOperation::RoadJunctions` (“Kavşak temizle”) ve `MedianClose` (“Refüj
  kapat”); adları iki işleyicide, katalog, TypeScript ve Python tipleri yeniden üretildi.
- **Çekirdek:** `crates/shared/geometry-core/src/ops/road.rs`: `road_parts` (koridorlar `geom::parallel`'den), `median_ring`,
  `round_inner_corners`, `road_junctions` (`union_areas` ve iç köşeler); köşe yuvarlama Tüm köşeleri yuvarla'nın kuralıyla:
  `ops::reshape::corners_of_path_where` (bir süzgeçle) ve `turns_inward`. Web'e `roadParts`, `roundInnerCorners`, `roadJunctions`,
  `medianRing` (`apps/web/src/model/planRoad.ts`).
- **Masaüstü:** `crates/native/interaction/src/plan_road.rs` (`PlanRoad`, `RoadJunctions` değiştirme tabanında, `MedianClose`); bellekte
  `road_kind`, `road_widths`, `road_kerb`, `road_median`, `road_axis`, `junction_ada`, `junction_kerb`, `median_round`.
- **Web:** `apps/web/src/tools/planRoadTools.ts`; katalogda Çizim'in yeni `road` bölümü (“Yol”); CBS şeridinde Harita'nın Yol paneli.
- **İkonlar:** sahip uyurken seçenek sayfasından: Plan yolu dolgulu yol (ilk seçenek Paralel çizgi'ye benziyordu), Kavşak temizle dört
  yuvarlak köşe, Refüj kapat kapsül.
- **Bir kenar notu:** Kavşak temizle sonucu yazınca nesneler seçili kalır; araç yeniden açılınca seçim önce gelir (değiştirme
  araçlarının kuralı), iz bunu Esc ile bırakır.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/plan_road_cases.py` (ADR'den, KentOS kodu olmadan): dört yol (düz, kırık ve kaldırımlı, refüjlü
  bulvar, ters yönlü yaya yolu), sekiz iç köşe durumu (artı ve T kavşak, uzak koordinat, birbirine yer bırakmayan iki köşe, sığmayan
  yarıçap, çevre yolunun adası (delik), ters yönlü halka, eğik kavşak), altı refüj (aynı ve ters yönlü kenarlar, kırık ve yaylı kenar,
  düz uç); `fixtures/plan-road/v1/cases.json`. Çekirdek aynı dosyayı yerli (`tests/all/plan_road.rs`) ve WASM üzerinden
  (`apps/web/src/tools/planRoad.wasm.test.ts`) 1e−9 içinde, halkayı herhangi bir köşesinden karşılaştırarak geçer.
- Komut durumları: üç adımın adı, geri alma (`create_command_cases.py`, `edit_command_cases.py`) iki işleyicide.
- Ortak iz `fixtures/interaction/v1/plan-road.json` üç klavye düzeninde iki platformda geçer; resimler 1440×900 ve 1100×650'de, koyu ve açık
  temada iki platformda aynı (`kentos-cad kullan plan-road`, `pnpm -C apps/web e2e:use plan-road`).
