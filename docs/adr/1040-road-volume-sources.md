# 1040 — Kübaj neyden ve hangi algoritmayla hesaplandığını söyler

Tarih: 2026-11-07. Durum: kabul (Y105). Kapsam ROAD-VOL-01, ADR 0906 (kübaj), ADR 1037 (kübajın ne ölçtüğü), ADR 1038
ve 1039 (aplikasyonun ve enkesitin kaynakları).

## Sorun

ROAD-VOL-01 bir kübaj hesabının şunları taşımasını istiyor: kaynak yüzeyleri (mevcut ve tasarım), çalışma alanı,
örtülmeyen ve verisiz alan, yöntem, tolerans, algoritma sürümü ve birimler. Denetimde (2026-11-05) yöntem, revizyon,
koridor ve örtülmeyen alan vardı; kaynak özetleri, çalışma alanı, tolerans ve algoritma sürümü yoktu. Sonra ADR 0974
örnekleme adımını (`step`), ADR 1037 kuralı, yanları ve km aralığını (`basis`) ekledi.

Eksik kalanlar:

- **Kaynaklar:** rapor hesabın hangi koridordan, aksın, boykesitin, tip kesitin ve dever çizelgesinin hangi
  sürümünden, hangi araziden hesaplandığını söylemiyordu.
- **Algoritma:** yöntemin adı vardı, sürümü yoktu.

Ayrıca bir açık bulundu. ADR 1038 ve 1039 araziyi yalnız son kabul özetiyle (`built`) anıyordu. Oysa yüzey her
okunuşta çizimin bugünkü hâlinden kurulur. Çizim kabulden sonra değiştiyse cevap kabul edilen araziden değil bugünkü
çizimden hesaplanmıştır, ama kaynak satırı bunu göstermiyordu.

## Araştırma (7 Kasım 2026)

- **Yapım İşleri Genel Şartnamesi** (Yapım İşleri İhaleleri Uygulama Yönetmeliği Ek-8; hakedis.org'un 18.05.2024
  derlemesi, 08.08.2019 değişiklikleri işlenmiş):
  - Md. 28/1: ataşman, röleve ve şantiye defterleri ile ilgili belgeler yüklenici ile birlikte yapı denetim
    görevlisi tarafından tutulur.
  - Md. 28/2: imzalayan yüklenici "içindekileri ve yapılan hesapların doğruluğunu kabul etmiş olur"; bir yanlışlıkta
    "ilk rakam ve yazı okunacak şekilde" üstü çizilip doğrusu yazılır.
  - Md. 39/1-c: "Metrajlar, yeşil defter ve eklerinde gösterilir"; geçici hakediş kesin ödeme ve kazanılmış hak
    sayılmaz.
  - Md. 40/1-a: kesin hakedişte "geçici hakediş raporlarındaki rakamlara itibar edilmez".

  Aynı kalemin miktarı iş boyunca birkaç kez hesaplanır ve imzalanır. İmzalanan rakamın neyden hesaplandığı rakamın
  yanında durmalıdır; yoksa iki hesabın farkı arazinin mi, tasarımın mı, yöntemin mi değişmesinden geldi, ayırt
  edilemez.
- **Civil 3D:** Volumes Dashboard her hacim yüzeyinin taban ve karşılaştırma yüzeyini adıyla listeler; kaynağı
  değişen yüzey ve koridor "out of date" olur. KentOS'ta karşılıkları arazinin iki özeti ve kaynakların sürümleridir.
- **KentOS'un kaynakları:**
  - Koridor, aks, boykesit, tip kesit ve dever tanımı birer revizyon taşır (ADR 0908).
  - Arazinin bugünkü özeti `road_terrains::sources_digest`'tir (çizimin noktaları ve çizgileri, yöntemiyle; ADR 0900).
    Son kabul özeti yüzeyin `built` alanıdır.
- **Matematik:** yok. Toplamlar satırların km sırasıyla binary64 toplamıdır (`road_volumes::compute`). Bu sıra
  algoritmanın parçasıdır: sıra ya da toplama biçimi değişirse son basamaklar değişebilir.

## Karar

- **`RoadSurfaceSource.digest`:** arazinin bugünkü özeti (projede yüzey yoksa yok). Aplikasyon, enkesit ve kübaj
  kaynakları onu `built`'in yanında verir. İkisi aynıysa yüzey bugünkü çiziminden kabul edilmiştir; farklıysa çizim
  kabulden sonra değişmiştir ve cevap bugünkü çizimindir.
- **`VolumeReport.sources` (`VolumeSources`):** yalnız `road.volumes.query`'nin cevabında ve hesaplanan kübajda; her
  biri bugünkü hâliyle:
  - koridor, aksı ve boykesiti;
  - bölgelerinin tip kesitleri, sırayla ve birer kez;
  - dever çizelgesi;
  - arazi (mevcut zemin; tasarım koridorundur).
  Projede olmayan kimlik sürüm 0 okunur. Hesaplanamayan kübajda yok.
