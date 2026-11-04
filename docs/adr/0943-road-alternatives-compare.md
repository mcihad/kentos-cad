# 0943 — Alternatiflerin karşılaştırması: nerede ayrılıyorlar, ne kadar tutuyorlar

Tarih: 2026-11-04. Durum: kabul (Y37.1 çekirdek). Kapsam ROAD-ALIGN-12 ve ROAD-VOL-15'in kalanı.

## Sorun

Bir yolun birkaç alternatifi çizilir: kurp yarıçapı büyütülmüş, PI kaydırılmış ya da kırmızı kot yükseltilmiş. Karar
için iki soru sorulur:

- Alternatif tabandan **nerede** ayrılıyor?
- Hacmi, uzunluğu ve onaylı hâli **ne kadar** farklı?

Bugün kübajlar tek tek açılıyor; iki aksın nerede ayrıldığı çizimde gözle aranıyor.

ROAD-ALIGN-12 şunu ister: "Alternative alignment ve compare için parent/source digest ve değişen kilometre aralığı
korunsun." ROAD-VOL-15'in kalanı da alternatif miktarların kaynak özetiyle karşılaştırılmasıdır.

## Dayanak

- **Araştırma (4 Kasım 2026):** OpenRoads ve Civil 3D'de iki alternatif aksı yan yana karşılaştıran yerleşik bir araç
  bulunamadı. Kullanıcılar kesit raporlarını tabloya aktarıp karşılaştırıyor. Trimble QuantM'in güzergâh
  eniyilemesinde bir "Compare Alignments" ekranı var.
- **Uzaklığın Lipschitz sınırı:** bir noktanın bir kümeye uzaklığı 1-Lipschitz'tir. Alternatifin üstünde birim hızla
  ilerleyen bir noktanın tabana uzaklığı da metre başına en çok 1 m değişir.

## Karar

### §1 Planda ayrılma (`kentos_road::alternatives::plan`)

- d(s), alternatifin s istasyonundaki noktasının taban aksına uzaklığıdır: en yakın ayağın ofseti; ayak yoksa yakın
  ucun uzaklığı.
- **Ayrılan aralık:** d(s) > t olan aralıktır (t tolerans).
- **Adım:** d, s'den itibaren |d − t| metre içinde t'ye ulaşamaz. Her adım bu kadar atılır, en az 1 cm. Tarafı
  değiştiren adım ikiye bölünerek 1e-9 m'ye inceltilir.
- **Garanti:** 1 cm'den kısa bir ayrılma kaçabilir; daha uzunu kaçmaz. Adım uzaklıkla büyüdüğü için uzak kısımlar
  birkaç adımda geçilir.
- **Aralığın uçları** tabanın istasyonlarıyla da verilir. Tabanın ucunun ötesindeki noktanın istasyonu yoktur.
- **En büyük ayrılma:** yarım metrede bir örneklenir, en iyi örneğin çevresinde altın oranla inceltilir.
- **Ters yönde çizilmiş aynı aks** ayrılmaz; ölçülen geometrinin kendisidir.

### §2 Kotta ayrılma (`kentos_road::alternatives::height`)

- **Nerede karşılaştırılır:** iki aksın t içinde birlikte gittiği yerlerde. Δz, alternatifin kotu ile tabanın en yakın
  istasyonundaki kotu arasındaki farktır.
- **Ayrılan aralık:** |Δz| > h olan aralıktır.
- **Lipschitz sabiti:**
  - Alternatifin en büyük eğimi artı tabanınki.
  - Tabanın istasyonu alternatifinkinden en çok 1/(1 − κ·t) kat hızlı ilerler (κ tabanın en büyük eğriliği). Tabanın
    eğimi bu oranla büyütülür.
  - Eğimler bir parçanın uçları arasında monotondur (parabol ve daire); en büyüğü uçlardadır.
- **Planda ayrılan yerde** kot karşılaştırılmaz; planın ayrılması zaten söyler.

## Ortak durumlar ve ölçüm

`scripts/fixtures/road_alternatives_cases.py`, KentOS kodu olmadan ve **KentOS'un yönteminden farklı** bir yoldan
çalışır:

1. alternatifi 0,25 m'de bir örnekler;
2. uzaklığı tabanın 5 cm'lik çoklu çizgisine numpy ile ölçer;
3. tarafın değiştiği örnekler arasını, aks başvurusunun 40 basamaklı yerleri ve altın oranla bulunan ayakla kesin
   uzaklık üzerinden ikiye bölerek inceltir.

Toleransa 1e-4 m'den yakın örneklerde taraf kesin uzaklıkla seçilir. İlk yazımda kaba çoklu çizgi bir sınırı yutmuştu;
kapalı biçim denetimi yakaladı.

15 durum var. Kapalı biçimle sınananlar:

- θ açısıyla ayrılan aliyman t / sin θ'da ayrılır;
- tabanın ucunu geçen aks ucun t ötesinde ayrılır;
- paralel 5 m'lik aks t = 1'de baştan sona ayrılır, t = 6'da hiç ayrılmaz;
- ortası 1 m yükseltilmiş kırmızı kot, h = 5 cm'de 25–975 m arasında ayrılır;
- aynı aks, ters yönde çizilmiş aynı aks ve aynı kotlar hiç ayrılmaz.

Ötekiler:

- yarıçap 300 → 400 (291,141–505,347 m, en çok 2,659 m);
- klotoid 50 → 80;
- PI'ın 15 m kaydırılması;
- ortada sapıp geri dönen aks;
- ötelenmiş koordinatlar;
- düşey kurp boyu 200 → 300;
- planı ayrılan aksın kotu.

Rust 15 durumda da aralık uçlarını, taban istasyonlarını ve en büyük ayrılmayı 1e-6 m içinde aynı bulur
(`crates/shared/road/tests/all/alternatives.rs`).
