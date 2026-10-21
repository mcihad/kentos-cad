# ADR 0578: Parçalı bağlı katmanlarda parçanın dışındaki engeller, anahtarsız yeniden yazımda iğneler ve alanın dışındaki iğnenin parkı

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-18'in kalanı (ADR 0533'ün “Sonraya kalan” listesinden üç iş):
  - sağlayıcı sayfalanırken (alan sınırlı parçada) parçanın dışındaki engeller;
  - anahtarsız yeniden yazım;
  - alanın dışına çıkan nesnenin iğnesinin parkı.

  Kapsamı koordinatörün sözleriyle ben belirledim.
- **Bağlam belgesi:**
  - ADR 0533 (etiket iğneleri, anahtar, `cad.labels.report`), 0212 (etiket motoru, iğneler);
  - ADR 0510 (alan sınırlı parça), 0531 (alanın dışındaki komşuları aynı dosya sürümünden okuma);
  - ADR 0503 (bağlı katman, bağın sürümü), 0538 (tabanlar);
  - ADR 0576 (görünümler; ADR'de “web ertelendi” aynı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. `SourceLink.parked` şema 74'ün içinde isteğe bağlı alandır (`extent` ADR 0510'da, `key` ADR 0533'te
  böyle geldi); kayıtta bağlar bütün yazılır ve denetlenir (`sources_problem`), bağımsız okuyucu (`tools/kcad/kcad.py`) kuralını bilir.
  Şema 98 ayrılmıştı, kullanılmadı.
- **Sonraya kalan:**
  - PostGIS tablosunda alan sınırı henüz yok (ADR 0510): bant da yalnız GeoPackage parçasında olur;
  - paftanın haritası bir ölçekli açıklamanın başka ölçeğinde kendi geometri deposunu kurar (`sheets.rs`'in `source`'u): o depoda bant
    yoktur, kenardaki etiketler oradaki parçanın nesnelerine göre yerleşir (ekranla PDF aynı ölçekte aynıdır);
  - bantta zaman, süzgeç ve temanın etkisi: bant satırları her zaman engeldir;
  - anahtarlı bağda anahtarı eksik ya da yinelenen satırın iğnesi için geometriyle eşleme (anahtar kuralı değişmedi);
  - çok büyük dosyada bandın SHA-256'sı (bütün dosyadan, ADR 0510'un notu): başsız `cad.labels.report` her çağrıda okur.

## Bağlam

ADR 0533 üç şeyi bilerek açık bıraktı:

1. Alan sınırlı bağlı katman (ADR 0510) tablonun yalnız parçasını taşır; etiket motoru yalnız parçanın nesnelerini görür. Alanın hemen
   dışındaki bir parsel hiçbir etikete engel olmaz; kenara yakın etiket, parça kaydıkça ya da genişledikçe öbür yana atlar.
2. Anahtarsız bağda tablo başka bir programla yeniden yazılınca (satır numaraları ve kalıcı kimlikler değişir, değerler aynı kalır)
   iğneler `deleted` diye söylenirdi: kullanıcı için aynı parsel olduğu hâlde iğne kaybolurdu.
3. Alanın dışına çıkan nesnenin (elle taşındı ve yazıldı, ya da alan daraldı) iğnesi `outside` diye söylenip atılıyordu: okumayı geri almak
   dışında geri gelmezdi, satır alana dönse de.

## Karar

### 1. Parçanın dışındaki engeller: bant

- **Genişlik kuralı:** parçanın alanı her yanda **kısa kenarının %15'i** kadar büyütülür (`sources::band_of`, `BAND = 0,15`). Kuralın birimi
  yoktur (alan neyse o birimde), alt ya da üst sınırı yoktur. Bir etiketin erişimi dünyada ölçeğe göre değişir; kuralın amacı parçanın
  kendi boyuna göre ölçülü bir halka vermektir. Sınırı: etiket, parçanın kısa kenarının %15'inden büyükse (çok uzaklaşılmış bir ölçekte) kenardaki
  bir etiket bandın ötesindeki satırı yine görmez (bkz. Sonraya kalan).
- **Satırlar:** alanı bandın kutusuna değen tablo satırlarından, katmanın nesnesi olmayanlar (kalıcı kimlikle; yani parçanın içindekiler
  değil). Yalnız geometri okunur; nesneye çevrilmez, çizilmez, etiketlenmez, seçilmez, kenetlenmez.
- **Sürüm:** dosyanın katmanın son eşleştiği sürümü (SHA-256'sı bağın `revision`'ı); aynı değilse ya da dosya yoksa bant okunmaz
  (`band_unread`) ve etiketler yalnız parçanın nesnelerine göre yerleşir. ADR 0531'in komşuları gibi (aynı dosya sürümünden, kutuyla).
