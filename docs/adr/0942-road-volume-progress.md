# 0942 — Dönem metrajı: ilk arazi, ara ölçü ve tasarım arasındaki iş

Tarih: 2026-11-04. Durum: kabul (Y36.1 çekirdek, Y36.2 komut, Y36.3 pencere). Kapsam ROAD-VOL-15.

## Sorun

Kübaj tasarımın ilk araziye göre kazısını ve dolgusunu verir. Şantiyede ise her ay bir ara ölçü (drone, GNSS, total
station) alınır ve aynı sorular sorulur:

- Bugüne kadar ne kadar kazıldı, ne kadar dolduruldu?
- Ne kadar kaldı?
- Tasarımın altına inilmiş (fazla kazı) ya da üstüne çıkılmış (fazla dolgu) yer var mı?
- Bu ay, geçen ayın ölçüsüne göre ne kadar iş yapıldı?

ROAD-VOL-15 şunu ister: "Alternative/as-built/period quantity kaynak digests ve önceki approved run'a referansla
karşılaştırılsın."

## Dayanak

- **Trimble WorksOS** ve benzeri saha yazılımları ilerlemeyi üç yüzeyle verir: ilk arazi, son ölçü ve tasarım. Son
  ölçü ile ilk arazi arasındaki hacim yapılan işi, son ölçü ile tasarım arasındaki hacim kalan işi söyler.
- **Civil 3D** aynı soruyu iki yüzey arası hacim yüzeyleriyle (TIN volume surface) ikişer ikişer cevaplar; üçünü
  birlikte yorumlamak kullanıcıya kalır.
- **Hakediş:** ara hakediş kümülatif metrajdan önceki hakedişin kümülatifi çıkarılarak bulunur. Dönemin işi iki ölçü
  arasındaki fark değil, iki kümülatif arasındaki farktır (§3).
- **Fazla kazının ödenip ödenmemesi** sözleşmeye bağlıdır. KentOS onu ayrı bir sayı olarak raporlar, karar vermez.

## Karar

### §1 Her noktada üç miktar

z_o ilk arazi, z_n ara ölçü, z_d tasarım olsun.

**Yarmada (z_o > z_d):**

- yapılan `max(0, z_o − max(z_n, z_d))`;
- kalan `max(0, min(z_n, z_o) − z_d)`;
- fazla `max(0, z_d − z_n)` (tasarımın altına inilmiş).

**Dolguda (z_d > z_o):**

- yapılan `max(0, min(z_n, z_d) − z_o)`;
- kalan `max(0, z_d − max(z_n, z_o))`;
- fazla `max(0, z_n − z_d)` (tasarımın üstüne çıkılmış).

**Değişmez:** ölçünün örttüğü her yerde **yapılan + kalan = |z_o − z_d|**. Ölçü ne derse desin, iş yalnız yapılanla kalan
arasında yer değiştirir; hacim yaratılmaz, kaybolmaz.

**Tasarımın işi olmayan malzeme ikisine de girmez:**

- yarmanın üstüne yığılmış malzeme (stok);
- dolgu yerinden kazılmış malzeme (şantiyenin ödünç çukuru).

### §2 Kesin hesap (`kentos_road::volume::progress`)

- Tasarımın her üçgeni, ilk arazinin ve ölçünün üçgenleriyle kırpılır. Aday üçgenler ızgarayla bulunur.
- Her parça z_o = z_d, z_n = z_d ve z_o = z_n doğrularından bölünür. Bir parçada üç kotun sırası sabit kalır, her
  miktar doğrusaldır. İntegrali, alan çarpı ağırlık merkezindeki değerdir; bu kesin eşitliktir.
- Bir parça baştan sona böyle bir doğrunun üstündeyse (ölçü tasarıma eşit), ikiye bölünmez, bir kez sayılır.
- Raporlanan alanlar:
  - örtülen: üç yüzeyin de bulunduğu alan;
  - örtülmeyen: tasarımın ilk arazi ya da ölçü dışında kalan alanı.

### §3 Dönemin işi: kümülatiflerin farkı

İki ölçü arasındaki işi bulmanın akla gelen yolu, önceki ölçüyü "ilk arazi" saymaktır. Bu yanlıştır:

- Önceki ölçüde tasarımın altına inilmiş bir yarma noktası (z_önceki < z_d) yeni karşılaştırmada dolgu sayılır.
- O noktaya sonradan serilen malzeme "yapılan dolgu" diye ödenir.

Hakedişteki gibi her ölçünün birikmiş işi ilk araziye göre ayrı hesaplanır ve dönemin işi ikisinin farkıdır. Fark eksi
çıkabilir (geri doldurma ya da ölçü hatası); işareti korunur ve uyarılır.

### §4 Komut: `road.volumes.progress` (Y36.2)

Komut yalnız okur ve şemayı değiştirmez. Ölçü, projedeki bir arazi yüzeyidir (Arazi'de noktalardan, kırık hatlardan
kurulur). Ayrı bir "ölçü" türü yoktur.

- **Girdi:** kübaj, `survey` (yüzeyin kimliği ya da adı), isteğe bağlı `previous`.
- **Yüzeyler:**
  - tasarım koridorun üst yüzeyidir;
  - ilk arazi koridorun yüzeyidir.
