# ADR 0349: Bağlı dosyaların durumu, arama yolları ve değişiklik bildirimi

- Durum: Kabul edildi (19 Ekim 2026)
- Kapsam: plan/cad.md `CAD-REF-08`'in kalanı (arama yolları), `CAD-REF-09` (bildirim ve etkilenen görünüm ve paftalar), `CAD-REF-19`
  (kaynağın durumu her yüzde aynı yapılı sonuç); CAD-REF'in ikinci grubu (8b). Kırpma (`CAD-REF-12`) resim altlığıyla (`16`) 8c'dedir.
- İlgili: ADR 0348 (dış başvurular), ADR 0506 (göreli yol ve Paketle), ADR 0347 (pafta seti ve etkilenen paftalar), ADR 0285 (adlı
  görünümler), ADR 0327 (bağlı tablo dosyası), ADR 0503 (bağlı katman).

## Bağlam

Dış başvurunun kaynağı bulunamadığında ya da değiştiğinde ADR 0348 bunu yalnız `cad.xrefs.list`'te ve açılışta söylüyordu. Kaynak çizim
başka bir klasöre taşınınca kullanıcının elinde yolu tek tek değiştirmekten başka çare yoktu. Çizim açıkken kaynağın değiştiğini
bilmenin yolu yoktu; değişikliğin hangi paftaları ve görünümleri etkilediği söylenmiyordu. Bağlı resim, raster, nokta bulutu, tablo,
ek ve GeoPackage kaynağının durumunu soran ortak bir yol da yoktu: Python ve yapay zekâ ajanı bunu dosya sistemine kendisi bakarak
çıkarmak zorundaydı.

## Karar

### 1. Arama yolları (`CAD-REF-08`)

Projenin `xrefPaths`'i (`.kcad` şema 84, `FORMATS_VERSION` 91): dış başvuruların kaynaklarının arandığı klasörler, sırasıyla; mutlak
ya da projenin dosyasının klasöründen göreli (`/` ile). Bir dış başvurunun kaynağı şu sırayla aranır, ilk bulunan alınır:

1. kayıtlı yol (`path`), mutlaksa;
2. göreli yol (`relative`), açık çizimin klasöründen;
3. her arama yolunda önce göreli yol, sonra dosyanın adı.

Bulunamazsa denenen yollar sırasıyla söylenir (`tried`). Arama yolları `cad.xrefs.edit`'in `paths` işlemiyle yazılır (ayar, geri alma
adımı değil; `cad.settings.set` gibi). Bulunan yer kayıtlı yolu kendiliğinden değiştirmez: Yeniden yükle bulunan dosyayı okur ve
kayıtlı yolu ona yazar (bir adım). Resim, raster, nokta bulutu, tablo ve ekler eskisi gibi kayıtlı yolla ve çizimin klasöründen bulunur;
arama yolları onlara 8c'de (`CAD-REF-16`) uygulanır. Bulut projesinin sürümüne bağlı kaynak (`CAD-REF-05`'in proje varlığı) ve bulut
proje varlığı yolu sonraki işlerdendir.

### 2. Bağlı dosyaların durumu (`CAD-REF-19`)

Salt okunur `cad.assets.check`: çizimin bütün bağlı dosyaları tek listede, her biri:

- `kind`: `xref`, `image`, `raster`, `pointCloud`, `table`, `attachment`, `source`;
- kimin: nesnenin kalıcı kimliği (`uid`), blok tanımı (`block`), bağlı katmanın kaynağı (`source`);
- `name` (gösterilen ad: dış başvurunun adı, dosyanın adı), kayıtlı `path`, `relative`;
- `found` (bulunduğu yer) ve `via`: `path`, `relative`, `searchPath`, yoksa `none`; adres (`address`) ve veritabanı (`database`)
  bakılmaz;