- **Etkisi:** bant satırı, katmanın kendi nesnesi gibi engeldir: noktaysa sembolü, katmanın etiketlemesi engelse (ADR 0212 `obstacle`)
  çizgisi, kenarı ve içi. Başka hiçbir şey değildir: etiket üretmez, zaman ve süzgeçle gizlenmez, yerleşimin belleğine girmez.
  Çekirdekte `Store::set_ghosts(katman, şekiller)` ve `clear_ghosts`; satırların kimlikleri sıfırın altındadır, hiçbir nesnenin
  kimliği değildir; motorun engel toplayışı nesne ile bant satırı için tek işlevdir (`room_of`).
- **Masaüstü:** bant, parçalı katman olan her çizimde çizimin iletisinden sonra ayrı iş parçacığında okunur (`band_ghosts.rs`); neye bağlı
  olduğuyla (bağın sürümü ve alanı) anılır: dosya ya da alan değişmedikçe yeniden okunmaz; gelen satırlar mağaza temizlenince (çizim yeniden
  yüklenince) geri verilir, çizim değişince ve katman parça olmaktan çıkınca bırakılır. Etiket önbelleğinin anahtarında bant satırı
  sayısı vardır. Etiketler penceresinin Denetle'si bandı yeniden **okumaz**: çizimin kullandığını `execute_with` ile verir (arayüz iş parçacığında
  dosya okunmaz).
- **Başsız:** `cad.labels.report` bandı kendisi okur (eşzamanlı) ve her parçalı katman için `band` (alan) ve `neighbours` (satır sayısı)
  söyler; okunamazsa `neighbours` yoktur ve `band_unread` uyarısı verilir. Parçalı bir katman, etiketi olmasa da raporda listelenir.
- **Neden okunur, saklanmaz:** satırlar dosyanın o sürümünün işidir; kaydetmek dosyayı ikinci kez tutmak (ve bayatlatmak) olurdu.

### 2. Anahtarsız yeniden yazım: eşleme

Anahtarlı bağın kuralı (ADR 0533 §1) değişmez. **Anahtarsız** bağda bir okuma, satırı tabloda olmayan (alanın dışında da olmayan) iğneli
nesnelerin iğnelerini, katmanın nesnesi olmayan **yeni satırlar** arasında şöyle arar (`ops::pin_match`):

1. **Geometri:** aynı türden, köşelerinin ve kutusunun koordinatları bit bit aynı satır. Tek adaysa ona gider (`by: geometry`); birden çoksa
   belirsizdir.
2. **En yakın:** birinci adımda hiçbir nesnenin adayı olmayan satırlar arasında aynı türden, kutusunun ortası nesnenin kutusunun ortasına en
   yakın olan, **0,05 m içinde** ise ve ikinci yakın daha uzaksa (eşit uzaklık belirsizdir) ona gider (`by: nearest`, `distance`).
3. **Yoksa** kaybolur (`deleted`).

İki nesnenin seçtiği satır ikisine de verilmez: ikisi de belirsizdir (`ambiguous`, yeni `PinLoss`). Cevap nesnelerin sırasına bağlı değildir.
Geometrisi olmayan kayıt (öznitelik satırı) eşlenmez. Raporda her iğne yine `kept`, `moved` ya da `lost`'tur; `moved`'da anahtar yerine `by` ve
(en yakında) `distance` vardır. Uyarılar: `label_pins_moved` (anahtarla taşınanlar ve anahtarsız eşlemeyle taşınanlar ayrı iletiyle), `label_pins_lost`.

**Tolerans neden 0,05 m:** başka programın yeniden yazması ya aynı koordinatları verir (adım 1) ya da topolojik temizlik, yuvarlama ya da
kenet payı kadar kaydırır; 5 cm bu payın üstünde, bir parselin boyunun çok altındadır. Daha büyük tolerans iğneyi komşu parsele verirdi.

### 3. Alanın dışına çıkan nesnenin iğnesi: park

Bugüne dek bu iğne atılırdı (`PinLoss::Outside`; okumayı geri almadıkça dönmezdi). Şimdi:

- **Park:** iğneli nesnenin satırı tabloda var ama alanın dışındaysa (`Stored.outside`), iğneleri bağda `parked` listesine yazılır:
  nesnenin kalıcı kimliği (satırınki), anahtarın değeri (bağın anahtarı varsa) ve iğneler. Çizilmez (nesne yoktur).
- **Geri verme:** sonraki okumada satır alana girmişse (aynı kalıcı kimlikle; anahtarlıda anahtarın değeriyle ve tek satırsa) iğneler
  nesnesine geri verilir (`restored`). Nesnenin kendi iğnesi varsa onunkiler kalır: aynı iğneler ise sessizce (bir geri almanın getirdiği nesne),
  başka ise park edilenler `taken` diye söylenip atılır.
