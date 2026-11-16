# ADR 8075: BIM ruhsat paketi — tek yayında PDF/A, IFC ve kontrol formu, dosyadan denetim

- **Durum:** ilk dilim yapıldı (16 Kasım 2026); açık işler §8'de. Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-14; sıra B14.
- **Bağlam:** ADR [1090](1090-general-2d-sheet-set-and-publication-flow.md) (yayın: hazırlık klasörü, sabitlenen çizim,
  tek adımda yerine taşıma, `yayin.json`), [8037](8037-turkish-title-block-sheet-sets-pdfa.md) (Ruhsat takımı, PDF/A-2b),
  [8065](8065-sheets-from-the-model-gate.md) (paftalar modelden), [8066](8066-2d-permit-package-pdfa.md) (2B paket kuralı,
  teslim listesi), [8074](8074-ek9-from-the-written-file.md) (EK-9 yazılmış dosyadan, IDS, iki bağımsız okuyucu);
  bim-plan K4 (doğrulama yazılmış dosyadan), K10 (Rust ölçer, Python karar verir).
- **Dayanak:** Hazırlama Yönetmeliği Md. 9/1 (model IFC; paftalar modelden üretilip PDF/A; ikisi uyumlu), Md. 15 (her
  model için EK-9 formu); e-PYS Yönetmeliği Md. 4/2 (yüklenen her dosya Hazırlama Yönetmeliğine uygun), 5/1–2 (dosyayı
  müellif kaydeder, doğruluğundan o sorumlu; eksik dosya yeniden yüklenir), 6 (e-imzasız teslimde basılı nüsha; dijital
  ile basılı aynı; taranmış PDF kullanılmaz); bim-plan Z4.

## §1. Araştırma

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Hazırlama Yön. Md. 9/1 | "CAD çizim dosyaları, BIM modelinden üretilir. BIM modeli ile CAD çizim dosyası PDF/A nüshasının birbiriyle uyumlu olması zorunludur" | IFC ile PDF/A aynı sabitlenmiş çizimden, aynı yayında üretilir; teslim listesi ikisinin aynı çizim revizyonundan çıktığını yazar |
| Md. 15, EK-9 | Teslim edilen her model için form | Form aynı yayının IFC'sinden cevaplanır (ADR 8074) ve paketin bir dosyasıdır |
| e-PYS Md. 4/4, 5/4, 5/6 | Yükleme biçimi, kimlik doğrulama ve imza yöntemi Bakanlıkça belirlenir; 15 Kasım 2026'da kılavuz yok | Paket e-PYS'nin biçimini varsaymaz: klasör, EK-2 adlı dosyalar ve teslim listesi; imza müellifin aracındadır, liste SHA-256'larla imzaya hazırdır |
| e-PYS Md. 5/1–2 | Müellif dosyanın doğruluğundan sorumlu; hatalı dosya iade edilir | Paket yayımlanmadan dosyadan denetlenir; kalan bulgu yayını durdurur (ifc-uyum §1: "ruhsat paketinde gerekli bir nesnenin ya da alanın eksikliği yayını durdurur") |
| e-PYS Md. 6 | Dijital ile basılı nüsha aynı; taranmış PDF kullanılmaz | PDF/A vektör, pafta başına dosya (ADR 8037, 8066) |
| TS EN ISO 19650-1 (bilgi kabının durumları) | Çalışma, paylaşım, yayın, arşiv | Bu paket "yayın"dır; durum kaydı BIM-15'tedir |

## §2. Karar

1. **Tek yayın:** BIM paketi, PDF/A yayınının (`cad.sheets.publish`) isteğe bağlı bir bölümüdür (`bim`): aynı sabitlenmiş
   çizimden, aynı hazırlık klasörüne paftalar, binanın IFC modeli (EK-2 adıyla), formun verisi ve teslim listesi yazılır;
   klasör tek adımda yerine taşınır. Ayrı bir komut yazılmaz: sabitleme, iptal, yarım kalmama ve önceki paketin korunması
   yayının kendi düzenidir (ADR 1090). Python ve MCP aynı komutu çağırır.
2. **Dosyadan denetim, sonra yayın.** Hazırlık klasörüne yazılan IFC dosyası diskten geri okunur (`kentos_ifc::read`),
   olguları kurulur (ADR 8074), form standart paketin kuralıyla (`dijital_proje.form`) cevaplanır. Kalan her bulgu yayını
   durdurur; klasör açılmaz, önceki paket yerinde kalır, rapor sebepleri nesneleriyle söyler.
3. **Müellifin cevabı:** formun "inceleme" soruları (dosya boyutu, gelişim seviyesi, emsal tablosu …) müellifin
   cevabını ister: yayının girdisinde soru numarası, Evet ya da Hayır ve not. Cevaplanmamış inceleme sorusu ve müellifin
   Hayır'ı yayını durdurur. Resmî cevap ile KentOS'un kanıtı ayrı yazılır.
4. **Beş sonuç ayrı** (ifc-uyum §1): dosya okunuyor (STEP), şema geçerli (okuyucunun hatası yok), Türkiye profili
   (formun kalan ve cevapsız sorusu yok), model ile pafta uyumu (türetim kuralı ve formun 21. sorusu), idare kabulü
   (bekleniyor: makine karar vermez).
