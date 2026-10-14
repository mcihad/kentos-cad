# ADR 0279: Tutamaç menüleri ve köşe komutu

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-15`: “Tutamaç menüleri nesneye göre
  move/stretch/rotate/scale/mirror, vertex add/remove ve ilgili semantik işlemleri sunsun.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0068 ve 0074 (tutamaçlar ve tutamaç menüsü), 0047 (Köşe ekle/sil), 0160 (topolojik düzenleme), 0249 ve 0263
  (alt nesne yerleri), 0254 (Esnet'in komutu), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Tutamaç menüsü yalnız seçili yolun köşe ve kenarında açılıyordu ve dört işi (Köşeyi sil, Ortasına köşe ekle, Düz kenar yap, Yaya dönüştür)
geometriyi kendisi hesaplayıp `cad.entities.edit`'e hazır geometri veriyordu. Köşe ekle/sil aracı da öyle. Python ve ajan aynı işi yapmak
için geometriyi kendileri hesaplamak zorundaydı. Çizginin ucunda, dairenin merkezinde ya da yazının noktasında menü yoktu; taşıma,
döndürme, ölçekleme ve aynalama için tutamaç taban noktası olarak kullanılamıyordu.

## Karar

1. **`cad.entities.vertex` v1:** nesne kalıcı kimliğiyle, yer alt nesne yeriyle (ADR 0249): `remove` bir köşe yeri (`vertex`), `insert`
   bir kenarın ortası (`mid`) ya da kenarın üzerinde bir yer (`along`, ADR 0263 §1), `straighten` ve `arc` kenarın ortası. Geometri
   komutundur: köşe silinince iki kenar tek düz kenar olur, açık yolun ucu kenarıyla gider; yay kenara eklenen köşe yayı çemberinde ikiye
   böler; çizgiye köşe eklenince iki kenarlı çoklu çizgi olur; Yaya dönüştür kenarı kirişin dörtte biri sehimle (büküm 0,5) halkanın
   dışına (saat yönünün tersindeki halkada +0,5, saat yönündekinde −0,5) ya da açık yolun sağına büker. Kenar zaten istendiği gibiyse
   hiçbir şey yazılmaz. Deliklerin köşe ve kenarları bu komutla değişmez (`hole_edit`); alanın delikleri ve çok parçalı alanın öbür
   parçaları kalır. `topology` verilirse ortak köşe ve kenarları olan görünen komşular aynı adımda izler (ADR 0160): eklenen köşeyi
   aynı düz kenar (iki yönde de) alır, yeni bükümü aynı kenar alır (ters yönde eksi işaretle), silinen köşe aynı iki köşe arasında
   olan komşudan da gider; kilitli komşu `neighbour_locked`, geçersiz kalacak komşu `neighbour_invalid` uyarısıyla sayılır.
   Retler sırasıyla: `invalid_uid`, `invalid_revision`, `revision_conflict`, `entity_not_found`, `layer_locked`, `not_a_path`,
   `wrong_place`, `hole_edit`, `no_such_place`, `too_few_corners`, `too_few_points`. Adım adı işlemin: Köşe sil, Köşe ekle, Düz
   kenar yap, Yaya dönüştür. Planı yazılacak nesneleri verir.
2. **Tutamaç menüsü her tutamakta:** yolun köşesinde Köşe N (Köşeyi sil), kenarın ortasında Kenar N (Ortasına köşe ekle; Düz kenar yap
   ya da Yaya dönüştür), öbür tutamaklarda (deliğin köşesi, çizginin ucu, dairenin merkezi, yazının noktası …) “Tutamaç”. Ardından her
   tutamakta Esnet, Taşı, Döndür, Ölçekle ve Aynala: Esnet tutamacı sıcak yapar (sonraki tık ya da yazılan nokta onun yeni yeri); öbürleri
   aracını başlatır ve tutamacın noktasını ilk noktası (taban, döndürme ve ölçekleme merkezi, aynanın ilk noktası) olarak verir; araç
   seçimin bütününe uygulanır.
3. **Araçlar komutla yazar:** tutamaç menüsünün köşe ve kenar işleri ve Köşe ekle/sil aracı `cad.entities.vertex` ile; araç tıklanan
   köşeyi ya da kenarın üzerindeki yeri (çekirdeğin `place_at`'i) verir. Kayıtta komutun adı görünür; manifestte Köşe ekle/sil
   `semantic`.

## Sonuçlar

- Python (`kentos.cad.entities.vertex`) ve MCP (yıkıcı listede) aynı komutu kullanır; geometriyi kendileri hesaplamaz.
- Esnet ve dönüşümler yeni komut istemez: var olan `cad.entities.transform` ve tutamacın yazması (`cad.entities.edit`'in `grip`'i).

## Doğrulama

- `python3 scripts/fixtures/vertex_command_cases.py --check` (46 durum, KentOS kodu olmadan; yarım çemberin bölünmesindeki tan(π/8)
  60 basamakla); `cargo test -p kentos-native-application --test all` (ortak durumlar ve yetenekler).
- `cargo test -p kentos-interaction` (tutamaç menüsü ve topolojik düzenleme), `cargo test -p kentos-desktop drawing_menus` (tutamacın
  dönüşümleri ve Esnet'i), bütün izler değişmeden geçer.
- Resim: `KENTOS_SHOTS_ONLY=tutamac-menusu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
