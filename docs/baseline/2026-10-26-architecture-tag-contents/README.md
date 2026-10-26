# Görünür etiket içerik örnekleri — ADR0664

Taban `2d396fcbf`, aynı mimari dal. Etiket/Hepsini etiketle mevcut
`cad.tags.create` şablon alanını doldurur; yeni API alanı, preset kimliği,
KCAD şeması, TS/TSX, birim/ölçek veya render değişikliği yoktur.
Bu masaüstü dilimi kabul edildi; bütün mimari plan kabulü değildir.

`test-runs.json` gerçek komut/zaman/çıkış ve ham günlük hash'lerini tutar.
`before-binary.json` kesin önceki kabulün değişmez ikilisidir; ayrı test
profili `.run/tag-contents` kullanılır. Kaynak örnek önceki net alan
kabulündeki dört nesnedir: yalnız mahalın açık yüksekliği 2,75m yapıldı.
`fixture-preparation.json` ve bağımsız okuyucu diğer bütün alanların
aynılığını doğrular; hazırlık yeni GUI işlemi olarak sayılmaz.

İlk odaklı testte kaynağı bulunmayan seed mahalı için yanlışlıkla 24m²
net beklenmişti. Ürün doğru olarak bilinmeyen değeri boş bıraktı. Olumlu
alan örneği açıkça çizilmiş Net sınıra çevrildi; seed ve önceki kayıp
sınır uyarısı kaldırıldı. Diğer katlı testte bilinmeyen netin boş kalması
açıkça beklendi. Üretim alan algoritması ve toleransı değiştirilmedi.
İlk odaklı ve ilk tam başarısız koşumlar saklanır, başarılı sayılmaz.

Son `interaction-final`: 63 birim +708 entegrasyon başarılı, 1 mevcut
entegrasyon ve 5 perf atlanan. Dört yeni test; yedi içerikte sabit beklenen
metin, sekiz seçeneğin klavye çevrimi, özel metin/geçersiz seçim, diğer
ayarların korunumu ve katlı toplu tek onay/undo/redo içerir.

## Son doğrulamalar

- `interaction-final`: 63 birim +708 entegrasyon, sıfır başarısız; bir
  mevcut entegrasyon ve beş performans testi atlandı.
- `build`: gerçek masaüstü ikilisi derlendi. `clippy`: interaction ve
  desktop, bütün hedefler, `-D warnings` başarılı. Mevcut ts-rs attribute
  uyarıları ve block0.1.6 gelecek uyumu bildirimi ham günlüklerde korunur.
- Cargo koşumları sırayla ve `TS_RS_EXPORT_DIR` çalışma dizinine yöneltilerek
  yapıldı. Yeni TS/TSX üretilmedi. Python/MCP ve WASM bu dilimde tekrar
  koşulmadı; mevcut komut/SDK/model değişmedi. ADR0663'ün önceki koşumları
  bu dilimin yeni koşumları olarak sayılmaz.
- `python3 docs/baseline/2026-10-26-architecture-tag-contents/verify.py`:
  15 kabul KCAD'ını bütün alanlarıyla sabit bağımsız beklentiye karşı okur.
  Önceki1810 takipli KCAD dosyası SHA256 ile aynıdır. Kaynak UID'leri,
  ayarlar, dört ilgisiz kaynak, katmanlar, numaralar, sınırlamalar ve
  geometri dahil yalnız belirtilen etiket/yükseklik farkları kabul edilir.
  Eşit beklenen modellerin dosya baytları da birebir eşittir.
- `verify-format.py`: değiştirilen üç Rust dosyasında yeni rustfmt farkı
  yoktur. tag_tools içindeki iki eski fark tabanla aynıdır. Bütün çalışma
  alanının biçimlendirmesi temiz ilan edilmez; eski kod yeniden biçimlenmedi.

## Gerçek native etkileşim ve bağımsız sonuç

Ürün ikilisi değişmez ayrı QA `.app` içine kopyalandı. Önceki ikili, yeni
ikili ve ayrı profil ikilisi hash'leri `*-binary.json` içindedir; aynı ada
sürekli derleme yapıp eski kanıtın ikilisini değiştirme yapılmadı. İlk QA
ve Isolated QA aynı yeni ikiliyi çalıştırır. Kullanıcının normal ayarları
yerine `.run/tag-contents` altında ayrı XDG config/data kullanıldı.

1. `01` önceki0663 sürümündeki ayarları gösterir. `02–04` yeni seçimlerin
   ve beş satırlı şablonun önizlemesidir; `preview.kcad` dört kaynaklı
   başlangıca bayt düzeyinde eşittir. `06–07` ilk temiz tek etiket örneğidir;
   sonradan karışan geçmiş kabulü için kullanılmaz.
