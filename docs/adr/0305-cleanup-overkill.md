# ADR 0305: Çizimi temizle'de tolerans, eş doğrultulu çizgiler ve rapor

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-19`: “Overkill/cleanup; yinelenen, üst üste,
  sıfır uzunluk ve birleşebilir geometriyi öneri/plan olarak sunsun; toleransla düzeltme kaybı raporlansın.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0140 (Çizimi temizle), CLAUDE.md §9.2 (araç = komut; dokunulan araç kurala getirilir), §23.3 (gizli tolerans
  yok).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Çizimi temizle (ADR 0140) yalnız tam yinelenenleri (1e-9 m), uzunluğu olmayan nesneleri ve ardışık tekrarlanan köşeleri buluyordu.
Tolerans, üst üste binen eş doğrultulu çizgiler, doğrudaki köşeler ve kayıp raporu yoktu. Hesap araçtaydı (`cleanup_findings`), yazma
`cad.entities.edit` ile oluyordu; manifestte düzeyi `geometry`'ydi.

AutoCAD'in OVERKILL'inin seçenekleri:

- sayısal tolerans (varsayılan 0,000001);
- yok sayılacak nesne özellikleri;
- çoklu çizgilerin içindeki parçaları iyileştir;
- kısmen örtüşen eş doğrultulu nesneleri birleştir;
- uç uca hizalı eş doğrultulu nesneleri birleştir;
- ilişkili nesneleri koru.

## Karar

### 1. Komut

`cad.entities.cleanup` v1. Girdi:

- `uids`: kapsam; verilmezse görünen ve kilitsiz katmanlardaki bütün nesneler;
- `tolerance`: metre, 0 ile 10; verilmezse `CLEANUP_TOLERANCE` = 0,000001;
- `ignore`: yinelenenlerin farklı olabileceği özellikler (katman, renk, çizgi tipi, çizgi tipi ölçeği, kalınlık, saydamlık, sembol);
- `overlaps` (varsayılan açık), `endToEnd` (varsayılan kapalı), `straightVertices` (varsayılan açık);
- `expectedRevision`.

Çıktı `EntitiesCleaned`'dir: değişenler, silinenler (kapsamın sırasıyla), rapor ve sürüm. Plan değişecek nesneleri yazılacakları gibi,
silinecek kimlikleri ve raporu verir. Tek geri alma adımının adı “Çizimi temizle”dir. Bulunacak bir şey yoksa adım yazılmaz.

### 2. Bulunanlar

Uzaklık `sqrt(dx·dx + dy·dy)`'dir. Sıra: boşlar, yinelenenler, eş doğrultulu çizgiler, köşeler.

**Boş nesne gider:**

- toleranstan kısa çizgi;
- yarıçapı toleranstan küçük daire ya da yay;
- bütün köşeleri ilkine tolerans kadar yakın yol;
- sözsüz yazı.

**Yinelenen gider, ilki kalır.** Yinelenenin şunları eşit olmalıdır: türü, kendi özellikleri (`ignore`'dakiler dışında; çizim sırası
sayılmaz) ve verisi (öznitelikler, etiket, ekler, etiket iğneleri). Verisi farklı nesne yinelenen sayılmaz, veri kaybolmaz. Noktaların
tolerans içinde eşleşmesi türe göredir:

| Tür | Eşleşen |
|---|---|
| Nokta | konum ve kot |
| Çizgi | iki ucu, iki yönden biriyle; kotları ve ölçüleri de onlarla |
| Daire | merkezi ve yarıçapı |
| Yay | merkezi, yarıçapı ve uçları |
| Çoklu çizgi ve alan | köşeleri sırayla, ters ya da (kapalıysa) herhangi bir köşeden başlayarak. Bükümleri o eşleşmeyle (ters yönde eksi işaretle) 1e-9 içinde. Kotlar, ölçüler ve genişlikler yalnız sırayla. Delikli ya da çok parçalı olan yalnız alanları birebir eşitse. |
| Öbür türler | taban dışındaki bütün alanları birebir eşitse |

Aday, ilk noktasının hücresinde ve komşu hücrelerde aranır. Hücre `2·tolerans` (en az 1 µm) genişliğindedir. Hücrelerde kalanların
noktaları dizinlidir: çizginin iki ucu, açık yolun uçları, kapalı yolun bütün köşeleri, nokta, merkez.

**Eş doğrultulu çizgiler** (`overlaps`): aynı gruptaki (tür, özellikler ve veri), kotsuz ve ölçüsüz çizgiler birleşir.

- Bir çizgi, kümenin ilk çizgisiyle şu koşullarda birleşir:
  - iki ucu birbirinin doğrusuna tolerans kadar yakın;
  - kümenin ilk çizgisi boyunca aralıkları bir uzunluk paylaşıyor; `endToEnd`'le tolerans içinde değmeleri de yeter.
- Bir çizginin bağladığı iki küme öncekinde birleşir.
- İlk çizgi kümenin en uzak uçlarına uzanır, kendi doğrusunun üstünde; kendi ucu en uzaktaysa yerinde kalır. Öbürleri gider.

**Köşeler:** yolun bir köşesi şu durumlarda gider:

- İki kenarı düz ve son kalan köşeyle bir sonraki arasındaki doğruya tolerans kadar yakın, ikisinin arasında:
  - tekrarlanan köşe her zaman;
  - doğrudaki köşe `straightVertices` ile.
- Kendi kenarı yay olan tekrarlanan köşe de gider; yay bir öncekinden başlar.

Kalanlar:

- açık yolun uçları kalır; son köşeyi tekrarlayan köşe onun yerini alır;
- kapalı yolun ilk köşesi kalır;
- kendi kotu ya da ölçüsü olan köşe kalır;
- genişliğin değiştiği köşe kalır.

Kalan kenar, atılan köşelerin düz olmayan kenarını alır. Genişliği atılan dizinin ilk kenarının başı ile son kenarının sonudur. Delikli
ve çok parçalı yollara dokunulmaz.

**Rapor:** sayılar ve kayıp. Sayılanlar boş, yinelenen, katılan ve katan çizgiler, atılan köşeler ve köşesi atılan yollardır. Kayıp,
giden bir noktanın kalandan en uzak olduğu yerdir:

- yinelenende noktalar arası en büyük uzaklık;
- boşta erişim;
- birleşmede katılan uçların kalan doğruya uzaklığı;
- köşede doğruya uzaklık.

Bu, toleransın gerçek bedelidir. Gizli bir onarım yoktur: tolerans kullanıcının açık seçimidir ve sonucu söylenir (CLAUDE.md §23.3).

### 3. Retler

1. `invalid_uid`, `no_entities` (boş liste);
2. tolerans: `not_finite`, `invalid_tolerance`;
3. `invalid_revision`, `revision_conflict`;
4. `entity_not_found`;
5. `layer_locked`: bütün verilen nesneler kilitli. Bir kısmı kilitliyse onlar uyarıyla atlanır.

### 4. Araç

Çizimi temizle komutla önizler ve yazar.

**Kapsam:** seçim; seçim yoksa bütün çizim. Bütün çizimde kilitli ve gizli katmandakilerin sayısı söylenir.

**Seçenekler** oturumda kalır:

- Tolerans (T): sayı yazılır, ilk değeri 0,000001 m;
- Örtüşenler (Ö): açık başlar;
- Uç uca (U): kapalı başlar;
- Köşeler (K): açık başlar;
- Görünüş (G): renk, çizgi tipi ve ölçeği, kalınlık, saydamlık ve sembol yok sayılır; kapalı başlar.

**Önizleme:** gidenler tehlike renginde kesikli, değişenler vurgu renginde çizilir. İstem sayıları ve en büyük kaymayı gösterir.

**Davranış:**

- Seçenek değişince plan yeniden yapılır.
- Enter yazar ve iletisi en büyük kaymayı da söyler.
- Bir şey bulunmadıysa araç seçeneklerle açık kalır; Enter “çizim temiz” deyip çıkar.

## Uygulama

- **Çekirdek:** `crates/shared/geometry-core/src/ops/overkill.rs` (`same_points`, `thin`, `kept_bulges`, `merge_lines`).
- **Sözleşme:** `crates/shared/contracts/src/cad_cleanup.rs` (`EntitiesCleanup`, `CleanupIgnore`, `CleanupReport`,
  `EntitiesCleaned`, `EntitiesCleanupPlan`); katalog ve yetenekler.
- **Komut:** `crates/native/application/src/cleanup_cmd.rs` (`entities_cleanup`). Yazma `cad.entities.edit`'in `cleanup` işlemiyle olur.
- **Araç:** `crates/native/interaction/src/cleanup.rs`; `Memory::cleanup_*`.
- **Python ve MCP:** `kentos.cad.entities.cleanup`; MCP yıkıcı listede. Komut geçmişinde “Çizimi temizle”.
- **Manifest:** `cleanup` `semantic`, `via` `cad.entities.cleanup`, `was` `cad.entities.edit`.
- **Resimler:** `apps/desktop/src/cleanup_scenes.rs`.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/cleanup_command_cases.py` KentOS kodu olmadan yazıldı. Çekirdeğin `thin` ve `merge_lines`'ı
  işlem sırasıyla yeniden yazıldı. Ortak durumlar `fixtures/commands/v1/cad.entities.cleanup.json`'dadır (37 durum):
  - tam, ters ve tolerans içinde yinelenen çizgi; varsayılan toleransta ayrı kalan;
  - örtüşen, içte kalan, uç uca (seçenekle ve seçeneksiz) ve köprülenen çizgiler;
  - boş ve toleranstan kısa çizgi;
  - rengi farklı yinelenen (yok sayılınca silinir), öznitelikleri farklı (silinmez);
  - daire, yay, nokta;
  - ters çoklu çizgi, başka köşeden ters başlayan kapalı alan;
  - kotlu köşe, yaylı tekrarlanan köşe, genişlikler, kapalı çoklu çizgi;
  - sözsüz yazı, kilitli nesne, bütün çizim, temiz kapsam;
  - bütün retler, plan ve doğrulama, sürüm çakışması.

  Yetenek testi her kodu görür.
- **Çekirdek testleri:** `ops::overkill`'in birim testleri.
- **Araç:** `crates/native/interaction/tests/all/splitting.rs`:
  - bulunanların iletisi, istem ve seçenekler;
  - Köşeler kapalıyken yalnız tekrarlanan;
  - yazma ve tek geri alma;
  - seçim kapsamı;
  - temiz çizimde açık kalıp Enter'la çıkma;
  - kilitli ve gizlilerin sayısı.
- **Süre** (`cargo test --release -p kentos-native-application --test all cleanup_cmd::timing -- --ignored --nocapture`, bu
  makinede): 100 000 nesnede (10 000 yinelenen, 10 000 katılan çizgi, 10 000 köşe) plan 293 ms, yazma 343 ms.
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
