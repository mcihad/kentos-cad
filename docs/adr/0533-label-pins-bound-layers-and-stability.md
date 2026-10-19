# ADR 0533: Etiket iğneleri, bağlı katmanlar ve yerleşimin kararlılığı

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-18'in ilk tam dilimi (“etiket motorunun bağlı katmanlarla, kaydırmada
  ve paftada tutarlı çalışması”). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0212 (etiket motoru, `cad.labels.pin`, `labelPins`), 0503 ve 0510 (bağlı katman, alan sınırlı çalışma
  kopyası), 0506 (taşınabilir proje), 0205 (açıklamaların ölçeği), 0252 (başsız kitap PDF'i).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** bu belgenin sonunda.
- **`.kcad`:** şema değişmez. `SourceLink.key` şema 74'ün içinde isteğe bağlı alandır (ADR 0510'un `extent`'i gibi); bağımsız
  okuyucu (`tools/kcad/kcad.py`) kuralını bilir.

## Bağlam

Etiket iğneleri (ADR 0212 §5) nesnenin üstünde durur (`EntityBase.label_pins`) ve nesneyle birlikte gider. Bağlı katmanda (ADR
0503) nesne bir tablo satırının çalışma kopyasıdır; kimliği (`uid`) satırın kalıcı kimliğinden gelir (`kentos_uids`, yoksa
tablo ve satır numarasından türetilir). Üç şey iğneyi yanlış kayda götürebilirdi ya da sessizce yok edebilirdi:

1. Tablo başka bir programla yeniden yazılır (satır numaraları ve kalıcı kimlikler değişir, değerler aynı kalır): iğne
   “kaybolur”, oysa kullanıcı için aynı parseldir.
2. Alan sınırı (ADR 0510) daralır ya da genişler: dışarıda kalan nesnenin iğnesi söylenmeden gider.
3. Etiket ve engel sorgusu yalnız yüklenen parçayı görür; kullanıcı bunu bilmez.

Ayrıca iki ayrı sorun vardı: küçük bir kaydırmada çok sayıda etiket yer değiştiriyordu (bir alanın kutbu sırtı boyunca her yerde
olabilir), ve paftanın haritasının önbelleği kaynağın sürümünü ve çıktı ölçeğini saymıyordu.

## Karar

### 1. İğne sürekliliği: anahtar

Bağın isteğe bağlı **anahtarı** (`SourceLink.key`, bir sütunun adı; boş değil, 128 harfi aşmaz, denetim karakteri yok;
`cad.sources.edit`'in `link`'inde `key`, sonradan `Key {id, key}`) satırı yeniden yazımlar boyunca tanıtan değerdir.
Anahtar belirtilmişse bağlama anında tablonun o sütunu okunur ve değerlerinin tekliği denetlenir (yinelenen değer
`key_not_unique` uyarısı: o değerin satırları anahtarla izlenmez).

**Kaynaktan oku** (ve alan sınırı değişikliği) bir nesneyi kaldırırken iğnesi varsa şunu yapar:

- aynı anahtar değerli, yeni gelen bir satır varsa iğne ona taşınır (`moved`: eski ve yeni kimlik, anahtar değeri);
- yoksa iğne atılır ve nedeni söylenir (`lost`): `deleted` (satır tabloda yok), `outside` (satır katmanın alanının dışında),
  `rebound` (aynı satır kimliği artık başka bir anahtar değeri taşıyor: iğne başka bir kayda bağlanmaz), `taken` (anahtarın
  yeni satırı başka bir iğneyi zaten taşıyor).

Cevap `pins {kept, moved[], lost[]}` taşır; uyarılar `label_pins_moved` ve `label_pins_lost`'tur (hem `cad.sources.edit`'in
`extent` ve `key` işlemlerinde hem `cad.sources.sync`'in `read`'inde). Anahtarsız bağda iğneler yalnız kimliği değişmeyen
satırlarda kalır; kalıcı kimliği değişen yeniden yazımın iğneleri `deleted` diye söylenir, hiçbiri başka satıra bağlanmaz.
Bağın ayarları (anahtar, alan) hiçbir zaman geri alma adımı değildir (ADR 0510); geri almak nesneleri getirir, bağın ayarını
değil.

