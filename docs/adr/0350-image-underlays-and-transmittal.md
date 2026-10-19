# ADR 0350: Resim altlığı ve taşınabilir paket

- Durum: Kabul edildi (19 Ekim 2026)
- Kapsam: plan/cad.md `CAD-REF-16` (resim altlığında yeniden yükleme, bulunamayan kaynak ve taşınabilir resim) ve `CAD-REF-20`
  (eTransmit eşdeğeri paket); CAD-REF'in üçüncü grubu (8c). Kırpma (`CAD-REF-12`) ve kaynağı yerinde düzenleme (`15`) sonraki
  gruptadır.
- İlgili: ADR 0192 (resim nesnesi), ADR 0506 (Paketle), ADR 0348 (dış başvurular), ADR 0349 (arama yolları, `cad.assets.check`),
  ADR 0324 (yazı tipleri), ADR 0251 (pafta kitabı projede).

## Bağlam

Resim nesnesi gömülü ya da bağlı olabiliyordu (ADR 0192). Bağlı resmin dosyası bulunamayınca çizimde hiçbir şey görünmüyordu; yalnız
Öznitelikler'in Kaynak satırı “(bulunamadı)” diyordu. Dosya değişince çizim onu yeniden okumuyordu. Gömme yalnız masaüstünün
Öznitelikler'indeki Göm düğmesindeydi: Python ve yapay zekâ ajanı bir resmi gömemiyor, bağlı resmin dosyasını değiştiremiyordu.
Arama yolları (ADR 0349) yalnız dış başvurulara uygulanıyordu.

Paketle (ADR 0506) projeyi bağlı GeoPackage'ları, resimleri, rasterleri, nokta bulutlarını, tabloları ve ekleriyle bir klasöre
topluyordu. Dış başvuruların kaynakları pakete girmiyordu: paket başka makinede dış başvurularını kaybediyordu. Pakette neyin
bulunduğunu ve neyin eksik kaldığını paketle birlikte taşıyan bir liste yoktu; yazı tiplerinin durumu söylenmiyordu.

## Karar

### 1. `cad.images.edit` (`CAD-REF-16`)

Resimlerin kaynağını değiştiren ürün komutu; her işlem tek adım, dosyalar bütün ya da hiç okunur:

