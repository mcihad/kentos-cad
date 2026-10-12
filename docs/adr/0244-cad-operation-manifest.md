# ADR 0244: CAD işlem manifesti

- **Durum:** kabul edildi (2026-10-12). Sahibin kararı: “Plan/CAD.md dosyası ile devam edelim”. Bu, planın §24.1'deki ilk iştir
  (TODOS.md `CADP-01`; plan `CAD-AUD-01`–`06`, `11`, `12`). Kapsamı ben belirledim.
- **Bağlam belgesi:** [plan/cad.md](../../plan/cad.md) §2.3, §3, §4, §5; ADR 0013 ve 0022 (ürün komutları), ADR 0131 (Python SDK),
  ADR 0133 (MCP), ADR 0228 (web'e ara, `catalog/v1/additions.json`), `docs/inventory/README.md`.
- **Web:** gerekmez. Manifest web'in donmuş envanterini okur; web'e dönüşte envanter yeniden üretilince manifest de yeniden yazılır.

## Bağlam

Planın ilk tespiti şudur: arayüzde bir işin bulunması, aynı işin Python'dan ve AI'dan yapılabildiği anlamına gelmez.

- `cad.entities.edit`'in 56 işleminin hepsi sonucun son geometrisini `changes` ile dışarıdan alır; işlemin adı yalnız geri alma
  adımını adlandırır.
- `cad.entities.create`'in türetilmiş işlemleri de (paralel, bölme, tarama, toplu alan …) çizilecek nesneleri hazır ister.
- Arayüz geometriyi ortak çekirdekle hesaplayıp yazdırır. Python'un “200 mm ötele” diyebilmesi için ötelemeyi kendisinin
  hesaplaması gerekir.
- Katman, stil, proje ayarı, pafta ve dosya alışverişinin bir kısmının ürün komutu yoktur; arayüz belgeye doğrudan yazar.
- İşlemler'in araçlarına (137) Python'dan ve MCP'den erişilemez.

Envanterin 603 komutu ile kataloğun 46 komutunu karşılaştırmak bu farkı ölçmez (plan §2.3). Ölçü, her arayüz işinin bir
düzeyidir.

## Karar

### 1. Düzeyler

Her arayüz öğesi tek bir düzey alır:

| Düzey | Anlamı |
|---|---|
| `semantic` | Ürün komutu kullanıcının verdiğini alır, sonucu kendisi hesaplar. Python ve MCP aynı işi yeniden hesaplamadan yapar. |
| `geometry` | Ürün komutu var, ama sonucun geometrisini çağıran hesaplar. |
| `processing` | İşlemler aracı; yalnız uygulamanın çalıştırıcısından. |
| `none` | Belgeyi ya da projeyi değiştirir, ama ürün komutu yok: arayüz doğrudan yazar. |
| `query` | Yalnız okur ya da ölçer. |
| `session` | Görünüm, seçim, çizim yardımcısı ya da aracın adımı: oturumun durumu. |
| `host` | Pencere, panel ya da yardım: uygulamanın sunumu. |

- Bir aracın yöntemleri ayrı satırdır (plan `CAD-AUD-03`). Örneğin Daire'nin “Merkez, yarıçap” yöntemi `semantic`tir, “3 nokta”
  yöntemi `geometry`dir: `cad.circle.create` merkez ve yarıçap ister, araç onları üç noktadan hesaplar.
- Ölçü aracının nokta yöntemleri (Hizalı, Doğrusal, Açı, Koordinat, Semt, Eğim) `semantic`tir: ölçü tanım noktalarını saklar.
  Nesneden alanlar (Yarıçap, Çap, Yay uzunluğu, Kırıklı yarıçap) `geometry`dir; hiçbiri nesneye bağlı değildir (plan §11.1).

### 2. Kaynaklar

Manifest tek bir eşleme dosyası dışında her şeyi kaynaklardan okur:

- arayüz öğeleri: web'in donmuş envanteri (`docs/inventory/web.json`: komutlar, araçlar, yöntemler) ve `catalog/v1/additions.json`;
- kapsam: iki proje türünün şeridi (envanterin `ribbonByMode`'u ve eklerin yerleri, ▾ listeleri dahil). `cad` yalnız CAD şeridinde,
  `gis` yalnız CBS şeridinde, `shared` ikisinde, `none` hiçbir şeritte (kısayol, menü, komut satırı);
- aracın ürün komutu: envanterin `productCommand`'ı. Eşleme başka bir komut derse denetim düşer;
- işlemin adı: sözleşmenin `EditOperation` ve `CreateOperation` sayımları, Rust kaynağından;
- Python ve MCP: ürün komutu kataloğu. Masaüstünde çalışan her katalog komutu bir `kentos.cad` çağrısı ve bir MCP aracıdır.
  Katalog dışı yollar eşlemede adlarıyla yazılır (`kentos.address`, `Document.undo`, `drawing.new` …);
- masaüstü sütunu: envanterinkine (`apps/desktop/ported.json`) `apps/desktop/equivalents.json` eklenir.

### 3. Eşleme dosyası: `catalog/v1/operations.json`

Elle tutulur, biçimi `kentos.catalog-operations` sürüm 1'dir:

- `levels`: düzeylerin açıklamaları;
- `rules`: sıralı kalıplar (`view.*` → `session`, `processing.*` → `processing` …); ilk uyan geçerlidir;
- `commands`: komut başına düzey, `via` (ürün komutları), `operation`, `python`, `mcp`, `note`. Kurallardan önce gelir;
- `tools`: araç başına düzey, isteğe bağlı `operation`, `via`, yöntem başına düzey (`methods`), `python`, `note`.

**Kural:** yeni komut, araç ya da yöntem eşlemesiyle aynı commit'te gelir. Bir iş düzey değiştirdiğinde (örneğin Ötele
`semantic` olduğunda), eşlemesi de o commit'te değişir.

### 4. Üreteç ve denetim: `scripts/cad/manifest.py`

Yazdıkları:

- `docs/cad/manifest.json`: satırlar (kimlik, tür, ad, kapsam, masaüstü, düzey, `via`, işlem, Python, MCP, iz sayısı, not);
- `docs/cad/manifest.md`: düzeyler, kapsam × düzey sayıları ve CAD tarafının boşlukları. Boşluklar, CBS'ye özgü olmayan satırlardan
  `geometry`, `none`, `processing` ve Python'da karşılığı olmayan `query` olanlardır; her boşluk ürün komutu ve notuyla listelenir.
  Masaüstünde olmayan CAD tarafı öğeleri de listelenir.

`--check` şunlarda düşer (plan `CAD-AUD-11`, `12`):

- düzeyi olmayan komut, araç ya da yöntem;
- envanterde ya da eklerde olmayan bir öğeyi anan eşleme;
- katalogda olmayan ürün komutu ya da sözleşmede olmayan işlem adı;
- envanterle çelişen ürün komutu;
- çıktıların üretilenle aynı olmaması.

Python SDK'sının ve MCP'nin katalogdan sapması ayrıca `scripts/python/sdk.py --check` ve MCP'nin testleriyle denetlenir.

## Uygulama

- `catalog/v1/operations.json`: 131 araç (11'inin yöntemleriyle), 151 komut ve 11 kural.
- `scripts/cad/manifest.py`: üreteç ve denetim.
- Çıktılar: `docs/cad/manifest.json` ve `docs/cad/manifest.md`.
- Belgeler: `catalog/README.md`'ye eşleme dosyası; TODOS.md §16.5 (planın on iki işi); kılavuzun §2'sine komut; tarifin takımına
  denetim.

## Doğrulama

- Manifest 654 satırdır. CBS'ye özgü olmayan 449 satırın düzeyleri:

  | Düzey | Satır |
  |---|---|
  | semantic | 75 |
  | geometry | 115 |
  | none | 98 |
  | processing | 13 |
  | query | 21 (17'si Python'da yok) |
  | session | 105 |
  | host | 22 |

- Sonuç: 243 satır, Python ve AI'ın arayüz gibi yapamadığı iştir.
- Denetim, beş bozuk eşlemeyle sınandı ve beşinde de düştü:
  - silinen bir aracın düzeyi;
  - var olmayan bir işlem adı (`trimm`);
  - envanterle çelişen ürün komutu;
  - olmayan bir komutu anan eşleme;
  - komutta olmayan işlem.

  Düzeltilince `--check` geçer.
- Kod değişmedi; Rust ve Python takımları bu iş için koşmadı. Üreteç Python'un standart kitaplığıyla çalışır.

## Sonuçlar

- Sonraki işler bu sayıları düşürür: `CADP-04` Ötele'yi, `CADP-05` Buda, Uzat ve Köşe yuvarla'yı, `CADP-08` katman, stil ve ayar
  işlerini `semantic` yapar.
- İşlemler'in araçlarının Python'a ve MCP'ye açılması planda ayrı bir iştir (§18); manifest onları `processing` olarak sayar.
- Envanterin masaüstü sütunu web'in donduğu gündeki hâlidir. `sheet.*`'in 59 komutundan 58'i masaüstünde görünmez, ama pafta
  tasarımcısı masaüstündedir. Düzeltmesi `CADP-02`'dir.
- Yöntemlerin düzeyi elle verilir. Bir yöntemin hesabı komuta geçtiğinde (örneğin “3 nokta” daire), düzeyi eşlemede değiştirilir.
