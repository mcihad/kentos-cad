# ADR 0298: Uzat-kısalt'ın anlamsal komutu, elips yayı ve yayda açı

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-12`: “Lengthen; delta, yüzde, total ve
  dynamic yöntemlerini analitik path üzerinde desteklesin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0047 (Uzat-kısalt aracı), 0142 (köşe kotu), 0224 (ölçü, M), 0258 (çoklu çizginin genişliği), 0174 (çok
  parçalı nesne), 0250 ve 0254 (araç = komut), 0253 (komut protokolü).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Uzat-kısalt aracı (ADR 0047) yeni şekli kendisi hesaplayıp `cad.entities.edit`'in `lengthen` işlemiyle hazır geometri yazıyordu
(manifestte `geometry` düzeyi). Python ve ajan aynı işi ancak geometriyi kendileri hesaplayarak yapabiliyordu. Ayrıca:

1. **Elips yayı** uzatılamıyordu (`lengthOf` boş dönüyordu); AutoCAD'in LENGTHEN'i elips yayını kendi elipsinde uzatır.
2. **Yayda açı yoktu:** AutoCAD'in DElta ve Total seçenekleri yayda içerdiği açıyı da alır (`Angle`).
3. **Kot ve ölçü (M):** uzayan ucun kotu ve ölçüsü yazılmıyordu; düzenlemenin taşıma kuralı onu eski değerinde bırakıyordu.

AutoCAD'le karşılaştırma:

| AutoCAD LENGTHEN | Önce | Şimdi |
|---|---|---|
| DElta: uzunluk farkı (eksi kısaltır) | var | aynı |
| DElta › Angle: yayın açısı | yok | `delta.angle` |
| Percent | var | aynı |
| Total: toplam uzunluk | var | aynı |
| Total › Angle: yayın toplam açısı | yok | `total.angle` |
| DYnamic: uç imleci izler | var | aynı, komutun planıyla |
| Ölçmek için seçilen nesnenin uzunluğu | önizleme etiketi | aynı (“12,142 m → 15,000 m”) |
| Çizgi, yay, açık çoklu çizgi | var | aynı |
| Elips yayı | yok | kendi elipsinde |
| Açık spline | yok | “Kalanlar” |
| Kapalı nesne, daire, bütün elips | uyarı | `not_lengthenable` |

## Karar

### 1. Komut

`cad.entities.lengthen` v1 (`crates/shared/contracts/src/cad_lengthen.rs`): girdisi nesnenin kalıcı kimliği (`uid`), taşınan uç
(`end`: `start` ya da `end`) ve değişiklik (`change`, `kind` ile):

- `delta`: `length` (metre, eksi kısaltır) ya da `angle` (radyan, yalnız yay); ikisinden yalnız biri;
- `percent`: yeni uzunluk şimdikinin yüzdesi;
- `total`: yeni `length` ya da yayın yeni `angle`'ı; ikisinden yalnız biri;
- `dynamic`: `to` noktası; ucun ötesinde uç kenarın doğrusunda, çemberinde ya da elipsinde izdüşümü, içeride yolun en yakın noktası.

Çıktısı ve planı `cad.entities.edit`'inkidir (`EntitiesEdited`, `EntitiesEditPlan`); geri alma adımı “Uzat-kısalt”. Yazma
`cad.entities.edit`'in `lengthen` işlemidir, yalnız komutun iç yoludur.

### 2. Yeni şekil (çekirdek)

`ops::lengthen::lengthen(entity, at_end, Change) -> Result<Lengthened, Why>` (masaüstü, başsız sunucu ve web'in WASM'ı tek kaynaktan):

- yeni uzunluk: Fark eski + fark; Yüzde eski × yüzde / 100; Toplam verilen; yayda açı r × yeni açı; Dinamik yukarıdaki kural;
- uzatma uç kenarı sürdürür: çizgi doğrusunda, yay ve çoklu çizginin yay kenarı çemberinde (büküm yeni açının `tan(θ/4)`'ü), elips yayı
  elipsinde (uzunluğu yeni uzunluk olan parametre, 80 adımlık ikiye bölmeyle);
- kısaltma yolu sabit uçtan yeni uzunlukta keser;
- yeni uzunluk 1e-9'dan büyük olmalı; yay ve elips yayı bir tam turdan kısa kalmalı.

### 3. Taşınanlar

- **Uzatma** yalnız taşınan ucu değiştirir: çoklu çizginin kenarları genişliklerini korur; taşınan ucun kotu ve ölçüsü uç kenar boyunca
  uzunlukla sürer (iki ucunun değerinden doğrusal). Komut bunları geometrisiyle yazar.
- **Kısaltma** yolu keser: kalan köşeler değerlerini korur, yeni uç kesilen kenarın kotunu, ölçüsünü ve genişliğini uzunlukla alır
  (düzenlemenin `Carry::Along` kuralı).
- İlişkili ölçünün bu nesnedeki noktası yerini izler (ADR 0249); ayrıca aranmaz.

### 4. Retler

Sırasıyla: `invalid_uid`, `not_finite` (değişikliğin sayıları), `invalid_change` (Fark ya da Toplam'da uzunluk ve açıdan ikisi de
ya da hiçbiri), `invalid_revision`, `revision_conflict`, `entity_not_found`, `layer_locked`, `not_lengthenable` (kapalı çoklu çizgi,
alan, daire, bütün elips, nokta, eğri; ayrıca uzunluğu olmayan nesne: “Nesnenin uzunluğu yok.”), `multi_part`, `angle_needs_arc`,
`invalid_length`, `arc_closes`. Uyarısı yoktur.

### 5. Araç

Uzat-kısalt aracı komutun önizlemesini `entities_lengthen::plan_in` ile, yazmasını `execute` ile yapar; yalnız hangi ucun taşınacağını
(çekirdeğin `near_end`'i) ve değişikliği sorar. Fark ve Toplam'da `<45` yazmak yayın açısını verir (projenin açı biriminde); düz sayı
açıyı bırakır. Açı beklenirken oturum `<`'yi doğrultu kilidine çevirmez (`Tool::takes_angle`). Değerler uygulama açık kaldıkça
hatırlanır (`Memory::lengthen_delta_angle`, `lengthen_total_angle`).

## Uygulama

- **Çekirdek:** `crates/shared/geometry-core/src/ops/lengthen.rs` (`lengthen`, `Change`, `Why`, `Lengthened`; elips yayı:
  `ellipse_run`, `ellipse_sweep_for`).
- **Sözleşme:** `cad_lengthen.rs`; `codes.rs`'te altı yeni kod; katalog ve yetenekler.
- **Komut:** `crates/native/application/src/lengthen.rs` (`check`, `geometry_of`, `lengths`); `DESKTOP_COMMANDS`, `registry.rs`.
- **Araç:** `crates/native/interaction/src/lengthen.rs`; `Tool::takes_angle` (`tool.rs`, `session.rs`).
- **Python ve MCP:** `kentos.cad.entities.lengthen` yeniden üretildi; MCP aracı katalogdan, yıkıcı listede.
- **Manifest:** `tool.lengthen` `semantic`, `via` `cad.entities.lengthen`.
- **Web'in kayıtlı çağrıları:** `fixtures/geometry/v1/calls-p6-path-editing.json`: beş elips yayının `lengthOf`'u (mpmath ile 40
  basamakta bağımsız hesaplandı, göreli fark en çok 1,4e-15) ve beş ret iletisinin yeni metni (web dururken koşmaz).
- **Resimler:** `apps/desktop/src/lengthen_scenes.rs` (`uzat-kisalt-toplam`, `-yay-aci`, `-elips`, `-yazildi`).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/lengthen_command_cases.py` KentOS kodu olmadan yazıldı (açılar ve elips yayının uzunlukları
  mpmath ile 40 basamakta). Ortak durumlar `fixtures/commands/v1/cad.entities.lengthen.json`'da (38 durum): çizginin iki ucunda Fark,
  kotun ve ölçünün sürmesi, yayda uzunluk ve açı, yarım daireye Toplam açı, tam tur, çizgide açı, Yüzde (kısaltma ve sıfır), çoklu
  çizginin Toplam'la uzaması ve kesilmesi (kot, ölçü, genişlik), yay kenarı, elips yayının iki ucu, Dinamik (çizgi, yay, içeride),
  3-4-5 çizgisi, bütün retler, sürüm çatışması. Planlar 1e-9 içinde, yazmalar bire bir karşılaştırılır. Masaüstü (`fixtures.rs`) ve
  Python aynı dosyayı çalıştırır; yetenek testi her kodu bir durumda görür. Testin dosyayı okuduğu bir değer bozularak denendi.
