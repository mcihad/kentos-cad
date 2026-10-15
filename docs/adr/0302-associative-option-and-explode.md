# ADR 0302: Dizi araçlarında İlişkili ve anlamsal Patlat

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-16`: “Mevcut bağımsız kopya dizileri korunmalı;
  ‘ilişkili’ açık seçenek olsun ve explode ilişkili diziyi bağımsız nesnelere dönüştürsün.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0300 (ilişkili dizi), 0301 (Diziyi düzenle), 0144 (bloğun patlatılması), 0047 (dizi araçları), CLAUDE.md
  §9.2 (araç = komut; dokunulan araç kurala getirilir).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 0300 ilişkili diziyi `cad.entities.array`'in `associative`'iyle kurdu, ama araçların bunu seçmenin yolu yoktu. Patlat da ilişkili
diziyi tanımıyordu: bir öğeyi patlatmak yalnız o yerleştirmeyi açıyor, dizide boş bir öğe bırakıyordu.

AutoCAD'de ARRAYASSOCIATIVITY dizi komutlarının ilişkili mi bağımsız mı yapacağını söyler, ARRAY'in AS seçeneği komut sırasında
değiştirir. Patlat (EXPLODE) ilişkili diziyi tek nesne sayar ve bütün öğelerini bağımsız nesnelere açar.

Patlat'ın kendi anlamsal komutu yoktu: parçaları araç hesaplıyor, `cad.entities.edit`'in `explode` işlemiyle yazıyordu (manifestte
`geometry`). §9.2'ye göre dokunulan araç kurala getirilir.

## Karar

### 1. İlişkili seçeneği

Dizi, Kutupsal dizi ve Yol boyunca dizi araçlarında **İlişkili (İ)** seçeneği vardır:

- İstemde değeriyle görünür (“İlişkili (İ): hayır”). İ yazmak açar ya da kapatır, Dizi'de bir sayı sorulurken de.
- Değer uygulama açık kaldıkça hatırlanır (`Memory::array_associative`).
- **Kapalı başlar:** bağımsız kopyalar eskisi gibi kalır (planın “mevcut bağımsız kopya dizileri korunmalı”sı). AutoCAD'in varsayılanı
  açıktır; KentOS'ta var olan iş akışı değişmesin diye kapalı.
- Açıkken araç komutun `associative`'ini verir. İleti “2 × 3 ilişkili dizi oluşturuldu: 6 öğe.” gibidir.
- Kopyala'nın Dizi'si (Yer değiştirme boyunca, ADR 0291) bağımsız kalır; AutoCAD'in COPY Array'i de ilişkisizdir.

### 2. Anlamsal Patlat

Komut `cad.entities.explode` v1'dir. Girdi `uids` ve `expectedRevision`'dır. Çıktı ve plan `cad.entities.edit`'inkidir; tek geri alma
adımının adı “Patlat”tır.

**Parçalar** çekirdektendir (`ops::explode::explode_entity`; bloğun `Blocks::explode`'u; ADR 0144):

| Nesne | Parçaları |
|---|---|
| Yol ve alan | kenarları; delikler ve parçalar da |
| Eğri | çoklu çizgi |
| Ölçü | çizgiler, oklar ve yazı (yazısı yoksa değeri projenin gösterimiyle, `Format::dimension`) |
| Desenli tarama | çizgiler |
| Kılavuz | çizgi, ok ve not |
| Tablo | çizgiler ve yazılar |
| Bloğun yerleştirmesi | tanımın nesneleri, bir düzey |

**Parçaların aldıkları:**

- Sıradan bir nesnenin parçası nesnenin katmanını, rengini ve kalınlığını alır; öznitelik almaz.
- Bloğun nesnesi çizimde varsa kendi katmanını alır, yoksa yerleştirmeninkini. Rengini ve kalınlığını, yoksa yerleştirmeninkini alır.
  Özniteliklerini ve etiketini de alır.

**İlişkili dizinin öğesi** bütün diziyi getirir. Önce girdinin nesneleri, sonra dizinin öbür öğeleri öğelerin sırasıyla patlar; dizi aynı
adımda kalkar (`remove_array`). Geri alma diziyi ve öğelerini geri getirir. Bloğu (“Dizi n”) kalır; Kullanılmayanları temizle onu siler.

**Atlananlar:**

- Patlamayan nesne (çizgi, daire, elips, nokta, yazı, dolu ya da degrade tarama, resim, raster, nokta bulutu) uyarıyla atlanır
  (`not_explodable`: sayı ve ilkinin sözü). Hiçbiri patlamazsa ret olur, ilkinin sözüyle ve yeriyle.
- Kilitli katmandaki nesne de ADR 0278'in kuralıyla uyarıyla atlanır (`layer_locked`); hepsi kilitliyse ret.

**Retlerin sırası:** `no_entities`, `invalid_uid`, `invalid_revision`, `revision_conflict`, `entity_not_found`, `layer_locked`,
`not_explodable`, `slots_exhausted`.

**Araç:** Patlat aracı seçimi komuta verir. İleti “8 nesne patlatıldı: 24 parça.”dır ve parçalar seçilir. Patlatma hesabı (ölçünün
değeri, bloğun parçalarının alanları) araçtan komuta taşındı.

### 3. Bağımlılık yönü

Ölçünün değerini projenin gösterimiyle yazmak için uygulama katmanı (`application`) projenin modelini (`project`, `Format`) kullanır.
`project` yalnız `shared` ve `domain`'i kullanan saf bir modeldir, döngü yoktur. Uygulama katmanını kullanan `style` ve `render`
grupları da dolaylı olarak `project`'e ulaşır. `scripts/arch/deps.mjs` bunu gerekçesiyle izinli sayar.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/cad_explode.rs` (`EntitiesExplode`); `codes.rs`'in `not_explodable`'ı (ret ya da uyarı);
  katalog ve yetenekler (hedef türler: çoklu çizgi, alan, eğri, ölçü, tarama, kılavuz, tablo, yerleştirme, çok noktalı nokta).
