# ADR 0345: Görünüm işaretleri, başvurular ve mahalin kaplamaları

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §14 `CAD-ARCH-37`, `38`, `40` ve `CAD-ARCH-39`'un kalanı
  (döşeme deseni, tavan ızgarası, mobilya); `CAD-ARCH-01`'in işaret kısmı (sahibin sözü: “Devam arch ile”; yedi grubun altıncısının
  ikinci yarısı, 6b). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0285 (adlı görünümler), 0251 ve 0252 (pafta kitabı), 0342 (mahal), 0344 (etiket), 0328 ve 0329 (tarama
  sınırı ve bağı), 0318 (KentOS blok kitaplığı), 0228 (web'e ara), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Mimari çizimde planın üstündeki kesit, görünüş ve detay işaretleri başka bir çizime işaret eder: işaretin balonunun üstünde
detayın numarası, altında onun durduğu paftanın numarası yazar. Pafta yeniden numaralanınca ya da görünüm yeniden adlandırılınca
elle yazılmış balon eskir. İşaret hedefini kalıcı bir başvuruyla taşımalı, yazısını hedeften almalıdır.

Adlı görünümlerin (ADR 0285) kimliği adlarıdır; yeniden adlandırma başvuruyu koparırdı. Pafta kitabı belgede opak durur ve geri alma
adımı değildir (ADR 0251); adlı görünümler de proje ayarıdır. Bu yüzden işaretin yazısı, etiketinkinden farklı olarak, bir adımın
izleyicisinin yanında ayarlar, kitap, geri alma ve yineleme değiştiğinde de yenilenmelidir.

## Karar

### 1. Görünüm işareti (`CAD-ARCH-37`)

Yeni tür `viewmark` (`.kcad` şema 81, `FORMATS_VERSION` 88, sütunlu kayıtta tür 29, deponun paketinde tür 36, sunucuda göç 0028):

- `form`: `section` (kesit), `elevation` (görünüş), `detail` (detay).
- Kesit: `points` iki ya da daha çok nokta (kesme çizgisi, kırıklarıyla), `flip` bakış yönünü çevirir (yoksa çizginin solu).
  Çizim: kesme çizgisinin uçlarında balonlar, her balonun bakış yönünde dolu üçgeni; aradaki çizgi uçlarda kalın kısa parçalar.
- Görünüş: `points` tek nokta (balonun ortası), `angle` bakış yönü (derece, doğudan saat yönü tersine). Çizim: balon ve bakış
  yönünde dolu üçgen.
- Detay: `points` tek nokta (sınırın ortası), `radius` sınırın yarıçapı, `at` balonun yeri. Çizim: kesikli sınır çemberi, balon
  ve sınırdan balona kol.
- Balon yatay çizgiyle ikiye bölünür: üstte `mark` (işaretin numarası: “1”, “A”), altta hedefin numarası. `size`: yazının kâğıttaki
  yüksekliği (mm, yoksa 2.5); balonun çapı yazıdan (`geom::viewmark`).
- `target`: `{ view: id }` (adlı görünüm) ya da `{ sheet: id, item: id }` (paftadaki görünüm penceresi). Yoksa alt yarı “—”.
- Belgenin yazdığı: `words` (`top`, `bottom`; nesnenin kendi `label`'ıyla karışmasın diye bu ad): `top` `mark`'tır; `mark` boşsa paftadaki pencerenin o paftadaki pencereler
  arasındaki sırası (1'den). `bottom` paftanın adı ya da adlı görünümün adı. Hedef yoksa `broken: targetGone`, yazı kalır.

### 2. Başvuruların yenilenmesi (`CAD-ARCH-38`)

- Adlı görünümün kararlı kimliği (şema 81): görünümün kendi `id`'si, yoksa adından türeyen (`v-` ve adın FNV-1a'sının 8
  onaltılığı; `view_key`). Yeniden adlandırma eski adın kimliğini `id` olarak yazar, aynı adla Kaydet kimliği korur; adının
  kimliğini yeniden adlandırılmış başka bir görünüm taşıyan yeni görünüm sıradaki boş kimliği alır (`-2`, `-3` …). Kimliksiz
  eski görünüm kimliğini adından alır, dosyaya yazılmaz.
- Belge (`kentos_domain::viewmark_follow`) işaretlerin yazısını ve kopukluğunu hedeflerinden yeniden yazar: işaretlere dokunan
  her adımın sonunda (adımın içinde) ve adım dışında, adlı görünümler ya da pafta kitabı değişince, geri almada ve yinelemede
  (yerinde, geri alma adımı olmadan: yazı türetilmiş bir değerdir). Pafta kitabının okunması `viewmark_follow::sheet_frames`:
  kitabın JSON'undan paftaların kimliği ve adı, harita çerçevelerinin kimliği, adı ve sırası.

### 3. Komutlar (`CAD-ARCH-37`, `40`)

- `cad.viewmarks.create` (“Görünüm işareti”): `marks` (her biri `form`, `points`, `radius`, `at`, `angle`, `flip`, `mark`,
  `target`, `size`), katman. Retler: `invalid_viewmark` (biçime göre nokta sayısı, yarıçap, yazı), `target_not_found`.
- `cad.viewmarks.edit` (“Görünüm işaretini düzenle”): `set` (`mark`, `size`, `flip`, `angle`, `radius`), `retarget` (`target`),
  `detach` (hedefi bırakır). Kilitli katman reddeder; hepsi ya da hiçbiri.
- Salt okunur `cad.viewmarks.references` (“Başvurular”): her işaret (kimlik, biçim, `mark`, hedef, yazı, kopukluk) ve her hedef
  (adlı görünüm ya da paftadaki pencere: adı, numarası, ona bakan işaretler). Plan–detay başvuruları buradan yönetilir (`CAD-ARCH-40`).
- Python `kentos.cad.viewmarks`, MCP; `cad.viewmarks.edit` yıkıcı listede.

### 4. Mahalin kaplamaları (`CAD-ARCH-39`)

- `cad.spaces.finish` (“Döşeme ve tavan”): `uids` (mahaller), `surface` (`floor`, `ceiling`), `pattern` (kitaplığın ya da projenin
  desen adı; yoksa döşemede mahalin döşeme kaplamasının adıyla aynı adlı desen, o da yoksa `ANSI31`) ya da `grid` (`dx`, `dy`, `angle`,
  `origin`: `center` ya da `corner`; tavan ızgarası iki çizgi ailesiyle kullanıcı deseni), katman (yoksa `A-ZEMN-DSN` ya da
  `A-TAVN-IZG`). Her mahale mahalin sınırına bağlı tek tarama yazılır, tek adımda; mahal duvarlarla değişince tarama izler (bağın
  sınırı mahal: tarama izleyicisi mahalin halkalarını okur). Aynı yüzeyin eski taraması varsa yerine geçer (`replaced`).
- Mobilya ve donanım KentOS blok kitaplığıyla (ADR 0318: masa, sandalye, kanepe, yatak, dolap, lavabo, klozet, duş, küvet): Mimari
  sekmesinde Donatı panelinde kitaplığı alma ve Blok ekle.

### 5. Masaüstü

- Mimari › Görünüm işaretleri: Kesit (iki ya da daha çok nokta, Enter; Çevir (Ç), Numara (N)), Görünüş (nokta ve bakış yönü),
  Detay (merkez, yarıçap, balonun yeri), Başvurular penceresi (işaretler ve hedefler; Göster, Hedefe git, Yeniden bağla, Bağı kopar,
  Görünüm oluştur: detayın sınırından adlı görünüm ve işareti ona bağlar). Hedef araçların Hedef (H) seçeneğiyle adlı görünümlerden ya
  da paftaların pencerelerinden seçilir.
- Mimari › Mahal: Döşeme deseni, Tavan ızgarası (mahallere tıklanır; Desen (D), Aralık (A)). Donatı paneli: KentOS blok kitaplığı,
  Blok ekle.
- Öznitelikler'de işaretin satırları (Biçim, Numara, Hedef, Yazısı, Yazı yüksekliği, Durum); seçim süzgecinde Görünüm işareti; kopuk
  işaretin uyarı üçgeni.

## Sonuçlar

- İşaretin yazısı hedeften türer: paftayı yeniden adlandırmak ya da görünümü yeniden adlandırmak işaretleri aynı anda değiştirir;
  bu değişiklik geri alma adımı değildir (kitap ve görünümler de değildir).
- DXF'e işaret düz parçalarla (balon, çizgiler, yazılar) gider; hedef KENTOS verisinde.
- Kot işaretleri ADR 0322'nin kot ölçüsüdür; bu ADR yeni tür eklemez.

## Doğrulama

- Ortak durumlar: `python3 scripts/fixtures/viewmark_command_cases.py --check` (cad.viewmarks.create, .edit, .references,
  cad.spaces.finish; KentOS kodu olmadan, en az 20'şer durum).
- Çekirdek: `python3 scripts/fixtures/viewmark_cases.py --check` ve `cargo test -p kentos-geometry-core --test all viewmarks_draw`.
- `.kcad` şema 81: `fixtures/kcad/v2/viewmarks.json` ve bozuk örnekleri (`python3 scripts/fixtures/kcad_v2_reference.py --check`).
- Resimler: `KENTOS_SHOTS_ONLY=isaret-serit,isaret-plan,isaret-kesit,isaret-detay,isaret-basvurular,isaret-yeniden-adlandir,mahal-doseme,mahal-tavan
  cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