- `embed` (Göm): verilen resimler (`uids`), verilmezse çizimin bütün bağlı resimleri. Her birinin dosyası okunur (aşağıdaki arama
  kuralıyla), PNG ya da JPEG ve en çok 32 MB olmalı; baytları projenin kitaplığına içeriğinden kimlikli resim olarak girer
  (`resim-` ve SHA-256'nın ilk on altı hanesi; aynı resim bir kez, “Resimler” altında, adı dosyanın adı, boyu pikselleriyle) ve
  resim gömülü olur. Adım “Göm”.
- `relink` (Yeniden bağla): verilen resimler yeni dosyayı gösterir (`file`: mutlak ya da çizimin klasöründen göreli). Dosya okunur
  ve PNG ya da JPEG olmalı; gömülü resim bağlı olur, kitaplıktaki resim yerinde kalır (Kullanılmayanları temizle onu alır). Resmin
  çerçevesi, dönüşü, kırpması ve saydamlığı değişmez. Adım “Yeniden bağla”.

Kilitli katmandaki resimler atlanır ve sayılır (`layer_locked` uyarısı; hepsi kilitliyse ret, ADR 0278). Retler: `no_entities`,
`invalid_uid`, `entity_not_found`, `not_an_image`, `layer_locked`, `nothing_to_embed` (gömülecek bağlı resim yok),
`invalid_file_path`, `file_not_found`, `import_unreadable` (PNG ya da JPEG değil, bozuk ya da 32 MB'tan büyük). Biçim ve boy
dosyanın başlığından okunur (PNG'nin IHDR'si, JPEG'in çerçeve başlığı). Girdinin `drawing`'i açık çizimin
dosyasıdır; göreli yollar onun klasöründen aranır.

Masaüstünün Göm düğmesi bu komutla yazar; Öznitelikler'in Kaynak satırında bağlı resme Yeniden bağla… gelir.

### 2. Resimlerin ve rasterlerin araması

Bağlı resim ve rasterin dosyası dış başvurununki gibi aranır (ADR 0349 §1): kayıtlı yol mutlaksa o, çizimin klasöründen göreli yol,
sonra projenin arama yollarında önce göreli yol, sonra dosyanın adı. Arama yolları penceredeki adıyla (“Arama yolları”) artık
resimler ve rasterler için de geçerlidir; `cad.assets.check` onların `via`'sını ve denenen yollarını da verir. Nokta bulutları,
tablolar ve ekler eskisi gibi kayıtlı yol ve çizimin klasörüyle aranır.

### 3. Bulunamayan ve değişen resim

Bağlı resmin dosyası okunamazsa çizimde resmin çerçevesi tehlike renginde kesikli çizilir, köşegenleriyle ve ortasında uyarı
üçgeniyle; Öznitelikler “Bağlı: ad (bulunamadı)” der. Masaüstünün dosya izleyicisi (ADR 0349 §3) bağlı resimlerin dosyalarına da
bakar: dosya değişince ya da yeniden bulununca resim yeniden okunur ve çizilir; çizim değişmez, kart çıkmaz (resmin pikselleri
çizimin verisi değildir).

### 4. Taşınabilir paket (`CAD-REF-20`)

`file.package` (ADR 0506) dış başvuruların kaynaklarını da toplar:

- Her dış başvurunun kaynağı (boşaltılmış olanlarınki de, iç içe gelenlerinki de) arama kuralıyla bulunur ve `veri/`'ye bayt bayt
  kopyalanır (`kind: xref`); bağı kopyaya yazılır (`path` paketin içindeki mutlak yol, `relative` `veri/ad.kcad`). Bayt bayt
  kopya olduğu için dış başvurunun özeti (`digest`) değişmez; paket açılınca dış başvurular `ok`'tur. İç içe gelen dış
  başvurunun içeriği getirenin dosyasındadır (ADR 0348); kaynağı ayrıca kopyalanır ki getiren kaynak tek başına da açılsın.
- Bulunamayan kaynak `left`'te `missing`'dir.
- Resimler ve rasterler arama yollarıyla da aranır (§2).
- Yazı tipleri: KentOS yalnız kendi yedi yüzüyle çizer, onlar uygulamayla gelir, kopyalanmaz. Cevabın `faces`'i çizimin
  kullandığı yüzlerdir (`cad.fonts.report`'unki). Başka bir yazı tipini adlandıran (SHX, KentOS'ta olmayan) ve yerine başka yüzle
  çizilen yazı stilleri `left`'tedir (`kind: font`, `why: substituted`, `at` stilin adı).
- Stiller (yazı, ölçü, kılavuz, duvar, çizgi tipleri, tarama desenleri), kitaplık, gömülü resimler ve pafta kitabı (ADR 0251) proje
  dosyasının içindedir; ayrı kopyalanmaz.

Paketin kökünde **`paket.json`** (`kentos.package/1`): projenin dosyası (adı, boyu, SHA-256'sı), kopyalanan her dosya (paketteki
yolu, kaynağı, türü, boyu, SHA-256'sı, bağ sayısı), kopyalanmayan her bağ ve nedeni, yazı yüzleri. Liste anahtarları sabit sırayla,
zaman damgasız yazılır: aynı çizim aynı paketi verir. Manifest de geçici klasörde yazılır, geri okunup özeti karşılaştırılır; cevapta
`manifest` ve `manifestSha256`.

## Sonuçlar

- Bağlı resmin kaynağı Python'dan ve yapay zekâ ajanından da gömülür ve değiştirilir; masaüstü aynı komutla yazar.
- Kaynağı bulunamayan resim çizimde görünür; değişen resim kendiliğinden yeniden çizilir.
- Paket dış başvurularıyla başka makinede açılır; manifest neyin içinde, neyin dışında kaldığını paketle birlikte taşır.

## Doğrulama

- Ortak durumlar: `python3 scripts/fixtures/images_edit_cases.py --check` (`cad.images.edit`, PNG ve JPEG başlıkları KentOS kodu
  olmadan okunur) ve `python3 scripts/fixtures/assets_check_cases.py --check` (resim ve rasterlerin arama yolları).
- Paket: `python3 scripts/fixtures/package_cases.py` (başsız sunucunun paket testlerinin klasörleri bağımsız KCAD okuyucusu ve
  hashlib'le: dış başvurunun kopyası ve özeti, `paket.json`'un her satırı, yerine çizilen yazı tipi).
- Resimler: `KENTOS_SHOTS_ONLY=resim-bulunamadi,resim-yeniden-bagla,resim-gomuldu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
