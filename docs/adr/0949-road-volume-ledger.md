# 0949 — Kübaj defteri: toplamın nereden geldiği

Tarih: 2026-11-05. Durum: kabul (Y43.1 çekirdek, Y43.2 komut). Kapsam ROAD-VOL-18, ROAD-VOL-16.

## Sorun

Kübaj raporu her kesit aralığının kazısını ve dolgusunu, toplamları ve birikimli kütleyi veriyor. Toplam
beklenenden büyük çıktığında cevap aranan sorular:

- Hangi km'de? Hangi bölgede (tip kesitte)?
- Hangi malzemede?
- Birkaç aralık mı toplamı taşıyor, yoksa hacim yola yayılmış mı?
- Büyük aralık, kesiti büyük olduğu için mi, aralık uzun olduğu için mi büyük?

ROAD-VOL-18 bunu ister: "station/material/AOI bazında büyük toplamın nedeni görülsün." ROAD-VOL-16 de raporda
yuvarlamanın sona bırakılmasını, yuvarlanmış satırların yeniden toplanıp toplam yapılmamasını ister.

## Dayanak

Araştırma (5 Kasım 2026):

- **Civil 3D** toplam hacim tablosunu (kazı, dolgu, birikimli) ve malzeme hacim tablosunu (bir malzemenin kazı, dolgu,
  birikimlisi) ayrı verir. Malzemeler örnek hat grubunda tanımlanır. Hacim, iki kesitin alan ortalaması çarpı
  aralıktır, birikimli toplam bunların toplamıdır.
- **OpenRoads** (Civil Analysis) miktar raporunu km aralığı ve pafta sınırı için alır. Uç alan raporu kesitin adlı
  sınırlarıyla çalışır.
- **Yoğunlaşma ölçüsü:** Lorenz eğrisi ve Gini katsayısı. Burada yol boyuna uygulanır: hacmin ne kadarı yolun ne
  kadarında toplanıyor.

KentOS'un kübaj cetveli (ADR 0906) zaten toplamları yuvarlanmamış değerlerden yazıyor (`road_volumes::sheet`). Eksik olan,
yuvarlanmış satırların toplamının yuvarlanmış toplamı neden tutmadığını göstermekti.

## Karar (`kentos_road::volume_ledger`)

- **Gruplar aralıkları bütün alır, ortalarından.** İki kesit arası tek bir hacimdir (uç alan ya da prizmoidal); onu
  bölmek yöntemin hiç yapmadığı bir bölüşüm uydurmak olurdu. Bu yüzden gruplar toplamı tam verir ve sınırları
  kesittir. km dilimleri dilim boyunun katlarından başlar; bölgeler koridorun bölgeleridir.
- **Toplamlar telafili** (Neumaier). Grupların toplamı ile genel toplamın farkı çıktıda durur; ortak çizimde ~1e-12 m³.
- **Malzeme her biri bir sütundur:** kazı, dolgu, düzeltilmiş kazı ve dolgu, varsa bitkisel toprak, diş kazısı ve her
  üstyapı tabakası. Her grubun her sütundaki payı verilir.
- **Yoğunlaşma.** Aralıklar metre başına hacimle, en yoğundan başlayarak sıralanır. Eğri, boyun payına karşı hacmin
  payıdır. Her aralığın hacmi kendi boyuna yayılmış sayıldığı için eğri aralık içinde doğrusaldır, paylar ve alan
  kesindir.
  - Gini = 2·∫C − 1: 0 düzgün yayılmış, 1'e doğru kısa bir yerde toplanmış.
  - Hacmin yarısını ve beşte dördünü taşıyan boy payı da verilir.
  - Yoğunlukları 9 anlamlı basamakta eşit aralıklar kendi sıralarını korur. Eşit iki kesit farklı aritmetikle
    hesaplanınca yer değiştiriyor ve eğrinin bir noktasını kaydırıyordu (üstyapılı durumda görüldü).