- **Sürer:** satır hâlâ alanın dışındaysa park durur; tablodan silinmişse iğneler de gider (`deleted`).
- **Sınır:** bir bağda en çok 100 000 nesne park edilir; fazlası `outside` diye atılır ve söylenir.
- **Geri alma:** bağın ayarları geri alma adımı değildir (ADR 0510); park da öyle. Geri alma nesneyi iğneleriyle getirir, park listesindeki
  aynı iğneler bir sonraki okumada sessizce düşer.
- **Cevap ve rapor:** `SyncedPins.parked` ve `restored` (nesnelerin kimlikleri); uyarılar `label_pins_parked`, `label_pins_restored`;
  `cad.labels.report` her katman için `parked` listesini (kimlik, anahtar, iğne sayısı) söyler; Etiketler penceresinin Denetle'si bir satır yazar.
- **Alan sınırı komutu:** `cad.sources.edit`'in `extent` işlemi de bu yoldan geçer (alan daralınca dışarıda kalanlar park, genişleyince
  geri); daha önce `label_pins_lost` verirdi.

### 4. Komutlar

Yeni komut yoktur; çıktılar genişledi (`SyncedPins`, `PinMove`, `LayerLabelReport`, `SourceLink.parked`), `cad.labels.report`'un uyarısı eklendi. Python
(`kentos.cad` yeniden üretildi) ve MCP katalogdan gelir, işlem manifesti değişmedi (`catalog/v1/operations.json`).

## Doğrulama

| Denetim | Nasıl | Sonuç |
|---|---|---|
| Eşleme kuralı | `python3 scripts/fixtures/pin_match_cases.py --check` (bağımsız başvuru, KentOS kodu yok), `cargo test -p kentos-geometry-core --test all pin_match` ve `--lib pin_match` | 41 durum (geometri, en yakın, eşit uzaklık, tür, iki nesne bir satır, sıra, rastgele 24); dört cevabın hepsi görülür |
| Bant engeli | `cargo test -p kentos-geometry-core --test all ghosts` | bant satırı, aynı yerdeki gerçek nesne gibi etiketi aynı yere iter; nesne sayılmaz, kaldırılınca etiket eski yerine döner |
| Ortak komut durumları | `bound_sources_cases.py` (başvurusu `pins_after`: park, geri verme, geometri ve en yakın eşleme; OGR'la satırlar ve kutular), `labels_report_cases.py`, `cargo test -p kentos-native-application --test all every_command_case`, `.run/py/bin/python -m unittest python.tests.test_command_cases` | `cad.sources.edit` 61, `cad.sources.sync` 48, `cad.labels.report` 32 durum; masaüstü işleyicilerinde ve Python'da aynı sonuç. Yeni: park ve geri verme, Denetle'de park listesi, bant ve komşu sayısı (alanı 1 satır), parçası olmayan katman, üç anahtarsız eşleme |
| Dosyanın yeniden yazımı | `cargo test -p kentos-native-application --test all sources::` | tablo başka programla yeniden yazılınca iğneler anahtarsız aynı geometriyle, 3 cm yakınla taşınır (`moved`, `by`, `distance`), yarım metre uzakta kaybolur, iki eşit satırda `ambiguous`; nesne alanın dışına taşınıp yazılınca iğne park edilir, `cad.labels.report` listeler, alan genişleyince geri verilir |
| `.kcad` | `python3 scripts/fixtures/kcad_v2_reference.py --check` (bağımsız okuyucu `tools/kcad/kcad.py`), `cargo test -p kentos-kcad --test all` | `sources.kcad` park edilen iğnelerle; üç bozuk dosya (iki kez, iğnesiz, boş anahtar değeri) `bad_value` |
| Masaüstü | `cargo test -p kentos-desktop band_ghosts` | bant ayrı iş parçacığında okunur, mağazada nesne değil yalnız engel olur; kenardaki üç isim bant varken batıya geçer, yokken doğuda komşunun üstünde; daraltılan parçada iğne parkta, bant yeniden istenir, Denetle ikisini söyler; 2 test geçti |
| Resimler | `KENTOS_SHOTS_ONLY=bant-cevre,bant-yalniz-parca,bant-denetle,bant-park cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` | `.run/shots/arac-bant-*` (iki tema, iki boyut) |
| Lint | `cargo clippy -p kentos-geometry-core -p kentos-contracts -p kentos-commands -p kentos-native-application -p kentos-interaction -p kentos-plot -p kentos-desktop --all-targets -- -D warnings` | temiz |

Ölçülmeyenler: büyük dosyada bandın okunma süresi (SHA-256 bütün dosyadan hesaplanır, 1 milyon satırlık dosyada ADR 0510'un 13,7 s'lik
ölçümünün büyük kısmı budur; bant masaüstünde arayüzü bekletmez, başsız rapor bekler); 100 000 park edilen nesne.
