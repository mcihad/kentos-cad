# 3500 — Nokta bulutunda ters matris ve nesnenin dönüşüm geçmişi

Tarih: 2026-11-09. plan/map-cad.md MAP-13, altıncı tur. Durum: taslak. ADR 3100'ün “Sonra”sındaki “ters matris ve
nesnenin dönüşüm geçmişi”.

İlgili:
- [ADR 1800](1800-scan-registration-check.md): tarama kaydı;
- [ADR 3100](3100-apply-a-transform-to-a-point-cloud.md): bulutta dönüşüm uygulama.

## Dayanak

- **PDAL, filters.transformation** ([belge](https://pdal.io/en/latest/stages/filters.transformation.html), 9 Kasım
  2026'da okundu): matris boşlukla ayrılmış 16 sayıdır, satır satır; `invert` açıksa verilen matrisin tersi uygulanır
  (öntanımlı kapalı).
- **CloudCompare** ([Apply Transformation](https://cloudcompare.org/doc/wiki/index.php/Apply_Transformation),
  [Transformation history](https://www.cloudcompare.org/doc/wiki/index.php/Transformation_history), 9 Kasım 2026'da
  okundu):
  - pencerede bir kutu tanımlı dönüşümün mü tersinin mi uygulanacağını seçtirir;
  - dönüşüm her nesnenin geçmişine eklenir. Nesne 2.6'dan beri bir geçmiş matrisi tutar (o zamandan beri yapılanların
    bileşimi); BIN dosyasında saklanır;
  - forumda yöneticinin önerisi: eski yere dönmek için geçmiş matrisinin tersi uygulanır.

  KentOS bileşik tek matris yerine uygulanan matrislerin listesini tutar: hangi adımın ne yaptığı okunur.
- **ADR 3100 §6:** ters yapılmamıştı. Bağımsız başvurunun aynı sayıları üretmesi için tersin hesap sırası
  sabitlenmeliydi. Noktaların tam sayıları bayt bayt karşılaştırıldığından bir bitlik fark yuvarlamayı değiştirebilir.

## Karar

1. **Ters** (`Affine::inverse`, sabit sıra; Python'un float'ı da aynı sırayla aynı bitleri verir):
   - doğrusal kısmın tersi kofaktörleri bölü determinanttır, satır satır. Determinant `determinant`'ın sırasıyladır:
     m0·(m5·m10 − m6·m9) − m1·(m4·m10 − m6·m8) + m2·(m4·m9 − m5·m8);
   - öteleme, tersin her satırının ötelemeyle çarpımının eksisidir; üç çarpım sırayla toplanır;
   - son satır 0 0 0 1.

   Katı matriste devriği kullanılmaz. Hep aynı kural uygulanır; katılık sınırı (10⁻⁹) içinde ortonormal olmayan matris
   de aynı sonucu verir.
2. **Dönüşüm uygula'nın Tersini uygula'sı** (`invert`, öntanımlı kapalı): matrisin tersi uygulanır. Özet, çıktılar ve
   denetimler (ızgara, katılık, açı) uygulanan matrisindir.
3. **Nesnenin dönüşüm geçmişi** (yazılan bulut nesnesinin öznitelikleri):
   - **Kaynak:** zincirin ilk dosyası. Kaynak nesnenin kendi Kaynak'ı varsa o, yoksa onun dosyası;
   - **Dönüşümler:** ilk dosyadan beri uygulanan matrisler. Her biri 16 sayı, satır satır; sayılar Rust'ın yazdığı en
     kısa biçimde, üssüz, eksi sıfır 0 diye. Matrisler “ | ” ile ayrılır, en yenisi sondadır.

   Ters uygulandığında geçmişe uygulanan ters yazılır. Geçmiş öznitelik olduğu için `.kcad` şeması değişmez.
4. Koordinat sistemi değişmez (ADR 3100). Kaynak dosyaya dokunulmaz.
5. **Kapsam işareti:** ADR 1800'deki gibi Harita + CBS.

## Kanıt

- **Çekirdek** (`ops::transform::tests`):
  - dönen ve ötelenen matrisin tersi noktayı geri getirir;
  - ötelemenin tersi tam eksisidir; yazısı;
  - tersin tersi matrisin kendisidir (yuvarlama içinde).
- **Bağımsız başvuru** (`scripts/fixtures/pointcloud_processing_cases.py`; laspy, numpy; ters aynı formülle betikte,
  sayıların yazımı Decimal ile üssüz):
  - üç yeni durum (44): dönen matrisin tersi (LAZ), Tarama kaydı tarifinin tersi (LAS), ötelemenin tersi (verilen
    ada);
  - noktaların tam sayıları bayt bayt (FNV-1a) eşleşir;
  - çizime eklenen her bulutun öznitelikleri Kaynak ve Dönüşümler;
  - Rust ve Python (başsız bulut testleri) geçer.
- **MCP** (`crates/native/mcp/tests/cloud_transform_history.rs`):
  - döndürülen bulutun geçmişi durumunki;
  - onun üstüne ters uygulanınca üçüncü bulutun Kaynak'ı koy.laz, Dönüşümler'i iki matris; ters durumun yazdığı.
  ADR 3100'ün MCP testi değişmeden geçer.
- **Masaüstü** (`register_window_tests::tersini_uygula_takes_the_scan_back_and_the_history_grows`):
  - Buluta uygula, sonra aynı matris Tersini uygula ile taşınan taramaya;
  - geçmiş iki matris, kaynak aynı;
  - Tarama kaydı geri gelen taramada ilk kaydın başlangıç RMS'ini bulur;
  - resim `.run/gui-evidence/tersini-uygula-3500/` (Tersini uygula açık pencere).

  Tarama kaydının öbür testleri ve çapraz denetimler geçer.

## Sonra

- Çoklu tarama demeti (MAP-13'ün öbür maddesi).
- Geçmişten bir adımı geri alma (son matrisin tersini kendiliğinden uygulama).
- Gerçek veride süre ve bellek ölçümü.
- Web (TODOS §0.2).
