# 4400 — Atlasın güncelleme PDF'i: yalnız değişen ve yeni sayfalar

Tarih: 2026-11-10. plan/map-cad.md MAP-18, sekizinci tur. Durum: taslak. ADR 4000'in “Sonra”sındaki “yalnız değişen
ve yeni sayfaların ayrı bir PDF'e yazılması (güncelleme teslimi), sayfa numaralarıyla”.

İlgili:
- [ADR 4000](4000-atlas-update-from-its-record.md): atlasın kaynak kaydı ve önceki kayda göre değişen sayfalar;
- [ADR 1320](1320-sheet-sources-and-update-publication.md): paftaların güncelleme yayını;
- [ADR 0534](0534-atlas-pdf-pinned-revision.md): atlas, nesne başına bir sayfa.

## Dayanak

- **PDF sayfa etiketleri:** pdf-writer 0.15'in kaynak kodu (10 Kasım 2026'da okundu): kataloğun `/PageLabels` sayı
  ağacı (PDF 1.3 ve sonrası); her girdi bir sayfa sırasından başlar, `PageLabel` sözlüğü numaralama biçimini (`/S`,
  `/D` Arap rakamı) ve başlangıç numarasını (`/St`, 1 ya da büyük) verir. Görüntüleyici sayfayı bu etiketle gösterir.
  Bağımsız okuyucu pypdf'in `page_labels`'ı aynı ağacı okur (testte).
- **KentOS bugün** (ADR 4000):
  - atlasın kaydı her sayfanın kaynaklarını ve nesnelerin özetlerini tutar;
  - önceki kayda göre her sayfa `changed`, `new` ya da `same`;
  - PDF her zaman bütün atlastır.
- **Güncelleme teslimi** için yalnız değişen ve yeni sayfalar gerekir. Her sayfa atlastaki numarasını taşımalı ki teslim
  alan değişen sayfayı eskisinin yerine koyabilsin.

## Karar

1. **`file.export.atlas`'a `changedOnly`** (komut sürümü değişmez; `previous` ister):
   - PDF yalnız önceki kayda göre `changed` ya da `new` sayfaları atlasın sırasıyla tutar;
   - her sayfa atlastaki gibi numaralanır: içeriği kendi numarasını ve atlasın sayfa sayısını gösterir (“Sayfa 12 /
     100”);
   - PDF'in sayfa etiketleri aynı numaralardır.
2. **Sayfa etiketleri:** atlas sayfaları birinciden başlayıp sırayla gitmiyorsa yazılır, her sayfaya kendi numarası
   (`/S /D`, `/St`). Bütün atlasın PDF'i eskisi gibidir, etiket yazılmaz; baytları değişmez.
3. **Cevap:** `written`, PDF'in tuttuğu sayfalar (sıra numaralarıyla). Plan da söyler (plan sayfaları yapar, yazmaz).
4. **Kayıt** (`<ad>.kaynaklar.json`) yine bütün atlasındır: sayfalar ve nesnelerin özetleri. Sonraki güncelleme ondan
   söylenir. `written` kaydı da yazılır; kaydın biçimi aynı kalır (`kentos.atlas-sources/1`, ek alan).
5. **Retler:**
   - `changedOnly` önceki kayıtsız: `no_previous` (`previous`), “Yalnız değişen sayfaları yazmak için önceki atlasın
     kaydını (previous, …) verin.”;
   - değişen ya da yeni sayfa yok: yeni `atlas_unchanged` (`changedOnly`), hiçbir şey yazılmaz.
6. **Sıra:** sayfaların kaynakları ve değişiklikler PDF'ten önce bulunur (eskiden sonra), sonra yalnız seçilen sayfalar
   yapılır.
7. **Masaüstü** (Atlası PDF yap):
   - Önceki kayıt seçilince **Yalnız değişen ve yeni sayfalar** kutusu (kapalı başlar, Kaldır onu da kapatır);
   - yazıldı iletisi: “…: güncelleme, 100 sayfanın 6'sı (sayfa 1, 2, 3, …)”.
8. **Kapsam işareti:** ADR 4000'deki gibi **CAD + CBS** (pafta).

## Sınırlar

- “Değişti” sayfanın kaynaklarının değiştiğidir, görünüşünün değil (ADR 4000 §3). Yalnız çizilmeyen bir öznitelik
  değişince de sayfa güncelleme PDF'ine girer.
- Artık olmayan sayfalar (`gone`) güncelleme PDF'inde yer almaz; cevapta ve önizlemede söylenir.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/atlas_cases.py`; KentOS kodu yok), 4 yeni durum (44):
  - güncelleme PDF'i: yazılan sayfalar 2 numaralı parseli listeleyenler;
  - planda da;
  - değişmeyen atlasta `atlas_unchanged`;
  - önceki kayıtsız `no_previous`.

  Eski 40 durum bayt bayt aynı bırakıldı: bu makinenin PROJ'u iki durumun `$near` değerinin son basamağını oynatıyor,
  yeni durumlar adla birleştirilerek eklendi. Başsız koşucu 44 durumu geçer. Bozma denemesi: `written`'ın bir sayfası
  değişince koşucu yakalar.
- **Python** (`python/tests/test_atlas_update_pdf.py`), bağımsız okuyucularla:
  - pypdf: sayfa sayısı, sayfa etiketleri atlas numaraları;
  - pdftotext: her sayfada “Sayfa N / 100” ve parselin adı;
  - kayıt 100 sayfa ve `written`;
  - bütün atlasta `/PageLabels` yok.

  ADR 4000'in Python testi geçer.
- **MCP** (`crates/native/mcp/tests/atlas_update_pdf.rs`):
  - değişmeyen atlasta ret, dosya yazılmaz;
  - bir öznitelik değişince `written` durumunki;
  - kayıt bütün atlasın.

  Atlasın öbür MCP testleri geçer.
- **Masaüstü** (`exchange::atlas_update_pdf_tests`): kutu, ileti, kaydın `written`'ı durumunki. Atlas pencere testleri,
  pafta PDF testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/atlas-guncelleme-pdf-4400/`.

## Sonra

- Atlas sayfalarının bayt özetiyle “görünüşü değişti” ayrımı (ADR 4000 Sonra): [ADR 4820](4820-atlas-page-look.md)'de.
- Total station ve nivelman kalitesi (önce dengeleme doğruluklarının saklanması; MAP-18'in öbür maddesi).
- Web (TODOS §0.2).
