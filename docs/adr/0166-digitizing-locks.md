# ADR 0166: Sayısallaştırma kilitleri

- **Durum:** kabul edildi (2026-10-03). `HYB-10`. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-03
- **Bağlam belgesi:** TODOS.md `HYB-10` (ilgili: `UX-04`, `UX-05`), ADR 0018 (araç oturumu ve giriş), ADR 0029 (kenet), ADR 0059 (sağ tık menüleri ve tek seferlik kenet), ADR 0083 (nokta hesaplayıcı), ADR 0085 (nesne izleme), ADR 0163 (kenet ekleri: Paralel kenedi), ADR 0165 §4 (türün eksen ve açı düzeni); [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); Netcad Çizim Hesap Araçları (Açı, Sapma, Referans Noktası) ve ayarları, AutoCAD dinamik girişi (Tab ile uzunluk kilidi, `<açı` ile açı kilidi), ArcGIS Pro geometrik kısıtları (Direction, Deflection, Length, Parallel, Perpendicular) ve Right Angle Line (Square and Finish), QGIS Advanced Digitizing (d ve a kilitleri, yinelenen kilit, paralel ve dik, construction mode).

## Bağlam

KentOS'ta nokta veren her araç imleci ortak bir kuraldan geçirir (`tools::point_input::constrain_cursor`; web `constrainPoint`, masaüstü `points::constrain`): kenet ve nesne izleme noktası olduğu gibi kalır, Orto yatay ya da dikeye, kutupsal izleme açı adımına kilitler. Yazılan değer tek seferliktir: `12` imlecin doğrultusunda 12 m, `@dY,dX` ve `@mesafe<açı` son noktaya göre (ADR 0018, ADR 0165 §4). Paralel kenedi bir kenarda durulunca alınır ve yalnız imleç o doğrultuya yakınken çalışır (ADR 0163).

Netcad'in, AutoCAD'in, ArcGIS'in ve QGIS'in olup KentOS'ta olmayanlar:

- **Kilit:** uzunluğu ya da doğrultuyu nokta konana dek (ya da sürekli) sabitlemek; imleç yalnız kalanı seçer. AutoCAD'de uzunluk yazılıp Tab, açı `<45`; QGIS'te d ve a alanlarının kilidi, yinelenen kilit; ArcGIS'te Direction, Length.
- **Sapma:** doğrultunun önceki kenara göre açısı (Netcad Sapma, ArcGIS Deflection, QGIS'in göreli açısı).
- **Nesneye paralel ve dik kilit:** bir kenar seçilir, doğrultu ona paralel ya da dik kalır (ArcGIS Parallel ve Perpendicular, QGIS P).
- **Referans noktası ve yapım kipi:** tıklanan nokta köşe olmaz, sonraki noktanın başvurusu olur (Netcad Referans Noktası, QGIS construction mode).
- **Dik açılı çizim:** her kenar bir öncekine dik; son köşe ilk kenara dik kapanır (ArcGIS Right Angle Line, Square and Finish).

## Karar

### 1. Kilitler

Kilit, çalışan komutun bir sonraki noktasını kısıtlar. Oturumun durumudur: projeye ve ayarlara yazılmaz, komut değişince ya da bitince gider.

| Kilit | Ne sabitler | Değer |
|---|---|---|
| **Uzunluk** | başvurudan uzaklık | projenin uzunluk biriminde (ADR 0165 §2) |
| **Açı** (CBS'de **Semt**) | doğrultu | türün düzeni ve projenin açı birimi: CAD'de doğudan saat yönünün tersine, CBS'de kuzeyden saat yönünde (ADR 0165 §4) |
| **Sapma** | doğrultunun önceki kenarın doğrultusuna göre dönüşü | projenin açı biriminde; artı yön türün düzenindedir: CAD'de saat yönünün tersine (sola), CBS'de saat yönünde (sağa) |
| **Nesneye paralel** | seçilen kenarın doğrultusu | — |
| **Nesneye dik** | seçilen kenara dik doğrultu | — |

- Bir uzunluk ve bir doğrultu kilidi birlikte durabilir; ikinci doğrultu kilidi öncekinin yerini alır.
- **Başvuru** son noktadır (aracın verdiği `from`); referans noktası verilmişse odur (§5).
- **Tek seferlik:** kilit, nokta konunca gider. **Kalıcı** açıkken (komutun kilitleri için tek düğme) sonraki noktalarda da durur.
- **Esc** önce kilitleri kaldırır (ADR 0018'in “bir adım geri”si); kilit yokken bugünkü gibidir.
- Sapma için önceki kenar gerekir: aracın en az iki noktası (yol araçları, Çizgi zinciri, Eğri, Kılavuz …). Önceki kenarı olmayan adımda Sapma verilmez, nedeni söylenir.

### 2. Kilitli noktanın kuralı

Başvuru `o`, imlecin noktası `c` (kenet ya da nesne izleme noktasıysa o), doğrultu birim vektörü `u`, uzunluk `L`:

- **Doğrultu ve uzunluk:** `o + s·L·u`. Açı ve Sapma tek yönlüdür (`s = 1`); Paralel ve Dik iki yönlüdür, yönü imleç seçer (`s = işaret((c − o)·u)`, sıfırsa 1).
- **Yalnız doğrultu:** imlecin doğru üstündeki izdüşümü, `o + t·u`, `t = (c − o)·u`. Tek yönlü kilitte `t < 0` ise `t = 0` (nokta başvurudadır; araç sıfır uzunluklu kenarı almaz).
- **Yalnız uzunluk:** `o + L·(c − o)/|c − o|`; imleç başvurunun üstündeyse nokta yoktur. Orto ve kutupsal izleme açıksa doğrultuyu önce onlar seçer, uzunluk sonra uygulanır.
- **Kenet ve nesne izleme:** kenetlenen nokta da aynı kuralla izdüşer: “bu doğrultuda, şu köşenin hizasına kadar” (QGIS'teki gibi). Uzunluk kilidinde doğrultuyu kenetlenen nokta verir.
- Doğrultu kilidi varken Orto ve kutupsal izleme uygulanmaz.

Dik açılı çizimin (§4) örtük kilidi “önceki kenara dik, iki yönlü” bir doğrultu kilidi gibidir; elle verilen doğrultu kilidi ondan önce gelir.

### 3. Nesneye paralel ve dik

Komut seçilince oturum bir sonraki tıklamayı bekler; tıklama araca gitmez:

- seçme yarıçapındaki en yakın düz kenarın doğrultusu alınır (çizgi, çoklu çizgi ve alanın kenarı, yardımcı çizgi, ışın, ölçü çizgisi değil); yayda tıklanan yerdeki teğet;
- kenar yoksa “Tıklanan yerde düz kenar ya da yay yok.” denir, bekleme sürer; Esc vazgeçer;
- seçilen kenar önizlemede vurgulanır; paralel ya da dik doğrultu başvurudan geçen kesikli doğru olarak çizilir.

Paralel kenedinden (ADR 0163) farkı: kenet yumuşaktır (imleç doğrultuya yakınken), kilit serttir (nokta konana dek her yerde).

### 4. Dik açılı çizim

Durum çubuğunda Orto'nun yanında **Dik açı** (`drafting.rightAngle`, oturum ayarı, kapalı; komutu `draft.rightAngle`, şeritte Çizim yardımcıları):

- Açıkken aracın ikinci kenarından başlayarak her yeni kenar öncekine diktir (iki yönlü, yönü imleç seçer). İlk kenar serbesttir (Orto ve kutupsal izleme uygulanır).
- Kapalı yol araçlarında (Kapalı alan, Parsel oluştur, Alan hesapla, Bitişik alan) en az üç köşe varken **Dik kapat** seçeneği: son köşeden önceki kenara dik doğru ile ilk köşeden ilk kenara dik doğrunun kesişimi son köşe olur, şekil kapanır. Doğrular paralelse (kesişim yoksa) nedeni söylenir.
- Elle verilen doğrultu kilidi dik açının önüne geçer; uzunluk kilidi onunla birlikte durur.

### 5. Referans noktası ve yapım kipi

- **Referans noktası** (tek seferlik): sonraki tıklama ya da yazılan nokta köşe olmaz, başvuru olur. Kilitler ve göreli giriş (`@dY,dX`, `@mesafe<açı`, yalın mesafe) ondan ölçülür; aracın kenarı yine son köşeden başlar. Nokta konunca başvuru son köşeye döner.
- **Yapım kipi** (sürekli): açıkken her tıklama ve yazılan nokta başvuruyu yeniler; kapatılınca sonraki nokta köşedir. Komut bitince kapanır.
- Başvuru çizimde küçük çarpı ve “R” ile, kenet renginde gösterilir.

### 6. Arayüz

İki platformda aynı:

- **Değer kartı** (imleç yanında değer girişi, DESIGN.md §7.4.2): yalın bir sayıdan sonra **Tab** uzunluğu kilitler, alan boşalır ve kart açık kalır (AutoCAD'in dinamik girişi). **`<açı`** doğrultuyu kilitler (CBS'de semt): `<45`. Kartın altında etkin kilitler çip olarak durur (“Uzunluk 12.500 m”, “Açı 45°”, “Sapma 90°”, “Paralel”, “Dik”), her biri × ile kalkar.
- **İmleç yanında** kart kapalıyken kilit etiketi: “Kilit: 12.500 m · 45°” (nesne izlemenin etiketi gibi).
- **Önizleme:** kilitli doğrultu başvurudan geçen kesikli doğru, kilitli uzunluk başvurunun çevresinde kesikli yay; ikisi de kenet renginde.
- **Komut satırı:** istemin notlarında etkin kilitler; “Kilitleri kaldır” düğmesi.
- **Sağ tık komut menüsü:** “Tek seferlik kenet”in yanında **Kilit ▸**: Uzunluk…, Açı… (CBS'de Semt…), Sapma…, Nesneye paralel, Nesneye dik, Referans noktası, Yapım kipi (işaretli), Kalıcı (işaretli), Kilitleri kaldır. “…” olanlar değeri değer kartında, kendi adıyla sorar.
- **Komutlar:** `draft.lock.length`, `draft.lock.angle`, `draft.lock.deflection`, `draft.lock.parallel`, `draft.lock.perpendicular`, `draft.lock.reference`, `draft.lock.construction`, `draft.lock.keep`, `draft.lock.clear`, `draft.rightAngle`. Çalışan komutu durdurmazlar (nokta hesaplayıcı gibi); komut çalışmıyorken ya da çalışan adım nokta beklemiyorken devre dışıdır.

### 7. Ortak çekirdek

`tools::locks` (geometri çekirdeği), web WASM'dan, masaüstü yerli çağırır:

- `lock_point(o, c, length, direction) -> Option<Vec2>`: §2'nin kuralı;
- `deflected(prev, from, angle, angles) -> Option<Vec2>`: Sapma'nın birim doğrultusu, türün düzeni ve açı biriminde;
- `direction_of(angle, angles) -> Vec2`: Açı kilidinin birim doğrultusu;
- `perpendicular(u) -> Vec2`;
- `square_corner(first, second, prev, last) -> Option<Vec2>`: Dik kapat'ın köşesi;
- yazılan kilit metni (`<açı`) `tools::point_text`'in dilbilgisine girer (`parse_lock_text`).

Bağımsız başvuru `scripts/fixtures/lock_cases.py` (`fixtures/locks/v1/cases.json`), KentOS kodu olmadan: kesin kesirler (doğrultu kesirli vektörle verilince) ve 50 basamaklı `mpmath` (açıyla verilince); tek ve iki yönlü kilitler, izdüşüm, başvurunun arkası, imleç başvuruda, kenet noktası, sapma (iki türde, derece ve grad), dik kapatma (paralel doğrular dahil), büyük koordinatlar.

### 8. Kapsam dışı

- **Koordinat kilitleri** (QGIS x ve y): ayrı karar.
- **Eğim kilidi** (Netcad Eğim) ve açıortaya göre sapma: ayrı karar.
- **Canlı alanlar** (AutoCAD'in imleçle değişen uzunluk ve açı kutuları, QGIS floater): değer kartı yazınca açılmaya devam eder.
- **Değiştirme araçlarında** (Taşı, Döndür …) kilitler: onların yazılan değerleri zaten tek seferlik kilittir; ayrı karar.

### 9. İş sırası

1. Çekirdek: kilitli nokta, açı ve sapma doğrultuları, Dik kapat'ın köşesi, kilit metninin dilbilgisi; WASM; bağımsız başvuru ve ortak durumlar.

   *(4 Ekim: tamam.)*
   - **`tools::locks`:** `lock_point` (§2), `constrain_locked` (imlecin kuralı kilitlerle: doğrultu kilidi Orto'yu ve kutupsal izlemeyi dışarıda bırakır, yalnız uzunluk onlardan sonra gelir; başvuru yokken kilit yoktur), `direction_of`, `deflected`, `perpendicular`, `unit`, `square_corner`, `parse_lock_text` (`<açı`; sayı noktanın dilbilgisindeki gibi, ondalık virgülsüz: `point_text::parse_plain_number`).
   - **WASM:** `lockPoint`, `constrainLocked`, `lockDirection`, `lockDeflected`, `squareCorner`; web `tools/locks.ts` kilit metnini kendi düzenli ifadesiyle okur (`coordinateInput.ts`'in nokta metnini okuduğu gibi).
   - **Başvuru:** `scripts/fixtures/lock_cases.py` (`fixtures/locks/v1/cases.json`): 15 kilitli nokta, 11 imleç kuralı (Orto, kutupsal izleme, kenet noktası, TM koordinatları), 10 doğrultu (iki tür, derece ve grad), 9 sapma, 8 Dik kapat (paralel ve boyu olmayan kenarlar), 20 kilit metni. Çekirdek (yerli ve çağrı tablosu) ve web (WASM) aynı: noktalar 1e-8 m, doğrultular 1e-14 içinde (bir TM kuzey değerinin double adımı 0,93 nm'dir).
2. Uzunluk, Açı ve Sapma kilitleri iki platformda: oturumun kilitleri, imlecin kısıtı (nokta veren bütün araçlar), Tab ve `<açı`, değer kartının çipleri, imleç yanındaki etiket, önizleme, Esc, sağ tık Kilit ▸ menüsü, komut satırı, komutlar; ortak iz `locks.json`; resimler.

   *(4 Ekim: tamam.)*
   - **Oturum:** kilitler oturumun durumudur, kaydedilmez (masaüstü `kentos_interaction::LockState` ve `Session::lock_length`, `lock_toward`, `keep_locks`, `clear_locks`; web `tools/locks.ts` ve `ctx.settings.locks`): uzunluk, doğrultu (Açı ya da Semt, Sapma), Kalıcı ve yapıldıkları başvuru. Başvuru değişince (nokta kondu, adım geri alındı) tek seferlik kilitler gider, kalıcılar onu izler; yeni komut hiçbir şeyle başlamaz. Sapma aracın son doğrultusundan ölçülür (`Tool::travel`, web `travelDirection`: Çizgi zincirinin son kenarı, yol araçlarının teğeti, yay kenarında da; Mesafe ölç'ün sabit ilk noktasında yoktur).
   - **İmleç:** nokta veren bütün araçlarda imlecin kuralı (`points::constrain`, web `constrainPoint`) kilit varken `constrain_locked`'dır; kilit yokken değişmez.
   - **Değer kartı:** yalın sayıdan sonra Tab uzunluğu kilitler, kart doğrultuyu sorar (solunda adı: “Açı”, CBS'de “Semt”); kilit alanında Tab yazılanı kilitler ve öbür değere geçer. Enter yazılanı kilitler, uzunluk ve doğrultu ikisi de kilitliyse noktayı koyar; boş Enter ya da Boşluk kilitlerin tuttuğu noktayı koyar, kilitli olmayanı imleç verir (AutoCAD'in dinamik girişindeki gibi). Kilitler kartın altında kenet renginde çip, her biri ×'iyle. `<açı` kartta da komut satırında da doğrultuyu kilitler; `<` çizim alanında kartı açar (`looks_like_coordinate`). ADR 0018'in “Tab yazılan değeri kaybetmez” kuralı sürer: değer kilit olur (`polygon-keys` izinin Tab adımı).
   - **Çizimde:** kilitli doğrultu başvurudan geçen kesikli doğru (tek yönlüyse ışın), kilitli uzunluk başvurunun çevresinde kesikli daire, ikisi kenet renginde; kart kapalıyken imlecin sol üstünde “Kilit: Uzunluk 12.000 m · Açı 45.0000°” (kart sağ üstte, aracın ölçüsü sağ altta).
   - **Esc** önce kilitleri kaldırır (“Kilitler kaldırıldı.”). Komut satırında istemin yanında “Kilit: …” ve ×'i (Kilitleri kaldır).
   - **Komutlar ve menü:** `draft.lock.length`, `draft.lock.angle`, `draft.lock.deflection`, `draft.lock.keep`, `draft.lock.clear`, ikonlarıyla; sağ tık komut menüsünde Kilit ▸: Uzunluk…, Açı… (CBS'de Semt…), Sapma…, Kalıcı (işaretli), Kilitleri kaldır. Başvuru yokken (ilk nokta verilmeden) kilitler, önceki kenar yokken Sapma devre dışıdır; yazılan `<açı` nedenini söyler.
   - **Ortak iz** `fixtures/interaction/v1/locks.json`: Tab'la uzunluk, `<100` (CBS semti), kilitli doğrultuda yazılan mesafe, Sapma, Kalıcı, Esc; iki platform ve izlerin gözlemi kilitlerin sözleriyle (`locks`).
3. Nesneye paralel ve dik; Dik açılı çizim (durum çubuğu, şerit, ayar) ve Dik kapat; ortak izler; resimler.
4. Referans noktası ve yapım kipi; ortak iz; resimler.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- KentOS'un nokta girişi Netcad'in Çizim Hesap Araçları'nı, AutoCAD'in dinamik giriş kilitlerini, ArcGIS'in kısıtlarını ve QGIS'in gelişmiş sayısallaştırmasını karşılar.
- Kilitler bütün nokta araçlarına ortak kuraldan gelir; araç başına kod yoktur (Sapma ve Dik kapat önceki kenarı bilen araçlarda).
- Kilit yokken bugünkü davranış değişmez.

## Doğrulama

- **Çekirdek:** bağımsız başvuruya göre iki platformda.
- **Arayüz:** ortak izlerle iki platformda; resimler iki temada, 1440×900 ve 1100×650.
