# 8037 — Türk anteti, pafta takımları ve PDF/A

Tarih: 2026-11-09. Durum: kabul (M37, üç bölüm; ADV-24; plan S22).

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

### §2 Türk anteti, çizimden okunan bilgiler ve Ruhsat takımı (2. bölüm)

- **MO paftası** (`sys:mimari-ruhsat`, `templates/mimari-ruhsat.json`):
  - Sağda 180 mm'lik sütun. Yukarıdan aşağı:
    - kuzey oku (MO: sağ üst);
    - açıklamalar (`@notlar`);
    - revizyon tablosu: başlığı (REV., AÇIKLAMA, TARİH, İMZA) altta, satırlar aşağıdan yukarı dolar (MO);
    - en altta pafta başlığı.
  - Pafta başlığı MO'nun saydığı alanlar ve dosya adıdır:
    - iş adı ve büyük "M";
    - yapı sahibi, il/ilçe, mahalle/pafta/ada/parsel;
    - pafta adı, ölçek, tarih, pafta no, blok no;
    - proje no ve EK-2 dosya adı;
    - müellif, unvan ve imza çizgisi, sicil no / BTB no, büro adresi;
    - tasarlayan, çizen, kontrol eden.
  - Kâğıtlar A3–A0 yatay kısıtlarla. A4 yatay ve dikey kâğıtların kendi düzeni var: dikeyde antet altta, açıklamalar
    gizli.
- **Cevapsız soru yol verir** (`expr::Scope::lookup`):
  - Paftanın boş (`null`) değişkeni kitabın ya da çizimin aynı adlı değerine, o da yoksa aynı adlı yerleşik değere
    yol verir. Hiçbiri yoksa eskisi gibi eksiktir (‹ad?›).
  - Böylece şablon alanlarını soru olarak tanımlar (galeriden açılan tek pafta sorar), Ruhsat takımı boş bırakır ve
    değerler çizimin değişkenlerinden okunur.
  - Önceden boş pafta değişkeni çizimdeki değeri gölgeliyordu: Paftaları toplu yayınla 2+1 dairenin tek paftasını
    3 boş antet alanıyla durdurdu.
- **Yeni yerleşik değerler:** `@pafta_no`, `@pafta_basligi`, `@revizyon` pafta setinin bilgisinden. Antet onları
  okur, sormaz.
- **Çizimin kendisi hakkında söyledikleri** (`Document::title_facts`, `kentos_domain::title_facts`):
  - İmar durumundan: `@arsa_alani`, `@emsal_orani`, `@taks_orani`, `@emsale_konu_alan` (katlar alanı), `@taban_alani`.
  - `@yapi_insaat_alani` ve `@toplam_yapi_insaat_alani`: bütün katların mahalleri; tek binada ikisi aynıdır.
  - Katlardan `@kat_adedi` (±0,00 ve üstü, mahali olan) ve `@bodrum_kat_adedi`.
  - Bina programından `@bb_sayisi` ve `@kullanim_amaci` (konut, ticaret, büro …).
  - Paftaya çizimin değişkenlerinden sonra gider: kullanıcının aynı adlı değişkeni önce gelir
    (`kentos_plot::inputs::drawing_variables`; ara yüzsüz yayın her seferinde hesaplar, masaüstü çizim değiştikçe bir
    kez).
- **Künye** (`permit::PARTICULARS`): 29 değişken (büronun adı ve vergi kimlik no 11 Kasım'da eklendi: IFC modelinin
  EK-7 kuruluşu, ADR 8047 §7; hiçbir pafta onları istemez, varsayılanları "—"; pencerenin Müellif grubunda). Çizimde
  EK-9 model kalite kontrol formu varsa takım onu yeniden cevaplayıp kendi paftasına (FR/MKF) koyar (ADR 8047 §10).
  Zorunlular:
  - il, ilçe, mahalle, ada, parsel;
  - yapı sahibi, taşıyıcı sistem;
  - mimar, sicil no, BTB no, büro adresi.
  Öbürlerinin varsayılanı:
  - kadastro paftası, tapu kaydı, müteahhit, denetçiler, yapı denetim, tasarlayan, çizen, kontrol eden, proje no ve
    blok no: "—";
  - mevcut yapılar "Yok", unvan "Mimar";
  - proje kodu "000000" (idareden alınır, EK-1), bina kodu "00", revizyon no "000";
  - iş adı çizimin adı.
  Künye çizimin değişkenlerine yazılır. Bu bir ayar düzenlemesidir, geri alma adımı değildir (`set_settings`).
