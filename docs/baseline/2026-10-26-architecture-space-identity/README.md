# Numarasız mahal denetimi — 26 Ekim 2026

Bu denetim diliminin kabulü tamamlandı; [ADR 0689](../../adr/0689-room-number-review.md).
ARCH-SPACE-02'nin denetim dilimi; kat öneki/ad/kullanım listesi bu teslimle
tamamlanmış sayılmaz. Yeni komut veya KCAD alanı yoktur; mevcut
`cad.tags.check` isteğe bağlı eksik numara kontrolü alır.

## Anlam ve uyumluluk

Mevcut “Etiketleri denetle” penceresi numarasız mahalleri açık ve görünür
seçenekle listeler. Gizli/kilitli katmanlar ve başka katlar sayı denetiminin
içindedir; yalnız açık `layers` listesi sınırlar. Bulgu kaynak UUID'sini,
adını, katını ve mevcut gösterme noktasını taşır. Kat yoksa “Kat atanmamış”
yazar. Sorgu düzeltme yapmaz, nesne eklemez ve undo adımı üretmez.

Python/MCP için `missingSpaceNumbers` yok/null/false eski sonuçları ve sırayı
aynen korur. True yalnız eksik numaraları ekler. Tam metin numaralar sayıya
çevrilmez; Z-001/Z-1/z-001/sonunda boşluk ayrıdır. Eski yinelenen grup
sırası korunarak grup araması bir indekse taşındı. Belgeye indeks/cache
kaydedilmez. KCAD, birim, ölçek, geometri ve renderer değişmedi.

Yeni TS/TSX dosyası veya web arayüz uygulaması yoktur. Mevcut üretilmiş
`TagsCheck.ts`, `TagFinding.ts` ve komut kataloğu ortak sözleşmeye uyarlanır; toplu TS üretim
çıktısı ignore edilen `.run/architecture-contours/contracts-ts` içine gider.
Web görünür iş akışı ADR 0228 gereği TODOS.md §0.2'de ertelenmiştir.

## Otomatik kapılar

`test-runs.json` gerçek komut/çıkış/zamanı ve ham günlük hash'lerini tutar.
Erken başarısız deneme olumlu kabul sayılmaz:

- Kurulumun ilk denemesinde boş metinle numara verilmesi mevcut mahal
  doğrulayıcısınca reddedildi. Ürünün bu kuralı korunur; numarasız alan
  kanonik olarak yoktur. Boşaltma mevcut edit komutunda null kullanır.
- `python-red`: eski üretilmiş SDK yeni anahtar argümanını tanımadı.
- `cases-first`: önceki 21 check vakası ve ortak kurulum aynen korunarak
  sekiz yeni bağımsız kurulum geçti. **233 ortak durum** (107 create/26 edit/
  29 check/36 schedule create/35 schedule edit). Yeni vakalar dokuz mahalı
  (üç numarasız, gizli/kilitli, tam metin ayrımları) etiketledikten sonra
  yok/null/false/true, validate/plan/execute ve sekiz katman süzgecini sınar.
- `python-build-final` ve `python-focused`: güncel yerel host/SDK ile beş
  test geçti. Tam metin ayrımı, katmanlar, plan/validate/execute, model ve
  geçmiş korunumu; gerçek KCAD kayıt-aç, birim/ölçek değişimi, açık onarım,
  boş metnin atomik reddi/null boşaltma ve tek undo/redo denetlenir.
- `python-all`: **323 test** geçti. Önceki sqlite test yardımcılarının açık
  bağlantı ResourceWarning çıktıları ham günlükte korunmuştur; bu dilimde
  çözülmüş sayılmaz.
- `native-final`: **68 domain / 697 interaction / 219 application** ana
  takımı geçti. Diğer test ikilileriyle birlikte ayrıntı günlükte bulunur.
  Interaction'da bir, application ana takımında 22 mevcut atlanan test
  başarı sayılmaz. Önceki duvar/mahal/açıklık/etiket ve ortak 233 durum dahildir.
- `mcp`: **üç gerçek Server testi** geçti. Yeni denetimin plan/execute tam
  sonuç eşitliği ve salt okunurluğu, mevcut düzenleme/tek undo; önceki
  numaralandırma ve net alan etiket akışı da çalıştırıldı.
- `desktop`: **iki test** geçti; açılışta numarasızların dahil olması,
  seçenek/yenile/kapat salt okunurluğu, geç gelen olayın kapalı pencereyi
  açmaması ve güncel kaynak alanının yeniden okunması.
- `catalog`: üç test; bağımsız referans ve SDK üretim sapması kontrolleri,
  `kcad-reference`: önceki **1170** dosya; masaüstü ikilisi derlemesi geçti.
- `contracts`: **809 commands / 999 contracts** testine ek 32 document/10
  settings testi geçti; sayılar otomatik TS dışa aktarma testlerini de içerir.
  Yalnız bu dilimin değişen iki mevcut tip çıktısı geri alındı, sondaki
  anlamsız boşluklar temizlendi. Yeni TS dosyası eklenmedi.
- `clippy`: contracts/commands/domain/application/interaction/MCP/Python/
  desktop, bütün hedefler `-D warnings` geçti. Mevcut ts-rs serde uyarıları
  ve `block 0.1.6` gelecek uyumluluk bildirimi ham günlükte korunur.
- `contract-audit.json`: **267 diğer komut** aynen korundu; yeni komut yok.
  Yalnız isteğe bağlı boolean/null girdi, yeni bulgu kodu ve açıklamalar
  değişti; mevcut çıktı/plan yapıları ve bütün diğer katalog alanları aynı.
