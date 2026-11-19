# KentOS Mimari — BIM geliştirme ve yönetmelik uyum planı

İnceleme: **15 Kasım 2026** (ikinci tur; ilki aynı gün, `8bc6bba77`). Kod tabanı: `8bc6bba77`. Dal: `codex/bim-ifc-uyum`.

**Eş belge:** [IFC uyum ve teslim planı](ifc-uyum.md) · **Dayanak:**
[araştırma notu](../docs/research/2026-11-15-bim-ifc-yonetmelik.md) · **Önceki işler:** [Mimari](architecture.md),
[İleri mimari](architecture-advanced.md) (M1…M51), [mevzuat araştırması](../docs/research/2026-11-07-mimari/W4-mevzuat-standart.md).

**İlerleme (15 Kasım 2026):** B1 tamamlandı: BIM-01 ve IFC-01 `[x]`
([ADR 8062](../docs/adr/8062-digital-project-rule-catalog.md)). Ekler sayfa sayfa kural kataloğunda; 17 nokta karara bağlı;
katalog IFC 4.3 şemasıyla, bağımsız dökümle ve 20 rastgele satırla sayfadan karşılaştırıldı; bugünkü çıktı dosyadan
ölçüldü; Mimari › Proje ▾ › Dijital proje kuralları penceresi. B2 tamamlandı: BIM-02 `[x]`
([ADR 8063](../docs/adr/8063-dated-digital-delivery-scope.md)): Ruhsat takımı her yükümlülüğün bu projede ne zaman, hangi
maddeyle başladığını söyler; karar ilk standart kural paketinin Python kuralıdır (`catalog/standards/dijital-proje`), 43
bağımsız durumla sınandı. B3 tamamlandı: BIM-03 `[x]` ([ADR 8064](../docs/adr/8064-ek3-ek4-2d-checks-python.md)):
EK-3 ve EK-4'ün 12 kuralı Python'da, çizimin `cad` olguları Rust'ta; bulgular Mimari denetimin Mevzuat grubunda, altısının
tek adımlık düzeltmesi var; 2+1 dairede uçtan uca ve ezdxf'li bağımsız okumayla sınandı. B4 tamamlandı: BIM-04 `[x]`
([ADR 8065](../docs/adr/8065-sheets-from-the-model-gate.md)): ruhsat yayınının kapısı eski kesit ve görünüşte, model
katmanındaki serbest çizimde ve kopmuş bağda Yayınla'yı kapatır; kural Python'da, türevin durumu olguda. B5 tamamlandı:
BIM-05 `[x]` ([ADR 8066](../docs/adr/8066-2d-permit-package-pdfa.md)): paket kuralı (PDF/A, bir dosya bir pafta, A4 ve
katları, EK-2 adı) kapıda, teslim listesi klasörde; kesit binasının ve 2+1'in paketi qpdf ve Poppler'ın bağımsız okumasından
hatasız geçti. B6 büyük ölçüde tamamlandı: BIM-06 `[~]` ([ADR 8067](../docs/adr/8067-building-3d-view-and-room-overlaps.md)):
3B görünüm binanın IFC'ye yazılan gövdelerini yolun ortak sahne çekirdeği ve çizicisiyle gösterir (kesit binası 58 eleman ve
10 mahal, dosyadaki 68 ile eşit); 3B'de tıklanan nesne planda seçilir; örtüşen mahaller EK-9/13'ün Python kuralıyla
bulgudur ve 3B'de kırmızıdır. Kalan: gerçek pencerede kabul, kesit kutusu, tadilat rengi. B7 tamamlandı (ADR 8124): BIM-07 ve IFC-02 `[x]`
([ADR 8068](../docs/adr/8068-fire-performance-material-declarations.md)): yangın dayanım sınıfı, acil çıkış, malzemenin
dayanımı ve binanın yangın koruma sınıfı dayanağıyla beyan edilir (`cad.arch.performance`), modele yazılır; modelin eksik
zorunlu alanları nesneleriyle Denetim'de (Python `zorunlu`), EK-9/9 modelden. Sıradaki B8 (BIM-08).

Bu tur **araştırma ve plandır**; uygulama kodu değişmedi. İlk taslağa göre değişenler: iki yönetmeliğin ana metni madde madde
yeniden okundu; eklerin bağımsız bir dökümü depodaki sayfa okumalarıyla karşılaştırıldı; bugünkü kod yönetmeliğin sınıf
ve alan listesine karşı ölçüldü (§3); mimari modülün zayıf yanları kanıtıyla sayıldı (§3.2); iş paketleri yasal takvime ve
bağımlılığa göre yeniden kuruldu (§5, §6). Paket numaraları [TODOS.md §18](../TODOS.md) ile aynıdır.

## 0. Özet