- **Ruhsat takımı** (`kentos_sheet::permit`, `kentos_native_application::permit_set`):
  - Evre: Ön proje ÖP 1/100; Ruhsat UP 1/100 (MO Yön. 18: mesleki denetim uygulama projesiyle; PAİY 57(27)); Uygulama
    UP 1/50. Alt küme "Mimari / Ruhsat takımı".
  - Sıra MO'nun sunuş sırasıdır:
    - UP-01 Proje bilgileri: PAİY 57(10) bilgi tablosu, 57(3)'ün emsal, taban alanı ve BB sayısı, canlı pafta listesi;
      haritası ve kuzey oku yoktur, ölçeği "—";
    - UP-02 Vaziyet planı: zemin kat ve katsız nesneler (parsel), 1/500, sığmazsa 1/1000, parselin ortası;
    - planlar en alttan en üste, hepsi aynı zemini paylaşır. Nesnesi olmayan katın paftası yoktur.
  - **EK-2:**
    - Her pafta kendi tür, tip, kat ve sırasını cevaplar (`dosya_turu`, `dosya_tipi`, `kat_konum`, `sira_no`). Sıra,
      önceki bütün alanları aynı paftalar arasında sayılır (2.10).
    - Kat kodu kotundan: ±0,00 K00, üstü K01…, altı B01… (Tablo 2.14).
    - Antetteki dosya adı ile yayının adı (`permit::FILE_NAMING`) aynı ifadedir.
  - **Kâğıt:** bütün haritaların %10 payla sığdığı en küçük A3–A0 yatay kâğıt. Haritalar kilitlidir (ADR 1090 §2).
  - **Kuzey oku:** koordinat sistemi olmayan projede konmaz; CAD profili de gizler. Kuzey oku koordinat sistemi ister
    ve olmadan yayını durdurur.
  - Paftalar tek `cad.sheets.edit` değişikliğidir, tek geri alma adımı.
- **Yayın adlarında `{@ad}`** (`file.publish.sheets` `naming`):
  - Değer paftanın, kitabın, sonra çizimin değişkeninden okunur; cevapsız olan yol verir.
  - Değeri olmayan `naming_value_missing` ile reddedilir ("Proje ayarları › Değişkenler'de verin").
- **Masaüstü:** Mimari › Proje › **Ruhsat takımı** (yeni simge `permitSet`; RUHSAT, PAFTATAKIMI).
  - Pencere: evre düğmeleri; Arsa, Yapı, Müellif, Dijital teslim (EK-1, EK-2) ve Denetim gruplarında künye; yıldız
    zorunlu.
  - "Paftaları oluştur" takımı kurar ve Paftaları toplu yayınla'yı açar: takım seçili, her pafta ayrı dosya, PDF/A,
    EK-2 adı, klasör "Ruhsat-<tarih>", Denetle yapılmış. Kullanıcı Yayınla'ya basar.

### §3 Hesap paftası (3. bölüm)

