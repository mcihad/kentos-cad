# ADR 0194: Hizala ve dağıt

- **Durum:** kabul edildi (2026-10-07). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-31`'in ardından `CAD-32`; madde tek parçada biter ve
  sahibin 6 Ekim gecesi sözüyle CAD-36'ya dek ara verilmez. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler Netcad'in Hizala'sı,
  ArcGIS Pro'nun Align and Distribute'u ve ofis programlarının Hizala menüsüdür (Sola hizala, Ortala, Sağa hizala, Üste hizala, Ortaya
  hizala, Alta hizala, Yatay olarak dağıt, Dikey olarak dağıt).
- **Bağlam belgesi:** TODOS.md `CAD-32`. ADR 0037 ve 0047 (değiştirme araçları, `cad.entities.transform`, bugünkü Hizala), ADR 0144 (blok),
  ADR 0175 (bağlı yazı), ADR 0186 (ilişkili tarama).

## Bağlam

Bugünkü Hizala (AutoCAD'in ALIGN'ı) nesneleri iki nokta çiftiyle taşır, döndürür ve ölçekler: bir dönüşümdür, bütün seçim aynı yolu
gider. Semboller, yazılar, bloklar ve paftadaki çizimler için gereken başka bir iştir: her nesne kendi yerinden kayarak kenarı ya da
ortası bir başvuruya oturur, ya da nesneler eşit aralıkla dizilir.

## Karar

### 1. Araç

Hizala ve dağıt (`alignDistribute`; Değiştir sekmesinde Dönüştür panelinin ▾'inde, seyrek araçlar arasında: CBS'nin en dolu sekmesi
Düzenle geniş pencerede her panelini tam boy gösterebilsin diye; sekiz yöntemin her biri kendi ikonuyla katalogda): **Sola hizala**
(varsayılan), **Ortala** (O), **Sağa hizala** (A), **Üste hizala** (Ü), **Ortaya hizala** (R), **Alta hizala** (T), **Yatay dağıt** (Y),
**Dikey dağıt** (D). Araç açıkken harfler (Sola hizala için S) yöntemi değiştirir; istemde etkin olanın yanında “açık” yazar.

- Seçim önce ya da araç açıkken yapılır (değiştirme araçlarının ortak tabanı, ADR 0037); kilitli katmandaki nesne seçilmez.
- **Hizalama** (ilk altı): istem başvuruyu bekler. Bir nesneye tıklamak onun kutusunu başvuru yapar (kilitli katmanda da olabilir); boş
  yere tıklamak ya da nokta yazmak o noktayı (kenet ve nokta hesabı çalışır); **Enter** seçimin kutusunu (seçilenlerin birleşik kutusu).
  İmleç bir nesnenin üstündeyken, değilken imlecin noktasına göre önizleme: seçilenlerin varacakları yerde kesikli izleri ve başvurunun
  çizgisi (Sola hizala'da başvurunun batı kenarından geçen düşey çizgi).
- **Dağıtma** (son ikisi): istem Enter'ı bekler (tıklamak da uygular), önizleme varılacak yerleri gösterir. Üçten az nesneyle söylenir,
  araç bekler.
- Yazılır: tek adım, adı yöntemin adı (“Sola hizala” … “Dikey dağıt”); araç biter, seçim kalır (değiştirme araçları gibi). Hiçbir nesne
  yer değiştirmeyecekse yazılmaz, söylenir (“Seçilenler zaten hizalı.”).

### 2. Kutular ve kurallar

- Bir nesnenin **kutusu** çizimin onu Tümünü göster'de ölçtüğü kutudur (geometri deposunun kutusu): yazı projenin çizim yazı tipiyle,
  blok yerleştirmesi bloğunun parçaları ve yerleştirme noktasıyla, yardımcı çizgi ve ışın yalnız taban noktasıyla. Eksenler doğu ve
  kuzeydir; sol batı, üst kuzeydir (iki proje türünde de ekranın solu ve üstü).
- **Hizalama:** her nesne yalnız bir eksende kayar: Sola hizala kutusunun batı kenarını, Ortala ortasını, Sağa hizala doğu kenarını
  başvurunun aynı doğusuna (`at`); Üste hizala kuzey kenarını, Ortaya hizala ortasını, Alta hizala güney kenarını aynı kuzeyine getirir.
  Başvuru bir kutuysa `at` onun aynı kenarı ya da ortasıdır; bir noktaysa onun doğusu ya da kuzeyi.
- **Dağıtma:** kutular eksen boyunca ortalarına göre sıralanır (eşitlikte seçimin sırası); ilki ve sonuncusu yerinde kalır; aradakiler
  sırayla, ardışık kutuların arasındaki boşluk eşit olacak biçimde kayar: boşluk = (sonuncunun uzak kenarı − ilkinin yakın kenarı −
  kutuların boylarının toplamı) / (n − 1); kutular örtüşüyorsa boşluk eksi olabilir, kural aynıdır.
- Bağlı yazı ve ilişkili tarama, nesneleri taşındığında olduğu gibi izler (ADR 0175, 0186).

### 3. Komut

`cad.entities.transform`'un yeni türü `arrange`: `{ kind: "arrange", mode, at }`, `mode` `left`, `center`, `right`, `top`, `middle`,
`bottom`, `horizontal`, `vertical`; `at` ilk altısında sonlu bir sayı (doğu ya da kuzey), son ikisinde verilmez. Her nesne kendi
kaymasıyla yazılır (taşımanın kuralıyla); kilitli katmandakiler dönüşümün kuralıyla atlanır ve söylenir. Denetim sırası dönüşümünkü,
sayılardan sonra: `at` eksik ya da fazlaysa `invalid_transform`; dağıtmada üçten az kilitsiz nesne `too_few_objects`. `copy` ile
kopyalar yerleşir. Python ve MCP aynı komutla hizalar (`at` ile istenen doğuya ya da kuzeye).

### 4. Ortak kurallar ve başvuru

Kaymalar ve kutular çekirdekte: `ops::arrange` (`displacements`, başvurunun `at`'ı, seçimin kutusu) ve nesne kutusu (`object_bounds`:
deponun kutusunun kuralı, blok parçalarıyla). Web'e `arrangeMoves` işlemiyle. Bağımsız Python başvurusu kutulardan kaymaları kesirlerle
hesaplar (`scripts/fixtures/arrange_cases.py`, `fixtures/arrange/v1/cases.json`); komut durumları dönüşümün durumlarına eklenir (çizgi,
alan, daire, nokta ve basit bloklu yerleştirme kutularıyla).

## Uygulama

- **Çekirdek:** `kentos_geometry_core::ops::arrange` (`Mode`, `object_bounds`, `at_of`, `union`, `moves`; web'e `arrangeBoxes`,
  `arrangeAt`, `arrangeUnion`, `arrangeMoves`). Bloklu yerleştirmenin kutusu deponunkiyle aynı kuraldır: `block::pieces_bounds`'u
  depo da kullanır.
- **Sözleşme:** `Transform::Arrange { mode: ArrangeMode, at }` (`cad_transform.rs`, TS `Transform.ts`, `ArrangeMode.ts`, katalog ve
  Python SDK'sı yeniden üretildi); hata kodu `too_few_objects`.
- **Komut:** masaüstünde `kentos_native_application::transform` (iki geçiş: önce kilitsiz nesnelerin kutuları, sonra her birinin kendi
  kayması, taşımanın matrisiyle), web'de `product/entitiesTransform.ts` (`arranged`).
- **Araç:** masaüstünde `kentos_interaction::align_distribute` (değiştirme araçlarının tabanı, `stage_preview` ile nesne başına izler),
  web'de `tools/alignDistributeTool.ts`; komut `tool.alignDistribute` (takma adlar `HIZALAVEDAGIT`, `DAGIT` …), seyrek araç: CAD'de
  Değiştir › Dönüştür ▾, CBS'de Düzenle'de aynı panelin ▾'i; yöntemler araç açıkken komut satırının çiplerinde (harfleriyle). Aracı
  panelde göstermek CBS'nin Düzenle sekmesini tam boyda 3053 px yaptı (masaüstünün geniş pencere denetimi 3000 px'te her paneli tam
  ister). İkonlar `arrangeLeft` … `arrangeVertical`, seçenek sayfasının A'ları (çerçeveli çubuklar ve başvuru çizgisi; sahip yokken
  önerilen).

## Doğrulama

- `scripts/fixtures/arrange_cases.py --check` (KentOS kodu olmadan, ADR'nin işlem sırasıyla çift duyarlıkta ve kesirlerle 10 nm içinde
  denetlenerek): 23 durum (altı hizalama seçimin ve bir kutunun kenarına, noktaya; beş, üç, örtüşen, eşit ortalı ve iki kutulu
  dağıtma), 18 başvuru, bir birleşim. Çekirdek `tests/all/arrange.rs` ve web `tools/arrange.wasm.test.ts` bit bit tutar; bloklu
  yerleştirmenin kutusu deponunkine eşittir.
- `cad.entities.transform` durumlarına dört durum (`transform_command_cases.py`): Sola hizala planı ve yazması kilitli nesneyle, Ortaya
  hizala ve Yatay dağıt, Sağa hizala kopyası, retler (sonlu olmayan `at`, eksik ve fazla `at`, üçten az nesne, hepsi kilitli).
- Ortak iz `align-distribute.json` (`align-distribute.kcad`) iki platformda üç klavyede; resimler `kentos-cad kullan align-distribute`,
  web'in `e2e:use`'u ve ikon turunun `hizala-dagit` sahnesi.

## Kapsam dışı

Aynı boya getirme (Make same size), sayfa ya da kenar boşluğuna hizalama (pafta tasarımcısının işi), verilen aralıkla dağıtma (Dizi'nin
işi), döndürerek hizalama (bugünkü Hizala).