5. **Teslim listesi** (`teslim-listesi.csv`) IFC'yi ve formun verisini de sayar: dosya, SHA-256, bayt, tür; `yayin.json`'un
   `bim` bölümü beş sonucu, formun satırlarını, kural kataloğunun, standart paketin ve KentOS'un sürümlerini yazar.
6. **Kuralın süreci uygulamanın değil ortak çekirdeğin:** standart paketin kuralını çağıran küçük süreç (bugün masaüstünün
   `standards.rs`'i) `kentos-python-env`'e taşınır; masaüstü, yayın ve MCP aynı yolu kullanır. Python'u olmayan
   kullanıcıda BIM paketi yapılamaz ve rapor bunu çözümüyle söyler (Ayarlar › Python); 2B paket Python'suz yapılır.

## §3. Uygulama

1. **Sözleşme** (`file.publish.sheets`, sürüm 1, eklenen alanlar isteğe bağlı): girdide `bim` (`building`: programın
   binası; `answers`: soru numarası, Evet/Hayır, not), raporda `bim` (`model`, `form`, beş sonuç `results`: anahtar,
   tutup tutmadığı, sözü; formun 21 satırı `rows`: başlık, resmî cevap, kanıt, not). Katalog özeti, TS bağları ve
   Python SDK'sı yeniden üretildi; MCP aynı katalogdan.
2. **Yayın** (`kentos-headless`, `publish_bim.rs`): paftalardan sonra, kararı vermeden önce; modelin eşlemesi
   `kentos_native_application::ifc_facts` (masaüstünün denetimiyle ortak: dosyanın olguları, kaynağın beklentisi,
   dışa aktarımın bıraktıkları), dosya hazırlık klasörüne yazılır ve oradan geri okunur (`export.memory` yanlış: teslim
   edilen dosya, EK-9/18 dosyadan cevaplanır). Kural `dijital_proje.bim.denetle(cizim, ifc, cevaplar)`; çizimin
   olguları `kentos_processing::audit_facts::cad` ve paketin dosyaları. Kuralın her bulgusu yayının hata maddesidir:
   Denetle onları gösterir, Yayınla `publish_blocked` ile reddedilir, klasör açılmaz.
3. **Dosyalar:** model teslim edilen dosyadır (teslim listesinde, "Pafta adı" sütununda "IFC modeli"). Formun kaydı
   (`kalite-kontrol-formu.json`: model adı, baytı ve SHA-256'sı, 21 satır, beş sonuç, müellifin cevapları, KentOS'un,
   katalogun, standart paketin ve şemanın sürümleri) KentOS'un kendi kaydıdır: `yayin.json`'un yanında, onun `bim`
   bölümünde özetiyle; teslim listesine ve EK-2'nin ad kuralına girmez. Olgular artık hangi kataloğa göre okunduklarını
   yazar (`catalog`: kimlik ve sürüm).
4. **Kuralın süreci** yeni `kentos-standards` crate'indedir (yorumlayıcının seçimi ve tek çağrı); masaüstünün
   `standards.rs`'i ve Python konsolunun yorumlayıcısı ona bağlanır. `kentos-python-env`'e (eklenti sisteminin de
   kullandığı) yalnız okumak için dokunulur.
5. **Masaüstü** (`permit_bim.rs`): Ruhsat takımından açılan Paftaları toplu yayınla'da "BIM modeli ve model kalite kontrol
   formu (Md. 9/1, 15)" anahtarı (açılınca PDF/A ve ayrı PDF'ler), Denetle'den sonra beş sonuç ve incelemeye kalan her
   soru için Evet/Hayır ve not. Cevap verilince rapor görünür kalır ama hazır sayılmaz: Denetle yeniden. Modelin
   bulgusu "Model" diye listelenir.
6. **Paket kuralı:** EK-2'nin ad denetimi uzantıyı alan saymaz (PDF de model de aynı şablonda).

## §4. Paketin ilk koşusunda bulunan

Paketin denetimi KentOS'un kendi yazıcısında iki hata daha buldu, ikisi de düzeltildi:

- **`Qto_BuildingBaseQuantities.FootprintArea`:** IFC 4.3'ün şablonu ve EK-6 Tablo 6.7 `FootPrintArea` yazar (formun 20.
  sorusu "özellik bu setin değil" dedi). Yazıcı düzeltildi (IFC-02'nin "FootPrintArea yazımı" maddesi).
- **`Pset_SiteCommon.FloorAreaRatio` ve `SiteCoverageRatio`:** şablon `IfcPositiveRatioMeasure` ister, KentOS
  `IfcRatioMeasure` yazıyordu; yazıcıya pozitif oran eklendi. `catalog_schema::mistyped` artık şablonda olmayan özelliği
  de bulur; uygulamanın testi kesit binasını künyesiyle ve İmar durumuyla da sınar.

## §5. Sınamalar

- **Paket kuralı** (`tests/test_bim.py`, 6): eksiksiz model ve cevaplarla temiz paket (beş sonuç, 21 Evet; incelemede
  resmî cevap müellifin, kanıt KentOS'un); cevapsız ve Hayır cevaplı inceleme; kalan form satırı; eski kesit ve modele
  girmeyen nesne model ile pafta uyumunu bozar; okuyucunun hatası şemayı bozar; 2B paket kuralı da durdurur.
  `test_paket.py`: modelin adı da EK-2 (uzantı alan değil).
- **`kentos-headless`** (`bim_package`): kesit binasının Ruhsat takımı; gerçek kural planda üç cevapsız soruyu ve zorunlu
  alanları (bağlamın gerçek kuzeyi dahil) hata sayar, beş sonuç [okunuyor, şema, model–pafta tutar; profil tutmaz;
  idare bekler]; cevaplarla yayın `publish_blocked`, klasör yok. Geçen bir kuralla (`KENTOS_STANDARDS_DIR`, sınama
  paketi) yayın bütün: model klasörden geri okunmuş, özeti teslim listesinde ve raporda aynı; formun kaydı modeli
  özetiyle adlandırır, müellifin notunu taşır; `yayin.json` kaydı özetiyle adlandırır; kayıt teslim edilen dosya değil.
- **Masaüstü** (`bim_permit_tests`): pencerede anahtar, Denetle, üç inceleme sorusu, cevaplar ve not; ikinci Denetle'de
  cevapsız soru yok, satır 1 "Evet" ve müellifin notu; 20. soru tutar; zorunlu alanlar yayını durdurur. Kesişen filtreler
  (162) geçiyor.
- **Bağımsız okuyucular** (geçen kuralla yayımlanmış paket): qpdf ve Poppler dört PDF'i geçirir, teslim listesi
  klasördeki teslim dosyalarını (dört PDF ve model) birer kez ve doğru özetle sayar (`scripts/mevzuat/pdfa_bagimsiz.py`,
  KentOS'un kayıtlarını saymaz); IfcOpenShell'in denetleyicisi (EXPRESS kurallarıyla) model dosyasında 0 bulgu; IfcTester
  aynı dosyada 28 gereklilik ve 2 eksik EK-7 sınıfı bulur: gerçek kuralın bu paketi durdurmasının sebebiyle aynı.
- **Resimler:** `.run/gui-evidence/bim-ruhsat-paketi-8075/` (açık ve koyu).

## §6. Kabul (plan: BIM-14 ilk kabul)

| Ölçüt | Durum |
|---|---|
| Bölüm binasında ve 2+1'de paket | Kesit binasında paket kuruluyor ve dosyadan denetleniyor; yayımlanamıyor: modelin zorunlu alanları eksik (yangın dayanımı, bina kimliği, gerçek kuzey …; IFC-02'nin kalanı). Geçen kuralla mekanik bütün yayımlanıyor. 2+1 sınanmadı |
| Bir pafta eski revizyondan kaldığında paket durur ve sebebini söyler | Kural düzeyinde sınandı (eski kesit `digital_derived_stale` model ile pafta uyumunu bozar); yayın düzeyinde 2B kapının sınaması (ADR 8065) aynı kuralı kullanır |
| İki bağımsız model okuyucusu ve bağımsız PDF/A doğrulayıcısı hatasız | IfcOpenShell (0 bulgu) ve IfcTester modeli okur; qpdf ve Poppler PDF'leri; veraPDF yok (ADR 8066 §8) |
| Tek komut, Python ve MCP aynı işi | `file.publish.sheets` `bim` ile; SDK ve katalog yeniden üretildi |
| İptal ve hata kısmi paket bırakmaz; önceki paket erişilebilir | Yayının kendi düzeni (ADR 1090); BIM'in bulgusu da klasörü açmaz |

## §7. Ölçüm

Kesit binasının paketi (4 pafta; model 2 962 kayıt): `bim_package` sınaması (plan, plan, yayın ve geçen
kuralla yayın) 6,8 s (hata ayıklama derlemesi, kural süreçleri dahil). 100 dairelik model için eşlemenin ve okumanın
tabanı ADR 8074 §6'dadır.

## §8. Açık işler

- **Modelin zorunlu alanları** (IFC-02'nin kalanı: sahanın TAKS/KAKS/çekmeleri, kat SSL kotu, mahal kaplamaları,
  merdivenin tipi ve malzemesi, gerçek kuzey, posta kutusu …): gerçek bir paketin yayımlanabilmesi için.
- **Formun paftası müellifin cevabını göstermiyor:** FR/MKF paftası çizimdeki tablodan basılır (KentOS'un kanıtı;
  inceleme satırı boş); müellifin cevabı formun kaydında ve `yayin.json`'da. Formun kalıcı kaynak türü (cevapların çizimde
  saklanması, dosya ya da profil değişince eskimesi) sıradaki dilimdir.
- **2+1 dairede paket**, ortak durumlar (≥ 20, bağımsız başvuruyla) ve MCP'nin uçtan uca sınaması.
- **e-imza:** liste imzaya hazır; imzanın kendisi müellifin aracında (e-PYS Md. 5/4, 6).
