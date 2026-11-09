# 3037 — Türk anteti, pafta takımları ve PDF/A

Tarih: 2026-11-09. Durum: kabul, 1. bölüm (M37; ADV-24; plan S22).

## Sorun

- **PDF/A yoktu.** Pafta yazıcısı PDF 1.7 yazıyordu: yazı yüzleri gömülü, katmanlar isteğe bağlı içerik, yazılar
  aranabilir. Ama XMP üst verisi, çıktı niyeti (OutputIntent) ve renk profili yoktu, PDF/A kimliği taşımıyordu. 2026
  dijital proje yönetmeliği ise her 2B çizimi PDF/A ister.
- **Antet projeden gelmiyordu.** `sys:cad-mimari` şablonunun antet alanları (büro, il, ilçe, mahalle, ada, parsel,
  çizim no, mimar …) her pafta açılırken ayrı ayrı soruluyordu. Boş bırakılan alan yayını durduruyordu ("“@ada”
  değerinin değeri yok"). Ölçüm, 2+1 daire (M26) üzerinde gerçek pencerede: tek paftanın yayınını 3 hata durdurdu.
- **Takım tek tek kuruluyordu.** Kat planı paftaları komutu vardı (ADR 0718). Vaziyet planı, ilk pafta bilgi tablosu
  ve hesap tabloları ise elle açılıyordu.
- **Plan M37'nin kabulü:** "Türk anteti, pafta takımları, PDF/A | Ruhsat takımı tek komutla".

## Araştırma (9 Kasım 2026)

### Türkiye mevzuatı

**Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik** (RG 5.8.2026/33331;
resmigazete.gov.tr `eskiler/2026/08/20260805-2.htm`, ekleri `20260805-2-1.pdf` taranmış 241 sayfa, resim olarak
okundu):
- **3/ı:** PDF/A, "belgelerin arşivlenmesi için farklı cihazlarda, işletim sistemlerinde ve yazılımlarda biçim
  bozulmadan görüntülenmesini sağlayan vektör tabanlı dosya formatı". Hangi bölümün (PDF/A-1, -2, -3) istendiği
  yazmıyor.
- **4/2–3:** 2B proje çizim dosyaları ve öbür belgeler PDF/A olarak teslim edilir.
- **6:** proje kodu EK-1'e, dosya adları EK-2'ye göre.
- **8/1:** paftalar A4 ve katları (TS EN ISO 216; TS 5734 ISO 9961 doğal aydınger kâğıdı). **8/2:** her çizim ve
  belge ayrı dosya; birden çok paftayı tek dosyada birleştirmek yasak. Her pafta çizimini, bilgisini ve varsa detay ve
  hesap tablolarını taşır.
- **EK-1:** proje kodu 6 hane: başvuru yılının iki hanesi ve idarenin o yıl verdiği dört haneli sıra numarası. Proje
  bilgileri: yıl, il, ilçe, mahalle, ada no, parsel no, durum (yeni ya da kaçıncı tadilat).
- **EK-2 Tablo 2.1:** dosya adı `ProjeKodu(6)_BinaKodu(2)_Disiplin(2)_AltDisiplin/Öge(4)_DosyaTürü(2)_DosyaTipi(3)_
  Kat/Konum(3)_SıraNo(2)_RevizyonNo(3)`, alanlar alt çizgiyle ayrılır.
  - Bina kodu: tek binada "00". Disiplin: Mimari "MM" (Tablo 2.2). Öge kullanılmazsa "XXXX" (Tablo 2.3).
  - Dosya türü (Tablo 2.12): PL plan, KS kesit, GR görünüş, DT detay, SM şema, MD model, FR form, LS liste,
    DK doküman, RP rapor, HS hesap, GN genel.
  - Dosya tipi, mimari (Tablo 2.13): MMC mimari çizim, MAP mimari açıklama raporu, ERP erişilebilirlik çizimi, YKG
    yangın kaçış ve güvenlik planları, CAT çatı planı ve detayları, VAZ vaziyet planı, MIR mimari hesap raporu. Her
    disiplinde: MLZ malzeme listesi, MHL mahal listesi, XXX tip yok, MKF model kontrol formu.
  - **2.8.1:** Mimari hesap raporunda TAKS, emsal, otopark, ağaç, toprak kazı ve sığınak hesapları yer alır.
  - Kat/konum (Tablo 2.14): B03, B02, B01, K00 (zemin), K01 …, NNN (kattan bağımsız). Asma kat ve çatı piyesleri gibi
    tabloda olmayan katlar "KXX". Kesit ve görünüşte müellifin verdiği üç karakterli konum.
  - Sıra no "01"den başlar. Revizyon no: revizyon yoksa "000", idare isterse "001"den artar.
- **EK-4:** katman ve sembol adları; 4.21 yazılar (TS EN ISO 3098-1, 128-2); ölçülerde Times New Roman, Arial,
  Calibri, Romans, Tahoma. Antet düzeni EK-4'te yoktur.

**Mimarlık ve Mühendislik Projelerinin Elektronik Ortamda Teslimi ve Yönetilmesi Hakkında Yönetmelik**
(`20260805-3.htm`), Md. 6: güvenli e-imzasız PDF/A teslimine ek olarak basılı teslim gerekir. E-imzalı teslimde
PDF/A yeter. Kâğıda basılıp taranan PDF kullanılamaz.

**MO Mimarlık Hizmetleri Şartnamesi, Mimari Proje Çizim ve Sunuş Standartları** (mimarist.org, 2015):
- **Pafta başlığı:** mimarın adı soyadı, unvanı, sicil no, BTB no, adresi; işin ismi, proje numarası, pafta adı,
  çizim ölçeği, çizim tarihi, pafta no, blok no; tasarlayan, çizen, kontrol eden; "M" harfi.
- **Pafta düzeni:** başlık sağ alt köşede; hâkim rüzgâr, manzara ve kuzey sağ üstte. Revizyonlar başlığın üstüne,
  aşağıdan yukarı yazılır: sıra no, açıklama, tarih, imza. Açıklamalar sağda, başlık ile revizyonların üstünde.
- **Pafta boyutları:** mesleki denetime verilen paftalar A4'e katlanabilir.
- **Sunuş sırası:** genel bilgi paftaları (proje başlığı, onay başlığı, numerotaj krokisi), vaziyet planı, yerleşim
  planı, planlar (en alttan en üste), kesitler, cepheler, sistem detayları, mahal listesi, otopark hesabı, teknik
  bilgiler.
- **Pafta numaraları:** FP, ÖP, KP, UP, SD, ID, KM, IH, MK, TM; `UP-01`, `SD-A-01`. Blok numarası pafta numarasına
  girmez.
- **Ölçekler:** vaziyet 1/1000 (1/2000, 1/500); ön ve kesin proje genellikle 1/100; uygulama genellikle 1/50 (1/100
  olabilir); sistem detayı 1/20; imalat detayı 1/1.

**PAİY 57(10):** ilk pafta bilgi tablosu. **57(27):** belediye "1/100 ölçekli, 1/50 tekniğinde" isteyebilir.
**MO Yön. 18:** mesleki denetim uygulama projesiyle yapılır.

### Başka uygulamalar (yalnız görünüş)

- Archicad 17'den beri görünüm, düzen ve yayın setlerini PDF/A olarak kaydeder
  ([Graphisoft, PDF/A Support](https://help.graphisoft.com/AC/17/INT/AC17Help/NewFeatures/NewFeatures-18.htm)); hangi
  bölüm olduğunu yazmaz.
- Revit ayrı PDF dosyalarının adını pafta parametrelerinden kurar
  ([Setup PDF Naming Rules](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-DocumentPresent/files/GUID-DE8C98E7-FF9F-4C00-9719-F57FC45D92E3.htm)).
- AutoCAD 2026'nın PDF çıktısında PDF/A seçeneği bulunamadı.

## Karar

### §1 PDF/A-2b (1. bölüm)

- **Bölüm 2, düzey B** (ISO 19005-2). Yönetmelik bölüm söylemediği için seçildi:
  - Bölüm 1 katmanları (isteğe bağlı içerik) ve saydamlığı yasaklar. CAD paftası ikisini de taşır.
  - Bölüm 2 PAdES imzasını taşır (e-PYS Md. 6'nın güvenli e-imzası).
  - Düzey U'nun koşulu (her yazıda Unicode) yazıcının yazılarında zaten var. Ama harita içeriğinde yeni bir yazı yolu
    eklendiğinde bozulabileceği için söz verilen düzey B'dir.
- **`PdfOptions.pdfa`** (`kentos_sheet::pdf::archive`):
  - XMP üst verisi: `pdfaid:part` 2, `pdfaid:conformance` B; sıkıştırılmamış.
  - Bilgi sözlüğünün başlık, yazar, konu, anahtar sözcük, oluşturan, üreten ve tarihleri XMP'de aynen tekrarlanır.
    Tarih projenin günüdür, 00:00 UTC: çekirdek saat okumaz.
  - Çıktı niyeti `GTS_PDFA1` ve sRGB ICC profili (sürüm 2.1, ekran sınıfı). Profil kodda üretilir: her hedefte aynı
    baytlar, kimsenin telif hakkı yok.
  - CMYK JPEG reddedilir (`pdfa_cmyk_picture`, resmin adıyla): dosyada CMYK profili yoktur.
  - Geri kalanını yazıcı zaten yapıyordu: gömülü alt küme yüzler, `ToUnicode`, dosya kimliği, başlık yorumu,
    adlandırılmış katman yapılandırması.
- **Kendi PDF/A-2 denetimimiz** (`kentos_formats::pdf::pdfa::pdfa2_report`), projenin PDF okuyucusuyla. Okunan
  maddeler:
  - başlık (6.1.2), sonlandırıcı ve dosya kimliği (6.1.3), XMP ve kimliği (6.6.2.1, 6.6.4), bilgi ↔ XMP eşitliği
    (6.6.2.3.1);
  - çıktı niyeti ve profil başlığı (6.2.3), CMYK resim (6.2.4.3), resim Interpolate/Alternates/OPI (6.2.8),
    grafik durumu TR/TR2/HTO (6.2.5);
  - gömülü yazı yüzü ve CIDToGIDMap (6.2.11), betik ve eylem (6.6.1), gömülü dosya (6.8), katman yapılandırmaları
    (6.9).
  Bir sertifika değildir; yazıcının dokunduğu maddeleri okur.
- **Yayın:** `file.publish.sheets`'in `pdfa` girdisi; Python `publish_sheets(pdfa=True)`, MCP aynı.
  - Her dosya yazıldıktan sonra denetimden geçer. Bozuk madde `pdfa_rule` hatasıdır ve yayını durdurur: "PDF/A'yım"
    diyen ama olmayan dosya teslim edilmez.
  - `yayin.json` ve devam kaydı `pdfa`'yı yalnız açıkken yazar: eski kayıtlar değişmeden devam eder.
  - Güncellemede eski sayfa yalnız aynı `pdfa` ile taşınır.
- **Masaüstü:** Paftaları toplu yayınla penceresinde "PDF/A-2 arşiv biçimi (e-PYS teslimi)" anahtarı, "Her pafta ayrı
  PDF"nin altında.

## Ölçüm

- **Renk profili:** Little CMS (Pillow ImageCms) üretilen profili açtı: "sRGB IEC61966-2.1", mntr, RGB, XYZ, 2.1.
  Kırmızı, yeşil, mavi, gri, beyaz ve (30, 60, 90) Little CMS'in kendi sRGB'siyle Lab'a birebir aynı çevrildi (6/6).
  2524 bayt.
- **Test** (`archive::tests`): profilin başlığı, 9 etiketi, 4 bayt hizası, eğrinin 0/1 ve orta gri (0,2147) değeri;
  XMP'nin kimliği, kaçışları ve tarihi.
- **Test** (`pdf::an_archival_sheet_reads_back_as_pdf_a_2`): ifraz paftası (yazı yüzleri, haritanın katmanları,
  coğrafi görünüm) ve yarı saydam bir PNG ile.
  - PDF/A olarak yazılıp okunduğunda bulgu 0; `/OCProperties` ve `/SMask` korunur; aynı baytlar her seferinde.
  - Aynı pafta düz yazılınca bulgular 6.6.2.1 ve 6.2.3. CMYK JPEG `pdfa_cmyk_picture` ile reddedilir; düz PDF'te
    kalır.
- **Test** (`sheet_publish::an_archival_publication_delivers_pdf_a_2_files`): ayrı dosyalı yayın. PDF/A ile her dosya
  denetimden geçer ve `yayin.json` `pdfa: true` yazar; PDF/A'sız dosyalar geçmez.
- **Takımlar:** pafta 431 + 81, PDF okuyucu 217, sözleşmeler 1369, katalog 1725, ara yüzsüz yayın 4, masaüstü yayın 6.
- **Gerçek pencere** (1920 × 1080, 2+1 daire): Paftaları toplu yayınla.
  - Önce (M36 ikilisi): PDF/A seçeneği yok.
  - Sonra: "PDF/A-2 arşiv biçimi (e-PYS teslimi)" anahtarı açılır.
  - Yayını antetin boş üç alanı durdurdu; PDF/A'dan bağımsız eski durum (§2'de çözülür).
  - Resimler: `.run/gui-evidence/mimari-m37/`.
- **Bağımsız doğrulama:** veraPDF'i (Maven Central, jbang) bu oturumda çalıştırma izni verilmedi. PDF/A'ya
  uygunluğun resmî doğrulaması veraPDF ile yapılmalıdır.

## Sonraki adımlar (M37'nin 2. ve 3. bölümleri)

- Türk anteti şablonu (MO alanları, revizyon tablosu üstte, "M" harfi), antet ve 57(10) bilgileri projeden.
- EK-2 dosya adları, "Ruhsat takımı" komutu: vaziyet planı, kat planları, hesap paftası; tek tıkla PDF/A.
- A4 katlama izleri.
