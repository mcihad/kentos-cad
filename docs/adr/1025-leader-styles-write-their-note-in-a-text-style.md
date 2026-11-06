# ADR 1025: Kılavuz stili notunu bir yazı stiliyle yazar; SHX yazı stilinde not fontun çizgileridir

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11`, ADR 1125 §3'ün sınırı “kılavuz
  notları ... yüzle çizilir”. `.kcad` şema 187. ADR 1023'ün (ölçü stili) kılavuz karşılığı.

## 1. Kaynak

AutoCAD'in MLEADERSTYLE'ı içerik yazısını bir yazı stiline bağlar (Multileader Style Manager › Content › Text style;
DXF MLEADERSTYLE grup 342, yazı stili kaydının tanıtıcısı). KentOS'ta kılavuz stili notun yüksekliğini, okunu ve
zeminini taşıyordu, yazı stilini taşımıyordu.

## 2. Karar

- `LeaderStyleDef.textStyle` (isteğe bağlı): notun yazıldığı proje yazı stili, **adıyla** (ADR 1023, 1126 gibi;
  harf büyüklüğü ayrımsız). Ad kuralı ölçü stilininki. `kentos_contracts::leader_text_style` stili bulur.
- Çizim: stil SHX dosyası adlandırıyorsa not, sol tabanından (notun yeri) ve kılavuzun dönüşünde, notun yüksekliğinde
  fontun çizgileriyle çizilir: ekran, PDF ve CPU baskısı (`shx_text::leader_note`). Notun zemini kendi kutusudur.
- Denetim: not gömülü fontla çizilemiyorsa `text_shx_unsupported` (hata; PDF reddedilir), font gömülü değilse
  adlandırılmış `text_font_substituted`. Pafta ön denetimi kılavuzları da artık sorar (önce yalnız ölçü, yazı, tablo).
- `.kcad` şema 187 (`SCHEMA_WITH_LEADER_TEXT_STYLES`): kılavuz stilleri JSON olarak saklanır; 186 ve öncesi
  `textStyle` taşıyan kılavuz stilini `unknown_field` sayar. Bağımsız okuyucu `tools/kcad/kcad.py` ve başvuru aynı
  kuralla; örnek `leader-text-style.kcad`, bozuklar `leader-text-style-{in-schema-186,empty}.kcad`; şemadan yeni bozuk
  örnek `schema-version-188.kcad`.
- Standart paketi `leaderStyles[i].textStyle`'ı kabul eder. Masaüstü Kılavuz stilleri penceresinde **Yazı stili**.

## 3. Kanıt

- `kentos-plot` `shx_texts::a_leaders_note_is_written_in_its_styles_shx_font`: “SU” notu bağımsız başvurunun “Sol taban”
  durumunun notun yerine kaydırılmışıdır (1e-9), kılavuzun kalınlığında; stil yazı stili anmazsa yüzle; fontta olmayan
  harf (A) `text_shx_unsupported` (U+0041) ve PDF reddi.
- `kentos-kcad` 242 test, `kcad_v2_reference.py --check` 1610 dosya.
- Resimler `shx-olcu-cizim` (kılavuz notu SHX ile) ve `shx-kilavuz-penceresi`, bakıldı.

## 4. Sınırlar

DXF MLEADERSTYLE'ın 342'si okunmaz ve yazılmaz; blok içindeki kılavuz notu yazı stilsizdir; web codec'i şema 187'yi
bilmez (TODOS §0.2).