- **Araç:** `crates/native/interaction/tests/all/edits.rs`: eski iz değişmeden; yayda Toplam `<200` (grad), çizgide açının reddi,
  düz sayının açıyı bırakması.
- **Komut testi:** `crates/native/application/tests/all/lengthen.rs`: Dinamik'le uzayan çizginin kotu, tek geri alma.
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
- **Süreler:** release, 100 000 çizgilik çizim; `lengthen::timing`:

  | İş | Süre |
  |---|---|
  | Önizleme: çizgi (Dinamik) | 0,003 ms |
  | Önizleme: 10 000 köşeli çoklu çizginin kısaltılması | 0,30 ms |
  | Önizleme: elips yayı (Fark) | 0,37 ms |
  | Yazma (çizgi, Fark) | 0,005 ms |

## Kalanlar

- **Eğri (Catmull–Rom) ve NURBS** uzatılmaz (`not_lengthenable`); AutoCAD açık spline'ı uzatır. NURBS'ün uç türevinden sürdürme
  (`CAD-EDIT-17`'nin düzenlemeleriyle) ayrı iştir.
- Çok parçalı çoklu çizginin hangi parçasının ucunun taşınacağı sorulmaz (`multi_part`).
- Web'in aracı ve komutun web işleyicisi: TODOS.md §0.2.