- `state`: `ok`, `changed` (kayıtlı özet var ve dosyanınki başka: dış başvurunun `digest`'i, bağlı tablonun `digest`'i, bağlı katmanın
  `revision`'ı), `missing`, `unloaded` (boşaltılmış dış başvuru), `remote` (adres), `database`;
- `digest` (kayıtlı) ve `current` (değişmişse dosyanınki), `tried` (bulunamadıysa denenenler).

Girdi: `drawing` (açık çizimin dosyası), `kinds` (yalnız bu türler), `digests` (yanlışsa özet hesaplanmaz, `changed` söylenmez; büyük
dosyalarda hızlı bakış). Cevapta türlere göre sayılar. Masaüstünün açılış uyarısı, Dış başvurular penceresi, Python ve MCP aynı cevabı
okur.

### 3. Değişiklik bildirimi ve etkilenenler (`CAD-REF-09`)

`cad.xrefs.list`'in `affected` girdisi: her dış başvurunun yerleştirmelerinin (iç içe olanın getireninkiler) etkilediği paftalar
(`sheets`: harita çerçevesi yerleştirmeyi gösteren paftaların adları, ADR 0347'nin `shows` kuralıyla) ve adlı görünümler (`views`:
penceresi yerleştirmenin kutusuyla kesişen görünümlerin adları). Masaüstü açık çizimin yüklü dış başvurularının dosyalarına beş saniyede
bir bakar: önce dosyanın boyu ve değişme zamanı, değiştiyse özeti arka planda hesaplanır. Kaynak değişince çizimin üstünde bir kart:
“Plan dış başvurusunun kaynağı değişti; 2 paftayı ve 1 görünümü etkiler.”, Yeniden yükle, Dış başvurular ve Kapat. Çizim kendiliğinden
yeniden yüklenmez, kaydedilmez; kapatılan kart dosya yeniden değişince yeniden gelir.

### 4. Arayüz

Dış başvurular penceresinin üç bölümü (üstte bölüm seçici, pencere 1100 × 650'ye sığsın diye): Dış başvurular (satırda bulunduğu yer
`via`'yla, “arama yolundan bulundu”; bulunamayanda denenen yollar), Arama yolları (Klasör ekle…, her satırda kaldır) ve Diğer bağlı
dosyalar (resim, raster, nokta bulutu, tablo, ek ve kaynakların durumları, salt okunur; resim, tablo ve eklerin özeti karşılaştırılır,
büyük olabilen bağlı katman kaynağı yalnız aranır). Açılışta dış başvuruların özetleri karşılaştırılır, öbür bağlı dosyalar yalnız
aranır; değişen ve bulunamayan durum çubuğunda ve günlükte söylenir.

### 5. Yeniden yüklemede iç bloklar

Dış başvurunun daha önce getirdiği iç blok, aynı adla yeniden gelince kimliğini korur (ADR 0348'in iç blok kimliği kuralının eki):
başka bir yazılımın ya da eski bir sürümün yazdığı kimlikler de yeniden yüklemede çakışmaz, yerleştirmeler ve bağlar iç blokla kalır.

## Sonuçlar

- Proje başka bir makinede kaynakları ayrı bir klasörde bulsa da arama yoluyla açılır; denenen yollar ne arandığını söyler.
- Kaynağın değişmesi açık çizimde görülür; neyi etkilediği bilinir, kullanıcının işi kendiliğinden değişmez.
- Bağlı dosyaların durumu arayüzde, Python'da ve yapay zekâ ajanında aynı yapıyla okunur.

## Doğrulama

- Ortak durumlar: `python3 scripts/fixtures/assets_check_cases.py --check` (`cad.assets.check`, 30 durum: türler, göreli yol,
  arama yolları, özet, sıra, adres ve veritabanı) ve `python3 scripts/fixtures/xref_command_cases.py --check` (`cad.xrefs.edit` 47,
  `cad.xrefs.list` 27: arama yolları, `paths`, denenen yollar, `affected` ile adlı görünümler; paftaların etkilenmesi `cad.sheets.set`'in
  `shows` kuralıdır, onun durumlarıyla).
- `.kcad` şema 84: kodek, bağımsız okuyucu ve yazıcı, örnek ve bozuk dosyalar.
- Resimler: `KENTOS_SHOTS_ONLY=dis-basvuru-bildirim,dis-basvuru-yollar,bagli-dosyalar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (bildirim kartı, kaynağın dosyası değiştikten sonra izleyicinin bir bakışıyla).