- **`VolumeBasis.algorithm`:** yöntemin algoritması adı ve sürümüyle (`kentos-road`'un `volume.rs` sabitleri). Bir
  yöntem aynı kaynaklardan başka rakam verirse sürüm artar:
  - `kentos.end-area/1`: ortalama alan;
  - `kentos.end-area-curved/1`: kurp düzeltmeli ortalama alan (ADR 0934);
  - `kentos.prismoidal/1`;
  - `kentos.tin-overlay/1`: yüzeyler arası;
  - `kentos.grid-midpoint/1`: ızgara (ADR 0974).
- **Ortak türler:** `RoadSource`, `RoadSurfaceSource` ve `VolumeSources` sözleşmeler sandığına (`kentos-contracts`)
  taşındı. Onaylı hesabın kaynaklarını saklamak (Y106) bunları `VolumeApprovalDef`'te kullanacak; komutlardaki adları
  değişmedi.
- **Uygulama:** `road_sources.rs` (`surface`, `corridor`, `acceptance_words`, `surface_words`). Aplikasyon, enkesit
  ve kübaj aynı işlevleri kullanır.
- **Pencereler:** ortak yazı `road_dialog`'da (`chain_words`, `surface_source_words`, `volume_sources_words`).
  - Kübaj: formda Koridor alanının altında "Kaynaklar: koridor c-1 (1), aks g-1 (1), boykesit b-1 (1), tip kesit tk-1
    (1), tk-2 (2), arazi t-1 (2751c2db…, kabul edilmedi)". Satır önce diyagramın altındaydı: tam ekranda (1920 × 1080)
    diyagramı 453 px'ten 428 px'e indirdi ve en az 440 px kuralını bozdu (`picture_tests`). Form sütunu kendi içinde
    kayar, diyagramın yerinden almaz.
  - Yöntem satırı algoritmayı söyler: "Yöntem: Ortalama alan (kentos.end-area/1), en büyük kesit aralığı 25 m."
  - Aplikasyon ve Enkesit araziyi aynı biçimde yazar.
  - Aplikasyonun Kaynaklar sayfası arazinin 64 haneli özetini ve kabulünü yazar.
- **Tolerans:** iki yerden okunur. Örnekleme adımı `step`'tedir (ADR 0974). Arazinin kot toleransı ve kenar yöntemi
  özetin girdisidir. Onayla karşılaştırmanın eşiği (1e-6 m³) ADR 0950'nindir. Yeni alan gerekmedi.
- **Açık (Y106):** onaylı hesap (`VolumeApprovalDef`) onaylandığı kaynakları saklamıyor. Onaydan sonra hangi kaynağın
  değiştiği ancak bugünkü kaynaklar onayla birlikte saklanınca söylenebilir. Bu yeni bir KCAD şeması ister. ROAD-VOL-01
  bu yüzden kısmen kalır.

## Ölçüm

- **Bağımsız başvurular** (KentOS kodu yok):
  - `road_profile_command_cases.py::surface_source` özeti arazi başvurusunun (`road_terrain_command_cases.digest`)
    girdisinden kurar: çizimin kotlu noktaları, katmanı ve alt katmanlarıyla, çizim sırasıyla.
  - `road_corridor_command_cases.py::corridor_sources` koridorun zincirini kurar; aplikasyon ve kübaj onu kullanır.
  - Kübajın algoritmasını `with_basis` yöntemden ve kurp düzeltmesinden okur.
- **`road.volumes.query`:** 50 durum (2 yeni). Kaynakları hesaplanan her kübajda karşılaştırılır. Yeni durumlar:
  - bugünkü çizimden kabul edilmiş arazi: iki özet aynı;
  - ilk zemin noktası eklenmeden önce kabul edilmiş arazi: özetler farklı.
- **Öbür dosyalar:** `road.stakeout.query` 26, `road.sections.query` 37 durumla geçiyor; özet artık bunlarda da
  karşılaştırılıyor.
- **Bozma denemesi:** bir özet ve bir algoritma sürümü bozulunca 48 durumun 2'si düşüyor.
- **Testler:**
  - Uygulama: `acceptance_words`'ün üç hali.
  - Masaüstü: kübaj penceresinde kaynak satırı ve algoritma. Arazi `road.terrains.edit accept` ile kabul edilince
    "kabul edildi", ilk zemin noktası silinince "kabulden sonra değişti". Aplikasyon ve enkesit testleri yeni yazıyla.
- **Resimler** `.run/gui-evidence/yol-y105/`: kabul edilmemiş, kabul edilmiş (açık, 1440 px) ve kabulden sonra
  değişmiş (koyu, 1100 px).