**Alan sınırı** değişikliği için ayrı işlem eklendi: `Extent {id, extent}` (katman tablosuyla aynı olmalı: bekleyen değişiklik
yok). Daralınca dışarıda kalan nesnelerin iğneleri `outside` diye söylenir; genişleyince gelenler eklenir.

### 2. Yüklenen parça söylenir

Etiket ve engel sorgusu ayrı bir “bütün tablo” yolu kurmaz: bağlı katmanın etiketleri ve engelleri yüklenen parçanınkidir ve
bu `cad.labels.report`'ta `partial` ve `part` (parçanın alanı) olarak her katmanda söylenir; Etiketler penceresinde Denetle
bunu bir satırla yazar. Bu bir sınırdır, gizli bir kısıtlama değil.

### 3. Yerleşimin kararlılığı

Motor (`labels::engine::place`) artık önceki yerleşimi bilir: `Prior` (nesne, sınıf, parça numarasından etiketin dünyadaki
ortası; yalnızca yerleşen etiketler) ve her birimde `prefer` noktası. Aday seçimi maliyet sırasını korur ama **önceki
yerin 2 px içindeki, hâlâ boş olan aday** ilk boş adayın önüne geçer (en çok 48 aday denenir); çizgi boyunca yürüyen
etiketler son yerlerine en yakın yürüyüş yerinden (`Walk::nearest`) başlar; alanın kutbu yerine önceki yer, sığıyorsa, adaydır.
Önceki yer boş değilse ya da sığmıyorsa motor eski davranışına döner: öncelik, çakışma ve sığdırma kuralları değişmez.
Bellek yalnızca ana görünümde (`Spatial.keep`) tutulur, çizim değişince ve yeniden yükleyince
(`reload`) silinir; paftanın haritası ve `cad.labels.report` belleksizdir, yani her çağrı aynı sonucu verir.

Ölçüm (`label_stability.rs`, 840 parselli sentetik harita, 24 kaydırma, her biri en çok 40 px): ortak 3 640 etikette belleksiz
yeniden yerleşimde **73** etiket yer değiştirdi (bir kaydırmada en çok 4); önceki korunarak **0**.

### 4. Önbellek: kaynağın sürümü ve çıktı ölçeği

Ana görünümün etiket önbelleği anahtarına bağlı katmanların kaynağının son eşleşen hâli ve tuttuğu parça girer
(`labels::sources_key`); bir okuma etiketin metnini ya da yerini değiştirirse, belgenin sayacı değişmese de, yeniden kurulur.
Paftanın haritasının sürümü de (`SheetPainter::revision`) aynısını, oturumu ve belge kuşağını sayar.

### 5. Ekran ve PDF aynı yerleşim

Paftanın haritası etiketlerini hem ekrana çizerken hem PDF'e vektör yazarken aynı `SheetPainter::label_layout`'tan alır;
pencere boyutu kâğıt piksellerinde (`PX_MM = 25,4 / 96`) milimetre altı yuvarlama hatasıyla bir piksel şaşabiliyordu: boyut
binde birine yuvarlanıp yukarı alınır. `a_map_s_labels_are_placed_the_same_on_the_screen_and_in_the_pdf` aynı kâğıt ölçeğinde
iki yolun yerleşimini karşılaştırır.

### 6. `cad.labels.report` ve Denetle

Salt okunur **`cad.labels.report`** (v1) bir pencerenin etiketlerini bir ölçekte, motorun kendisiyle (`Store::place_labels`)
sayar ve hiçbir şey yazmaz: yerleşen, yerleşemeyen, elle gizlenen, iğneli, zorla başka etiketin üstüne konan, kutuları
gerçekten kesişen (eğri etiketler kutulanmaz) ve yinelenen metin (katman, sınıf ve metne göre). Girdi: `pxPerMetre` ya da
`paperScale` (1:N; tam biri), isteğe bağlı `window` ve `layers`. Çıktı: toplam, katman başına sayılar, `window`,
`pxPerMetre`, `revision`. Retler: `invalid_scale`, `invalid_window`, `layer_not_found`, `not_a_layer`, `service_layer`.
Etiketler penceresinde **Denetle** düğmesi komutu o katman ve görünüm için çalıştırır.