- `wasm-contracts`: contracts/no-default-features WASM kontrolü geçti;
  `dependencies`: 47 crate / 61 crate×hedef bağımlılık yönü temiz.

## Gerçek pencere ve dosya karşılaştırması

`before.kcad` beş mahal/beş etiket içerir: üç numarasız, iki Z-001; ikinci
yinelenen gizli başka kattadır. `seed.json` UUID beklentilerini tutar.
Ham JPEG'ler kırpılmaz, yeniden boyanmaz veya üretilmez; `gui-events.json`
zaman/hash ve değişmez eski/yeni test paketini eşler. Paketler debug'dır;
`after-binary.json` kaynak hash'lerini de tutar. Kullanıcının çizimleri ve
asıl profili kullanılmaz; testin kendi config/data alanı vardır.

- 01 eski pencere yalnız yinelenen grubu gösterir. 02 yeni pencerede üç
  numarasız mahal ve yinelenen grup vardır; ad/kat bilgileri okunur.
- 03–04 seçenek kapatılınca eski tek gruba dönülür; Yeniden denetle kapalı
  tercihi korur. 05 Ana giriş için Göster/Kapat kaynak mahalı yakınlaştırır.
  Önceden var olan Göster davranışı korunur: kamera modalın arkasında
  değişir; sahneyi görmek için Kapat gerekir.
- 06–09 özellikte N-001 tek Enter ile yazılır; etiket aynı UUID'yi izler,
  yeniden denetimde eksik sayı 3→2 olur. Denetimden sonra tek undo ilk tam
  belgeyi, tek redo N-001'i getirir.
- 10–13 numarayı boşalt/tek Enter ile eksik sayı yeniden 3; araya giren
  sorguya rağmen tek undo N-001'i geri getirir. N-999 taslağı Escape ile
  iptal edilince kayıt N-001'de kalır.
- 14–15 açık 18 dar pencerede seçenek/bulgular/eylemler sığar; kat atanmamış
  Mutfak için Göster/Kapat çalışır. 16–17 koyu 13 geniş/dar görünümde aynı
  sonuç ve alt düğmeler erişilebilirdir. Bu işlemler dosyaya yazmaz.
- 18 uygulama gerçekten kapatılıp yeni süreçte açıldığında N-001 ve iki
  eksik numara korunur; tekrar Kaydet gerçek dosyası da karşılaştırılır.

`verify.py` bağımsız KCAD okuyucusuyla **12 tam gerçek GUI kaydını** denetler.
Beş 4×3 m dikdörtgenin alanı ayrı shoelace hesabıyla 12 m², ankrajları merkezdir.
On kalıcı UUID, her geometri, kat/katman/ayar ve etiket yeri kesin eşitlikle
korunur. Yalnız ilk kaynak `number` ve bağlı etiket `text` alanları değişir.
Bütün kayıtlar beklenen iki özgün dosyayla bayt bayt da aynıdır. **18 ham
ekran**, kaynak/ikili/günlük hash'leri ve gerçek görünüş profilleri doğrulandı.

`image-dimensions.json` ham JPEG boylarını içerir (2× Retina):

| Gerçek profil | Geniş | Dar |
| --- | --- | --- |
| Açık 18 | 2880×1720 px =1440×860 mantıksal | 2200×1364 px =1100×682 |
| Koyu 13 | 2880×1720 px =1440×860 mantıksal | 2200×1364 px =1100×682 |

İstenen 1440×900/1100×650 ile gözlenen çerçeve eşit sayılmaz. Bu dört
görünüm, bütün mimari araçların sekizli kabul matrisini kapatmaz. Test
kurulumunda ayrı etiketler aynı mahal yazısının üzerine konmuştur; bu
çakışma eski pencerede de vardır ve yeni render davranışı değildir.

## Yoğun belge ve kalan işler

`measure.py` herkese açık Python plan/execute sorgusunu ölçer; her çağrıda
tüm UUID/sıra/noktalar, eksik mesajları ve tam model korunumu denetlenir.
Apple M5/32 GiB, debug yerel Python SDK; `environment.json`. Ölçümde test
penceresi, derleme ve otomatik takım çalışmıyordu. Kurulum/kayıt/model
karşılaştırma süreleri tabloya dahil değildir. Bütün mahaller etiketsizdir;
eski etiketsiz bulguları da sorgu maliyetindedir. Üç plan + üç execute her
belgede ayrı ölçüldü; tüm bulguların sıra/UUID/noktası, eksik mesajları,
eski varsayılan sonuçlar ve bütün model her çağrıda denetlendi.

| Belge | Eksik / toplam bulgu | Plan ortancası | Execute ortancası | Altı sorgunun en uzunu |
| --- | ---: | ---: | ---: | ---: |
| 10.000 mahal+20.000 çizgi | 3 /10.004 | 254,099ms | 241,663ms | 260,846ms |
| Aynısı | 9.998 /19.999 | 456,076ms | 452,363ms | 467,202ms |

Ham örnekler `timing-10000-20000-{3,9998}.json` dosyalarındadır. Önceki
sürümle hızlanma karşılaştırması veya release/p95/bellek/GPU/kare süresi
kabulü değildir; GUI'nin binlerce satırını render etme ölçümü sayılmaz.

Kat öneki/ad/kullanım proje listeleri, eski etiket Koru/Yenile yönetimi ve
tam mimari matris açık kalır. ARCH-SPACE-02'nin tamamı bitmiş sayılmaz.