- **Çıktı:**
  - üç yüzeyin kimliği ve **kaynak özeti** (digest): rapor hangi ölçünün hangi hâliyle okunduğunu söyler (ROAD-VOL-15);
  - şimdiki, önceki ve dönemin işi;
  - yapılan pay (yüzde);
  - örtülen ve örtülmeyen alan;
  - her aralığın işi ve örtülmeyen alanı.
- **Hacimler** yerindedir; kabarma ve sıkışma katsayısı uygulanmaz.
- **Uyarılar:**
  - `volume_progress_uncovered`: ölçü, ilk arazinin örttüğü bir yeri örtmüyor; orada iş sayılmaz;
  - `volume_progress_backward`: dönemde yapılan kazı ya da dolgu eksi.
- **Retler:**
  - `invalid_volume_progress`: ölçü ilk arazinin kendisi, önceki ölçü ara ölçünün kendisi, ya da yüzey kurulamıyor;
  - `unknown_terrain`.

Python başvurusu (`road_volume_progress_command_cases.py`) ölçüleri kendi kurar. Her ölçü ilk arazinin 120 noktasına
koridor boyunca 87 nokta ekler; o noktalarda kot, araziden tasarıma doğru işin %40'ı (Eylül) ya da %80'i (Ekim) kadar
ilerletilir. Ekim ölçüsünde bir aralık %115'e götürülür. Batı yarısı ölçüsü yalnız x < 300 m'yi kapsar. Kaynak özetleri
arazi komutlarının başvurusuyla hesaplanır. 20 durumun hepsinde Rust ve Python tipli katmanı aynı sonucu verir.

### §5 Pencere: Kübaj → Dönem metrajı (Y36.3)

- **Form:** "Dönem metrajı" bölümünde Ara ölçü ve Önceki ölçü seçimleri var. Koridorun kendi yüzeyi listede yoktur.
  Seçimler pencereye aittir; kübajla kaydedilmez.
- **Gösterim:**
  - Bir cümle yapılan payı, kalanı, tasarımın ötesini ve dönemin işini söyler.
  - Uyarılar ayrı bantlardır.
  - "Dönem metrajı" tablosu her aralığın işini ve örtülmeyen alanını listeler.
  - Resim her aralığı, aralık kadar geniş ve metre başına hacmi kadar yüksek bir çubukla çizer: kazı eksenin üstünde,
    dolgu altında; yapılan dolu, kalan açık renk, tasarımın ötesi turuncu. Ölçünün örtmediği aralıkların altında gri
    bir bant vardır; o aralıklarda iş yok görünmesi ölçünün eksikliğidir.

**Örnek proje (Ana kübaj; ilk arazi Mevcut zemin, ara ölçü Ekim, önceki Eylül):**

| | Yapılan | Kalan | Tasarımın ötesi | Pay |
|---|---|---|---|---|
| Kazı | 20.606 m³ | 4.937 m³ | 3.470 m³ | %80,7 |
| Dolgu | 2.000 m³ | 1.560 m³ | 195 m³ | %56,2 |

- **Dönemin işi:** Eylül'den bu yana kazı +10.035 m³, dolgu +892 m³.
- **Değişmezin sınaması:** yapılan + kalan, iki ölçüde de kazı için 25.542,5 m³, dolgu için 3.560,0 m³'tür. Bunlar aynı
  kübajın yüzeyler arası yöntemle verdiği değerlerdir (`road.volumes.uncertainty` durumu "Yüzeyler arası ana
  koridorda"). Ortalama alan yöntemi aynı kesitlerden 25.878 m³ ve 3.507 m³ verir.
- **Tasarımın ötesi:** Eylül ölçüsünde bile 534 m³ çıkar. Bunun sebebi, ölçünün seyrek noktaları arasındaki doğrusal
  yüzeyin tasarımın kırıklarını kesmesidir. Fazla kazı ödemesine karar vermeden önce ölçü sıklığına bakılmalıdır.

Resimler `.run/gui-evidence/yol-y36/` altındadır.

## Ortak durumlar ve ölçüm (çekirdek)

`scripts/fixtures/road_volume_progress_cases.py` KentOS kodu olmadan çalışır. Kesir aritmetiğiyle aynı bindirmeyi
yapar; kırpmayı Y8'in başvurusundan (`road_volume_cases.py`) alır.

21 durum var:

- **Kapalı biçim:** düz yüzeylerin 11 durumu yazılmadan önce elle hesaplanan değerlerle karşılaştırılır. Örnek: 2 m'lik
  yarma 1 m inilmiş → yapılan = kalan = alan × 1 m.
- **Değişmez:** ölçünün her yeri örttüğü 20 durumda yapılan + kalan, Y8'in iki yüzey başvurusunun kazısına ve dolgusuna
  eşittir.
- **Zor durumlar:**
  - farklı ızgaralar (7 m'lik ölçü, 10 m'lik arazi);
  - üç yüzeyin birbirini kestiği eğik düzlemler;
  - tasarımın içinden geçen ölçü;
  - ölçünün yalnız bir kısmı örttüğü ve tasarımın araziden taştığı durumlar;
  - 5000 m ötelenmiş koordinatlar;
  - ters dönük üçgenler.

Rust sonuçları 21 durumda da başvuruyla 1e-9 içinde aynıdır (`crates/shared/road/tests/all/volume_progress.rs`).

## Açık

- **Kabarma ve sıkışma:** miktarlar yerindeki hacimdir; katsayı uygulanmaz.
- **Katmanlar:** yapılan iş üst yapı katmanlarına (alt temel, temel) ayrılmaz.
