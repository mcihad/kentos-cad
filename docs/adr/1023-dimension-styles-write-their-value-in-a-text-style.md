# ADR 1023: Ölçü stili değerini bir yazı stiliyle yazar (DIMTXSTY); SHX yazı stilinde değer fontun çizgileridir

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11`, ADR 1125 §3'ün sınırı “Boyut
  değerleri ... yüzle çizilir (stilleri ayrıdır)”. `.kcad` şema 186.
- **Bağlam belgesi:** ADR 0183 §3, §7 (ölçü stili, DXF DIMSTYLE), ADR 1125 (SHX yazısı), ADR 1126 (tablo stilinin
  yazı stilini adla anması).

## 1. Kaynak

Autodesk DXF başvurusu, DIMSTYLE grup kodu 340: DIMTXSTY, ölçü yazısının yazı stili kaydının tanıtıcısı; ölçü değeri
o stilin fontuyla yazılır (AutoCAD Dimension Style Manager › Text › Text style). KentOS'ta bugüne dek DIMTXSTY yalnız
bir yazı tipi ailesine çevriliyor, SHX stil projenin yazı tipiyle değiştiriliyordu.

## 2. Karar

- `DimensionStyleDef.textStyle` (isteğe bağlı): değerin yazıldığı proje yazı stili, **adıyla** (harf büyüklüğü
  ayrımsız; ADR 1126'nın tablo hücresi gibi, paket ve DXF arasında taşınabilir). Ad boş olamaz, kenarı boşluklu olamaz,
  en çok 256 bayt. `kentos_contracts::dimension_text_style` stili bulur.
- Çizim: stil SHX dosyası adlandırıyorsa (`shx_text::is_shx`) değer, taban ortası hizasıyla, ölçünün değer noktasında
  ve açısında, ölçünün yüksekliğinde fontun çizgileriyle çizilir: ekran (`labels.rs`), PDF ve CPU baskısı
  (`MapLabel.shx`); maskeli değerin maskesi kendi kutusudur; blok içindeki ölçü de (`PieceDimension`). SHX olmayan yazı
  stilinde `font` değerin yazı tipi olarak kalır (DXF okuyucusu ikisini birden yazar).
- Denetim: değer gömülü fontla çizilemiyorsa (`text_shx_unsupported`, hata; PDF reddedilir), font gömülü değilse
  adlandırılmış `text_font_substituted`.
- DXF içe alma: DIMTXSTY'nin gösterdiği stil (Standard dışında) hiçbir yazı onu kullanmasa da proje yazı stili olarak
  alınır ve ölçü stiline `textStyle` olarak yazılır. İçe almanın aynı adlı proje stilini yeniden kullanması adla bağı
  korur.
- `.kcad` şema 186 (`SCHEMA_WITH_DIMENSION_TEXT_STYLES`): yalnız bir ölçü stili `textStyle` taşıyınca; 185 alanı
  reddeder. Bağımsız okuyucu `tools/kcad/kcad.py` ve başvuru `kcad_v2_reference.py` aynı kuralla; yeni örnek
  `dimension-text-style.kcad`, üç bozuk örnek (şema 185'te, boş ad, boşluklu ad); şemadan yeni örneği 187 oldu.
- Standart paketi: `dimensionStyleDefinitions` `textStyle`'ı, `textStyleDefinitions` ADR 1120'nin `bigFontFile`'ını
  kabul eder (önce Big Font'lu ofis yazı stili paketi reddediliyordu).
- Masaüstü Ölçü stilleri penceresinde **Yazı stili** seçimi (“Yok (yazı tipi)” ve çizimin yazı stilleri).

## 3. Kanıt

- `kentos-plot` `shx_texts::a_dimension_writes_its_value_in_its_styles_shx_font`: “SU” yazılı yatay ölçünün çizgileri
  bağımsız başvurunun “Taban ortası” durumunun (aynı sözcük, yükseklik, taban ortası) ölçünün değer noktasına
  kaydırılmışıdır (1e-9), ölçünün kalınlığında; yazı stili anılmayan ölçü eskisi gibi yüzle; ölçülen değerin rakamları
  test fontunda olmadığından `text_shx_unsupported` (U+0031) ve PDF reddi.
- `kentos-formats` `dimension_text_style`: ezdxf'in yazdığı dosya (`scripts/fixtures/dimension_text_style_dxf.py`,
  KentOS kodu yok) — OFIS → A-OLCU (romans.shx) yazı stili hiçbir yazı kullanmadan alınır, DUZ (Standard) yazı stilsiz,
  kullanılmayan A-NOT alınmaz. ezdxf'in nesne sırası çalıştırmadan çalıştırmaya değiştiği için `--check` dosyayı ezdxf
  ile geri okuyup kurulduğu değerleri denetler.
- `kentos-kcad` 242 test (yeni örnekler `expected.json`'da), `kcad_v2_reference.py --check` 1607 dosya.
- Resimler `shx-olcu-cizim` (iki ölçünün değeri SHX çizgileriyle, biri dönmüş) ve `shx-olcu-penceresi` (Yazı stili
  seçimi), bakıldı.

## 4. Sınırlar

Ölçü stili penceresinin önizlemesi değeri hâlâ yazı tipiyle gösterir. Kılavuz notu ve tablo hücresi SHX ile
çizilmez. Ölçünün kendi görünüşü (`DimensionLook`) yazı stili taşımaz; stilin değeri geçerlidir. DXF'e yazarken
DIMTXSTY'nin KentOS yazı stiline bağlanması bu dilimde yok (okuma var). Web codec'i şema 186'yı bilmez (TODOS §0.2).
