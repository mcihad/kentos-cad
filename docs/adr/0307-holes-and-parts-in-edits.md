# ADR 0307: Delikli ve çok parçalı nesnelerde düzenlemeler

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-21`: “Holes ve multipart üzerinde mevcut açık
  redleri incele; destek genişlerken boşluklar, attribute, Z/M ve kalıcı ID kaybedilmesin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0143 (çok parçalı alan), 0174 (çok parçalı çizgi ve çok noktalı nesne), 0142 (köşe kotu), 0224 (ölçü),
  0250 (anlamsal Buda ve Uzat), 0296 (Kır'da taşınanlar).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 0143 ve 0174, bir yol ya da halka boyunca çalışan düzenlemelerin çok parçalı nesneyi reddetmesini istedi: hangi parçanın
kastedildiği bilinmiyordu, öbür parçalar da kaybolabilirdi. Bu ret yerinde ama bazı işlemler için gereğinden geniş: Kır, Buda ve
Uzat bir yere tıklanarak çalışır ve tıklama parçayı söyler.

## Karar

### 1. Envanter

Bugünkü durum, düzenleme komutlarının koduna bakılarak (“—”: işlem o nesneye uygulanmaz ya da burada incelenmedi):

| İşlem | Delikli alan | Çok parçalı alan | Çok parçalı çizgi | Çok noktalı nesne |
|---|---|---|---|---|
| Kır (`cad.entities.break`) | ret: delikler kaybolurdu | ret | **tıklanan parça (bu ADR)** | — |
| Buda (`cad.entities.trim`) | ret: delikler kaybolurdu | ret | **tıklanan parça (bu ADR)** | — |
| Uzat (`cad.entities.extend`) | — (alanın ucu yok) | — | **tıklanan parça (bu ADR)** | — |
| Uzat-kısalt (`cad.entities.lengthen`) | — | — | ret (`multi_part`) | — |
| Ötele (`cad.entities.offset`) | delikler öbür yana (ADR 0293) | her parça | her parça | — |
| Köşe yuvarla, Pah: bütün köşeler | — | her parça | her parça | — |
| Köşe yuvarla, Pah: iki nesne | — | — | ret (uç kenar bilinmez) | — |
| Yönü çevir | halkalar ters döner | her parça | her parça | — |
| Sadeleştir | — | her parça | her parça | — |
| Birleştir | — | — | ret (hangi uç bilinmez) | — |
| Paralel kaydır | deliğin kenarı da (ADR 0191) | ret | ret | — |
| Biçim değiştir | delikler kalır | — | ret | — |
| Eğriyi düzenle | — | — | ret (`multi_part`) | — |
| Köşe komutu, tutamaçlar, köşe tablosu | halkalar | parça ve köşe | parça ve köşe | — |
| Patlat | halkalar da | her parça | her parça | noktalarına |
| Çizimi temizle | atlanır | atlanır | atlanır | atlanır |
| Parçalara ayır | — | parçalarına | parçalarına | noktalarına |

Retlerin hepsi sebebini ve çözümünü söyler (çoğunlukla “önce Parçalara ayır”).

### 2. Kır, Buda ve Uzat çok parçalı çizgide

Tıklanan nokta parçayı seçer: tıklamaya en yakın parça (çekirdeğin `ops::parts::part_near`'ı; eşit yakınlıkta ilki).

- **Kır:** ilk nokta parçayı seçer; ikinci nokta aynı parçada olmalıdır, değilse ret (`break_failed`, `second`). Parça çekirdeğin
  kuralıyla kırılır.
- **Buda:** her tıklama kendi parçasında. Kural ADR 0294'tekiyle aynıdır, ama bayrak artık nesnenin değil parçanındır: önceki bir
  kesimin bıraktığı, sınırın kesmediği parça bütünüyle gider; hızlı kipte (`eraseUncut`) hiç kesilmemiş bir parça yalnız kendisi
  gider. Çit ve kesişen pencere parçaları tek tek görür.
- **Uzat:** tıklanan parçanın yakın ucu ilk sınıra uzar; her parçanın her ucu bir kez.

Parçalardan çıkan parçalar **aynı nesnenin parçaları** olarak, geldikleri parçanın yerine yazılır. Nesne tek kalır: kimliği,
katmanı, görünüşü, öznitelikleri, etiketi ve ekleri korunur. Kotlar ve ölçüler ADR 0142'nin kurallarıyla taşınır: kaynak köşede
kendi değeri, kaynak kenarda kenar boyunca doğrusal, uzayan uçta uç kenarın eğimi. Buda bütün parçaları alırsa nesne silinir.

Çok parçalı alan ve delikli alan yine reddedilir: parçalar alan olarak kalamaz, delikler kaybolurdu (Alan böl, Patlat ya da
Parçalara ayır).

### 3. Araçlar

Kır, Buda ve Uzat araçları aynı komutlarla çalışır. Önizleme yalnız tıklanan parçanın gidecek kısmını gösterir. İleti çok parçalı
çizginin parça sayısını söyler: “Budandı: çizgi 3 parçalı.”, “Parça bölündü: çizgi 4 parçalı.”

### 4. Kalanlar

Uzat-kısalt'ın girdisinde tıklama yok; çok parçalı çizgi için parça numarası alan bir sürüm (`part`) ayrı bir iştir. Paralel
kaydır, Birleştir, Biçim değiştir ve Eğriyi düzenle de öyledir.

## Uygulama

- **Çekirdek:** `crates/shared/geometry-core/src/ops/parts.rs` (`part_near`, `polyline_of_pieces`).
- **Komutlar:** `crates/native/application/src/break_join.rs` (`check_break`), `crates/native/application/src/trim.rs` (`Work`'ün
  `fresh`, `parted` ve parça başına `used`'ı; yazmada parçalar tek nesne).
- **Araçlar:** `crates/native/interaction/src/{breaking,trim}.rs` iletileri; `edge::line_parts`.
- **Resimler:** `apps/desktop/src/multipart_scenes.rs`.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/multipart_edit_cases.py` KentOS kodu olmadan yazıldı (eksenlere paralel kenarlar,
  kesirler). Ortak durumlar `fixtures/commands/v1/cad.entities.{break,trim,extend}.parts.json`'dadır (her biri 20 durum, toplam 60):
  - tek ve iki noktalı Kır, köşeli parça, kotsuz parça, parçanın ucu, başka parçada ikinci nokta;
  - Buda'nın tek ve iki tıklaması, kesilmiş parçanın yeniden kesilmesi, hızlı kip, çit, kesişen pencere, bütün parçaların gitmesi;
  - Uzat'ın uçları, aynı ucun bir kez uzaması, pencere ve çit, önünde sınır olmayan uç;
  - çok parçalı ve delikli alanın, kilitli katmanın retleri; plan, doğrulama, sürüm çakışması, geri alma ve yineleme.

  Her durumda nesnenin kimliği, öznitelikleri, rengi, kotları ve ölçüleri denetlenir.
- **Çekirdek testleri:** `ops::parts`'ın birim testi.
- **Araç:** `crates/native/interaction/tests/all/multipart.rs` (Buda, Uzat, Kır; iletiler, kot, geri alma).
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