- **Büyük aralığın nedeni.** Hacim = boy × ortalama alan. Hacmi olan aralıkların medyanına oranla iki çarpan verilir;
  büyük olanı nedendir: `section` (kesit) ya da `length` (boy).
- **1 cm³'e kadar hacim yoktur** (1e-6 m³, kübajın başka yerlerinde de aynı eşik). Kesit aritmetiği dolgu olmayan
  yerde 1e-29 m³ bırakıyordu. Bu aralıklar medyanı 1e-29'a çekti (alan oranı 1e29 çıktı) ve eğrinin kuyruğunu
  yeniden sıraladı.
- **Yuvarlama.** Her sütunda aralıklar `decimals` basamağa (yarım yukarı, sıfırdan uzağa) yuvarlanıp toplanır;
  toplam da yuvarlanır; ikisinin farkı verilir. Rapor toplamı hiçbir zaman yuvarlanmış satırlardan kurulmaz.

## Komut: `road.volumes.ledger` (Y43.2)

- **Girdi:** kübaj; `by` = `stations` (dilim boyu 1–100000 m) ya da `regions`; `top` en büyük aralık sayısı (1–100,
  yoksa 10); `decimals` (0–3, yoksa 2).
- **Çıktı:** sütunlar (toplam, yuvarlanmış satırlar, yuvarlanmış toplam, fark), gruplar (ad, aralık, aralık sayısı,
  değerler, paylar), grupların toplamdan farkı, kazının ve dolgunun en büyük aralıkları (pay, birikimli pay, boy,
  ortalama alan, iki çarpan, neden), yoğunlaşma, hacimsiz aralıklar.
- **Uyarılar:** kübajın kendi uyarıları.
- **Retler:** `invalid_volume_ledger` ve kübajınkiler.

## Ortak durumlar ve ölçüm

- `scripts/fixtures/road_volume_ledger_cases.py`: 11 durum, tam kesirli; KentOS kodu yok. Düzgün, tek büyük
  aralıklı, farklı boylu, boşluklu, sınırda ortası sayılan, km'si 137,5'ten başlayan, gürültülü ve üç sütunlu durumlar
  ile iki yuvarlama durumu var. Rust 1e-12 içinde aynı bulur (`crates/shared/road/tests/all/volume_ledger.rs`).
- `scripts/fixtures/road_volume_ledger_command_cases.py`: 24 durum. Kübajın raporu `road_volume_command_cases.py`'nin
  bağımsız başvurusu, defter tam kesirli işlevlerle. Rust ve Python 1e-6 içinde aynı.

Ortak çizimin ana kübajı (uç alan, 36 aralık), 100 m'lik dilimler:

| Dilim | Kazı | Kazının payı | Dolgu | Dolgunun payı |
|---|---|---|---|---|
| 1+000 – 1+100 | 5310,7 | %20,5 | 0 | — |
| 1+300 – 1+400 | 181,5 | %0,7 | 2181,7 | %62,2 |
| 1+500 – 1+600 | 13257,5 | %51,2 | 0 | — |

- **Kazı:** yolun %16,1'i kazının yarısını, %37,6'sı beşte dördünü taşıyor (Gini 0,57). En büyük aralık 1+550 – 1+575:
  3837,0 m³, %14,8. Boyu medyanın 2,5, ortalama alanı 3,68 katı: büyüklüğünün nedeni kesit.
- **Dolgu daha yoğun:** yolun %8,6'sı yarısını taşıyor (Gini 0,81).
- **Yuvarlama:**
  - 2 basamakta kazı satırları 25878,14, toplam 25878,15 (fark −0,01 m³);
  - tam sayıda kazı satırları 25879, toplam 25878 (+1 m³), düzeltilmiş dolgu −1 m³.

  Satırlardan toplam kurulsaydı hakediş cetveli 1 m³ yanlış olurdu.

## Açık

- Pencere (Y43.3).
- Pafta sınırına göre gruplama (OpenRoads'un sheet clipping'i) ve yanlara (sol/sağ) göre bölüş; ikincisi kesitin
  alanını eksenin iki yanına ayırmayı ister.