- **Yönetmelik ne istiyor:** ruhsat projesinin 2B paftaları pafta başına PDF/A (1.9.2027'den kademeli), mimari modeli IFC 4.3
  (1.9.2029'dan kademeli). e-PYS Yönetmeliği 1.9.2028'de yürürlüğe girer; fiilî elektronik teslim takvimini ve biçimini
  Bakanlık belirler. Çizimler **modelden üretilir**; model ile
  pafta uyumlu olmak zorundadır; her teslimde 21 soruluk model kalite kontrol formu düzenlenir.
- **KentOS nerede:** 2B teslim büyük ölçüde hazır (Ruhsat takımı, PDF/A-2b, EK-2 adları, EK-3 numarası, EK-4 katmanları).
  IFC dışa aktarma çalışıyor ve şema kurallarından hatasız geçiyor, ama mimari modelde karşılaşılan 29 zorunlu sınıfın
  15'ini yazıyor; yangın dayanımı ve acil çıkış gibi zorunlu bilgiler çizimde tutulmuyor; emsal alanları modelde nesne
  değil; form yazılmış dosyadan değil bellekteki modelden cevaplanıyor.
- **Mimari modülün zayıflığı:** binanın 3B görünümü yok; eleman sınıfları dar; tek bina; eğri geometri yaklaşık; dış model
  okunamıyor; paftanın modelden çıktığı kanıtlanamıyor (§3.2, Z1…Z14).
- **Yol:** önce kaynak (ekler sayfa sayfa kataloğa), sonra 2B teslimin kalanı, sonra ruhsat düzeyinde model (3B görünüm,
  yangın bilgisi, zonlar, eksik elemanlar, geometri, çok bina), sonra dosyadan doğrulama ve tek teslim paketi, sonra dış
  disiplinler ve belediyenin inceleme tarafı (§6).
- **IFC görüntüleyici (15 Kasım, sahibin kararı; K9):** uygulamanın içinde gelir ama gevşek bağlıdır; gerektiğinde ayrı
  program olur ya da WASM ile web'de çalışır. Okuma motoru bir denemeden geçince bağlanır. Öbür paketleri beklemez; erken
  başlayabilir (B12, B12b).

## 1. Hedef ve sınır

- **Ürün hedefi.** Mimar, bugünkü hızlı 2B araçlarla çizerken duvar, açıklık, mahal, döşeme ve öbür elemanları kimlikli yapı
  nesneleri olarak üretir. Aynı model revizyonu kat planını, kesiti, görünüşü, vaziyet planını, çizelgeleri, mimari IFC'yi,
  model kalite kontrol formunu ve PDF/A paftaları besler. Bir duvar yer değiştirdiğinde hangi mahalin, ölçünün, kaçış
  yolunun, paftanın ve teslim dosyasının etkilendiği görülür.
- **İki kullanıcı.** Proje müellifi hazırlar ve yükler; idare ve yapı denetim kuruluşu inceler (e-PYS Md. 5). Müellifin ön
  denetimi ile idarenin incelemesi aynı kural kataloğunu kullanır.
- **İlk üretim dilimi.** Konut ve ticaret+konut karma kullanımlı yeni binalar: yönetmelik önce onlara uygulanır, öbür
  binalar 1.9.2033'te gelir. Yönetmeliğin kapsamı "yeni yapılacak binalar"dır (Md. 1/2); tadilat projeleri aynı modelle
  sürer (ADR 8039).
- **Disiplin sınırı.** KentOS Mimari yalnız mimari disiplinin modelini (MM) yazar. Statik, elektrik ve tesisat modellerini
  kendi müellifleri kendi yazılımlarıyla verir; KentOS onları değiştirmez, altlık alır, hizalar ve karşılaştırır.
- **Beş ayrı kabul.** "Uygun" sözü bunlardan hangisinin sağlandığı söylenerek kurulur:
  1. dosya okunuyor (STEP);
  2. şema geçerli (IFC 4.3'ün kuralları);
  3. Türkiye teslim profili (EK-1…EK-7);
  4. model ile pafta uyumu (Md. 9/1);
  5. idare kabulü.

  Tasarımın hukuki ve mesleki uygunluğuna müellif ve idare karar verir; KentOS ölçer ve kanıt gösterir.
- **Kapsam dışı.** Statik ve tesisat tasarımı; e-imzanın üretimi (yöntemini Bakanlık belirleyecek, e-PYS Md. 5/4); resmî
  onay iddiası; web (ADR 0228; gerekenler TODOS.md §0.2'ye).

## 2. Yönetmelik ve takvim

Metinler ve ekler [araştırma notunda](../docs/research/2026-11-15-bim-ifc-yonetmelik.md) §1–§3'tedir.

### 2.1 Takvim

| Tarih | Ne başlar | Kimde | KentOS'ta hazır olması gereken |
|---|---|---|---|
| 1.9.2027 | PDF/A paftalar (Hazırlama Md. 4/2–3) | İl > 2.000.000, belediye > 600.000, toplam yapı inşaat alanı > 5.000 m² | Kapı 1: BIM-01…05 |
| 1.9.2028 | e-PYS Yönetmeliği yürürlüğe girer (Md. 8) | Fiilî yükleme kapsamı, biçimi ve tarihi Bakanlığın kararıyla (Md. 4/4) | Paket ve teslim kaydı erken hazır; resmî aktarım bu karara bağlı |
| 1.3.2029 | PDF/A paftalar | Kapsamdaki öbür binalar | Kapı 1 |
| 1.9.2029 | IFC modeli (Md. 4/5), 1. grup | İl > 2.000.000, belediye > 600.000, alan > 10.000 m² | Kapı 2 ve 3 |
| 1.9.2030 | IFC, 2. grup | İl > 2.000.000, belediye > 400.000, alan > 10.000 m² | — |
| 1.9.2031 | IFC, 3. grup | İl > 1.000.000, belediye > 300.000, alan > 10.000 m² | — |
| 1.9.2032 | IFC | Önceki IFC gruplarına girmeyen kapsam içi binalar | — |
| 1.9.2033 | Yönetmeliğin tamamı | Konut ve ticaret+konut dışındaki binalar | Kapı 5 |

Ürün hedefi Türkiye genelinde erken hazırlıktır. Önce ilk zorunlu grupları kapsayan PDF/A ve IFC teslimi hazırlanır. Aynı
işlevler, geçiş takvimi daha sonra başlayan idare ve projelerde teknik ön kontrol ve pilot için daha erken kullanılabilir.
Kapsam motoru kanuni başlangıcı proje bazında ayrı gösterir. Proje türü ve kurum profili ile ülke genelinde aynı çekirdek
kullanılır. Yasal tarihler dış sınırdır; iş sırası bağımlılıkla kurulur (§6).

### 2.2 Yönetmeliğin yazılımdan istedikleri

✓ var · ◐ kısmen · ✗ yok.

| # | İstenen | Dayanak | KentOS'ta bugün | Paket |
|---|---|---|---|---|
| R1 | Her pafta ayrı PDF/A dosyası; A serisi kâğıt | Md. 4/2, 8 | ✓ Ruhsat takımı, PDF/A-2b (ADR 8037); paket kuralı, teslim listesi ve qpdf/Poppler ile bağımsız doğrulama (ADR 8066); veraPDF yok | BIM-05 |
| R2 | Çizim dışı belgeler PDF/A | Md. 4/3 | ◐ hesap ve liste paftaları var; dış belge pakete girmiyor | BIM-05 |
| R3 | Proje kodu ve dosya adları | Md. 6; EK-1, EK-2 | ✓ pafta ve model adı (ADR 8037, 8047 §7) | BIM-01 |
| R4 | CAD katmanı, sembol, çizgi; mahal numarası ve adı | Md. 7; EK-3, EK-4 | ✓ katman anahtarı (ADR 8008), EK-3 numarası (ADR 8027) ve 12 kurallı denetim (ADR 8064); sembolün içi (4.17.5), punto ve kesit hattı yok | BIM-03 |
| R5 | Çizimler modelden üretilir; model ile PDF/A uyumlu | Md. 9/1; EK-9/21 | ✓ kesit ve görünüş türev (ADR 8042–8043), ruhsat yayınının kapısı (ADR 8065); EK-9/21'in cevabı formda (IFC-08) | BIM-04, BIM-14 |
| R6 | IFC teslimi | Md. 4/5; EK-5 §5.4 | ◐ IFC 4.3 dışa aktarma (ADR 8047) | IFC-02…06 |
| R7 | Zorunlu sınıflar ve alanları; vekil sınıf kullanılmaz | Md. 12; EK-5, EK-6 | ◐ 29 sınıftan 15'i; yangın ve dayanım alanları yok | BIM-07…09; IFC-02, IFC-03 |
| R8 | Proje bilgileri, kişiler, koordinat sistemi | Md. 13; EK-7 | ◐ künye, mimar, büro, TUREF dilimi var; proje tipi ve açıklaması yok | IFC-02, IFC-05 |
| R9 | Model, mahal ve varlık adları | Md. 10, 11; EK-5 §5.5 | ◐ kalıp var; iki yorum açık | BIM-01, IFC-02 |
| R10 | Gelişim seviyesi | Md. 14; EK-8 | ◐ gövdeler dosyadan ölçülüyor (EK-9/7, ADR 8133); ayrıntının yeterliliği müellifin | BIM-06, BIM-10, IFC-06 |
| R11 | Model kalite kontrol formu | Md. 15; EK-9 | ◐ 21 soru bellekteki modelden | IFC-08, BIM-14 |
| R12 | Bilgi yönetimi | Md. 4/4; TS EN ISO 19650-1/2 | ◐ teslim kaydı, nesneye bağlı iade gerekçesi ve öncekiyle karşılaştırma (ADR 8099); künye role göre (ADR 8100); ortak veri ortamının durumları yok | BIM-15 |
| R13 | Tarihe ve koşula bağlı kapsam | Md. 1; Geçici 1–2 | ✓ Ruhsat takımının Dijital teslim kapsamı, Python kuralı (ADR 8063); MCP yok | BIM-02 |
| R14 | e-PYS: müellif yükler, idare inceler; e-imza ya da basılı; taranmış PDF kullanılmaz | e-PYS Md. 4–7 | ◐ vektör PDF/A var; paket, kayıt ve inceleme yok | BIM-14, 15, 18 |

## 3. Bugünkü durum (15 Kasım 2026)

### 3.1 Güçlü taban

- **Model nesneleri:** duvar (katmanlı, birleşimli), kapı/pencere/boşluk (tip ve örnek), mahal, döşeme, aks, kolon, kiriş,
  çatı, korkuluk, merdiven, rampa; katlar ve düşey model (ADR 8040–8041, 8044–8045).
- **Türevler:** kesit ve görünüş modelden, eskime işaretiyle (ADR 8042–8043); çatı planı; çizelgeler ve metraj.
- **Türkiye'ye uygun proje:** PAİY alanları, emsal ve TAKS, kat irtifakı, vaziyet planı, otopark ve sığınak hesapları (ADR
  8029–8032, 8036); mevzuat denetimi ve kaçış yolları (ADR 8033–8035); Ruhsat takımı, Türk anteti, PDF/A-2b (ADR 8037–8038).
- **Dijital proje:** EK-4 katman anahtarı (ADR 8008), EK-3 mahal numarası (ADR 8027), EK-2 dosya adları, EK-9 formu, IFC 4.3
  bina modeli ayrı pakette (`kentos-ifc`, ADR 8047), eski plandan tanıma (ADR 8048).
- **Ortak altyapı:** komut kataloğu, Python ve MCP; bağımsız başvurular; gerçek pencere kabulü (ADR 8000); proje bulutu ve
  dosya revizyonları (ADR 0031); CBS tarafı (parsel, plan, koordinat sistemi); yolun 3B sahnesi ve çizicisi (ADR 0910).

### 3.2 Mimari modülün zayıf yanları

Önem: K uyum için şart, Y yüksek, O orta.

| # | Zayıflık | Kanıt | Önem | Paket |
|---|---|---|---|---|
| Z1 | **Binanın 3B görünümü yok.** Model yalnız planda, kesitte ve dışa aktarılan dosyada görülüyor; gövde hatası, mahal örtüşmesi, yanlış kot karşı programda fark ediliyor | 3B pencere yalnız yol sahnesini açıyor (`apps/desktop/src/shell/scene3d_window.rs`); bina gövdelerini kesit modeli kuruyor, gösteren yok. **16 Kasım 2026:** büyük ölçüde giderildi (ADR 8067): bina 3B'de, örtüşen mahal bulgu; kesit kutusu yok | K | BIM-06 |
| Z2 | **Yangın ve performans bilgisi tutulmuyor:** yangın dayanım sınıfı, acil çıkış, malzeme dayanımı, yapım yöntemi, yangın koruma sınıfı | Sözleşmelerde alan yok (`crates/shared/contracts/src/arch`); EK-9'un 9. sorusu "hayır" (ADR 8047 §9). **16 Kasım 2026:** büyük ölçüde giderildi (ADR 8068) | K | BIM-07 |
| Z3 | **Emsal alanları modelde nesne değil;** bağımsız bölüm modelde grup olarak durmuyor | EK-9/10 ve EK-6 6.60 mekânsal zon istiyor; yazılmıyor | K | BIM-08 |
| Z4 | **Paftanın modelden üretildiği kanıtlanamıyor:** plan paftasındaki serbest çizim ayırt edilmiyor; IFC ile PDF/A'nın aynı revizyondan çıktığı kayıtlı değil | EK-9/21 "modele girmeyen nesne yoksa evet" (`quality.rs`); Ruhsat takımı ile `cad.arch.ifc` ayrı komutlar | K | BIM-04, BIM-14 |
| Z5 | **Eleman sınıfları dar:** giydirme cephe, kaplama ve asma tavan nesnesi, baca, dikme yok; asansör, vitrifiye ve mobilya sınıfsız blok; rampanın gövdesi yok | `Entity` listesi (`contracts/src/core/entity.rs`); `arch.rs` `Why::NotYet`; ADR 8045 | Y | BIM-09 |
| Z6 | **Geometri her yerde gerçek değil:** ~~eğri duvar düz parçalarla~~ (yay yay olarak, miktarlar kaynaktan: ADR 8071); ~~eğri merdiven düz parçalarla~~ (ADR 8071 §9); ~~mahalin hacmi düz prizma~~ (tabanından tavanına: ADR 8071 §7; çatı altında çatıya kadar: §11); ~~açıklık gövdeden kesilmiyor~~ (Reference View: ADR 8071 §4); ~~içe dönen saçak yok~~ (ADR 8071 §10); ~~çatı katmanları yok~~ (ADR 8071 §8) | ADR 8047 §6; `arch.rs` (mahal `bottom + height`); ADR 8044 "sonraya" | Y | BIM-10 |
| Z7 | **Çok bina (kapandı, ADR 8072):** program bina ve kanat tanıyor (kodu, kullanım türü, ±0.00'ı ve ortak katın sayıldığı binası); bina başına IFC, metraj ve ruhsat takımı; parselin plan kararları | `building.rs` tek saha, tek bina; `program.rs` | Y | BIM-11 |
| Z8 | **2B kuralların denetimi yarım:** EK-4 katmanı ve çizgi tipi, EK-3 numara biçimi ve mahal adı sözlüğü denetlenmiyor | ADR 8047 §9 "yalnız varlığı"; M9 "profil ve blok katmanı kaldı" | Y | BIM-03 |
| Z9 | **Mevzuat denetimi yarım:** başvuru tarihine göre kural seti, plan notu katmanı, yangın dayanımları, erişilebilirlik, emsalin 22. madde sınırları | ADV-22 ve ADV-23 "Kalanlar" | Y | BIM-13 |
| Z10 | **Dış model okunamıyor:** statik ve tesisat modeli altlık olamıyor, çakışma bulunamıyor, iki revizyon karşılaştırılamıyor; belediye gelen IFC'yi KentOS'ta açamıyor | Depoda IFC okuyucu yok | Y | IFC-07, IFC-12, IFC-09; BIM-17, BIM-18 |
| Z11 | **Tip sistemi yarım:** duvar stili ve kapı/pencere tipi var; döşeme, kolon, kiriş, çatı, merdiven, korkuluk tipsiz | W2 §2.1; IFC'de tip nesnesi ve katmanlı malzeme takımı yok | O | BIM-12 |
| Z12 | **Teslim kaydı (kısmen kapandı, ADR 8099, 8100):** ~~hangi revizyon, hangi dosyalarla, kime verildi, ne cevap geldi~~ çizimde kayıtlı; ~~künyenin role göre görünürlüğü~~ (ADR 8100); ortak veri ortamının durumları kaldı | Dosya revizyonu var (ADR 0031); teslim ve iade kaydı Mimari verisi 17'de | O | BIM-15 |
| Z13 | **Hız hedefi tutmadı:** 2+1 daire gerçek pencerede 82 etkileşim ve 121,3 s (hedef ≤ 80 ve ≤ 100 s) | `architecture-advanced.md` §7; ADR 8052 | O | BIM-20 |
| Z14 | **Kalanlar ve kabul:** ADV paketlerinin çoğu `[~]`; gerçek ofis DWG'siyle tadilat kabulü sahibin dosyasını bekliyor; örnek proje, çokgen oda, kot etiketi, 1:20 kasa profili açık | `architecture-advanced.md` §3–§5 | O | BIM-20 |

### 3.3 Ölçülen başlangıç değerleri

| Ölçü | Bugün | Kaynak |
|---|---|---|
| IFC'ye yazılan zorunlu sınıf (mimari modelde karşılaşılan 29'dan) | 15 | `crates/shared/ifc/src/arch.rs`; araştırma notu §5 |
| EK-9'un sorularından yazılmış dosyadan kanıtlanan | 0 / 21 | `quality.rs` bellekteki modeli okur |
| EK-6 ve EK-7'nin zorunlu satırlarından dosyada değeri olan | künyeli bölüm binası 81 / 121; künyesiz 46 / 95 | ADR 8062 §10, `fixtures/arch/v1/ifc/katalog-olcum.json` |
| Kural kataloğu | 1 870 satır, her biri sayfasıyla; 27 karar (6 açık); IFC 4.3 şemasıyla 235 fark, hepsi kararlı | ADR 8062 |
| EK-9'da bugün ek kanıt gerekenler | 9 "hayır" (zorunlu özellikler); 10 elle (emsal zonu yazılmıyor); 19 "evet" deniyor ama çatı ve merdivenin tipi `NOTDEFINED` | ADR 8047 §9; araştırma notu §4–§5 |
| Şema kuralları, bağımsız okuyucu | bölüm binasında 0 bulgu | ADR 8047 |
| Türkiye profilini sınayan bağımsız okuyucu | yok | — |
| 3B'de görülebilen bina gövdesi | 0 (IFC'de 68) | Z1 |
| Yangın dayanımı taşıyabilen nesne türü | 0 | Z2 |
| Ruhsat takımı | tek komut, PDF/A-2b, pafta başına EK-2 adlı dosya | ADR 8037 |
| 2+1 daire, gerçek pencere | 82 etkileşim, 121,3 s | ADR 8052 |

## 4. Kararlar

- **K1 — Model tek kaynaktır.** KentOS'ta model, kimlikli yapı nesneleri ve katlardır; plan onun kendi görünüşü, kesit ve
  görünüş türevi, IFC aynı kesit modelinin gövdeleridir. Yeni olan bunun kanıtıdır (BIM-04) ve tek teslim girdisidir
  (BIM-14).
- **K2 — Kural veridir.** Sınıf, alan, ad kalıbı, sözlük, eşik ve tarih kodda sabit durmaz; kaynak sayfasıyla sürümlü
  katalogdadır. İdarenin ek tanımları (Md. 11/3, 12/2, 13) ve Bakanlığın sonraki kararları (e-PYS Md. 4/4) veri
  değişikliğidir. IFC'ye ait katalog `kentos-ifc`'tedir. 2B ekler, ortak veri modeli (kaynak, zorunluluk, karar, profil
  katmanı) ile tarih ve kapsam kuralları ayrı crate'tedir: `kentos-digital-project` (ADR 8062 §3). Çizimi ölçen mevzuat
  kuralları mevcut kural motorundadır (ADR 8033).
- **K3 — Bilinmeyen, yanlış değildir.** Bilinmiyor, uygulanmaz, hayır, sıfır ve eksik ayrı durumlardır. Yangın dayanımı
  gibi bir değer varsayılanla doldurulmaz. Her performans değerinin kaynağı görünür: tip, örnek, türetim, müellif beyanı,
  belge.
- **K4 — Doğrulama yazılmış dosyadan yapılır.** Formun cevabı bellekteki modelden değil, diske yazılan IFC ve PDF/A'dan
  okunur. `kentos-ifc` bunun için bir okuyucu kazanır. Okuyucu dosyanın olgularını çıkarır ve denetim olgularının bir
  bölümü olarak verir: nesneler, sınıflar, adlar, ön tanımlı tipler, özellik ve miktar setleri, malzeme, mekânsal yapı.
  Kuralları K10'un Python betikleri uygular. Aynı okuyucu dış disiplin modeline ve idare incelemesine hizmet eder.
  Okuyucunun motoru K9'da; ürünün motoru bağımsız başvuru sayılmaz, bağımsız okuyucular ürünle dağıtılmaz.
- **K5 — Gerçek ruhsat modeli dış servise gönderilmez.** Doğrulama çevrimdışıdır; yalnız açık ve sentetik örnekler dış
  doğrulayıcıda sınanır.
- **K6 — Kimlik kalıcıdır.** Nesnenin kimliği, tipinin kimliği ve pafta ya da mevzuat etiketi ayrı alanlardır. Taşıma ve tip
  değişikliği kimliği korur; kopya ve modül örneği yeni kimlik alır; bölme ve birleştirmede eski–yeni kaydı tutulur; silip
  geri almak nesneyi ilişkileriyle geri getirir.
- **K7 — Genel bir katı modelleme çekirdeği şimdi alınmaz.** Ruhsat modelinin gövdeleri bugünkü kesit modeliyle (prizma, kol,
  eğik yüz) ve ona eklenecek yay ve kesilmiş gövdeyle kurulur. Çekirdek kararı (TODOS.md §23) ölçülen bir ihtiyaç çıkınca
  ADR ile yeniden açılır.
- **K8 — Bir madde, bütün yüzleriyle.** Her paket `architecture-advanced.md` §6'nın talimatıyla yapılır: araştırma, ADR,
  çekirdek ve sözleşme, komut (katalog, ≥ 20 bağımsız durum, Python, MCP), arayüz kapıları, gerçek pencere kabulü, resimler.
- **K9 — IFC görüntüleyici uygulamanın içindedir ve gevşek bağlıdır** (sahibin kararı, 15 Kasım;
  [ADR 8061](../docs/adr/8061-ifc-viewer-and-reading-engine.md)). Saf çekirdeği yerel ve wasm32 için derlenir, arayüz
  bileşeni kendi paketindedir, uygulamayla dar bir yüzden konuşur. Gerektiğinde hızlıca ayrı program olur ya da WASM ile
  web'de çalışır; ikisi de şimdi yapılmaz. Dosyayı okuyan ve gövdeleri üçgene çeviren motor IFClite'ın Rust çekirdeğidir
  (MPL-2.0): yalnız `kentos-ifc`'in içinde, sabit sürümle, denemeden geçerse. Üçgen ağ gösterimdir; ölçü kaynağı
  değildir. K7 değişmez: bu, okunan dosyanın gösterimi içindir, KentOS'un kendi gövdeleri için bir katı modelleme
  çekirdeği alınmamıştır.
- **K10 — Denetim kuralları Python betikleridir: Rust ölçer, Python karar verir** (sahibin kararı, esneklik için; ADR 8057
  §36, [eklenti planı](eklenti.md) ADIM 20, ADR 8062 §14).
  - **Kapsam:** yönetmeliğin denetimleri uygulamaya gömülmez: EK-9 formunun kanıtı, Türkiye profili, EK-3 ve EK-4,
    kapsam kararı, mevzuat kuralları. Hepsi ortak Python betiğidir (`@ext.audit`). Yeri eklenti planının ADIM 20'de
    öngördüğü `catalog/standards/`; dizin henüz yok, ilk betikle kurulur. Masaüstünde ve bulut işinde aynı betik çalışır.
  - **Rust yalnız ölçer ve okur:** çizimin olguları (`facts.project` ve mimari bölümü, ADR 8057 §36, §39), yazılmış
    dosyanın olguları (K4), kural kataloğu ve kararlar betiğe veri olarak gider. Alan, çakışma, kapalılık, kot ve kaçış
    Rust'ta bir kez, aynı kodla hesaplanır; betik geometriyi yeniden hesaplamaz.
  - **İdarenin ek kuralı** bir betik ve bir profil katmanıdır; uygulama yeniden derlenmez.
  - **Bugün Rust'ta olanlar taşınır:** dijital proje bulguları (ADR 8047 §9, `digital_audit.rs`) ve EK-9 cevapları
    (`kentos_ifc::quality`) IFC-08'de betiğe geçer. ADR 8033–8035'in mevzuat kuralları BIM-13 dokundukça taşınır; yeni
    kural Rust'a yazılmaz.

## 5. İş paketleri

Her paket bir bütündür; alt maddeleri aynı dilimde biter. `[ ]` başlanmadı, `[~]` sürüyor, `[x]` bütün kabulü sağlandı.

### 5.1 Kaynak ve kapsam

#### BIM-01 — Birincil kaynak ve kural kataloğu [x]

**Yapıldı (15 Kasım 2026, [ADR 8062](../docs/adr/8062-digital-project-rule-catalog.md)):** 1 870 kural satırı (EK-1…4 ve 8
`kentos-digital-project`'te; EK-5…7 ve 9 `kentos-ifc`'te), her biri ek, tablo, satır ve PDF sayfasıyla;
`scripts/mevzuat/katalog.py` transkripsiyondan üretir (`--check`). 27 karar (17 nokta, 2B ve şema kararları; 6'sı açık).
Profil katmanları (temel → kurum → proje) çekirdekte. Bağımsız dökümle 244 satır/tablo karşılaştırıldı, 16 fark sayfadan
kapandı. Rastgele 20 satır 20'si birebir. Şema ve döküm karşılaştırması transkripsiyonda 14 okuma hatası buldu; düzeltildi.
Başlangıç ölçümü künyeli bölüm binasında: 121 zorunlu satırın 81'inde değer var (§3.3). Pencere: Mimari › Proje ▾ › Dijital
proje kuralları; Mimari denetim bulguları ek ve sayfayı söyler. **Kalan:** 2+1 örneğinin ölçümü (depoda model dosyası
yok; örnek proje BIM-20), Python ve MCP için salt okuyan sorgu komutu.

**Kullanıcı sonucu:** Her denetim bulgusu ve dışa aktarılan her alan, yönetmeliğin hangi ekinin hangi sayfasından geldiğini
söyler. İki okumanın ayrıştığı yerler kapanmıştır. [IFC-01](ifc-uyum.md#10-iş-paketleri) ile aynı dilimdir.

- Eklerin okunabilir metni [mevzuat/bim/](../mevzuat/bim/README.md) altındadır (15 Kasım 2026, OCR; EK-1…5, 8, 9 ve
  bütün kod tabloları sayfa görüntüsüyle karşılaştırıldı). Katalog bu metinden kurulur; okunan her satır kataloğa
  sayfası, tablosu ve satırıyla girer: EK-2'nin sözlükleri (2.2–2.14), EK-3 (3.1 ve 3.2'nin yaklaşık 150 adı), EK-4'ün
  katman ve çizgi kuralları, EK-5 (5.1, 5.2, 5.3), EK-6'nın mimariye düşen tabloları, EK-7, EK-8 (LOD 300 hükmü; nesne
  tablosu yok), EK-9. EK-6 ve EK-7'nin uzun açıklamalarına dayanan satır PDF sayfasıyla bir kez daha karşılaştırılır.
- Araştırma notu §4'teki 17 nokta tek tek karara bağlanır: sayfadan okunan, şemadan çözülen, idareye sorulan.
- Katalog bağımsız dökümle satır satır karşılaştırılır; fark listesi ADR'ye yazılır.
- **Başlangıç ölçümü:** bölüm binasının ve 2+1 örneğinin bugünkü çıktıları katalog satırlarına karşı dosyadan ölçülür
  (satır başına var ya da yok).

**İlk kabul:** katalogdaki her satırın sayfası var; rastgele seçilen 20 satır sayfa görüntüsüyle birebir; ölçüm raporu ADR'de.

#### BIM-02 — Tarihli kapsam kararı ve proje bağlamı [x]

**Yapıldı (15 Kasım 2026, [ADR 8063](../docs/adr/8063-dated-digital-delivery-scope.md)):** tarih, eşik ve 15 girdi
`kentos-digital-project`'in `data/kapsam-2026.json`'unda, her biri maddesiyle. Karar `catalog/standards/dijital-proje`
paketinin Python kuralı (`dijital_proje.kapsam.karar`; KentOS'un ilk standart kural paketi, `kentos kek verify` temiz).
Olguların yeni `permit` bölümü künyeyi, çizimden ölçülen toplam yapı inşaat alanını ve katları verir. Eksik girdi her aday
değeriyle denenir: sonucu değiştirebiliyorsa "teyit gerekli". Ruhsat takımında "Dijital teslim kapsamı" bölümü ve tablosu;
denetimde `kapsam` kuralı; Python'da `kentos.standards.kapsam`. 43 bağımsız durum (metinden elle) geçiyor. Okuma kararları
K23–K28 katalogda. B1'in penceresinin masaüstüne yalnız test bağımlılığıyla bağlandığı hata düzeltildi. **Kalan:** MCP
aracı (çağırıcı `kentos-python-env`'e taşınacak), bulut işi (eklenti planı ADIM 20), nüfusun kaynağı ve hangi belediye
(K27, idare).

**Kullanıcı sonucu:** Ruhsat takımı penceresi, bu proje için hangi yükümlülüğün hangi tarihten başladığını maddesiyle
söyler (PDF/A, IFC, e-PYS). Bilgi eksikse "teyit gerekli" der, tahmin etmez.

- **Girdiler (künyede):** başvuru tarihi ya da kamuda ihale kararı/tarihi; il ve işlemi yapan belediye; ikisinin nüfusu,
  nüfusun kaynağı ve yılı; kullanım (konut, ticaret+konut, öbürü); toplam yapı inşaat alanı ve kat adedi (çizimden);
  müstakil olup olmadığı; tarım/hayvancılık ve 3194'ün 4, 26 ve 27. maddeleri.
- **Kural:** Hazırlama Md. 1/2, Geçici 1–2; e-PYS Md. 1/2, 4/4, 8. Yürürlük ve yetkili makamın fiilî yükleme kararı
  ayrı girdilerdir. Sonuç dört durumdur: uygulanır, uygulanmaz (maddesiyle),
  henüz başlamadı (tarihiyle), teyit gerekli.
- Geçici 1/1-b'nin son cümlesi (IFC başlayınca 4/2–3 uygulanmaz) ile Md. 9/1 (modelden üretilen çizimler PDF/A) ayrı
  satırlardır; tek bir bayrak PDF/A'yı kapatmaz.
- Nüfus yönetmelikte bir yıla bağlanmamıştır: kullanıcı girer, kaynağı kaydolur.
- Karar bir Python kuralıdır (K10). Tarihler ve eşikler katalogda veridir, girdiler künyeden olgu olarak gelir. Ruhsat
  takımı, Python ve MCP aynı kuralın sonucunu okur.

**İlk kabul:** ≥ 20 bağımsız durum (her eşiğin tam eşiti, altı ve üstü; her tarihin önceki günü ve kendisi; müstakil 200 m²
ve iki kat; karma kullanım; kamu ihalesi). Ülke geneli test verisi, ilk ve sonraki IFC gruplarını, kalan grubu, konut dışı
ertelemeyi ve bir e-PYS uygulama kararının farklı başlangıç tarihini kapsar. Nüfus veya fiilî uygulama için gereken karar
eksikse ilgili sonuç "teyit gerekli" olur.

### 5.2 2B teslim

#### BIM-03 — 2B kuralların denetimi: EK-3 ve EK-4 [x]

**Yapıldı (16 Kasım 2026, [ADR 8064](../docs/adr/8064-ek3-ek4-2d-checks-python.md)):** `catalog/standards/dijital-proje`'nin
`ek34` kuralı (paket 0.2.0): mahal numarasının kalıbı, kat kodu, sırası ve saat yönü; mahal adının Tablo 3.2'deki yazımı;
katman adı, "0" katmanı, tarama katmanı, emsal katmanı, aks çizgisi, ölçü yazısı ve blok adı. Olgular `kentos.auditFacts`'ın
yeni `cad` bölümünden; kural verisi katalogdan `scripts/mevzuat/ek34_veri.py` ile. Bulgular Mimari denetimin Mevzuat
grubunda, "Dijital Proje Yön. 7" ve ekin maddesi ve sayfasıyla; kat kodu, sıra, saat yönü, mahal adı, katman adı ve aks
çizgisinin Düzelt'i tek adım. 2+1 dairenin izinde 10 kural iki durumda, blok adı yalnız kalan, tarama yalnız geçen; 12
kural paketin testinde iki durumda; ezdxf'li bağımsız okuma 9 kuralda aynı. Kararlar K29 (Arimo), K37 (tadilat eki), K38
(kat kodu). **Kalan:** nesnenin kendi çizgi tipi, Python bulgusunun istisnası, kat kodunun iki kuralının birleşmesi (K38),
blok adı düzeltmesi, 4.17.5, Tablo 4.7 ve 4.28.

**Kullanıcı sonucu:** Denetim, paftaya girecek çizimde EK-4'e uymayan katmanı, çizgi tipini ve yazıyı, EK-3'e uymayan mahal
numarasını ve adını, nesnesini seçerek söyler. Dayanak Md. 7; Z8.

- **EK-3:** numara kalıbı; kat kodunun mahalin katıyla uyuşması; kat ve bölüm içinde sıranın tekliği ve saat yönü; adın
  Tablo 3.2'nin sözlüğünden ya da kısaltmasından geldiği. Mahal adı seçicisi sözlükten beslenir.
- **EK-4:** katman adı kalıbı ve sözlüğü; nesne türünün doğru katmanda durduğu; çizgi tipi, ölçü ve yazı kuralları; blok
  katmanı ve profil (M9'un kalanı).
- DXF ve DWG aynı katmanlarla gider (ADR 8046).
- Bulgular denetim sekmesinde "Dijital Proje Yön." grubundadır; düzeltmesi tek adımdır.
- Kurallar Python betiğidir (K10). Rust çizimin katman, blok, yazı stili, çizgi tipi ve mahal numarası ile adı
  listelerini olgu olarak verir.

**İlk kabul:** 2+1 örneğinde her kural için bir geçen, bir kalan durum; bağımsız başvuru KentOS kodu olmadan sınar.

#### BIM-04 — Paftanın modelden üretildiğinin kanıtı [x]

**Yapıldı (16 Kasım 2026, [ADR 8065](../docs/adr/8065-sheets-from-the-model-gate.md)):** `cad` olgusuna katmanın nesneleri
sınıfıyla (model, açıklama, serbest), kesit ve görünüşlerin durumu, kopmuş etiket ve ölçü; Python kuralı `dijital_proje.turetim`
(paket 0.3.0): eski türev, kaynağı silinmiş türev, model katmanında serbest çizim, kopmuş bağ; hepsi hata. Ruhsat takımından
açılan yayında kapı: bulgu varken Yayınla kapalı, pencerede "Ruhsat kapısı" bölümü ve Denetim'de göster. Aynı bulgular
Mimari denetimin Mevzuat grubunda; eski kesitin Düzelt'i Yeniden çiz. Listeler canlıdır, eskimez (ADR 8038); kaynağın
revizyonu `yayin.json`'da. Kesit binasında: kapı değişince kapalı, Yeniden çiz'le açık; duvar katmanındaki çizgi
kimliğiyle. **Kalan:** Python bulgusunun istisnası, EK-9/21'in bu kayıttan cevabı (IFC-08), pafta düzeyinde sınıflama,
büyük çizimde ölçüm, PDF'in kendi metaverisinde revizyon.

**Kullanıcı sonucu:** Her ruhsat paftası hangi model revizyonundan üretildiğini taşır. Modelden gelmeyen çizim paftada ayrı
listelenir. Eski kesit, görünüş ya da çizelgeyle ruhsat yayını yapılamaz. Dayanak Md. 9/1; EK-9/21; Z4.

- **Türev kaydı:** kesit ve görünüş blokları, çizelgeler, hesap tabloları ve kontrol formu üretildikleri belge revizyonunu
  taşır. Kesit ve görünüşün "eski" işareti var; ötekilere genişler.
- **Pafta içeriği sınıflanır:** model nesnesi; modele bağlı açıklama (ölçü, etiket, kot); detay çizimi (kaynağı ve amacı
  kayıtlı); serbest çizim. Yapı katmanlarında duran serbest çizim bulgudur ("duvar katmanında 14 serbest çizgi").
- **Yayın kapısı:** eski türev, kopmuş ölçü ya da etiket, yapı katmanında serbest çizim varsa ruhsat paketi durur.
  Gerekçeli istisna kayıtlıdır (mevcut istisna düzeni, ADR 0732).
- Formun 21. sorusu bu kayıttan cevaplanır.

**İlk kabul:** bir kapı değişince kesit, görünüş, doğrama listesi ve mahal listesi "eski" olur ve paket durur; yeniden
üretimden sonra geçer. Duvar katmanına çizilen serbest çizgi raporda nesnesiyle görünür.

#### BIM-05 — 2B ruhsat paketi (PDF/A) [x]

**Yapıldı (16 Kasım 2026, [ADR 8066](../docs/adr/8066-2d-permit-package-pdfa.md)):** Python kuralı `dijital_proje.paket`
(paket 0.4.0): PDF/A seçilmemiş, bir dosyada birden çok pafta, A4 ve katları dışında kâğıt (K39), EK-2'ye uymayan ad;
ruhsat yayınının kapısı `ruhsat` (= `turetim` + `paket`) ile Denetle'nin raporu gelince sorulur. PDF/A yayını
`teslim-listesi.csv` yazar (dosya, SHA-256, pafta numarası ve adı, revizyon, model revizyonu ve özeti). Bağımsız okuma
`scripts/mevzuat/pdfa_bagimsiz.py` (qpdf, Poppler): kesit binasının 7 ve 2+1'in 5 dosyası geçer; yanlış ad, iki sayfa ve
gömülmemiş yazı tipi sebebiyle reddedilir. Kural KentOS'un kendi çıktısında kesit paftasının "B-B" konumunu yakaladı:
artık "BBX". **Kalan:** çizim dışı belgeler (yayının girdisine belge listesi: sözleşme değişikliği), veraPDF, kapıda
taranmış sayfa, PDF/A bölümünün kurum profilinden okunması.

**Kullanıcı sonucu:** Ruhsat takımı tek adımda, bağımsız denetimden geçmiş, adları EK-2'ye uyan PDF/A dosyaları ve bir teslim
listesi üretir. IFC zorunluluğu başlamamış proje için teslim budur. Dayanak Md. 4/2–3, 8; e-PYS Md. 6.

- **Bugün var:** PDF/A-2b, pafta başına dosya, EK-2 adı, künye, kendi denetimi (ADR 8037).
- **Eklenen:**
  - üretilen baytlarda bağımsız PDF/A doğrulaması;
  - vektör içerik ve gömülü yazı tipi denetimi (taranmış görüntüden pafta reddedilir, e-PYS Md. 6/4);
  - A serisi kâğıt ve "bir dosya, bir pafta" denetimi (Md. 8);
  - teslim listesi: dosya adı, özeti, pafta numarası, revizyonu, üretildiği model revizyonu;
  - atomik yayın: yarım paket kalmaz, önceki paket durur;
  - çizim dışı belgeler (rapor, hesap) pakete PDF/A denetimiyle eklenir.
- PDF/A'nın istenen bölümü ve düzeyi kurum profilinin bir satırıdır (yönetmelik yalnız "PDF/A" diyor).

**İlk kabul:** 2+1'in ve bölüm binasının paketi bağımsız doğrulayıcıdan hatasız geçer. Bilerek bozulan dosya (gömülmemiş
yazı tipi, yanlış ad, iki paftalı dosya) paketi durdurur ve sebebini söyler.

### 5.3 Ruhsat düzeyinde model

#### BIM-06 — Binanın 3B görünümü [~]

**Yapıldı (16 Kasım 2026, [ADR 8067](../docs/adr/8067-building-3d-view-and-room-overlaps.md)):** `scene3d.view` yol
projesi dışında binayı açar: IFC dışa aktarımının modeli (`arch_ifc::model`) `kentos_ifc::mesh` ile üçgenlenir (açıklıklar
duvardan oyulur, mahaller hacim), yolun penceresi `Source` girdisiyle genelleşti (ikinci motor yok). Sınıf rengi, yarı
saydam mahal, planda seçili turuncu, Denetim'de seçili bulgunun nesnesi kırmızı; kat süzgeci, mahaller aç ve kapa; tıklama
mahalin içinden geçer, nesneyi planda seçer; model değişince sahne yenilenir. Örtüşen mahal: Rust `cad.roomOverlaps`'ı
ölçer, `dijital_proje.kalite` 0,01 m² ve üstünü hata sayar (EK-9/13). Kesit binası: 58 eleman ve 10 mahal, IFC dosyasında
68 gövde. Şeritte Mimari › Proje › 3B görünüm.

**Görüntüleyicide (18 Kasım 2026, [ADR 8119](../docs/adr/8119-building-3d-in-the-model-viewer.md), K242–K251):** 3B görünüm
görüntüleyici bileşeninin kendisidir: solda ağaç ve arama, sağda özellikler, sahnede kesitin kartı (düzlem ve seçimin
çevresinde kutu), gizle ve yalnız bırak, sürüklenen bölmeler; şeridin sonunda Katlar ▾. Model dışa aktarımın o an
yazacağı IFC'dir: bellekte yazılır, okuma yüzüyle gövdeleriyle okunur, ayrı iş parçacığında kurulur. Çizim ya da altlıklar
değişince yeniden kurulur, görünüm yerinde kalır. Planla seçim, Denetim bulguları ve altlık modeller (menekşe, ağaçta
“(altlık model)”) sürer. Kesit binası bugün 58 eleman, 10 mahal ve 4 emsal zonu: dosyadaki 72 gövdeli nesne. Emsal zonu
katının altında; sahne dar olunca sığdırma sahnenin biçimine göre. **Kalan:** gerçek pencerede (işletim sisteminin
faresiyle) kabul, tadilat durumuna göre renk, katlar arası örtüşme, büyük modelde yalnız değişeni yeniden okumak.
Çizim kalitesi ([ADR 8120](../docs/adr/8120-3d-render-quality.md)): kenar yumuşatma, kenar çizgileri ve ortam gölgesi;
Uygulama ayarlarından Hızlı, Dengeli, Kaliteli. Ölçme ve katlanan bölmeler
([ADR 8121](../docs/adr/8121-viewer-measuring-and-folding-panes.md)): uzaklık, açı ve düz yüzün alanı, köşe ve kenar
yakalamasıyla; yan bölmeler kenar şeridine katlanır. 3B görünüm her zaman ayrı işletim sistemi penceresindedir
([ADR 8122](../docs/adr/8122-3d-windows-always-apart.md)). Mahalleri çatıya yükselen katın emsal zonu çatının alt
yüzüyle kesilir; görüntüleyicide zonlar kendiliğinden gizlidir ([ADR 8123](../docs/adr/8123-far-zone-under-the-roof.md)).

**Kullanıcı sonucu:** Mimar, modelin gövdelerini (dışa aktarılacak olanların aynısını) 3B'de görür. Bir nesneye tıklayınca
planda seçilir. Katları ayırır, kesit kutusuyla içine bakar. Denetim bulguları 3B'de boyanır. Dayanak Md. 14; EK-9/13–15; Z1.

- **Kaynak:** kesit modelinin gövdeleri, yolun 3B görünümünün kullandığı ortak sahne çekirdeği ve çizicisiyle
  (`kentos_scene3d`, `kentos_render_wgpu::scene3d`). İkinci bir 3B motoru kurulmaz.
- **Görünüş:** sınıfa, kata, tadilat durumuna ve bulguya göre renk; mahaller yarı saydam hacim.
- **Etkileşim:** yörünge, kaydırma, yaklaşma; hazır bakışlar; kat süzgeci; kesit kutusu; 3B'de tıkla → planda seç, planda
  seç → 3B'de vurgula.
- Model değişince sahne yenilenir. 2B proje açılışı 3B'yi yüklemez (kılavuz §20).
- Çizimi değiştirmez: görünümdür, komut değildir.
- **Ortak parçalar:** görüntüleyici çekirdeği ve pencere parçaları (ağaç, özellikler, kat süzgeci, kesit kutusu, gizle ve
  yalnız bırak) IFC görüntüleyiciyle aynıdır ([IFC-12](ifc-uyum.md#10-iş-paketleri), K9). Çekirdek kaynağını bilmez: burada
  kesit modeli, orada IFC dosyası aynı girdiyi doldurur. Hangisi önce yapılırsa parçaları o kurar.

**İlk kabul:** bölüm binasının 68 gövdesi 3B'de; gövde sayısı dışa aktarılan dosyadakiyle eşit; 3B'de tıklanan duvar planda
seçili; örtüşen iki mahal kırmızı; gerçek pencerede resim.

#### BIM-07 — Yangın, performans ve malzeme bilgisi [x]

**Yapıldı (16 Kasım 2026, [ADR 8068](../docs/adr/8068-fire-performance-material-declarations.md)):** beyan ve dayanağı
(`Declared`, `DeclaredBasis`: müellif ya da belgenin adı ve günü; `DeclaredExit`) nesnede (`arch.fireRating`, açıklığın
`fireRating`'i, merdivenin ve kapının `exit`'i), duvar stilinde, kapı-pencere tipinde, malzemenin `strength`'inde ve
binanın `performance`'ında; nesnenin değeri tipinkini geçer, varsayılan yok. `cad.arch.performance` v1 (42 bağımsız durum);
`.kcad` mimari verisi 1. IFC'de FireRating, FireExit, ConstructionMethod (EK-6 Tablo 6.7: inşaat eyleminin türü, künyenin
yapı türünden), FireProtectionClass, NetPlannedArea, `Ad_Dayanım`. Öznitelikler'de Yangın dayanımı ve Acil çıkış, Duvar
stilleri'nde sınıf alanı. Eksik zorunlu alanlar `zorunlu` kuralıyla Denetim'de; bir stilin sınıfı silinince duvarları
bulguda. **Yapıldı (18 Kasım 2026, [ADR 8124](../docs/adr/8124-b7-remaining-declarations-and-ifc-fields.md)):** Kapı ve
pencere tipleri penceresinde yangın dayanım sınıfı ve dayanağı, Proje ayarları › Mimari › Yangın güvenliği'nde binanın
yangın koruma sınıfı ve dayanağı (malzemeninki ADR 8091'de); dayanak tek satır ("Belge (2026-02-28)", boş: müellif
beyanı) ve Öznitelikler'de "Dayanak belgesi"; türetilenin gerekçesi ("katın dış çizgisinde olduğu için dışarıda",
"çerçevenin kolonu olduğu için taşıyıcı") ve örnekte değiştirme (duvarın, döşemenin `arch.external`'ı, kolonun
taşıyıcılığı; `.kcad` mimari verisi 20); acil çıkış önerisi (kaçış yollarının çıkış saydığı kapılar ve merdivenler
Denetim'de, düzeltmesi `cad.arch.performance` `exits` ile tek adımda; beyan yolla uyuşmazsa uyarı). Kaplama kalınlıkları ve
asansör kapasitesi ADR 8070'te, formun 9. sorusu ADR 8076'da yapılmıştı. Ek-3 olgularının dış duvarı çerçeveli katta da
modelinkiyle aynı (yüzün ortası). **Kalan:** gerçek pencerede kabul (B6'nın düzeneğiyle).

**Kullanıcı sonucu:** Duvar stili, kapı ve pencere tipi, döşeme, kolon, kiriş, çatı ve merdiven yangın dayanım sınıfını;
kapı ve merdiven acil çıkış rolünü; taşıyıcı malzeme dayanım sınıfını taşır. Eksik olan, Denetim'de nesnesiyle listelenir;
hiçbiri varsayılanla dolmaz. Dayanak EK-6 6.3, 6.7, 6.16, 6.20, 6.25, 6.52, 6.56, 6.58, 6.61, 6.64, 6.66; EK-9/9; Z2.

- **Alanlar:** yangın dayanım sınıfı (Binaların Yangından Korunması Hakkında Yönetmeliğe göre); acil çıkış (evet, hayır,
  bilinmiyor); malzemenin dayanımı ("Beton_C30" biçiminde yazılır); binanın yapım yöntemi ve yangın koruma sınıfı;
  planlanan net alan; kaplama kalınlıkları; asansörün kişi ve yük kapasitesi.
- **Kaynağı:** tipte ya da örnekte. Her değerin dayanağı saklanır: müellif beyanı, belgenin adı ve tarihi, türetim.
- **Acil çıkış önerisi:** kaçış yolları motorunun (ADR 8035) kullandığı kapılar ve merdivenler aday gösterilir; müellif
  onaylar. Beyan ile rota uyuşmazsa bulgu.
- Türetilen değerler (dışarıda, taşıyıcı) açıklanır ("katın dış çizgisinde olduğu için dışarıda") ve örnekte değiştirilir.
- Yeni `.kcad` alanları; düzenleme komutlarında alanlar; Öznitelikler ve tip pencereleri; çizelge sütunları.

**İlk kabul:** bölüm binasında bütün zorunlu yangın alanları dolunca formun 9. sorusu evet. Bir duvar stilinin sınıfı
silinince o stilin duvarları bulguda. Girilmemiş değer dosyaya boş metin olarak yazılmaz; eksik sayılır.

#### BIM-08 — Mahal, zon ve emsal modeli [x]

**Yapıldı (16 Kasım 2026, [ADR 8069](../docs/adr/8069-room-zone-far-model.md)):** balkonların sınırı (PAİY 22/1-l, resmî
metinden: bağımsız bölüm net alanının %20'si; ADR 8030'un okuması düzeltildi), aşan kısım emsale ve tabloda kendi satırına;
her kat ve sayılış biçimi için emsal zonu (`IfcSpatialZone`, `TREpys_EmsalOzellikSeti.EmsalDurumu` DAHIL/HARIC, dayanağı
Description'da; balkonun dahil kısmı açıklamada, K41); bina programının bölümleri `IfcZone` ile mahallerini gruplar. İki
bölümlü örnekte zonların emsale dahil alanı tablonun emsale esas alanına eşit (bağımsız başvuru). **Kalan:** katlar arasında
yer değiştiren alanın denetimi (teslim edilen dosyada, IFC-08), eğik çatı altında kesilmiş mahal hacmi, mahalin türü,
elle düzeltilen emsal durumu, kot aralığı denetimi. **Yapıldı (18 Kasım 2026,
[ADR 8125](../docs/adr/8125-room-emsal-state-type-and-storey.md)):** mahalin kendi emsal durumu (Öznitelikler'de Emsal
durumu ve Emsal dayanağı; `cad.spaces.edit` `emsal`; `.kcad` mimari verisi 21) ve emsal artışı (DİĞER: dayanağıyla,
katlar alanına ve %30'a girmeyen kendi toplamı, tabloda satırı, modelde DIGER zonu açıklamasıyla); mahalin türü (otopark
PARKING, dış alan EXTERNAL, öbürü INTERNAL); doğru katta (EK-9/13: döşemesi
katının kot aralığında olmayan mahal Denetim'de); eğik çatı altında kesilmiş mahal hacmi ADR 8071 §11 ve 8123 ile
yapılmıştı. **Yapıldı (18 Kasım 2026, [ADR 8126](../docs/adr/8126-far-zones-storey-by-storey.md)):** formun 10. sorusu
zonları dosyada kat kat ve durum durum emsal tablosuyla karşılaştırır (zonun katı başvurusundan, alanı gövdesinin
izinden, payı float32 sınırından); genel toplam tutsa da yer değiştiren alan ve yanlış durum kalır; teslim edilen model
de böyle okunur. **Kalan:** ikinci okuyucunun zon izi; mevzuat paketinin DİĞER'i okuması; gerçek pencerede kabul.

**Kullanıcı sonucu:** Emsal ve TAKS tablosu ile modeldeki alanlar aynı çokgenlerden gelir. Her emsal alanı dahil, hariç ya
da öbürü diye maddesiyle işaretlidir. Bağımsız bölüm ve ortak alan modelde gruptur. Mahalin hacmi kapalıdır ve üstü doğru
yerde biter. Dayanak EK-6 6.58, 6.60, 6.68; EK-9/10, 12–14; PAİY 4, 5/8, 20–22; Z3.

- **Emsal alanı:** kat kat kapalı çokgen; emsal durumu (dahil, hariç, öbürü) ve dayandığı madde. Bugünkü mahal
  sınıflamasından (ADR 8030) türer, elle düzeltilir. Tablo bu çokgenlerden kurulur. IFC karşılaştırması kat ve emsal
  sınıfı bazında yapılır. Dahil, hariç ve diğer alanlar birbirini telafi edemez (IFC-03, EK-9/10).
- **Mahalin türü:** iç, dış (balkon, teras, açık alan), otopark; kullanımından.
- **Mahalin hacmi:** tabanı döşemenin üstü; üstü tavan, döşeme altı ya da çatı yüzü. Eğik çatı altında kesilir. Galeri ve
  dubleks tek mahaldir.
- **Çizimde denetim (dışa aktarmadan önce):** aynı katta örtüşen mahal (gerçek çokgenle), kapanmayan mahal, katının kot
  aralığında olmayan mahal, numarası yinelenen mahal.
- **Gruplar:** bağımsız bölüm, eklenti, ortak alan (bina programı, ADR 0773 ve 8031); yangın kompartımanı (BIM-13; ADR 8088'de
  yapıldı: paketin kompartımanları `IfcZone`).

**İlk kabul:** iki bağımsız bölümlü örnekte emsale dahil alanların toplamı tablonun emsale esas alanına eşit (bağımsız
başvuru). Genel toplam eşit kaldığı hâlde iki kat arasında yer değiştiren alan veya yanlış emsal sınıfı denetimden kalır.
Balkonun %20 sınırını aşan kısmı dahil alana geçer. Kırma çatı altındaki mahalin hacmi prizmadan küçük ve kapalı.

#### BIM-09 — Eksik yapı elemanları [x]

**Yapıldı (16 Kasım 2026, [ADR 8070](../docs/adr/8070-missing-building-elements.md)):** rampanın gövdesi (eğik kollar ve
sahanlıklar; kesit, görünüş, 3B, `IfcRamp`; tipi kollarından); merdivenin IFC tipi ve malzemesi; niş ve duvar bacası
kesitte; baca (`IfcChimney`: duvar bacası ve üstündeki çatı bacası tek eleman), niş (RECESS); aks ızgarası (`IfcGrid`);
asansör, vitrifiye ve mobilya kitaplık bloğunun kartında yapı sınıfı ve gövdesi (asansör kapasitesiyle); mahalin kaplama
kalınlıkları (EK-6 Tablo 6.20'nin birinci yolu, `IfcCovering` yazılmaz: K44); giydirme cephe (çizgisi, yüksekliği, en
büyük dikme ve kayıt aralığı, çerçevesi, paneli; planda, kesitte, görünüşte, 3B'de; `IfcCurtainWall` gövdesi panelleri,
dikme ve kayıtları `IfcMember` MULLION: K45; Giydirme cephe aracı GC, `.kcad` mimari verisi 3); dikme ve kayıt (kolonun
ve kirişin eleman rolü, kolonun kendi yüksekliği; `IfcMember` POST ve USERDEFINED "Kayıt"; Dikme DK ve Kayıt KY araçları,
`.kcad` mimari verisi 4: K46); giydirme cephe doğrama listesinde (Kapı ve pencere çizelgesinin salt okunur satırı),
dikme Kolon çizelgesinde; dışa aktarmanın rampa, baca, giydirme cephe, eleman, donatı ve aks sayıları; **ilk kabul:**
rampalı, giydirme cepheli, asansörlü, dikmeli örnek binada kayıp listesi boş (ADR 8070 §9); kaplama ve asma tavan
(döşemenin kaplama rolü: mahale bağlı türer ya da elle çizilir; yapı değildir; `IfcCovering`; Kaplama aracı KM, `.kcad`
mimari verisi 5: K47, §10); Kiriş ve Kaplama çizelgesi (kayıt ve kaplama da bir çizelgede; `.kcad` mimari verisi 6,
§11). Kaplamanın mahale bağı 18 Kasım'da (`IfcRelCoversSpaces`: mahale bağlı ya da iç noktası mahalde duran kaplama,
[ADR 8127](../docs/adr/8127-covering-covers-its-room.md)); duvar kaplamasının kalınlığı ADR 8076 K57 ve 8124 K281'de.
Merdiven ve rampa korkuluğu 18 Kasım'da ([ADR 8128](../docs/adr/8128-stair-and-ramp-railings-in-section-and-ifc.md)):
kesitte, görünüşte ve 3B'de küpeşte ve çubuklar, yükseklik kolda basamak burunlarının çizgisinden, rampada yüzeyden,
sahanlıkta döşemeden 1,10 m; IFC'de her yanın korkuluğu merdivenin ya da rampanın parçası bir `IfcRailing` BALUSTRADE;
dikey çubuklu korkuluğun çubukları IFC gövdesinde. Giydirme cephede panel yerine kapı ve pencere 18 Kasım'da
([ADR 8129](../docs/adr/8129-doors-and-windows-in-curtain-walls.md)): açıklık kapsadığı hücreleri doldurur (kapı döşemeye
iner, üstü istenenden alçak olmaz), ızgara değişince yeniden oturur; planda cam ve iç dikme orada yok; IFC'de kapı ve
pencere giydirme cephenin parçası. Eğimli eleman 18 Kasım'da ([ADR 8130](../docs/adr/8130-sloped-members.md)): elemanın
`rise`'ı (ucu başından yüksek, iki noktalı eksen; `.kcad` mimari verisi 22), derinliği eksenine dik eğik gövde, IFC'de
`Pset_MemberCommon.Slope`; Kayıt aracında ve Öznitelikler'de Yükselme. **B9 bitti.**
**Sonraya (iyileştirme):** dönen basamaklı ve eğri merdivenin korkuluğu; giydirme cephede kapı başı için ızgaradan ayrı
kayıt; kırık eksenli eğimli eleman.

**Kullanıcı sonucu:** Konut ve karma kullanımlı bir binada karşılaşılan her eleman modelde kendi nesnesidir ve kendi
sınıfıyla dışa gider. "Modele girmeyen nesne" listesi boşalır. Dayanak EK-5 Tablo 5.1; EK-6 §6.1.3 ve 6.14, 6.20, 6.21,
6.35, 6.40, 6.49, 6.53, 6.64; Z5.

- **Rampa:** gövdesi (eğik döşeme, sahanlıklar) kesit modelinde; kesitte, görünüşte ve 3B'de.
- **Giydirme cephe:** çizgisi, yüksekliği, dikme ve kayıt aralıkları, panelleri; planda, görünüşte, kesitte; doğrama
  listesinde.
- **Kaplama ve asma tavan:** mahalin kaplamalarından ve asma tavanından (ADR 8038, 8040) türeyen nesne; elle de çizilir.
- **Baca:** çatı bacası (ADR 8044) ve duvardaki baca tek yapı elemanı.
- **Asansör, vitrifiye, mobilya:** kitaplık bloğunun yapı sınıfı ve gövde ölçüleri. Sınıfı olmayan blok yapı elemanı
  sayılmaz.
- **Dikme ve benzeri** (pergola dikmesi, cephe kaydı).
- Temel ve öbür statik elemanlar yazılmaz; statik modelden altlık gelir (BIM-17).
- Her araç: komut, katalog, ≥ 20 ortak durum, Python, MCP; kendi ikonu, şerit yeri, kısa adı, terim sözlüğü.

**İlk kabul:** rampalı, giydirme cepheli, asansörlü örnek binada dışa aktarmanın kayıp listesi boş. Her yeni eleman planda,
kesitte, 3B'de ve çizelgede.

#### BIM-10 — Geometri doğruluğu [x]

**Yapıldı (16 Kasım 2026, [ADR 8071](../docs/adr/8071-geometry-accuracy.md)):** gerçek yay: duvarın ve döşemenin
(kaplamanın, mahalin) yayı kesit modelinde kaynaktan tutulur, IFC'de `IfcIndexedPolyCurve`'ün `IfcArcIndex`'iyle
yazılır, eğri duvardaki açıklık yüzlerin yayını izler; birleşimde yay kendi çemberinde uzar; 3B ve kesit kendi adımıyla
böler. Hacim ve alan kaynaktan: duvar, döşeme, kolon, kiriş ve kaplamanın EK-6 miktar setleri (IFC 4.3'ün tanımlarıyla:
duvarın brüt hacmi birleşimsiz kendi gövdesi, neti yazılan gövde eksi açıklık ve girintiler; yaylar kapalı formla).
Kapalılık denetimi (`kentos_ifc::closed`); rampa sahanlığındaki art arda aynı köşe giderildi. **İlk kabul:** eğri
cepheli, iki farklı kotlu örnekte (`fixtures/arch/v1/curved-building.kcad`) bütün gövdelerin hacmi bağımsız kapalı
formlara 10⁻⁹ içinde eşit, açıklıklı duvarın neti dolu eksi açıklık, bütün gövdeler kapalı; IfcOpenShell 0 bulgu, kendi
geometrisiyle hacimler 5,8·10⁻⁷ içinde. Açıklığı kesilmiş gövde (ADR 8071 §4): tek kesme işlevi net hacmi, 3B'yi ve
IFC'nin Reference View çıktısını verir (Dosya › Dışa aktar › IFC dışa aktar (bina, kesilmiş gövde)…; `cad.arch.ifc`
`view: reference`). Döşemede boşluk ve şaft (ADR 8071 §5): boşluk mahali kendi katının döşemesini keser; IFC'de döşemenin
deliği ve boşluğun parçası döşemenin açıklığı. Eğimli döşeme (§6): eğim oku, Öznitelikler'de Eğim ve Eğim yönü; kesit,
3B, IFC (PitchAngle) ve metraj eğimi okur, altındaki duvar ona göre biter. Kapalı mahal hacmi (§7): IFC mahali bitmiş
tabanından tavanına, yükseklik ve hacimleriyle. Çatının katmanları (§8): kaplama, lata, su yalıtımı, kaplama tahtası,
ısı yalıtımı, mertek üstten aşağı, yüze dik kalınlıkla, her biri EK-2 öğesinde; kesitte ayrı plaklar, IFC'de
`IfcMaterialLayerSet`, Öznitelikler'de "Çatı katmanları" (`.kcad` mimari verisi 8); kesitte yalıtım katmana oturan
zikzakla. Döner merdiven (§9): her basamak gerçek yaylı bir halka parçası, merkez direği silindir; IFC'de SPIRAL_STAIR,
IfcOpenShell'de hacmi kapalı formla 1,04·10⁻⁶ içinde. İçe dönen saçak (§10): L, T biçimli çatının yüzleri ağırlıklı
düz iskeletten, iç köşeden dere; kesit, plan, 3B ve IFC onu okur. Çatı altındaki mahal (§11): üstünde döşeme olmayan
mahal çatının alt yüzüne kadar, parçalarıyla; hacmi kapalı formdan, Height yok. Bütün maddeler ve ilk kabul tamam.
**Sonraki ayrıntılar** (ADR 8071 "Sıradaki adımlar"): çatının yüz başına `IfcSlab`'e ayrılması, mahalin tavan alanları ve
Net Çevre'si, eğik tavanlı mahalin yükseklik kuralı, döner merdivenin helis alt yüzü.

**Kullanıcı sonucu:** Kesit, görünüş, 3B ve dışa aktarılan model aynı gövdeyi gösterir ve o gövde çizilen yapıyla aynıdır:
eğri duvar eğridir, açıklık duvardan kesilmiştir, döşemenin boşluğu boştur. Dayanak Md. 14; EK-8; EK-9/14; Z6.

- Yay duvar ve döner merdiven gerçek yayla; parça sayısı görüntünün işidir, kaynağın değil.
- Açıklığı kesilmiş duvar gövdesi (IFC-06'nın iki çıktısından biri bunu kullanır).
- Döşemede boşluk ve şaft; eğimli döşeme; çatının katmanları ve içe dönen saçak (ADR 8044'ün kalanı).
- Her gövde kapalı ve kendini kesmeyen bir katıdır.
- Hacim ve alan kaynaktan hesaplanır; üçgenlenmiş görüntü ölçü kaynağı değildir (kılavuz §23.3).

**İlk kabul:** eğri cepheli ve farklı kotlu örnekte gövdelerin hacmi bağımsız başvuruya eşit (yay için kapalı formül).
Açıklıklı duvarın hacmi, dolu hacim eksi açıklıklardır. Bütün gövdeler kapalı.

#### BIM-11 — Çok bina, ortak bodrum ve vaziyet [x]

**Yapılan (16 Kasım 2026, [ADR 8072](../docs/adr/8072-buildings-shared-basement-location.md) §1–§5):** programın binası
kodu (EK-2 §2.3: iki büyük harf ya da rakam, tek binada "00"), kullanım türü ve projenin ±0.00'ına göre ±0.00'ıyla;
birden çok binanın kapladığı kat (ortak bodrum) seçilen binanın katı sayılır, alanı bir kez; her nesnenin binası
katından okunur (`building_of_level`); `.kcad` mimari verisi 9; Bina programı › Binalar'da alanlar ve "Ortak katlar".
Bina başına model (§2): `cad.arch.ifc`'nin `building`'i; bina kendi ±0.00'ında, katları ona göre; bodrum sayıldığı
binada; proje ve saha her dosyada aynı kimlikle; IFC dışa aktar her binaya EK-2 adlı bir dosya. Konum (§3): yerel
sistemde çizilen model IFC'ye taban sistemiyle (TUREF dilimi) ve yerel düzlem döndürülüp ölçeklenerek yazılır
(`IfcMapConversionScaled`; yükseklik ölçeksiz); bina ve yolun dosyası aynı yerden; ±0.00'ın kotu bilinmiyorsa konum
yazılmaz ve söylenir (0 sayılmaz); IfcOpenShell üç kontrol noktasını anketteki yerine oturtmanın artığı içinde (en çok
0,45 mm) düşürür; yerleşim zinciri bütün örnek dosyalarda bağımsız denetlenir. Alanlar (§4): taban alanı (TAKS) her
binanın zemin katından (PAİY 4/1-bbbb "yapıların … izdüşümü"; önce yalnız bir zemin kat sayılıyordu); binanın payı
(saydığı katlar, ortak bodrum yalnız seçildiği binada) ve binalarınki birlikte projeninki; bina başına IFC binanın kendi
alanlarıyla; Metraj ve `cad.spaces.quantities` bina seçer; Ruhsat takımı bina seçer (katları, ±0.00'ından kat kodları,
paftalarda bina kodu). Onaylı plan (§5): `cad.zoning.parcel` parselin CBS'ten gelen karar alanlarını payı, değerleri,
kaynak planı (onay tarihi, durumu), notlarıyla okur; çelişen iki kararı ve imar durumuyla farkı söyler, hiçbir şey
yazmaz; Proje ayarları › Mimari › İmar durumu'nda gösterilir. İlk kabul sağlandı. **İyileştirmeler:** vaziyet planında
bütün binalar birlikte (harita tek kat çizer); kararın değerlerini kaynağıyla imar durumuna almak (yapıldı 18 Kasım 2026,
[ADR 8140](../docs/adr/8140-plan-decision-values-into-zoning.md), K349–K353: kararın altında Değerlerini al; KAKS ve TAKS
İmar durumu'na, Hmax ve kat adedi Plan notları'na kaynağıyla, Kaydet onaylar; emsalin ve TAKS'ın kaynağı İmar
durumu'nda, mimari verisi 27; imar denetiminin bulgusu kaynağı anar; kalan nizam ve çekmeler); bina düzeyinde denetim
bulguları.

**Kullanıcı sonucu:** Bir parselde birden çok blok ve ortak bodrum tek projede durur. Her bina kendi kodu, katları ve
künyesiyle pilotta ayrı model dosyası verir; saha ve parsel bilgileri hepsinde aynıdır. Dayanak EK-2 §2.3 ve Tablo 2.1; EK-6 6.7, 6.55;
EK-7'nin "ortak öznitelikler aynı" notu; Z7.

- **Bina:** kodu, adı, kullanım amacı, kat takımı, ±0.00'ı. Her yapı nesnesinin binası bellidir.
- **Ortak bodrum:** hangi binanın katı sayılacağı açık bir seçimdir; alanı iki kez sayılmaz.
- Ruhsat takımı ve dışa aktarma bina seçer. İlk pilotta bloklar ayrı hazırlanır ve dosya adlarına bina kodu girer
  (EK-2 §2.3). Nihai dosya kapsamı ilgili idarenin teslim profiline bağlanır.
- **Saha:** parsel, çekmeler ve saha röperi (ADR 8032) ortaktır. Her bloğun ±0.00 kotu ayrı tutulur ve ortak düşey
  referansa bağlanır.
- **Onaylı plan:** parseli etkileyen plan kararları (fonksiyon, nizam, yaklaşma sınırı, emsal, TAKS, yükseklik) CBS
  tarafından kaynağı, onay tarihi ve plan notuyla gelir. Plan gösterimi ya da bir kesişim tek başına hukuki hesap kuralı
  olmaz; çelişen iki karar kullanıcıya gösterilir.

**İlk kabul:** ortak bodrumlu ve giriş kotları farklı iki blok için pilot profilde iki model dosyası. Aynı adlı katlar
karışmaz, ortak proje ve saha bilgileri eşleşir, blok kotları doğru kalır ve bodrum alanı bir kez sayılır.

#### BIM-12 — Tipler ve katmanlı yapı [~]

**Kullanıcı sonucu:** Her eleman türünün tipi vardır. Bir değerin tipten mi örnekten mi geldiği görünür. Tip değişince
örnekler izler; örnekte ayrılan değer korunur. Dayanak Z11; W2 §2.1.

- Döşeme, kolon, kiriş, çatı, merdiven, korkuluk, kaplama ve giydirme cephe tipleri; duvar stili ve kapı/pencere tipiyle
  aynı düzende.
- "Tipten" ve "örnekten" işareti; önce çiz, beğenince tipe çevir.
- Katmanlı yapı: duvar, döşeme, çatı ve kaplamada sırası, kalınlığı, malzemesi ve çekirdeği belli katmanlar.
- Uyum için şart değildir (EK-6 tip istemez); kalite ve hız işidir, sırası sondadır.

**İlk kabul:** kolon tipinin ölçüsü değişince 16 kolon izler, örnekte ayrılan biri izlemez. Öznitelikler'de değerin kaynağı
yazılıdır.

**Durum (17 Kasım 2026, [ADR 8106](../docs/adr/8106-element-types-and-their-exceptions.md)):** ilk kabul sağlandı. Kolon,
kiriş, döşeme (kaplama da), çatı, korkuluk ve merdiven tipleri `settings.arch.elementTypes`'ta; eleman tipe bağlıdır, sürümü
ve istisnaları (`overrides`) her düzenlemede doğru tutulur. `cad.elementtypes.edit`: yeni tip, Tipe çevir (önce çiz, beğenince
tipe çevir), değiştir, ad, sil, bağla, ayır, döndür, uygula, yeniden uygula; takip raporu (izledi, korundu, atlandı,
reddedildi); 45 bağımsız durum. Eleman tipleri penceresi ve Öznitelikler'de "Tipten" / "Örnekten"; IFC'de her tip kendi
sınıfının tipi (IfcOpenShell doğruladı). Mimari verisi 19. **Kalan:** giydirme cephe ve rampa tipi, kolon ve kiriş tipinin
profil takımı, eski sürüm denetimi, ofis kitaplığı, katmanların pencerede düzenlenmesi, gerçek pencerede kabul.

### 5.4 Denetim

#### BIM-13 — Mevzuat denetimlerinin tamamlanması [~]

**Yapıldı (17 Kasım 2026, ilk dilim, [ADR 8078](../docs/adr/8078-regulation-rule-set-by-application-date.md)):** kural seti
künyenin başvuru tarihine göre (K60): maddelerin hâlleri veridir (metin, Resmî Gazete, yürürlük günleri, geçiş hükmü);
PAİY 34/1'in 28.7.2018, 11.7.2021 ve 1.7.2026 hâlleri, 34/8'in 11.7.2021 ve 1.7.2026 hâlleri, Sivas 34/1 ve 34/8; Geçici
3/11 1.7.2026'dan önceki başvuruya eski hâli uygular. Gün yoksa her hâl denenir, sonuç değişiyorsa bulgu eksik veridir;
bilinen metinden önceki gün uzman incelemesi (K61). Bulgu kaldı, eksik veri ya da uzman incelemesi; geçti ve kapsam dışı
maddenin özet satırında (`ArchAuditFinding.state`, `ArchAudited.ruleSet`, K62). Kurallar yeni standart pakette
(`tr.kentos.mimari-mevzuat`, K63; main'in `tr.kentos.mevzuat`'ı dalda yok, birleştirmede birleşir): 34/1 Rust'tan taşındı,
34/8 yeni; ölçüler olguların `building` bölümünde. Masaüstü Denetim kural setini tablonun üstünde, durumu satırın başında
söyler; başsız `cad.arch.audit` (MCP, Python) aynı bulguyu ve kural setini verir. İlk kabul tuttu: 25 bağımsız durum;
aynı çizim 30.6.2026'da geçer, 2.7.2026'da 34/8'den kalır, tarihsiz eksik veridir; yağmurlama sınaması ADR 8035'in
(`fire_escape_tests`). İç yükseklik ADR 8040 §3'te Rust'ta yapılmıştı.
**Kalan:** (Ek-3'te korunumlu şaft ve yangın kesici 18 Kasım'da ADR 8136'da: şaft mahalinin ve duvarın, döşemenin
beyanı, Ek-3/B 8 REI 120, 10 EI 30; duvar stilinin taşıyıcılığı 18 Kasım'da ADR 8137'de: kendi beyanı olmayan duvarlar
taşır, `wallStyle`'ın `loadBearing`'i uyumlu); mahal kaplamalarının içte uygulanan yalıtımı 19 Kasım'da ADR 8151'de (duvarın iç yüzündeki Yalıtım katmanları, 20/3; ADR 8091 §6; mahal kaplamalarının malzemesi 18 Kasım'da ADR 8138'de: kaplamanın adı malzemedir, 20/3, 26/2, 26/4; çatının Broof sınıfı 18 Kasım'da ADR 8134'te; 62/4'ün kuyu
havalandırması 18 Kasım'da ADR 8135'te: beyan kuyunun mahalinde, katlarca tek kuyu, en az 0,1 m² ve alanın 0,025 katı);
erişilebilir yol (TS 9111 metni); plan notlarının öbür konuları (çatı, bodrum, çıkmalar: ADR 8084 §6; CBS'in Hmax'ı ve
KatAdedi'si 18 Kasım'da ADR 8140'ta: kararın Değerlerini al'ı; yapı nizamı 18 Kasım'da ADR 8141'de: İmar durumu'nda
seçim, PAİY 5/6, BYKHY 26/4 ve 28/3 onunla; bitişik kenar 18 Kasım'da ADR 8142'de: vaziyet planında komşu binaya
bitişik kenar, PAİY 39/4, BYKHY 25/1 ve 25/2; çıkmalar 19 Kasım'da ADR 8152'de: PAİY 41/1, 41/2; PAİY 6 ifrazın, yeni parsellerin asgari ölçüsünün kuralıdır: bina
denetiminin değil, parsel denetiminin işi; subasman kotu 19 Kasım'da ADR 8153'te: 4/1-yyy; tahvil 19 Kasım'da ADR 8157'de: planın yalnız bina yüksekliği ya da kat adedi varsa öbürü 9/1-c ile, 1,20 m subasman ve 28/1'in en çok kat yükseklikleri, aşan uzman incelemesi; planın subasman hükmü 19 Kasım'da ADR 8158'de: Subasman kotu en çok ve en az plan notları 4/1-yyy'nin uçlarının yerine, Mimari verisi 32; çatı kuralları 19 Kasım'da ADR 8159'da pakete taşındı: PAİY 40/4, 40/5, 42/1, BYKHY 58/2, Sivas 40/5, 42/5, olgularda çatının biçimi, düzeltmeler korundu; planın çatı notları sırada); blok başına bölge ve taşıyıcı sistem (ADR
8085 §6, 8094 §6; 18 Kasım'da ADR 8144 ve 8145'te: Bina programı'nda binanın kendi bölgesi ve taşıyıcı sistemi);
konut dışı (B22; dört dilim yapıldı, aşağıda ADR 8107, 8108, 8110, 8111);
emsalde 5/8-b'nin öbür asgari alanları (ADR 8148 §6; 22/1-g'nin gerekli sığınak alanı 19 Kasım'da ADR 8149'da; 22/1-j 19 Kasım'da ADR
8147'de: toprak altındaki bodrumun ortak yönetim, toplantı, spor ve sosyal mekânları katlar alanının %5'ine ve 1000
m²'ye kadar emsal harici, tabanı 21/2–3'ün kullanımıyla katlar alanı; 5/8'in bodrum listesi 19 Kasım'da ADR 8148'de:
bodrumda otoparkın zorunlu alanın 2 katı, sığınağın gerekli alanı, ortak mescidin 150/300 m²'si, gömülü katın ortak
alanları, çocuk oyun ve bakım alanlarının 100 m²'si %30 dışında).
**Yapıldı (17 Kasım 2026, ADR 8078 §6, K64):** uzman onayı kayıtlı: paketin bulgusu `cad.arch.exceptions`'a verilerek
istisna yapılır; uzman incelemesi bulgusunun istisnası "Uzman onayı"dır, notu uzmanın sözü; bulgu değişince düşer, katla
ilgili bulgu katın mahallerini adlandırdığı için mahal eklenince yeni bulgu açılır.
**Yapıldı (17 Kasım 2026, ADR 8078 §8, K65):** kaçış yolu kapıları: BYKHY 47/2 (kullanıcı yükü 50'yi aşan mekânın çıkış
kapısı dışarı açılır) ve 47/3 (kaçış merdiveni ve yangın güvenlik holü kapısının beyan edilen sınıfı: 60/90 dakika, duman
sızdırmaz, kendiliğinden kapanır); olgulara mahallerin kişisi ve kapılar eklendi; 20 bağımsız durum.
**Yapıldı (17 Kasım 2026, [ADR 8107](../docs/adr/8107-use-class-escape-articles.md), K188–K193, B22'nin ilk dilimi):**
kullanım sınıfına özel kaçış hükümleri (paket 0.22.0, `kullanim`): sınıf bina programının kullanımından, yoksa mahal
adlarından sıkı okumayla (12/2, 16/2). BYKHY 49/1-a: sağlık yapısında 15 kişiyi aşan hasta odasının 2 kapısı; 50/1-a:
otelde yatak odası ile iç koridor arası en az 60 dakika, ikiden çok katlı otelde her katta 2 çıkış (yatak sayısı çizimde
yok: 15,50 m'nin altında eksik veri); 50/1-b: koridora açılan kapı 30 dakika ve kendiliğinden kapanır; 52 (9.9.2009): 52'nin
binasında tek merdivende (a) yapı yüksekliği ve (b) katın kişisi tutmazsa kaldı, tutarsa uzman incelemesi. 25 bağımsız
durum; gerçek çizimde büro ve otel Denetim'de. Olgulara alan eklenmedi.
**Yapıldı (17 Kasım 2026, [ADR 8108](../docs/adr/8108-sprinklers-required-by-use.md), K194–K196):** yağmurlama zorunluluğu
(paket 0.23.0, `yagmurlama`): BYKHY 96/2'nin bentleri, beyan yoksa `bykhy_sprinklers_required` (30,50 m konut harici, 51,50 m
konut, 600 m² otopark, 100 odalı katlı konaklama ve 21,50 m yataklı tesis, 2000 m² katlı mağaza ve eğlence yeri, 1000 m²
parlayıcı madde yapısı); alanlar olguların mahal alanları (dış kontur, PAİY 4/1-ğ-1). 26 bağımsız durum; 33 m'lik büro
Denetim'de.
**Yapıldı (18 Kasım 2026, [ADR 8110](../docs/adr/8110-health-buildings-sleeping-floors-corridors.md), K200–K203):** sağlık
yapıları (paket 0.24.0, `saglik`): BYKHY 49/1-b, hastanenin ve bakımevinin 300 m²'yi aşan yatılan katı tek yangın
kompartımanıysa kaldı, iki ya da daha çoksa büyüklük şartı ve 2,8 m²/kişi uzman incelemesi; 49/2, hastanede adı Koridor
olan mahalin en dar yeri 2,00 m'den azsa kaldı. Koridorun en dar yeri ortak çekirdekte yeni ölçü (`geom::neck`: iki yanında
kendi genişliğinde daire kalan en kısa kesit; L, T, artı ve holle birleşen koridorda en büyük daire koridoru geniş
okuyordu), bina olgularında `least`; 24 durumluk bağımsız geometri referansı (Shapely), 25 bağımsız kural durumu, gerçek
çizimde L koridorlu hastane Denetim'de. Kalan: çıkmaza daralan koridor ve serbest kolon, yatay tahliye alanının kapasitesi.
Mimari denetimin PAİY 29/3 ve BYKHY 33/1, 33/2 koridor kuralları en dar yere geçti ([ADR 8112](../docs/adr/8112-corridor-rules-by-narrowest-neck.md),
K208–K210, 18 Kasım 2026): L hol artık kollarından ölçülür, bulgu en dar kesitte.
**Yapıldı (18 Kasım 2026, [ADR 8111](../docs/adr/8111-hotel-room-distance-to-its-door.md), K204–K207):** otel yatak
odasının içindeki kaçış uzaklığı (paket 0.25.0, `konaklama`): BYKHY 50/3, odanın en uzak noktasından (0,40 m önden) kendi
kapısına yürüyüş 15 m'yi aşan tek kapılı oda kaldı, tamamı yağmurlamalı otelde sınır 20 m; kaçış modülüne mahalin kendi
kapısına yürüyüşü, bina olgularına `toDoor`. 25 bağımsız durum; 18 m derin otel odası Denetim'de (ikinci kapıyla geçer).
Kalan: süitin grubunun kapısına yürüyüş, 50/2, 50/6.
**Yapıldı (19 Kasım 2026, [ADR 8156](../docs/adr/8156-bedroom-escape-from-its-door.md), K400–K403):** otel ve yurt yatak
odasının kapısından kaçış uzaklığı (paket 0.43.0, `yatak_kacisi`): BYKHY 50/4, 50/5; kaçış modülünde kapının dış
yanından en yakın kat çıkışına yürüyüş (`RoomEscape.door_exit`), olgularda `doorExit`; Ek-5/B'nin otel ya da yurt satırı;
tek çıkışta bütün yol tek yön, iki çıkışta tek yönlü kısım ve bütün; süitin iç odası süitin çıkış kapısından. Mimari
denetimin 32/4'ü bu odaları en uzak noktalarından ölçmez, Kaçış yolları yolu kapıdan çizer. 21 bağımsız durum; iki
odalı otel katı Denetim'de. Kalan: 50/2 dış koridor (Ek-5/B'nin notu), salonu yatak odası adı taşımayan süit, otelin
içindeki konut bölümü, 32/4'ün öbürünün betiğe taşınması.
**Yapıldı (18 Kasım 2026, [ADR 8113](../docs/adr/8113-boiler-room-and-fuel-store.md), K211–K215):** kazan dairesi ve
yakıt deposu (paket 0.26.0, `kazan`): BYKHY 54/2 komşu mahallerle en az EI 120 ayrım, 54/4 kapı merdivene değil hol ya da
koridora, 54/5 net alanı 100 m²'yi aşanda iki çıkış kapısı ve kapılar EI2 90 S C (ısıl kapasite çizimde yok: uzman
incelemesi), 56/1 yakıt deposu–kazan dairesi EI 120; olgulara mahalin net alanı. 26 bağımsız durum; zemin katta kazan
dairesi Denetim'de. Kalan: döşemelerin sınıfı, yakıt türü (55), ısıl kapasitenin beyanı, 57–60.
**Yapıldı (17 Kasım 2026, [ADR 8079](../docs/adr/8079-emsal-article-22-limits.md), K66–K69):** emsalin 22. madde
sınırları alan hesabında: toprak altındaki katta bölümün depo eklentisi bölüm net alanının %50'sine (22/1-k), kapıcı
daireleri toplam 70 m²'ye (22/1-ç), kütüphane 50 m²'ye (22/1-f) kadar dışarıda, aşan kısım emsale girer ve tablonun
kendi satırındadır; merdiven evi emsal harici (22/1-l). IFC'nin emsal zonları aşan payı söyler. Koşullar pakette
(`emsal`): emsal harici kapıcı dairesinin 44/1'e göre zorunluluğu (konut ve bölüm sayısından; ısıtmaya bağlı ara eksik
veri), kütüphanenin alışveriş merkezi ya da 40'tan fazla bölüm, normal kat ve ortak alan koşulları; 23 bağımsız durum.
Metrekare cetveli her katı ara toplamıyla kapatır (kat ara toplamı).
**Yapıldı (17 Kasım 2026, [ADR 8080](../docs/adr/8080-under-stair-1-80.md), K70–K73):** merdiven altı 1,80 m: merdivenin
gövdesi geometri çekirdeğinde tek yerde (kesit ondan çizer); başladığı döşemeden 1,80 m'den alçak yerler grupların net
alanından düşülür (4/1-ğ-3; Metrekare cetveli, Kat irtifakı, emsalin payları); 31/6 paketin kuralı: alçak boşluk bir
mahale açıksa kaldı, dolap, depo ya da merdiven altı mahalindeyse geçti, döşemesiz merdivende eksik veri; 18 bağımsız
durum.
**Yapıldı (17 Kasım 2026, [ADR 8081](../docs/adr/8081-fire-compartments.md), K74–K78):** yangın kompartımanı: her iki
mahal arasındaki ayrım ölçülür (duvarlar, açıklıklar, beyan edilen sınıfları, açık boy; `Document::separations`);
kompartımanlar paketin kuralı (en az EI 60 ve açık yer yoksa ayrı; döşemeler hep ayırır). BYKHY 24/2 tehlikeli
alanların ayrımı, 24/6 Ek-4'ün en çok alanı (tablo PDF'ten; yağmurlama notları), 24/7 kompartıman duvarındaki
açıklıklar; 22 bağımsız durum.
**Yapıldı (17 Kasım 2026, [ADR 8082](../docs/adr/8082-fire-resistance-of-elements.md), K79–K80):** beyan edilen
dayanım öbür elemanlarda: taşıyıcı sistemin kolon ve kirişleri, duvarlar (dış olup olmadıkları), döşemeler ölçülür
(`Document::fire_elements`), bina yüksekliği ve bodrum derinliğiyle; paketin `dayanim` kuralı Ek-3/B'nin satırlarını
Ek-3/C'nin kullanım, yağmurlama ve yükseklik dakikasıyla karşılaştırır (tablolar PDF'ten veri); 18 bağımsız durum.
**Yapıldı (17 Kasım 2026, [ADR 8083](../docs/adr/8083-dated-storey-height-and-doors.md), K81–K86):** kat yüksekliği ve
kapılar başvuru gününün hâliyle: PAİY 28/1-c'nin 1.10.2017 (3,60 m) ve 12.8.2023 (bodrum hariç 3 kata kadar 4,00 m)
hâlleri, 39/1'in 1.10.2017 (balkon ve tuvalet kapısı 0,80 m) ve 13.8.2025 (0,80'i yok) hâlleri, Sivas 28/1-c ve 39/1;
iki değişiklik de yayımı günü yürürlüğe girdi, geçiş hükmü bu maddeleri saymaz. Kurallar Rust'tan pakete taşındı
(`kat_yuksekligi`, `kapi_olculeri`); olgulara katın yüksekliği ve yatak odaları, kapının net genişliği, yüksekliği ve
iki yanı eklendi. Gün yoksa her kat ve kapı ayrı denenir: güne bağlı tek kapı öbürlerinin bulgusunu saklamaz. 41 bağımsız
durum; aynı ev 1.8.2023'te katlardan, 14.8.2025'te WC kapısından kalır (`dated_sizes_tests`).
**Yapıldı (17 Kasım 2026, [ADR 8084](../docs/adr/8084-plan-note-layer.md), K87–K92):** plan notu katmanı (ulusal →
belediye → parsel): parselin plan notları projede (Proje ayarları › Mimari › Plan notları: konu, değer, kaynak; mimari
verisi 11), olgularda ve kat yüksekliği kuralında; not bağladığı ölçüde Sivas'ın ve PAİY'nin önüne geçer (69/3; 28/1
plana bırakır), yönetmeliklerin değeri yanında yazılır; kural setinde notun satırı. İlk konular zemin kat ve kat
yüksekliği; KentOS notun metnini sayıya çevirmez. 7 bağımsız durum; masaüstünde notlar yazılır, kaydedilir, bağlar.
**Yapıldı (17 Kasım 2026, [ADR 8085](../docs/adr/8085-zone-storey-heights.md), K93–K97):** bölge türü: parselin plandaki
kullanımı İmar durumu'nda seçilir (konut, zemin katı ticaretli konut, karma, ticaret, öbür; mimari verisi 12); kat
yükseklikleri bölgenin bendiyle (PAİY 28/1-a, b, c, ç; Sivas'ınkiler), asma kat zemin katın parçası (asma katlı zemin
kat), karma alanda konut katı ayrı; bölge yazılmamışsa konut projesinde konut alanı, öbüründe eksik veri. 12 bağımsız
durum; masaüstünde Ticaret alanı seçilince zemin kat 4,50'ye kalır, Sivas'la çelişki.
**Yapıldı (17 Kasım 2026, [ADR 8086](../docs/adr/8086-compartments-across-storeys.md), K98–K101):** katlar arası
açıklıklar: döşeme deliği, şaft olmayan boşluk mahal ve merdiven ölçülür (`Document::vertical_links`) ve iki katın
mahallerini aynı kompartımana koyar (Ek-4 alanı katlar boyunca); BYKHY 24/4 (21,50/30,50 m'nin üstünde en çok üç kat;
merdiven yuvası sayılmaz) ve 24/5 (merdiven yuvası olmayan boşluk atrium: duman kontrolü ve perde çizimde okunmaz,
uzman incelemesi). 7 bağımsız durum; masaüstünde delikli döşeme iki katı birleştirir, delikten çıkan merdiven yuvadır.
**Yapıldı (17 Kasım 2026, [ADR 8087](../docs/adr/8087-shafts-elevators-roofs.md), K102–K109):** şaftlar, asansörler ve
çatılar: şaftın çevresi ölçülür (`Document::shaft_enclosures`: duvar, açıklık, açık yer), kapının asansörün ve geçide
açıldığı okunur; BYKHY 25/3 (yüksek binada tesisat şaftı EI 120, kapak E 90 S), 41/9 (kaçış merdivenine ve güvenlik
holüne açılan şaft kapağı), 62/2 (kuyu EI 60), 62/4 ve 62/6 (asansör kapısının önü), 62/7 ve Ek-3/B 12 (9.7.2015'ten kat
kapısı E 60/E 30), Ek-3/B 4 (teras REI, çatı EI). Ayrımların ölçümü düzeldi: açıklığın halkası, geçişin yeri 0,1 mm'ye.
26 bağımsız durum; masaüstünde 8 katlı büro.
**Yapıldı (17 Kasım 2026, [ADR 8088](../docs/adr/8088-fire-compartments-as-ifc-zones.md), K110–K113):** kompartımanın IFC
grubu: paketin `kompartimanlar`ı (0.12.0) kompartımanları verir, `cad.arch.ifc`'nin `compartments` girdisi her birini
mahallerinin `IfcZone`'u olarak yazar (`MM-ZON-Yangın_kompartımanı_N`); girdi verilmezse masaüstü, MCP ve komut satırı
paketi sorar. IfcOpenShell 0 bulgu; IFC modeli seçili mahalin zonunu "Grup" satırında söyler.
**Yapıldı (17 Kasım 2026, [ADR 8089](../docs/adr/8089-suspended-ceiling-fire-resistance.md), K114–K115):** asma tavan:
IFC türü CEILING olan kaplama Ek-3/B 11'in öğesi, alt taraftan EI 30, Ek-3/C'den bağımsız (paket 0.13.0); 4 bağımsız
durum, masaüstünde iki katlı evin asma tavanı.
**Yapıldı (17 Kasım 2026, [ADR 8090](../docs/adr/8090-declared-load-bearing-and-escape-roof.md), K116–K119):** beyan
edilen roller: duvarın taşıyıcılığı (`arch.loadBearing`, Öznitelikler'de "Taşıyıcı") ve çatının kaçış yolu rolü
(`arch.exit`, "Kaçış yolu"); `cad.arch.performance` ikisini yazar, `.kcad` mimari verisi 13; Ek-3/B 2 (taşıyıcı iç duvar
R), 5 ve 6–7 (taşıyıcıysa REI), Ek-3/C not (3) (taşıyıcı olmayan dış duvarda 90), 4-a (kaçış yolu çatısı REI 30); IFC
`Pset_WallCommon.LoadBearing`. 7 bağımsız durum, 10 komut durumu.
**Yapıldı (17 Kasım 2026, [ADR 8091](../docs/adr/8091-material-reaction-to-fire.md), K120–K123):** malzemenin TS EN
13501-1 yanıcılık sınıfı (`ArchMaterial.reaction`; Malzemeler'de "Dayanım" ve "Yanıcılık sınıfı"; `.kcad` mimari verisi
14); BYKHY 26/2 (döşeme kaplaması) ve 26/4 (tavan kaplaması, asma tavan) Ek-2/Ç'nin tablolarıyla (paket 0.15.0). 16
bağımsız durum, 9 komut durumu.
**Yapıldı (17 Kasım 2026, [ADR 8092](../docs/adr/8092-basement-lift-protected-corridor.md), K124–K126):** 62/4'ün ikinci
cümlesi: bodruma inen asansöre bodrumda yangın güvenlik holünden ya da korunmuş koridordan (4/dd, 36/1-a: ayrımları
Ek-3/C'nin bodrum süresince EI) ulaşılır (paket 0.16.0, 7 bağımsız durum).
**Yapıldı (17 Kasım 2026, [ADR 8093](../docs/adr/8093-double-door-leaf-revolving-door.md), K127–K128):** PAİY 39/1-b'nin
çift kanatlı bina girişinde bir kanadı (en az 1,00 m, ilk kanadın tek başına net geçişi) ve 39/1-ç'nin döner kapısı
(yanında normal bina girişi); olgulara kanat enleri (paket 0.17.0, 5 bağımsız durum).
**Yapıldı (17 Kasım 2026, [ADR 8094](../docs/adr/8094-structural-system-traditional-cut-stone.md), K129–K133):** taşıyıcı
sistem beyanı (Proje ayarları › Mimari › Taşıyıcı sistem: EK-2 Tablo 2.5 ve kesme taş yığma, geleneksel mimari öğeler;
`.kcad` mimari verisi 15); konut bölgesinde zemin dahil 3 kata kadar geleneksel kesme taş yığma binada PAİY ve Sivas 28/1-d
4,50 m; Emsal ve TAKS tablosu Taşlık, Dış sofa, Eyvan'ı emsal harici sayar, 22/1-m'nin koşulları pakette (0.18.0; 16
bağımsız durum).
**Yapıldı (17 Kasım 2026, [ADR 8095](../docs/adr/8095-thick-stone-walls-out-of-emsal.md), K134–K138):** böyle binada
taşıyıcı taş duvarların 25 cm'yi (iç duvarda 12,50 cm'yi) aşan kısmı Emsal ve TAKS tablosunda %30 dışında (5/8), dış
duvarlarınki taban alanı dışında (20/8-j); dış duvarda dış yüzdeki şerit, iç duvarda iki yüzdeki şeritler, katın duvar
kütlesiyle kesilir; emsal zonunun açıklamasında yazılır; beyansız kalın duvar eksik veri (paket 0.19.0, 6 bağımsız durum).
**Yapıldı (17 Kasım 2026, [ADR 8096](../docs/adr/8096-sivas-22k-thick-walls-within-30.md), K139–K141):** Sivas 22/k:
yerel yönetmelik Sivas ise kalın taş duvarlar %30'un içinde okunur, sınırı aşan emsale girer; olgulara 5/8'in %30'u ve
içinde sayılan (`exemptCap`); olgular Emsal ve TAKS tablosunu bir kez sayar (paket 0.20.0, 4 bağımsız durum).
**Yapıldı (17 Kasım 2026, [ADR 8097](../docs/adr/8097-emsal-zone-facade-corner.md), K142):** emsal zonlarının cephe
köşesi: iki mahalin konturunun ortak köşesi ortak duvarın ekseninden bölünür, zonun alanı mahallerinin brüt alanıdır.
**Yapıldı (17 Kasım 2026, [ADR 8098](../docs/adr/8098-plan-building-height-and-storeys.md), K143–K146):** plan notunda
planın bina yüksekliği ve kat adedi (`.kcad` mimari verisi 16); PAİY 5/11: ±0,00'dan son katın tavan döşemesinin
üstüne ve zemin ile normal katlar (asma kat, çatı arası, bodrum sayılmaz) planınkini aşamaz (paket 0.21.0, 8 bağımsız
durum).

**Kullanıcı sonucu:** Denetim, başvuru tarihinde yürürlükte olan kuralları uygular. Her bulgu geçti, kaldı, eksik veri,
uzman incelemesi ya da kapsam dışıdır. Yorum isteyen konu kendiliğinden "uygun" sayılmaz. Dayanak Z9; Ek A.

- **Kurallar Python betiğidir (K10):** yeni kural Rust'a yazılmaz; bugünkü Rust kuralları (ADR 8033–8035) dokunuldukça
  betiğe taşınır. Ölçüler (mimari olgular, ADR 8057 §39) Rust'ta kalır.
- **Kural seti** başvuru tarihiyle seçilir (PAİY'nin 1.7.2026 değişikliğinden önce ve sonra gibi); bölge türü; plan notu
  katmanı (ulusal → belediye → parsel).
- **Kalan kurallar:** iç yükseklik; kapı açılış yönü; emsalin 22. madde sınırları (%20, %50, 70 m² …) ve %30'u; kat ara
  toplamları; merdiven altı 1,80 m (ADV-22 ve ADV-23'ün kalanları).
- **Yangın:** kompartıman sınırı ve üstündeki açıklık; kullanım sınıfına göre kişi yükü; beyan edilen dayanımın (BIM-07)
  kuralın istediğiyle karşılaştırılması.
- **Erişilebilirlik:** parselden girişe, asansöre ve erişilebilir tuvalete kesintisiz yol; rampa, sahanlık, net geçiş. TS
  9111'in değerleri TSE metninden doğrulanmadan kural olmaz (W4 §2.6).
- **Bulgu durumu genişler:** "eksik veri" ve "uzman incelemesi". Uzman onayı kayıtlıdır; etkilediği nesne değişince eskir.
- **Konut dışı kullanımlar** (1.9.2033): kullanım sınıfına özel kural paketleri ayrı dilimlerde.

**İlk kabul:** her yeni kural için bir geçen, bir kalan durum, KentOS kodu olmadan yazılmış başvuruyla. Başvuru tarihi
30.6.2026 ve 2.7.2026 olan aynı çizim farklı kural setiyle denetlenir. Yağmurlama beyanı kaldırılınca kaçış uzaklığı
bulgusu değişir.

### 5.5 Teslim ve bilgi yönetimi

#### BIM-14 — BIM ruhsat paketi: IFC + PDF/A + kontrol formu [~]

**Yapıldı (16 Kasım 2026, ilk dilim, [ADR 8075](../docs/adr/8075-bim-permit-package.md)):** PDF/A yayınının `bim` bölümü:
aynı sabitlenmiş çizimden IFC (EK-2 adıyla) paftaların yanına yazılır, klasörden geri okunur; standart paketin kuralı
`dijital_proje.bim` paftaları ve paketi (`ruhsat`), formun 21 sorusunu (ADR 8074) ve müellifin cevaplarını birlikte
denetler; kalan her bulgu yayını durdurur, klasör açılmaz. Beş sonuç ayrı; formun kaydı (`kalite-kontrol-formu.json`)
KentOS'un kaydı olarak `yayin.json`'un yanında; teslim listesi modeli sayar. Kural süreci `kentos-standards`'ta;
masaüstünde Ruhsat takımının yayınında anahtar, beş sonuç, inceleme sorularına Evet/Hayır ve not. Paketin denetimi
yazıcıda iki hata buldu (FootPrintArea, sahanın oranları), düzeltildi. Bağımsız okuyucular (qpdf, Poppler, IfcOpenShell,
IfcTester) paketin dosyalarını okur.
**Yapıldı (17 Kasım 2026, [ADR 8076](../docs/adr/8076-ifc-mandatory-fields-complete.md)):** modelin zorunlu alanları
(IFC-02) tamamlandı; yazar gibi doldurulmuş kesit binasının paketi gerçek kuralla yayımlanıyor (başsız ve masaüstü):
beş sonuçtan dördü Tutuyor, idare Bekleniyor; qpdf/Poppler, IfcOpenShell ve IfcTester klasörü okudu.
**Yapıldı (17 Kasım 2026, [ADR 8077](../docs/adr/8077-ek9-author-answers-in-the-drawing.md)):** müellifin cevapları
çizimde (`settings.arch.formAnswers`, `cad.form.answer` v1, mimari verisi 10): basılan form ve yayın aynı cevabı okur;
pencere taslağı Denetle ve Yayınla'da tek geri alma adımıyla çizime yazar; 26 ortak durum.
2+1 dairesinin paketi de gerçek kuralla yayımlanıyor: kuralın bulguları çizimin eksikleriydi (duvar stili, döşeme
katmanı, üst döşeme, parselin katmanı), tamamlanınca geçti (ADR 8077 §5).
MCP'de uçtan uca: ajan durdurulur, `cad.form.answer` ile cevaplar, paket yayımlanır (ADR 8077 §6).
Cevap dayandığı kanıtı tutar; kanıt değişince eskir, soru yeniden açılır, yayın durur (ADR 8077 §7, K59).
İlk kabul tuttu: bölüm binasında ve 2+1'de paket, eski pafta paketi durdurur, bağımsız okuyucular hatasız.
**Yapıldı (18 Kasım 2026, [ADR 8133](../docs/adr/8133-ek9-development-level-from-the-file.md)):** 7. soru, gelişim
seviyesi, dosyadan: her yapı elemanının gövde gösterimi (var mı, sınır kutusu mu, katı mı, okunabildi mi); kitaplık
bloklarının kutusu müellife kalır; karışık gövde (eğik kol ve sahanlık) tek 'Tessellation' olarak yazılır.
**Kalan:** formun kalıcı kaynak türü (çizimdeki EK-9 tablosu bugün başlık satırından tanınıyor; tablo kaynağına yeni tür
`.kcad`'in mimari verisini ve onu okuyan bağımsız okuyucuyu ister); Python tipli katmanından ortak durumlar bağımsız
`.kcad` okuyucusunun mimari verisini bilmemesi yüzünden duruyor (TODOS.md'de ayrı iş).

**Kullanıcı sonucu:** Tek komut, aynı model revizyonundan mimari modeli, bütün PDF/A paftaları, çizelgeleri ve kontrol
formunu üretir. Hepsi yazıldıktan sonra dosyadan denetlenir; biri kalırsa hiçbir şey yayımlanmaz. Dayanak Md. 9/1, 15; e-PYS
Md. 4/2, 6; Z4.

- **Teslim girdisi:** belge revizyonu, kural kataloğunun ve kurum profilinin sürümü, bina ve kat seçimi, altlık modellerin
  özetleri. Değişmez.
- **Akış:** hazırlık klasörü → dosyadan denetim ([IFC-08](ifc-uyum.md#10-iş-paketleri) ve BIM-05) → müellifin form
  incelemesi → yayımlanmış revizyon.
- **Form:** resmî Evet/Hayır ile KentOS'un kanıt durumu ayrı tutulur. Elle verilen cevap, etkilediği dosya ya da profil
  değişince eskir. Form kalıcı bir kaynak türü olur (bugün başlık satırından tanınıyor).
- **Teslim listesi:** her dosyanın adı, boyutu, SHA-256 özeti, üretildiği revizyon, denetim sonuçları, araç ve katalog
  sürümleri. İmzalanacak nüshalar ve özet listesi hazırdır; imzanın kendisi müellifin aracındadır.
- Komut, Python ve MCP aynı işi yapar. İptal ve hata kısmi paket bırakmaz; önceki paket erişilebilir kalır.

**İlk kabul:** bölüm binasında ve 2+1'de paket. Bir pafta eski revizyondan kaldığında paket durur ve sebebini söyler. İki
bağımsız model okuyucusu ve bağımsız PDF/A doğrulayıcısı hatasız.

#### BIM-15 — Bilgi yönetimi ve teslim kaydı [~]

**Kullanıcı sonucu:** Projenin hangi revizyonu, hangi dosyalarla, ne zaman, kime teslim edildi ve ne cevap geldi sorusunun
cevabı projede kayıtlıdır. İade gerekçesi ilgili nesneye bağlıdır. Dayanak Md. 4/4; e-PYS Md. 4–7; Z12.

- **Durumlar:** çalışma, paylaşım, yayın, arşiv (TS EN ISO 19650'nin ortak veri ortamı); EK-2'nin revizyon kodu.
- **Teslim kaydı** bulut projesinde: paketin özeti, yayımlayan, tarih, kurum, cevap (kabul, iade), gerekçe, varsa sistemin
  makbuz numarası.
- **İade ve inceleme notu** nesneye bağlı bir sorun kaydıdır (mevcut gözden geçirme notları). Dış değişim için açık sorun
  biçimi IFC-10'dadır.
- **Sorumluluk ve gizlilik:** disiplin, müellif, sicil; kimlik bilgilerinde role göre görünürlük (6698 sayılı Kanun). Gerçek
  proje verisi açık test korpusuna girmez.
- **e-PYS:** resmî arayüz, yetki ve deneme ortamı yayımlanınca pilot. O güne kadar müellifin yetkili kanaldan yükleyeceği
  paket hazırlanır.

**İlk kabul:** bir paket yayımlanır; "iade" cevabı gerekçesiyle girilir; gerekçe iki nesneye bağlanır; düzeltilmiş ikinci
paket öncekiyle karşılaştırılır. Yetkisiz kullanıcı künyeyi göremez.

**Yapıldı (17 Kasım 2026, [ADR 8099](../docs/adr/8099-delivery-record.md), K147–K151):** teslim kaydı çizimde (Mimari
verisi 17): paket yayının `yayin.json`'unun SHA-256'sıyla, EK-2 revizyonu, kurum, kanal (e-PYS, idarenin yazılımı, basılı),
gün, yayımlayan, makbuz; cevap Kabul ya da İade, iadede gerekçe zorunlu, gerekçe nesnelere inceleme notuyla bağlı.
`cad.delivery.edit` (Python, MCP, 40 ortak durum); Mimari › Proje ▾ › Teslim kaydı. Öncekiyle karşılaştır aynı kuruma
önceki paketin dosyalarını (pafta, ad, içerik, revizyonla değişen model adı) ve çizimini (kalıcı kimlikle) karşılaştırır,
iade gerekçelerinin nesnelerinden kaçının değiştiğini yazar. İlk kabulün ilk dört adımı masaüstünde elle sınandı.
**Künye (17 Kasım 2026, [ADR 8100](../docs/adr/8100-kunye-personal-particulars-by-role.md), K152–K156):** bulut projesinde
künyenin kişisel bilgilerini (yapı sahibi, müellif, sicil, büro, tapu kaydı, teslim eden; 13 değişken) yalnız nesneleri
yazabilen (Düzenleyici ve üstü) görür; İzleyici, Yorumcu ve Veri girişi projenin bilgisinde ve anlık görüntüsünde "Gizli"
okur, künye taşıyan saklı kopyayı (kontrol noktası, dosya revizyonu, dışa aktarma, atlas) alamaz. Gerçek PostGIS'te
sınandı. İlk kabulün beş adımı da sağlandı.
**Kalan:** ortak veri ortamının durumları (çalışma, paylaşım, yayın, arşiv); karşılaştırmanın Python ve MCP için salt okuyan
komutu; dosya projesinde künyesi kesilmiş revizyon kopyası; e-PYS pilotu.

#### BIM-16 — Büro ve belediye pilotu [ ]

**Kullanıcı sonucu:** Bir konut ve bir ticaret+konut projesi gerçek bir büroda baştan sona KentOS'ta hazırlanır ve hedef
idarenin yazılı kabul ya da iade cevabını alır.

- Sıra: tek parsel konut → ticaret+konut ve ortak bodrum → çok bloklu parsel. İlk zorunlu nüfus/alan grubunu temsil eden
  bir idareyle birlikte farklı büyüklükte bir idare profili sınanır. Fiilî başvuru tarihi gelmeden teknik ön kontrol ve
  kontrollü pilot yapılır.
- Yetkili müellif incelemesi; gerçek pencerede akış (ADR 8000); aynı dosyayı yeniden üretebilen sürüm kaydı.
- İdarenin yorumları (ad kalıbı, mahal adı, görünüm tanımı, dosya sınırı) kurum profiline kaynağıyla girer.

**İlk kabul:** idarenin yazılı cevabı; cevaptaki her madde kataloğa ya da bir iş paketine bağlanmış.

### 5.6 Disiplinler ve idare

#### BIM-17 — Dış disiplin modeli ve çakışma [~]

**Kullanıcı sonucu:** Statik ve tesisat modeli KentOS'ta altlık olarak açılır. Aks, kot ve koordinat uyuşmazlığı ile
çakışmalar nesnesiyle listelenir. Dayanak Md. 9/1 ("disiplinler tarafından ayrı ayrı"); EK-7; Z10.

- Altlık model salt okunur ([IFC-09](ifc-uyum.md#10-iş-paketleri)); revizyonu ve özeti kayıtlıdır.
- **Hizalama:** koordinat sistemi, harita dönüşümü, ±0.00, aks etiketleri, kat kotları; projenin ortak alanları.
- **Çakışma:** mimari duvar ve açıklık ile statik kolon, kiriş, perde; döşeme boşluğu ile şaft; geçişler. Kutu ön
  süzgeçtir; kararı gerçek gövde verir.
- Bulgu kaynağına döner (nesne, disiplin, revizyon) ve açık sorun biçimiyle dışa verilir (IFC-10).

**İlk kabul:** bölüm binasının bağımsız üretilmiş statik modeliyle hizalama. Bilerek kaydırılan bir kolon ve bir kat kotu
bulgu olur. Farklı düşey datum, geometri denetiminden önce durdurur.

**Durum (17 Kasım 2026, [ADR 8102](../docs/adr/8102-underlay-alignment-and-clash.md)):** ilk kabul sağlandı. Bölüm binası
IfcOpenShell'in yazdığı statik modelle hizalı: iki kat, yedi aks, 27 eleman eşleşir, çakışma yok. 40 cm kaydırılan A-1
kolonu çakışır; "1. Kat" +0,150 m bulgu; ITRF96 datumlu model geometriye bakılmadan durur. `cad.underlay.check` (Python,
MCP, 29 bağımsız ortak durum); Altlık modeller › Denetim (Katlar, Akslar, Çakışmalar, Yalnız birinde; çakışma satırı
nesneyi seçer). **Kalan:** çakışma satırının sorun olarak BCF'ye yazılması (BCF yazımı ADR 8103'te var), boşluk çakışması, döşeme boşluğu ile şaft ve perde
kuralları, büyük modelde uzamsal ön süzgeç, bulgunun disiplin ve revizyonla kaydı.

#### BIM-18 — İdare inceleme çalışma alanı [~]

**Kullanıcı sonucu:** Belediyenin proje inceleyeni gelen paketi (model + PDF/A + kontrol formu) KentOS'ta açar. Biçim
denetimi kendiliğinden koşar. İmar durumuyla karşılaştırma belediyenin kendi plan ve parsel verisiyle yapılır. Eksikler
gerekçesiyle ve nesnesiyle iade notuna dönüşür. Dayanak e-PYS Md. 4/3, 4/5, 5/2–3, 7/7; Z10.

- **Paket ön denetimi:** beş kabulün ilk dördü (IFC-08) ve PDF/A; müellifin formu ile KentOS'un ölçtüğü yan yana.
- **Görüntüleme:** 3B ve kat kat plan; mahal, bağımsız bölüm ve emsal alanı süzgeçleri; pafta ile model yan yana. 3B
  kısmı IFC görüntüleyicinin penceresidir (IFC-12); gevşek bağı sayesinde idare için ayrı bir programa da çıkarılabilir.
- **İmar denetimi:** parsel, çekme, TAKS, emsal, kat adedi ve yükseklik; belediyenin CBS'sindeki onaylı plan ve parselle.
  Kaynağı ve onay tarihi görünür.
- **İade notu:** madde, ölçülen ve istenen değer, nesne; yazıyla ve açık sorun biçimiyle.
- **e-PYS:** belediyenin yazılımında yapılan işlem e-PYS'ye aktarılır; arayüz yayımlanınca bağlanır.
- **Yetki ve gizlilik:** müellifin verisi üçüncü kişiye verilmez, çoğaltılmaz; erişim kaydı tutulur.

**İlk kabul:** başka bir yazılımın ürettiği açık örnek model ve KentOS'un kendi paketi açılır. Bilerek bozulan paket (yanlış
dilim, eksik yangın alanı, farklı revizyonlu pafta) doğru katmanda bulgu verir. İade notu nesneyi seçer.

**Durum (17 Kasım 2026, [ADR 8104](../docs/adr/8104-administration-package-review.md)):** ilk kabul sağlandı.
`file.review.package` paketi müellifin çizimi olmadan okur; olguları Rust ölçer, kararı `dijital_proje.inceleme` verir
(paket, pafta, model, konum, form). KentOS'un paketi EPSG:5256'da her katmanda tutar; buildingSMART'ın IFC 4.3 örneği tek
başına açılır; bozuk paketin TM33 modeli konumda, 8. revizyondan paftası paftada, yangın sınıfsız duvarı modelde
(GlobalId'siyle) bulgu verir. İade notu BCF'ye ve yazıya; Modelde göster duvarı IFC görüntüleyicide seçer. **Kalan:** imar
denetimi (belediyenin plan ve parsel verisi), e-PYS aktarımı, erişim kaydı, kat kat görüntüleme ve süzgeçler, paylaşılan
komut durumları.

### 5.7 Genişleme ve borç

#### BIM-19 — Kat irtifakı için 3B yapı modeli (araştırma önce) [~]

Tapu ve Kadastro Genel Müdürlüğünün kat irtifakı ve kat mülkiyetinde istediği üç boyutlu sayısal yapı modeli aynı bina
modelinden üretilebilir mi? Genelgenin güncel metni, biçim şeması, ayrıntı düzeyi, bağımsız bölüm ve eklenti kimlikleri,
sorumlu meslek ve doğrulama servisi birincil kaynaktan okunur (araştırma notu §9). Karar ADR'ye yazılır: yapılır, ertelenir
ya da kapsam dışı. Kaynak okunmadan kod yazılmaz.

**Durum (17 Kasım 2026, [ADR 8109](../docs/adr/8109-tkgm-3d-building-model-research.md), K197–K199): ertelendi.** TKGM'nin
2025/2 sayılı genelgesi (13.02.2025, son değişiklik 25.02.2026) mevzuat portalında doğrulandı; açık maddeleri: model onaylı
mimari projeden, Genel Müdürlüğün sitesindeki standarda göre, yapı aplikasyon projesinin eki olarak o projenin müellifinin
sorumluluğunda; Otomatik Doğrulama Uygulaması geometri ve öznitelikleri denetler, tapu, kadastro ve MAKS ile bütünleştirir,
altı haneli doğrulama kodu verir. Biçim standardı ve 4–8. maddeler okunamadı: kod yazılmadı. Yapılırsa harita modülünün işi,
mimari veri besler. Yeniden açılma: standart belgesi ve tam metin okununca.

#### BIM-20 — Mimari kullanım borcu [~]

`architecture-advanced.md`'nin kalanları her fazda bir dilimle kapanır (Z13, Z14):

- hız: 2+1 dairede ≤ 80 etkileşim ve ≤ 100 s;
- çokgen oda ve odaya çift tık; kot etiketi ve çakışmasız yerleşim; 1:20 kasa profili ve perde taraması;
- tek tuş kısayol (sahibin kararı), yüzer pencere, örnek proje ve malzeme bağı, seçenekler arası eşleme;
- gerçek ofis DWG'siyle tadilat kabulü (sahibin dosyası); `kullan --modul` ve `snapshot --sekme`.

Yeni araçlar ADV-08…10'un kapılarından geçer: kendi ikonu, kısa adı, şerit yeri, terim sözlüğü, seçenek harfi.

## 6. Uygulama sırası (B1…B23)

Her satır bir ADR, bir commit, sahibe gönderilen resimler ve bir rapordur. IFC paketleri [eş planda](ifc-uyum.md#10-iş-paketleri).

### Faz 0 — Kaynak ve kapsam

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B1 ✓ | Ekler sayfa sayfa kataloğa; başlangıç ölçümü (yapıldı, ADR 8062) | BIM-01, IFC-01 | Her kural sayfasıyla; ölçüm raporu |
| B2 ✓ | Tarihli kapsam kararı | BIM-02 | Ruhsat takımında "bu projede ne, ne zaman" (ADR 8063) |

### Faz 1 — 2B teslim tam (ilk yasal grup 1.9.2027)

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B3 ✓ | EK-3 ve EK-4 denetimi | BIM-03 | Uymayan katman ve mahal adı bulguda (ADR 8064) |
| B4 ✓ | Paftanın modelden üretildiği kanıtı | BIM-04 | Eski türevle yayın durur (ADR 8065) |
| B5 ✓ | 2B ruhsat paketi | BIM-05 | Bağımsız denetimden geçmiş PDF/A takımı ve teslim listesi (ADR 8066) |

### Faz 2 — Ruhsat düzeyinde model

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B6 ◐ | Binanın 3B görünümü | BIM-06 | Bina 3B'de; tıkla, planda seç; örtüşen mahal kırmızı (ADR 8067); ağaç, özellikler ve kesit görüntüleyicide, model çizimden canlı (ADR 8119); gerçek pencere kaldı |
| B7 ✓ | Yangın, performans ve malzeme; modelin zorunlu alanları | BIM-07, IFC-02 | Beyan dayanağıyla modelde; eksik alanlar nesneleriyle Denetim'de; 9. soru modelden; yazar gibi doldurulmuş binada zorunlu satır eksiği yok (ADR 8068, 8076); pencerelerde beyan, türetilenin gerekçesi, acil çıkış önerisi, Tag, ad ve soyad, ölçüm anotasyonu (ADR 8124) |
| B8 ✓ | Mahal, zon ve emsal; mekânsal zonlar ve gruplar | BIM-08, IFC-03 | Emsal tablosu ile model aynı alanlardan (ADR 8069); mahalin kendi emsal durumu ve DİĞER, mahalin türü, doğru katta (ADR 8125); çatıyla kesilen hacim (ADR 8071, 8123); dosyada kat × durum (ADR 8126) |
| B9 ✓ | Eksik yapı elemanları; eksik sınıflar | BIM-09, IFC-03 | Kayıp listesi boş (ADR 8070); kaplamanın mahali (8127); merdiven ve rampa korkuluğu (8128); giydirme cephede kapı ve pencere (8129); eğimli eleman (8130) |
| B10 ✓ | Geometri doğruluğu; geometri ve görünüm tanımı | BIM-10, IFC-06 | Eğri eğri, açıklık kesik (ADR 8071) |
| B11 ✓ | Çok bina; konum, kot ve çok bina | BIM-11, IFC-05 | İki blok, iki model dosyası; yerel sistem haritada; plan kararları kaynağıyla (ADR 8072) |

### Faz 3 — Doğrulama ve erken teslim hazırlığı

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B12 ✓ | Okuma yüzü ve motor denemesi | IFC-07 | Yazılan dosya geri okunur; motor kararı ADR 8061'de (§11; gerçek dünyadan 35 dosya da aynı okunur) |
| B12b ~ | IFC görüntüleyici | IFC-12 | Dosya kendi penceresinde: ağaç, özellikler, kesit ve kesit yüzü; dışa aktarılan tek tıkla görülür (ilk dilim ADR 8073, kesit yüzü 8131, tutamaklar ve yüzden kesit 8132; gerçek pencerede kabul kaldı) |
| B13 ~ | Yazılmış dosyadan doğrulama, IDS ve kontrol formu | IFC-08 | 21 sorunun kanıtı dosyadan (ADR 8074; ikinci okuyucu ve IfcTester KentOS'la aynı; öbür disiplinlerin Tablo 9.2'si paket incelemesinde ADR 8114; eşleme pencereden çıktı ve 14 kat hızlandı, bütçesi ADR 8115; beş sonuç ADR 8075'ten beri ruhsat paketinde; Denetim formu son teslim edilen modelden okuyabilir ve çizimin farkını söyler ADR 8116; bulut işi kaldı) |
| B14 ~ | BIM ruhsat paketi | BIM-14 | Tek komut, tek revizyon, tek paket (ADR 8075; gerçek paket ADR 8076; müellifin cevapları çizimde ve basılan formda ADR 8077; 2+1 de yayımlanıyor, MCP'de uçtan uca, eskiyen cevap durdurur ADR 8077 §7; 7. soru gövdelerden ADR 8133) |
| B15 ~ | Mevzuat denetimlerinin tamamlanması | BIM-13 | Başvuru tarihine göre kural seti (ADR 8078: asansör 34/1 ve 34/8'in hâlleri, eksik veri ve uzman incelemesi, uzman onayı, kaçış yolu kapıları BYKHY 47; ADR 8079: emsalin 22/1-ç, -f, -k, -l sınırları ve koşulları, kat ara toplamı; ADR 8080: merdiven altı 1,80 m; ADR 8081: kompartıman (24/2, 24/6, 24/7); ADR 8082: Ek-3 dayanımları; ADR 8083: kat yüksekliği 28/1-c ve kapılar 39/1 tarihli hâlleriyle; ADR 8084: plan notu katmanı; ADR 8085: bölgeye göre kat yükseklikleri; ADR 8086: katlar arası açıklıklar, 24/4, 24/5; ADR 8087: şaftlar, asansörler, çatılar (25/3, 41/9, 62, Ek-3/B 4 ve 12); ADR 8088: kompartıman IFC zonu; ADR 8089: asma tavan Ek-3/B 11; ADR 8090: taşıyıcı duvar ve kaçış yolu çatısı beyanları; ADR 8091: malzemenin yanıcılık sınıfı, 26/2, 26/4; ADR 8092: bodruma inen asansör 62/4; ADR 8093: çift kanat ve döner kapı 39/1-b, ç; ADR 8094: taşıyıcı sistem, 22/1-m ve 28/1-d; ADR 8095: kalın taş duvarlar 5/8, 20/8-j; ADR 8096: Sivas 22/k; ADR 8097: emsal zonunun cephe köşesi; ADR 8098: planın bina yüksekliği ve kat adedi 5/11; ADR 8134: çatı kaplamasının BROOF'u ve 28/3; ADR 8135: asansör kuyusunun havalandırması 62/4; ADR 8136: korunumlu şaft ve yangın kesici Ek-3/B 8, 10; ADR 8137: duvar stilinin taşıyıcılığı; ADR 8138: mahal kaplamalarının malzemesi 20/3, 26/2, 26/4; ADR 8139: çatının oturduğu döşeme 28/3-a; ADR 8140: plan kararının değerleri bir onayla İmar durumu'na ve plan notlarına; ADR 8141: yapı nizamı, 5/6, 26/4, 28/3 onunla; ADR 8142: vaziyet planının bitişik kenarı, PAİY 39/4, BYKHY 25/1, 25/2; ADR 8143: bölge kararın kullanımından; ADR 8144: binanın kendi bölgesi, 28/1 bina bina; ADR 8145: binanın kendi taşıyıcı sistemi, 28/1-d ve 22/1-m bina bina; ADR 8146: künyenin taşıyıcı sistemi beyandan; ADR 8147: bodrumdaki ortak mekânlar 22/1-j, katlar alanının %5'i ve 1000 m²; ADR 8148: 5/8'in bodrum listesi, otoparkın 2 katı, sığınağın gerekli alanı, mescit, gömülü ortak alanlar, çocuk oyun alanı %30 dışında; ADR 8149: sığınağın gerekli alanı 22/1-g, aşanı emsale; ADR 8150: kalın taş duvarlar bina bina; ADR 8151: duvarların içte uygulanan yalıtımı 20/3; ADR 8152: çıkmalar PAİY 41/1, 41/2; ADR 8153: subasman kotu 4/1-yyy; ADR 8154: kurallarda binanın kendi ±0,00'ı, bina bina kat adedi; ADR 8155: vaziyet planında çıkma izdüşümleri 57/3; ADR 8157: bina yüksekliği ile kat adedinin tahvili 9/1-c; ADR 8158: planın subasman kotu notları; ADR 8159: çatı kuralları pakette (40/4, 40/5, 42/1, 58/2, Sivas 40/5, 42/5); b'nin öbür asgari alanları ve planın çatı notları kaldı) |
| B16 ~ | Bilgi yönetimi ve teslim kaydı | BIM-15 | Teslim ve iade kayıtlı (ADR 8099: kayıt, nesneye bağlı iade gerekçesi, öncekiyle karşılaştırma; ADR 8100: künye yalnız nesneleri yazabilene; ilk kabul tamam; ortak veri ortamının durumları ve e-PYS pilotu kaldı) |
| B17 ⏸ | Büro ve belediye pilotu | BIM-16, IFC-11 | İdarenin yazılı cevabı (17 Kasım 2026: gerçek büro, iki proje ve idarenin cevabı sahibin işi, bekliyor; yazılım hazırlığı: paket raporuna derlemenin commit'i ve eski sürüm dosyalarının kayıp listesi, ikincisi B18 ile) |

### Faz 4 — Disiplinler ve idare pilotu

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B18 ~ | Altlık model | IFC-09 | Statik model planda ve 3B'de altlık (ADR 8101: yerleştirme harita dönüşümüyle ya da istenince başlangıç noktasına, katın kesiti, 3B gövdeler, nesnenin ayrıntıları; gösterilemeyen nesnenin işareti ve bulutta dosyanın saklanması kaldı) |
| B19 ~ | Dış disiplin ve çakışma; karşılaştırma ve sorun değişimi | BIM-17, IFC-10 | Çakışmalar nesnesiyle (ADR 8102: hizalama, kat ve aks, eşleşen ve çakışan elemanlar, datum ret; ADR 8103: revizyon farkı GlobalId ile, BCF 3.0 yazma ve 3.0/2.1 okuma; iki ilk kabul de tamam; çakışmanın BCF'si, sorunların notlarda saklanması, masaüstü BCF görüntüleyicisinde elle açılış kaldı) |
| B20 ~ | İdare inceleme çalışma alanı | BIM-18 | Gelen paket açılır, denetlenir, iade notu çıkar (ADR 8104: beş katman, bozuk paketin bulguları kendi katmanında, iade notu BCF ve yazı, nesne görüntüleyicide; imar denetimi, e-PYS aktarımı, erişim kaydı kaldı) |

### Faz 5 — Kalite ve genişleme

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B21 ~ | Tipler ve katmanlı yapı; tip nesneleri | BIM-12, IFC-04 | Değerin kaynağı görünür (ADR 8105: duvar stili `IfcWallType`, katman takımları; ADR 8106: kolon, kiriş, döşeme, çatı, korkuluk, merdiven tipi, istisna, Öznitelikler'de Tipten/Örnekten, IFC tipleri; iki ilk kabul de tamam; giydirme cephe tipi, profil takımı, eski sürüm denetimi, kitaplık kaldı) |
| B22 ~ | Konut dışı kullanımlar (yasal en geç 1.9.2033) | BIM-13 | Kullanım sınıfına özel kurallar (ADR 8107: BYKHY 49/1-a hasta odası, 50/1-a ve 50/1-b otel, 52 tek merdivenli fabrika, mağaza, depo, büro; ADR 8108: 96/2 yağmurlama zorunluluğu kullanıma, yüksekliğe ve alana göre; ADR 8110: 49/1-b büyük yatılan katın kompartımanları, 49/2 hastane koridoru, koridorun en dar yeri; ADR 8111: 50/3 otel odasının içindeki kaçış uzaklığı; 101 bağımsız durum, gerçek çizimde Denetim; ADR 8156: 50/4–50/5 otel ve yurt yatak odasının kapısından kaçış uzaklığı, 21 durum; 50/2, 50/6, 51, eğitim, toplanma, endüstri ve depolama kaldı) |
| B23 ~ | Kat irtifakı için 3B yapı modeli araştırması | BIM-19 | Karar ADR'de (ADR 8109: ertelendi; TKGM 2025/2 genelgesinin açık maddeleri okundu, biçim standardı okunamadı; yapılırsa harita modülünün işi) |

- **Sürekli:** BIM-20 her fazda bir dilimle ilerler.
- **Erken hazırlık:** B12 model geliştirmesiyle paralel başlayabilir (aşağıdaki maddeye bakın). B15 model paketlerinden
  bağımsızdır. B14–B17’nin teknik ön kontrolü, zorunlu IFC teslimi ve e-PYS aktarımı devreye girmeden pilotlanabilir.
- **Görüntüleyici erken başlayabilir:** B12 ve B12b kataloğu (B1) ve model paketlerini beklemez; şema sözlüğü motordan
  gelir. Kataloğa bağlı olan B13'tür. B6 (binanın 3B görünümü) ile B12b aynı çekirdeği ve pencere parçalarını kullanır;
  hangisi önce yapılırsa parçaları o kurar.
- **Sahibin seçimi:** öncelik belediyenin inceleme tarafıysa B12 → B12b → B18 → B20 öne alınır; müellif tarafının sırası
  değişmez.

## 7. Kabul ölçüleri ve kapılar

| Ölçü | Bugün | Hedef | Kapı |
|---|---|---|---|
| Kapsam kararı | yok | ≥ 20 durumda doğru | 1 |
| PDF/A, bağımsız doğrulayıcı | ölçülmedi (kendi denetimi var) | 0 hata | 1 |
| Model ile paftaların revizyonu | kayıtlı değil | aynı ve kayıtlı | 1, 3 |
| Yazılan zorunlu sınıf (29'dan; temel ve tabela dışında 27) | 15 | 27 | 2 |
| Modele girmeyen nesne (rampalı, giydirme cepheli, asansörlü örnek) | rampa ve sınıfsız bloklar | 0 | 2 |
| EK-6 ve EK-7'nin zorunlu satırlarından dosyada bulunan | 81 / 121 (künyeli bölüm binası, ADR 8062) | hepsi dolu ya da gerekçesiyle "eksik" | 2 |
| 3B'de görülen gövde / dosyadaki gövde | 0 / 68 | eşit | 2 |
| Şema kuralları, bağımsız okuyucu | 0 bulgu (bölüm binası) | 0 bulgu, her örnekte | 2, 3 |
| Türkiye profili, bağımsız ikinci okuyucu | ölçülmedi | 0 hata | 3 |
| Kontrol formunda dosyadan kanıtlanan soru | 0 / 21 | makinece denetlenebilenlerin hepsi | 3 |
| Büyük modelde (100 daire) dışa aktarma süresi ve belleği | 4 202 nesne: model eşlemesi 216 ms (önce 3,1 s; emsal ölçümü komşularla, ADR 8115), yazma 19 ms, geri okuma ve olgular 334 ms (tepe 149 MB), form 57 ms; eşleme pencerenin iş parçacığından çıktı (pencere 0,08 ms okuma kopyası öder) | Bütçe (ADR 8115 K221): okuma kopyası ≤ 1 ms, eşleme 100 dairede ≤ 300 ms, Denetim'in Dijital proje cevabı ≤ 1 s | 3 |
| 2+1 daire, gerçek pencere | 82 etkileşim, 121,3 s | ≤ 80, ≤ 100 s | sürekli |

| Kapı | Yasal en geç | Koşul |
|---|---|---|
| 1 — 2B teslim | 1.9.2027 (ilk grup) | B1–B5; paket bağımsız doğrulayıcıdan hatasız |
| 2 — Ruhsat düzeyinde model | IFC'nin ön koşulu | B6–B11; kayıp listesi boş; zorunlu alanlar dolu ya da "eksik" diye listeli |
| 3 — BIM teslim | 1.9.2029 (ilk grup) | B12–B17; beş kabulün ilk dördü dosyadan; pilot idarenin yazılı cevabı |
| 4 — Disiplinler ve idare | Erken pilot; e-PYS aktarımı Bakanlık kararına bağlı | B18–B20 |
| 5 — Bütün binalar | 1.9.2033 | B22 |

Her kapıda `architecture-advanced.md` §7'nin faz kapısı da koşulur: kabul izleri gerçek pencerede, resim matrisi sahibe,
sahibin kısa listesi.

## 8. Açık sorular ve riskler

| Soru | Kim cevaplar | Etkilediği |
|---|---|---|
| ~~Mahalin modeldeki adı: EK-5 §5.5.4 mü, yalın EK-3 numarası mı~~ Cevaplandı: EK-5 5.5.4, şablonun açıklamasında EK-3 numarası ve ismi (araştırma notu §4.1) | Ek sayfası | IFC-02 |
| Yükseklik: EK-7 Tablo 7.6 ITRF96 (gerekirse TUDKA99 dönüşümü), EK-4 4.31.4 Helmert ortometrik | İdare | IFC-05 |
| Görünüm tanımı; PDF/A'nın bölümü ve düzeyi; dosya boyutu sınırı | Bakanlığın kılavuzu ya da idare | IFC-06, BIM-05 |
| e-PYS'nin arayüzü, imza yöntemi, yükleme biçimi ve tarihleri | Bakanlık (e-PYS Md. 4/4, 5/4, 5/6) | BIM-15, BIM-18 |
| Ortak bodrum hangi binanın; bir parselde kaç model dosyası | İdare | BIM-11, IFC-05 |
| 2B sembol donatı (mobilya) modele girer mi | EK-8; idare | BIM-09 |
| Gruplama sınıfının (zon) neyi temsil edeceği | Ek sayfası | BIM-08 |
| Nüfus hangi yıla ve kaynağa göre | Bakanlık ya da idare | BIM-02 |
| TS 9111 ve öbür TSE metinlerindeki ölçüler | TSE (ücretli metin) | BIM-13 |
| Pilot büro ve idare | Sahip | BIM-16 |
| İdare tarafı öne alınsın mı | Sahip | §6 |

- **Ekler yanlış okunursa profil yanlış olur.** Önlem: B1 her şeyden önce; iki bağımsız okuma; her satırın sayfası. B1'de
  şema ve döküm karşılaştırması 14 okuma hatası buldu (ADR 8062 §2.3); kurala dayanan her yeni satır sayfasıyla
  karşılaştırılır.
- **Bakanlık biçimi ve tarihi değiştirebilir** (e-PYS Md. 4/4). Önlem: K2.
- **Başka yazılımların dosyaları profile uymayabilir** (Estonya'nın dersi; bağımsız aracın açık örneklerdeki ölçümü,
  araştırma notu §6 ve §8). Önlem: idare tarafında hoşgörülü okuyucu, açık bulgu.
- **"Modelden üretildi" sözü 2B öncelikli bir araçta sorgulanır.** Önlem: BIM-04'ün kaydı ve yayın kapısı.
- **Büyük model.** Önlem: süre ve bellek tabanı B13'te; bütçe ondan sonra (kılavuz §6).
- **Okuma motoru dış bir projedir ve hızlı sürüm çıkarıyor** (on ayda 24 ana sürüm; araştırma notu). Önlem: sürüm sabit,
  motorun türleri yalnız `kentos-ifc`'te, yükseltme örnek takımıyla; deneme geçemezse kendi okuyucumuz (ADR 8061 §7–§9).
- **Gevşek bağ zamanla sıkılaşabilir.** Önlem: bağımlılık yönü denetimi, wasm32 derlemesi ve tek pencere örneği her
  dilimde koşar (ADR 8061 §3.3).

## 9. Kurallar

- **Madde talimatı:** `architecture-advanced.md` §6. Araştırma her dilimde yenilenir; değer uydurulmaz, kaynak beklenir.
- **IFC:** IFC'ye ait her şey `crates/shared/ifc`'tedir; uygulamada yalnız bağdaştırıcı kalır (kılavuz §14).
- **Araç = komut:** çizimi değiştiren her iş komut, katalog, ≥ 20 bağımsız durum, Python, MCP ve manifestle biter.
- **Denetim:** kararı Python betiği verir, Rust ölçer ve okur (K10). Kural uygulamaya gömülmez.
- **Adlar:** arayüzde, kodda ve yorumlarda başka uygulama anılmaz; karşılaştırma yalnız ADR'de, planda ve araştırma notunda.
- **İddia:** "yönetmeliğe uygun" denmez; beş kabulden hangisinin, hangi profil sürümüyle, hangi örnekte sağlandığı söylenir.
  İdarenin kabulü yazılı cevaptır.
- **Veri:** gerçek ruhsat modeli ve müellif bilgisi dış servise ve açık test korpusuna gitmez.
- **Bu planın kabulü** uygulamanın üretim kabulü değildir.

## Ek A — Tasarımı etkileyen mevzuat

İlk taslaktan aktarıldı; bu turda yeniden açılmadı. Ayrıntı ve işaretler W4'tedir. Her kaynak uygulama anında proje türü,
yeri, geçiş hükümleri ve değişiklikleriyle yeniden değerlendirilir.

| Kaynak | Ürün davranışı | Kabul kanıtı |
|---|---|---|
| 3194 İmar Kanunu (4, 8, 21–23, 26–27) | Parsel, plan, ruhsat türü, yetkili idare, proje evrakı | Resmî imar durumu ve başvuru türüyle izlenebilir karar. [Kanun](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.3194.pdf) |
| Mekânsal Planlar Yapım Yönetmeliği | Onaylı uygulama imar planı, plan notları ve raporundan fonksiyon ve yapılaşma zarfı; planın numarası, onayı, kaynağı | Birden çok plan değişikliği olan parselde onay tarihi ve not maddesiyle yeniden üretilebilir sonuç. [Konsolide metin](https://www.lexpera.com.tr/mevzuat/yonetmelikler/YR801Y2014N29030P1/3) · [Plan gösterimleri](https://mpgm.csb.gov.tr/plan-gosterimleri-4926) |
| Planlı Alanlar İmar Yönetmeliği (4–5, 19–23, 26–42, 57–58) | Alan tanımları, TAKS ve emsal, çekmeler, kot, piyes, merdiven, asansör, rampa, çatı, çıkma, ışıklık, ruhsat çizimleri; 1.7.2026 değişikliği ayrı profil | Bağımsız alan hesabı; istisna kalemleri maddesiyle. [Konsolide metin](https://www.lexpera.com.tr/mevzuat/yonetmelikler/planli-alanlar-imar-yonetmeligi-4) · [1.7.2026 değişikliği](https://www.resmigazete.gov.tr/eskiler/2026/07/20260701-7.htm) |
| Binaların Yangından Korunması Hakkında Yönetmelik | Kullanım ve kullanıcı yükü, kompartıman, malzeme ve eleman dayanımı, kaçış yolu, korunumlu merdiven, çıkış kapısı | En uzak nokta, net geçiş, kapasite, rota ve istisnanın görsel kanıtı; yangın uzmanı örnekleri. [Bakanlık kılavuzu](https://webdosya.csb.gov.tr/v2/meslekihizmetler/2026/05/Binalar-n-Yang-n-Korunmas-Hakk-nda-Y-netmelik-K-lavuzu-20260507112134.pdf) |
| 5378 ve erişilebilirlik hükümleri; TS 9111 | Parselden erişilebilir girişe ve mahallere güzergâh, rampa, sahanlık, net kapı, dönüş alanı, asansör, tuvalet | Standardın baskısı ve maddesi; bağımsız boyut örnekleri. [5378](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.5378.pdf) |
| Otopark ve Sığınak Yönetmelikleri | İhtiyaç, erişilebilir park, geometri, araç rampası; sığınak alanı ve kapasitesi | Karma kullanım, bodrum ve istisnayı içeren elle hesap. [Otopark](https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=24408&MevzuatTertip=5) · [Sığınak 2025](https://www.resmigazete.gov.tr/eskiler/2025/11/20251107-2.htm) |
| Binalarda Enerji Performansı Yönetmeliği; TS 825 | Isıl zarf, koşullandırılmış mahal, katman ve beyanlı U değeri; enerji hesabı mühendisin | Aynı dış zarfın mimari ve enerji modelinde eşleşmesi. [BEP](https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=13594&MevzuatTertip=5) · [TS 825 tebliği](https://www.resmigazete.gov.tr/eskiler/2025/02/20250220-2.htm) |
| 634 Kat Mülkiyeti Kanunu (2–4, 12) | Bağımsız bölüm, eklenti, ortak yer, arsa payı, proje listesi | Dubleks, ortak merdiven ve eklenti örneğinde alan tekrarsız. [Kanun](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.634.pdf) |
| Türkiye Bina Deprem Yönetmeliği; 4708 ve Yapı Denetimi Uygulama Yönetmeliği | Aks, kot, taşıyıcı sınırı; kontrol formu ve müellif rolleri | Çakışma ve müellif kanıtı; deprem hesabı statik disiplinde. [4708](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.4708.pdf) |
| 5070 ve 6698 sayılı Kanunlar | Güvenli e-imza; kişisel veri | [5070](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.5070.pdf) · [6698](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.6698.pdf) |

## Ek B — Model sözleşmesi

İlk taslaktan aktarıldı; hangi paketin kurduğu eklendi. Yeni `.kcad` alanları sürümlenir ve eski dosyanın okunması sınanır.
Kaynağı bilinmeyen sınıflandırma, performans ya da datum kullanıcıdan teyit bekler.

| Veri grubu | Asgari alanlar ve ilişkiler | Davranış | Paket |
|---|---|---|---|
| Proje, parsel, saha, bina, kat | Kalıcı kimlik; resmî ve yerel kod; yapı ve başvuru türü; plan profili; yatay ve düşey sistem; röper, ±0.00; bina ve kat sırası | Bir sahada birden çok bina, ortak bodrum, farklı kot. Her elemanın mekânsal sahibi belli | BIM-02, BIM-11 |
| Aks ve başvuru düzlemleri | Etiket, geometri, bina, kat ya da çatı düzlemi | Ölçülen değer ile hukuki sınır ayrı alanlar. Kot değişince bağlı elemanların raporu ve geri alma | BIM-11 |
| Duvar, kolon, kiriş, döşeme, çatı | Tip ve örnek; katman sırası; başvuru yüzü; alt ve üst kot; malzeme; taşıyıcı, dış ve zarf durumu; bağlantılar | Kesim ve birleşim tek geometri hesabından. Statik bilginin kaynağı belirsizse teyit istenir | BIM-07, BIM-10, BIM-12 |
| Kapı, pencere, açıklık | Duvarı; boşluğun profili; kasa, kanat ve yön; eşik; nominal ve net ölçü; parapet ve başlık; çıkış rolü; yangın, duman ve ısı performansı ile belgesi | Açıklık duvarı oyar, kapı ve pencere onu doldurur. Duvar taşınınca, silinince, bölününce, kopyalanınca bağlıları tek adımda izler | BIM-07 |
| Mahal, bağımsız bölüm, zon | Kapalı sınır ve hacim; alan türü; kot; kullanım; kaplamalar; komşuluk; kullanıcı yükü; ortak yer ve eklenti; yangın ve enerji zonu | Kendiliğinden bulunan sınır ile elle çizilen ayırıcı ayrı kökenlidir. Atrium, dubleks ve açık balkonda kat alanı tekrarlanmaz | BIM-08 |
| Merdiven, rampa, şaft, asansör, korkuluk | Alt ve üst kat; kol, sahanlık, eğim; net genişlik ve baş yüksekliği; boşluk ve korunum; kaçış ve erişilebilirlik rolü | Tek nesneden plan oku, kesit, 3B ve dolaşım bağlantısı | BIM-09, BIM-13 |
| Kaplama, donatı, tesisat rezervasyonu | Tip; ana eleman; işlev; hacim; disiplin sahibi; görünürlük; geçiş ve denetim durumu | Sembolik donatı, gerçek rezervasyon ve yetkili mühendislik nesnesi ayrı anlam taşır. Çakışma bulgusu kaynak nesneye döner | BIM-09, BIM-17 |

## Ek C — Olumsuz kabul örnekleri

İlk taslaktan aktarıldı. Her biri ilgili paketin bağımsız başvurusunda bir "kalan" durumdur.

| Konu | Örnek | Beklenen | Paket |
|---|---|---|---|
| İmar ve alan | Aynı balkon hem emsal harici hem ortak alan toplamında düşülmüş | Çift sayım gösterilir; etkilenen çokgen seçilir | BIM-08 |
| Yangın ve kaçış | Ayrı görünen iki çıkışın ortak dar boğazı; kapalı geçiş; yanlış yağmurlama beyanı | Üçü ayrı bulgu. Dayanım ve duman sızdırmazlık beyanı kaynak belgesinden | BIM-07, BIM-13 |
| Erişilebilirlik | Rampanın eğimi uygun, kapının açılışı sahanlığı kapatıyor | İkisi birlikte ölçülür | BIM-13 |
| Disiplinler | Aynı koordinat etiketi farklı yükseklik datumlarını gizliyor | Çakışma denetiminden önce dönüşüm doğrulaması istenir | BIM-17 |
| Bilgi bütünlüğü | PDF/A teknik doğrulamadan geçiyor, ama kat planı başka revizyondan | Paket durur | BIM-04, BIM-14 |
