# 0942 — Dönem metrajı: ilk arazi, ara ölçü ve tasarım arasındaki iş

Tarih: 2026-11-04. Durum: kabul (Y36.1 çekirdek). Kapsam ROAD-VOL-15.

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

## Ortak durumlar ve ölçüm

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