## Uygulama

- Çekirdek: `labels/engine.rs` (`Unit.prefer`), `labels/geom.rs` (`Walk::nearest`), `store/placing.rs` (`Prior`,
  `place_labels_after`), `store/legible.rs` (`labels_shown_after`), `store/label_audit.rs` (denetim), `interaction/spatial.rs`
  (`prior`).
- Sözleşme: `contracts/src/sources.rs`, `cad_sources.rs` (`Link.key`, `Key`, `Extent`, `pins`, `PinLoss`, `PinMove`, `LostPin`,
  `SyncedPins`), `cad_labels.rs` (`LabelsReport`, `LabelsReported`, `LabelCounts`, `LayerLabelReport`), `codes.rs`
  (`INVALID_KEY`, `LABEL_PINS_LOST`, `LABEL_PINS_MOVED`, `KEY_NOT_UNIQUE`, `INVALID_WINDOW`), `capabilities.rs`, `catalog.rs`.
- Uygulama: `application/src/sources.rs` (`key_check`, `follow_pins`, `change_extent`), `labels_report.rs`.
- Masaüstü: `labelling/mod.rs` (Denetle), `labels.rs` ve `sheets.rs` (önbellek).
- Python: `kentos.cad.labels.report`; MCP'de salt okunur araç; manifestte komut ve işlem düzeyi değişmedi (Denetle bir
  pencere düğmesidir, ribbon komutu değil).

## Doğrulama

- Ortak durumlar: `cad.labels.report` 32, `cad.sources.edit` 57, `cad.sources.sync` 37 (ikisi de iğne ve anahtar durumlarıyla genişledi); hepsi
  `scripts/fixtures/labels_report_cases.py` ve `bound_sources_cases.py`'nin, KentOS kodu olmadan yazılmış başvurusundan
  (`label_engine_cases.py`'nin `prefer` mantığının Python ikizi dahil); `fixtures/labels/v1/cases.json`'a 17 durum eklendi.
- Geçici kopyada (`sources.rs`): tablo başka programla yeniden yazılınca iğneler anahtarla yeni satırlara gider (2 `moved`,
  uyarı `label_pins_moved`), anahtarsız `deleted` (2 `lost`, uyarı `label_pins_lost`), yinelenen anahtar `key_not_unique`.
- Yoğun harita (840 parsel, 19 sokak, 70 blok): görünümde **718 etiket yerleşti, 1 yerleşemedi, 0 zorla, 0 örtüşen, 47
  yinelenen** (sokak adının parçaları); belleksiz yerleşimde kaydırmada 73 kayma, bellekle 0.
- Ekran ve PDF aynı yerleşimi verir (`sheets.rs` testi).
- Resimler: `.run/shots/arac-etiket-denetle-{1440x900,1100x650}[-acik].png`.

## Sonraya kalan

- Bağın ayarlarının (alan, anahtar) geri alınması: bağ ayarı geri alma adımı değildir (ADR 0510); onu adım yapmak ayrı karar.
- Alanın dışına çıkan nesnenin iğnesini park etmek (alan genişleyince geri vermek): bugün iğne `outside` diye söylenir ve atılır.
- Anahtarsız yeniden yazımı kendiliğinden anlamak (satırın değerleriyle eşleme): bugün yalnız anahtarla.
- Alan etiketlerinin (kutup) kararlılığı yalnız seviye etiketlerinde sınandı; çok parçalı alanın parça başına belleği ve
  eğri boyunca etiketlerin uzun kaydırmadaki davranışı ölçülmedi.
- Denetle'nin pafta ölçeği (`paperScale`) ve belirli bir paftanın haritası için düğmesi: komut hazır, arayüzü yok.
- Anahtar alanının arayüzü (GeoPackage'a bağla penceresinde ve Kaynak ▸'de): komut hazır (`key`), pencere henüz sormuyor.
- Web tarafı (ADR 0228).
