# ADR 0352: PDF içe aktarma

- Durum: Kabul edildi (19 Ekim 2026)
- Kapsam: plan/cad.md CAD-IO'nun PDF bölümü (§16.3) tek grupta: `CAD-IO-21` (sayfa ve bölge, katmanlar, çizgi, dolgu, yazı,
  birim ve ölçek), `22` (PDF'in taşımadığı CAD anlamı söylenir), `23` (eğrinin toleransı ve çıktı türü), `24` (yazı ve yazı
  tipleri), `25` (planda birim, kutu, katmanlar ve kayıplar; tek geri alma adımı), `26` (Python, MCP ve başsız sunucuda aynı
  davranış).
- İlgili: ADR 0351 §2 (kendi PDF okuyucumuz, PDF altlığı), ADR 0257 (DXF içe almanın komutu), ADR 0502 (GeoPackage içe
  aktarma), ADR 0259 (NURBS), ADR 0258 (kapalı çoklu çizgi), ADR 0253 (komut protokolü), ADR 0228 (web'e ara).

## Bağlam

ADR 0351 PDF'in bir sayfasını dış başvuru gibi **altlık** yaptı: sayfa çizimin içine girmez, kaynağıyla bağlı kalır, kenet ve ölçü
onun kâğıttan okunduğunu söyler. Mimarın ya da belediyenin yalnız PDF olarak gönderdiği planı düzenlemek için ise sayfanın
çizgilerinin çizimin **kendi nesneleri** olması gerekir (AutoCAD'in PDFIMPORT'u). PDF yazdırılanı taşır, çizimi değil: ölçü, kılavuz
ve tablo çizgiye ve yazıya dönmüştür, eğriler Bézier'dir, birim kâğıttır.

## Karar

### 1. Komut: `file.import.pdf`

Girdi: `path` (mutlak yol), `page` (1'den; varsayılan 1), `region` (sayfanın göründüğü hâliyle sol alt köşesinden milimetre,
iki karşı köşe; varsayılan bütün sayfa), `scale` (sayfanın basıldığı ölçeğin paydası: kâğıdın bir metresi çizimin `scale`
metresi; varsayılan 1), `at` (sayfanın sol alt köşesinin çizimdeki yeri; varsayılan başlangıç), `strokes`, `fills`, `texts`
(hangileri alınır; varsayılan hepsi), `curves` (`polyline` ya da `nurbs`), `tolerance` (kâğıtta mm; varsayılan 0,01), `skipLayers`,
`layers`, `group` (DXF içe almanın kuralı, ADR 0257), `expectedRevision`.

Sayfanın p noktası (punto, 1/72 inç) çizime `at + p · 0,0254/72 · scale` olarak gider. Çizim birimi ne olursa olsun telde ve
belgede uzunluk metredir (ADR 0165 §2).

Retler, sırasıyla: `invalid_file_path`, `invalid_revision` ve `revision_conflict`, `invalid_scale` (sonlu, 0'dan büyük, en çok
1 000 000), `invalid_tolerance` (0'dan büyük, en çok 100 mm), `invalid_pdf_region` (eni ya da boyu yok; sayfaya değmiyor),
`file_not_found`, `import_unreadable` (PDF değil, şifreli, bozuk), `pdf_page_not_found`, `layer_not_found`, `not_a_layer`,
`layer_locked`, `nothing_to_import`. Uyarı: `import_losses`.

Bütün alınanlar tek geri alma adımıdır: “PDF: ad.pdf”. `plan` dosyayı okur, alınacakları sayar, çizimdeki kutuyu, hedef katmanları,
açılacak katmanları ve kayıpları verir; hiçbir şey yazmaz (`CAD-IO-25`).

### 2. Okuyucu eğrileri saklar

ADR 0351'in okuyucusu bir eğriyi yalnız 0,01 punto içindeki noktalarıyla veriyordu. Artık her parçanın kübik parçaları da
`PdfPath.curves`'te durur: her eğrinin parçanın noktaları arasındaki başı (`at`), düzleştirmenin ondan sonra koyduğu nokta sayısı
(`n`) ve dört noktası (`c`: başı, iki denetim noktası, sonu). Altlık eskisi gibi noktaları kullanır.

### 3. Eğrinin çıktısı ve toleransı (`CAD-IO-23`)

- `polyline`: her eğri toleransla yeniden düzleştirilir: d denetim noktalarının kirişten büyük uzaklığı (kirişin boyu yoksa
  başından), `ceil(√(0,75 · d / tol))` parça (1 ile 512 arası), t'nin eşit adımlarında. tol kâğıtta mm × 72/25,4 puntodur.
- `nurbs`: uç uca eğrilerden oluşan her dizi tek bir üçüncü dereceden NURBS eğrisidir: denetim noktaları uç uca, düğümler
  `0 0 0 0, 1 1 1, …, m m m m` (m parça; Bézier'in kesin karşılığı), başladığı yerde bitiyorsa `closed`. Aradaki düz kenarlar
  çizgi ya da çoklu çizgidir; kapalı parçanın kapatan kenarı düzdür.
- Dolgu her zaman toleransla düzleştirilmiş alandır: alanın kenarı yay ya da düz olabilir, NURBS olamaz.

Daire ve yay tanıma yapılmaz: dört Bézier'le çizilmiş daire kesin daire değildir, onu daire saymak bir tahmin olurdu. Raster
sayfanın izlenmesi ya da OCR ayrı ve isteğe bağlı bir iştir; bu komut yalnız vektör ve yazı alır.

### 4. Nesneler

- Çizgi: iki noktalı açık parça çizgi, öbürleri çoklu çizgi (son noktası ilkine eşitse ya da parça kapanıyorsa `closed`). Renk
  çizginin rengi (siyah çizimin mürekkebidir: renk verilmez, katmanın rengini alır), kalınlık kâğıttaki genişliğin mm'si
  yüzlüğe yuvarlanmış (kıl çizgi kalınlıksız), kesik çizgi `dashed`.
- Dolgu: her parçası bir alan, kapanış ve yinelenen noktaları düşer, en az üç nokta; beyaz dolgu kâğıttır, alınmaz ve sayılır.
- Yazı: başı, yüksekliği × ölçek, dönüşü derece, rengi. Harf eşlemesi olmayan yazı tipinin harfleri okunamaz, sayılır
  (`CAD-IO-24`). SHX yazısı PDF'e çizgi olarak basılmışsa çizgi olarak gelir: onu yazıya çevirmek tahmin olurdu.
- Bölge: bir parça bütün noktaları (okuyucunun noktaları) içindeyse alınır, bir yazı başı içindeyse; kenarlar içeridedir.
  Dışarıda kalanlar sayılır (`left.outside`).
- Sıra: her yolun parçaları, her parçanın önce alanı sonra çizgisi; ardından yazılar.
- Katmanlar: sayfanın isteğe bağlı içerik grupları katmandır, hiçbirinde olmayan “PDF”. `skipLayers` dışarıda bırakır, `layers` var
  olan bir katmana yönlendirir, yoksa çizimdeki aynı adlı katmana (büyük küçük harf ve Türkçe işaretler bir), o da yoksa dosyanın
  adındaki gruba yeni katman (`import-<ad>`); PDF'in varsayılan görünümünde gizli grup gizli katman olur.

### 5. PDF'in taşımadığı (`CAD-IO-22`)

Çıktının `notes`'u her içe almada söyler: ölçüler, kılavuzlar, taramalar ve tablolar çizgileri ve yazılarıyla gelir, ilişkileri ve
değerleri yoktur; ölçüler kâğıttan okunur, kaynağın kesinliğini değil basılmış sayfanınkini taşır. `left` alınmayanları sayar:
resimler, gölgelemeler, okunamayan harfler, beyaz dolgular, bölgenin dışındakiler, sınırı aşan sayfa. Bunlardan biri varsa
`import_losses` uyarısı söyler.

### 6. Arayüz, Python ve MCP (`CAD-IO-26`)

- Masaüstünde CAD › Ekle › İçe aktar'da ve CBS › Veri'de **PDF içe aktar** (`file.importPdf`): dosya seçilir, pencere sayfayı,
  ölçeği, yeri (çizimden seçilir), alınacakları, eğrilerin çıktısını ve toleransını, bölgeyi, katmanları (al ya da alma) sorar;
  komutun planıyla kutuyu, sayıları ve kayıpları canlı gösterir, İçe aktar komutla yazar.
- Python `kentos.cad.file.import_pdf` ve MCP aracı katalogdan gelir; başsız sunucu kayıttan çalıştırır.
- Web'de: TODOS.md §0.2.

## Doğrulama

- Ortak durumlar: `fixtures/commands/v1/file.import.pdf.json` (40 durum), KentOS kodu olmadan `scripts/fixtures/pdf_import_cases.py`
  yazar: sayfalar `pdf_cases.py`'nin çizdiği modelden (`fixtures/pdf/v1/cases.json`), içe alma bu ADR'nin kuralıyla Python'da
  yeniden yazıldı. Çalıştırıcıda bilerek bozulan iki beklenti (bir nokta, bir sayı) yakalandı.
- Yeni örnek `egriler.pdf` (dört Bézier'li kapalı daire, çizgi–eğri–çizgi, dolgulu yaprak, beyaz dolgu, yazı); okuyucunun
  testi (`tests/all/pdf.rs`) onu da okur.
- Masaüstünün resimleri: `KENTOS_SHOTS_ONLY=pdf-ice-aktar,pdf-ice-aktar-cizim cargo test -p kentos-desktop tools_screens --
  --ignored --nocapture`.

## Sonuçlar

- PDF'ten gelen plan düzenlenebilir, ama kaynağın anlamını taşımaz: içe alma, altlığın yerini değil, üstünü tamamlar. Ölçmek
  için kaynağın DWG'si ya da DXF'i yeğlenir.
- Daire, yay ve ölçü tanıma, SHX yazısını çözme ve raster izleme ileride, kullanıcının gözden geçireceği ayrı işler olabilir.