- **Mimari hesap raporu** (EK-2 2.8.1: "TAKS hesabı, emsal hesabı, otopark hesabı, ağaç hesabı, toprak kazı hesabı,
  sığınak hesabı …"; PAİY 57(2)(a)): takımın son paftası.
  - EK-2 kodu HS (hesap), MIR (mimari hesap raporu), NNN.
  - MO'nun sırasıyla çizimlerden sonra gelir; MO otopark hesabını cephelerden sonra sayar.
- **Tablolar** (`kentos_interaction::calc_tables::ensure`): metrekare cetveli (kat kat bölüm çizelgesi), Emsal ve
  TAKS, Kat irtifakı, Otopark hesabı, Sığınak hesabı.
  - Çizimin canlı tablosu varsa olduğu gibi alınır; ikinci çağrı yeni tablo kurmaz.
  - Yoksa çizimin sağına, birbirinin altına, çizim ölçeğinin tablo yüksekliğiyle kurulur. Metraj ▾'nün elle koyduğu
    tabloların aynısıdır (ADR 8029–8031, 8036).
  - Çizimin dolduramayacağı tablo kurulmaz: imar durumu yoksa Emsal, bağımsız bölüm yoksa kat irtifakı, otopark ve
    sığınak.
- **Pafta:**
  - Her tabloya şablonun haritasının bir kopyası düşer: kilitli, katsız, çizim ölçeğinde, tablonun kutusuna
    ortalanmış, boyu tablonun kâğıttaki boyu.
  - Haritanın üstünde "<ad> — Ölçek 1/100" başlığı durur. Kuzey oku yoktur.
  - Yerleşim soldan sağa satırlardır (8 mm aralık, 6 mm başlık).
- **Kâğıt:** takımın kâğıdı hesap paftası da sığacak biçimde seçilir. Tablolar A0'a da sığmazsa takım hesap paftasız
  kurulur.
- **Masaüstü:** Paftaları oluştur tabloları ve paftaları tek geri alma adımında ("Ruhsat takımı") kurar.

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
- **Test, §2:**
  - `permit::storeys_codes_and_papers_follow_ek_2_and_the_maps`: kat kodları K00/K01/K02/B01/B02; 110×80 mm A3,
    400 mm genişlik A1, 1000×800 mm sığmaz.
  - `permit::a_permit_set_is_made_in_mo_s_order_with_its_file_names_fields`:
    - UP-01…04, adlar ve alt küme, EK-2 cevapları (GN/MMC/NNN, PL/VAZ/NNN, PL/MMC/K00, K01), sıra 01;
    - bilgi paftasında harita ve kuzey oku yok; haritalar katında, ölçekte (1/500, 1/100) ve kilitli;
    - ikinci takım UP-05'ten; koordinat sistemsiz projede kuzey oku yok.
  - `expr::tests`: cevapsız pafta sorusu çizimin değerini, boş değişken yerleşik ölçeği okur.
  - `templates`: yeni şablon her kâğıtta temiz; galerilerde Mimari paftanın yanında.
  - `title_facts`, `permit_set` birim testleri.
  - `permit_set_tests::a_permit_set_is_made_from_the_drawing_and_published_as_pdf_a_named_by_ek_2`: 2 katlı, 4
    daireli bina, 400 m² parsel.
    - Takım UP-01…04. 57(10) tablosu "400.00 m²", "2 (bodrum 0)", "1.50 / 0.40", "Konut", müellif ve sicil yazar.
    - Antette "260457_00_MM_XXXX_PL_MMC_K00_01_000".
    - Yayın dört dosya: `260457_00_MM_XXXX_GN_MMC_NNN_01_000.pdf`, `…_PL_VAZ_NNN_01_000`, `…_PL_MMC_K00_01_000`,
      `…_PL_MMC_K01_01_000`; hepsi PDF/A-2 kurallarından geçer.
- **Gerçek pencere, §2** (1920 × 1080; testin sakladığı bina):
  - Önce (1. bölümün ikilisi): "“ruhsat takımı” menülerde yok"; Proje panelinde düğme yok.
  - Sonra: Proje › Ruhsat takımı. Formda 13 alan gerçek fare ve klavyeyle dolduruldu; proje kodu ctrl+a ile değişti.
  - "Paftaları oluştur": altta dört pafta sekmesi; yayın penceresi kendiliğinden denetlenmiş: "4 pafta hazır; 4 PDF
    dosyası yazılacak", A3, 1/500 ve 1/100, EK-2 adları.
  - Yayınla: "4 pafta, 4 PDF yayınlandı". Teslim klasöründe dört PDF ve `yayin.json`.
  - Teslim edilen dosyalar:
    - KentOS denetimi dördünde "PDF/A-2b · uygun · 0 bulgu";
    - pdfinfo (Poppler): A3, PDF 1.7, XMP `pdfaid:part 2`, `conformance B`;
    - Ghostscript üçünü çizdi; pdftotext "MAHALLE", "Mimari / Ruhsat takımı" ve dosya adını çıkarır.
  - Resimler: `m37-ruhsat-serit.png`, `m37-ruhsat-pencere.png`, `m37-ruhsat-yayin.png`, `m37-paftalar.png`,
    `m37-teslim-pdf.png`, `m37-simge.png`.
- **Test, §3:**
  - `permit::the_calculation_sheet_shows_each_table_at_the_plot_scale`:
    - yerleşim (30, 26), (118, 26), (30, 90) mm; odadan geniş ya da uzun tablo sığmaz;
    - iki harita 80 × 50 mm, kilitli, katsız, 1/100, tablo ortasında; başlık; kuzey oku yok; HS/MIR/NNN.
  - Uçtan uca test:
    - beş tablo kuruldu, ikinci çağrı aynı beşini buldu;
    - takım UP-01…05, hesap paftasında "Otopark hesabı — Ölçek 1/100" ve EK-2 adı;
    - yayın beş PDF/A dosyası, beşincisi `260457_00_MM_XXXX_HS_MIR_NNN_01_000.pdf`.
- **Gerçek pencere, §3:**
  - Aynı senaryoyla beş pafta açıldı; kat irtifakı tablosunun genişliği yüzünden takım A2'ye çıktı.
  - Teslimde beş dosyanın beşi "PDF/A-2b · uygun · 0 bulgu".
  - pdftotext hesap paftasının tablolarını metin olarak okur ("Toplam (kesir bir üste, 8/4)").
  - Resim: `m37-hesap-paftasi.png`.
- **Bağımsız doğrulama:** veraPDF'i (Maven Central, jbang) bu oturumda çalıştırma izni verilmedi. PDF/A'ya
  uygunluğun resmî doğrulaması veraPDF ile yapılmalıdır.

## Sonraki adımlar

- Ağaç hesabı (PAİY 23/6) ve toprak kazı hesabı tabloları; hesap paftasının tablolarını birden çok paftaya bölmek.
- Takımda paftaların kendi kâğıdı (bugün takımın tek kâğıdı var; tablolar yüzünden planlar da A2'ye çıkabiliyor).
- **A4 katlama izleri:** DIN 824'ün ölçüleri (210 mm ilk katlama, 190 mm aralık) yalnız ikincil kaynaklarda okundu;
  Türk karşılığı bulunamadı. Tahminle çizilmedi.
- Çatı planı (ADV-20), kesit ve görünüşler (ADV-19) takıma kendi işleriyle girer.
- Python ve MCP için takım komutu. Revizyon satırları ve EK-2 revizyon no'nun pafta setinden yürümesi.
- Koordinat sistemsiz projede plan kuzeyi (kuzey açısıyla) kuzey oku.