- **Komut:** `crates/native/application/src/explode.rs`.
- **Araçlar:**
  - `crates/native/interaction/src/object.rs` (Patlat komutla);
  - `modify.rs` (`associative_option`, `associative_typed`, `array_selection`'ın `associative`'i);
  - `array.rs`, `polar.rs`, `array_path.rs`; `Memory::array_associative`.
- **Python ve MCP:** `kentos.cad.entities.explode`; MCP yıkıcı listede.
- **Manifest:**
  - `tool.explode` `semantic`, `via` `cad.entities.explode` (eskiden `geometry`);
  - dizi araçlarının notu İlişkili'yi anar.
- **Resimler:** `apps/desktop/src/assoc_array_scenes.rs` (`dizi-iliskili-secenek`, `dizi-patlat`).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/explode_command_cases.py` KentOS kodu olmadan yazıldı. Ortak durumlar
  `fixtures/commands/v1/cad.entities.explode.json`'dadır (23 durum):
  - açık, kapalı ve çok parçalı çoklu çizgi;
  - delikli alan (öznitelik geçmez);
  - bloğun ölçekli yerleştirmesi (katmanı ya da yerleştirmeninki, rengi ya da yerleştirmeninki, kalınlığı);
  - iki nesne birlikte, aynı kimlik iki kez;
  - patlamayan ve kilitli nesnenin uyarıları;
  - bütün retler, plan;
  - ilişkili dizinin öğesi (bütün dizi, sıra, dizi gider, geri alınınca döner), aynı dizinin iki öğesi, dizi ile başka bir nesne.

  Yetenek testi her kodu ve hedef türleri görür.
- **Araçlar:** `crates/native/interaction/tests/all/arrange.rs` ve `array_path.rs`: istemlerde İlişkili; İ ile ilişkili dizi kurulur,
  geri alınır. Patlat'ın araç testi (`edits.rs`) komutla geçer.
- **Süre** (`cargo test --release -p kentos-native-application --test all array_assoc::explode_timing -- --ignored --nocapture`, bu
  makinede): 100 × 100 ilişkili dizinin bir öğesinden bütün dizinin patlatılması (30 000 parça) 89,7 ms, geri alınması 10,5 ms.
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