2. Kontrollü tekrar `13–16`: Hazır içerik **Mahal ve kotlar**, Yuvarlak
   köşeli, Kol Var, 3.5mm; mahal kaynağından elle `9,2` konumuna **tek
   Return**. Net `(6−.2)×(4−.2)−.5²=21.79m²`, bilinen döşeme0 ve açık
   yükseklikten tavan2.750 görünür. `created-final.kcad` bu sabit beş satırı,
   gerçek şablonu, kaynak bağını, UID'yi ve bütün ayarları doğrular. Aynı
   süreçte tek undo dört kaynağa, tek redo aynı UID/konuma döner.
3. `17–19`: mahal yüksekliği Öznitelikler'de **312.5cm** girilir; metre
   projede **3.125** olur. Kaynak ve bağlı tavan metni aynı adımda güncellenir;
   etiketin elle yeri ve diğer her alanı korunur. Tek undo/redo birlikte döner.
   Sonra kaynak yüksekliği2.75'e geri alınır.
4. `20–22`: toplu **Yenile / Zemin kat / Ad**, `0 yeni ·1 yenilenecek`
   önizlemesi modele yazmaz. Tek Return mevcut etiketin yalnız `content`
   ve `text` değerini değiştirir; konum/çerçeve/kol/boy/UID aynı. Tek undo
   bütün beş satırı geri getirir.
5. `23–25`: serbest **Özel {name}** kısa Özel değerini ve Özel şablonu koru
   seçeneğini gösterir. Koruma seçimi taslağı değiştirmez. Tür/Koru şablonu
   boşaltır; Yenile mevcut beş satırı korur, önizlemede değişecek etiket yoktur.
   İptal ve Kaydet bütün modeli korur.
6. `26–31`: koyu18 geniş/dar (2200×1364) menünün dokuz girdisi pencere
   içinde kalır; dar menü yukarı açılır, son **Açıklık ve boyut** seçilir.
   Serbest düzenleme Özel'e döner, uzun seçili açıklama taşması yoktur.
   `{unknown}` reddedilir ve önceki taslak kalır. Gerçek komut satırında
   **İ + Return** Numara'yı seçer; Escape kaynak/etiketi değiştirmez.
   Açık13 geniş de önceki adımlarda incelendi. Ayarlardaki yazı boyutunu
   değiştirmek için Return ile alan onaylandı; ilk yalnız Tab denemesi13'te
   kaldığı için büyük yazı kanıtı sayılmadı.
7. `33–34`: temiz yeni süreçte dosya5nesneyle açılır, doğru net/kot/şekil/kol
   görünür; Kaydet sonrası `reopened-final.kcad`, `created-final.kcad` ile
   bayt düzeyinde aynıdır. Test uygulaması kapatıldı.

## Kesintiler ve kabul dışı kayıtlar

34 ham ekranın **yedisi** (`05,08,09,10,11,12,32`) tanısaldır; 27 görüntü
kabul adımlarına aittir. Görüntüler kırpılmadı/değiştirilmedi; bütün ham
hash/boyutları `gui-events.json` ve `verification.json` içindedir.

İlk testlerde dış tıklama/zoom/menü değişimleri oldu. İlk dosyadaki iki
fazla kolon etiketi `diagnostic-external.kcad/json` olarak korundu. Ayrı
profilin yeniden açılmış beş nesneli süreçteki boş undo geçmişi `10`'da
görüldü; `undo.kcad` ve `undo-confirmed.kcad`, **kabul dışı**
`diagnostic-models.json` içine alındı. Kurtarma modalı `11`, yanlışlıkla
Benzerini seç'e ulaşan giriş `12` de kabul sayılmadı. Kullanıcı test
pencerelerine dokunmamayı onayladıktan sonra13'ten kontrollü tekrar yapıldı.

İlk yeniden açma `32`'de beklenmeyen taşıma/yakınlaştırma görüldü; bu adım
kalıcılık kabulü değildir. Diskteki çizim kabul edilmiş `created-final`
baytlarıyla aynı kaldı. QA kapatıldı; eski QA recovery verisi silinmeden
ayrı dizine taşındı, boş recovery diziniyle33–34 tekrarlandı. Bu üç tanısal
KCAD yalnız hash ve korunma için doğrulanır; 15 kabul modelinin içinde değildir.

Tam tema/yazı/pencere bileşim matrisi, performans ölçümü, bütün kaynak
türleri için GUI, çoklu mimari görünüş/pafta ölçek bağlamı ve yasal alan
profilleri bu dilim değildir. ARCH-SPACE-02 [~] kalır.
