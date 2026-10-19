# ADR 0602: Toplamanın ekleri

- **Durum:** kabul edildi (2026-10-19). ADR 0600'ün üçüncü dilimi; sahibin isteği: “aggregate falan gibi bir sürü iç içe
  kullanılabilecek şeyler var”.
- **Bağlam belgesi:** ADR 0214 §2.6 (aynı ve başka katman üstünden toplama), 0200 §4 (`kentos.statistics/1`), 0600.
- **Web:** ertelendi (ADR 0228); çekirdek ortaktır.

## Bağlam

Dilde toplama işlevleri vardı: toplam, ortalama, sayı, benzersiz sayı, en küçük, en büyük, ortanca, standart sapma, birleştirme
ve dizi. QGIS bunlara ek olarak çeyrekleri, aralığı, en sık ve en seyrek değeri, boş sayısını, metin uzunluklarını ve
benzersizlerin birleştirilmesini verir. Bunlar kadastro ve imar tablolarında sık sorulur: “adadaki parsellerin alanlarının
çeyrekleri”, “mahallede en sık nitelik”, “maliki boş kaç parsel”.

## Karar

### 1. İşlevler

Hepsi ötekiler gibi `(ifade, grup, koşul)` alır; `benzersizleri_birleştir` ayraç da alır. `katman_toplamı`'nın işlem adı olarak da
geçerler (`katman_toplamı('Durak', 'en_uzun', Ad)`).

| İşlev | QGIS adı | Değer |
|---|---|---|
| `çeyrek1`, `çeyrek3` | `q1`, `q3` | birinci ve üçüncü çeyrek |
| `çeyrekler_arası` | `iqr` | üçüncü çeyrek eksi birinci |
| `değer_aralığı` | `range` | en büyük eksi en küçük, kesin ondalıkla |
| `en_sık`, `en_seyrek` | `majority`, `minority` | en çok ve en az geçen boş olmayan değer |
| `boş_say` | `count_missing` | değeri boş olan nesneler |
| `en_kısa`, `en_uzun` | `min_length`, `max_length` | boş olmayan değerlerin metinlerinin en kısa ve en uzun boyu |
| `benzersizleri_birleştir` | `concatenate_unique` | her değer bir kez, ilk geçtiği sırayla, ayraçla |

`aralık` adı kullanılmadı: dilde `arasında` vardır, karışmasın diye `değer_aralığı`.

### 2. Kurallar (QGIS 4.2'de ölçüldü)

- **Çeyrekler Tukey'in menteşeleridir:** sıralı sayılar ikiye bölünür, her yarı ⌈n/2⌉ uzunluktadır (tek sayıda ortadaki iki yarıda
  da), çeyrek o yarının ortancasıdır. Örnek: 1, 1, 2, 3, 4, 5, 6 için 1,5 ve 4,5; 1, 1, 2, 3, 4, 5 için 1 ve 4.
- Sayılar `kentos.statistics/1`'in okuduğu sayılardır (ADR 0200 §4). Ortanca ve fark kesin ondalıkla, sonra çift duyarlığa.
- **Eşitlikte en sık ve en seyrek değer dilin sırasında önce gelendir** (`dizi_sırala`'nınki: sayılar, Türkçe metin,
  doğru/yanlış). QGIS de en küçüğü verir. Ayrım yalnız Türkçe harflerde: QGIS kod noktasıyla sıralar, KentOS Türkçe abeceyle.
- Değerler anahtarlarıyla sayılır (`Key`): `3` ile `'3'` aynı değerdir; dönen, o değerin ilk geçen biçimidir.
- Uzunluk, değerin dilin yazdığı metninin UTF-16 birimleridir (`uzunluk` gibi). Sayının metni de sayılır; QGIS sayı alanını
  reddeder.
- Boş değer: dilin boş saydığı (boş ya da boş metin). `say` ile `boş_say`'ın toplamı nesne sayısıdır.

### 3. Performans

Toplama yeni iş eklemez: bir çağrının değerleri katman ve grup başına bir kez hesaplanır, sonuç her nesnede tablodan okunur
(ADR 0214 §3). Çeyrekler bir sıralama, en sık değer bir karma tablo ister.

## Doğrulama

- `scripts/fixtures/expression_extras.py`: dilin bağımsız başvurusuna on işlem ve 21 durum (katmanlar, gruplar, koşul,
  `katman_toplamı`), `fixtures/expression/v2/extras.json`; Rust `tests/language.rs` iki yoldan.
- `scripts/fixtures/aggregate_qgis_cases.py --check`: QGIS 4.2'nin bellek katmanında 57 değer kuralın kendisiyle (çift ve tek
  sayılı kümeler, tek değer, eşitlikler, ondalıklar, metinler).
