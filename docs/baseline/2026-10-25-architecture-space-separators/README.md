# Kalıcı sanal mahal ayırıcı — ADR0679

25 Ekim 2026, `codex/architecture-2d-workflows`, üst commit `c5049997b`.
**Bu işlev dilimi doğrulandı; bütün mimari plan ve genel performans kabulü tamamlanmadı.**

## Davranış ve sayısal başvuru

Mimari/Çiz **Mahal ayırıcı**, fiziksel duvar oluşturmadan açık planı böler.
Kalınlığı sıfırdır; mevcut çoklu çizgi genişliği bu araca aktarılmaz. Düz,
yaylı, açık veya kapalı path açık `spaceSeparator` rolü taşır. Sıradan
çizginin adı, rengi, katmanı veya kesik çizgi görünümü rol değildir.

10×8 m dış duvar ekseni ve 0,2 m duvar için net alan 76,44 m². x=5
sanal ayırıcıyla 38,22+38,22 m²; 2×2 m kolon sol tarafta düşülünce
34,22+38,22 m². Sanal ayırıcıyı x=6'ya taşıma 42,02+30,42 m² verir.
Aynı yerde fiziksel 0,2 m bölme farklı malzeme/alan hesabıdır.

`cad.spaces.separators` create/set/clear, plan ve tek execute/undo aynı
Rust hesabını kullanır; Python/MCP aynı anlamsal komuttur. Açık kaynak
listesi ayırıcı UUID'sini saklar. Eksik/kapatılmış kaynak son sınırı tutar;
bütün çizime sessiz genişlemez. Genel desteklenen edit/kopya rolü korur;
yanlış temsil, genişlik, kot/M veya çöken kenar atomik reddir.

Kök ayırıcıyı blok tanımı/içerik güncelleme/blok yapıştır içine alma açıkça
reddedilir. Yüklenmiş nested bloktaki rol KCAD'de korunur; desteklenmeyen
Patlat rolü atmak yerine ret verir. Blok içi kaynaklarla mahal hesabı bu
kabulün kapsamında değildir. Düz yapıştır rolü korur.

KCAD şema 106 yalnız yeni rolde seçilir; önceki kanonik dosyalar sabittir.
DXF metadata açıkken rol ve seçilmiş UUID kaynakları import eşlemesinden
geçer; kayıp/yinelenen dış kimlik keyfî kaynağa bağlanmaz. Metadata kapalı
aktarımda semantik rol kaybı raporlanır. DWG roundtrip kabulü yapılmamıştır.

## Gerçek arayüz kanıtları

`gui-events.json` ham JPEG'leri, notlarını ve kullandıkları derleme kaydını
eşler. JPEG baytları değiştirilmemiştir. `first-binary.json` ilk denemeleri;
`after-binary.json` açıklama düzeltmeli adayı; `verified-binary.json` son kaynakların
ve gerçek test executable’ının SHA-256 kimliğini ayırt eder. Executable
`Contents/MacOS/kentos-cad-bin`; `kentos-cad` izole profil launcher'ıdır.

İlk 39 görüntü: tek Enter, Undo/Redo, rol kaldır/geri al, kat/evre,
sağ tık → tek Onayla/bitir, Escape, ölçülü küçük zoom ve ters zoom,
kaydet/aç ve açık kaynak seçimi. Gerçek dar istemci 1100×650; geniş
istemci yaklaşık 1440×833. Son derlemede yeni araç ayarlarının açık/koyu × 13/18 px × dar/geniş sekiz
görünümü 43–44 ve 53–58 ile doğrulandı; `ui-matrix.json` gerçek ham ve
istemci boyutlarını kaydeder. Dar 1100×650, geniş 1440×833; bütün iş
akışlarının 1440×900 matrisi kabul edilmiş sayılmaz.

`verify.py` gerçekten GUI'nin kaydettiği KCAD dosyalarını bağımsız Python
okuyucuyla çözer; alanları Decimal köşe çarpımlarıyla ölçer. UUID, rol,
kaynak üyeliği, ilgisiz duvar/kolon, oda metadatası, proje ayarları ve tam
Undo/Redo eşitliği kontrol edilir. Test çizimi/profili `.run` altındadır.

### Başarısız veya tanısal denemeler

- 02, 03 ilk tıklamaları aracı açmadı; 04 gerçek araçtır.
- 09 oda seçimidir; 11 seçim listesinden gerçekten ayırıcıyı seçer.
- 19 odaksız alias yazımı mevcut kısayollarla Dizi açtı; ayırıcı kabulü değildir.
- 23–24 aşırı scroll denemesidir; 25 Home, 26–27 ölçülü ters zoom kabulüdür.
- 30 duvarı isabet ettirmedi; 31 bunun geçersiz kaynağını reddeder.
  32–33 iç duvar yüzüyle gerçek tam üç kaynak ve tek onaydır.
- 35 panel önceki kaydırma konumundadır; görünür ayar kabulü değildir.
  36 panel yukarıdayken Kat ve tam açıklamayı gösterir.
- 11/39 seçili nesnenin uzun açıklamasının kırpılmasını gösterir; son
  derlemede iki kısa açıklama 42/45 ile açık/koyu ve 18 px dar pencerede
  tam okunur. 41 açıklamalar aşağıda kaldığından kabul sayılmadı.
- İlk shared4 koşusu yanlış filtreyle sıfır test çalıştırdı; başarı sayılmaz.
- shared5 başvurusunda eski varsayılan `justify=center` ve revision ret kodu
  bekleneni eksikti; bağımsız generator düzeltildi, shared6 83 durum geçti.
- python6 296 testi geçti; son clippy iki iç içe if için başarısızdı.
  Sonraki clippy7 ayrıca input yolundaki `expect` ve testte yanlış modül
  çağrısını yakaladı. clippy8 temizdir.
- application8 blok KCAD kontrolünde testin blok içi yerel id'si 4 iken
  kanonik okuyucu 1'e numaralandırıyordu. Rol/alan/ret kontrolleri geçti;
  test kurulumu yerel id=1 yapılarak tam model karşılaştırması korunmuştur.

- 48 paste ile koordinat eklemedi; ilk noktayı bekleyen boş araçtır.
  49–52 gerçek klavye girişi, sağ tuş → tek onay ve tek undo tekrarlarıdır.
- desktop9: 1283 test başarılı/221 atlanan; yalnız yeni komutun yerelleştirilmiş
  history başlık listesi eksikti. İki satırlık başlık kaydı düzeltildi;
  history10 etkilenen üç testi tekrar geçti.
- 37–39 görüntülerinin event dizini eski bellek listesi tarafından atlanmıştı.
  Ham JPEG dosyaları değişmedi; dizin mtime kaynak notuyla onarıldı.
  Bağımsız verifier artık görüntü dosyaları ve event kümesinin tam eşitliğini de
  denetler.
- python10: 298 testten 297'si geçti; yeni yapıştır testinin `uids` yerine
  gerçek sonuç alanı `created` kullanması gerekiyordu. Üretim yapıştırma kodu
  değiştirilmedi; python14 tam koşusu 300 test ve son clippy dahil geçti.
- shared11: yeni köşe testi sözleşmede olmayan `edge` konumu kullandı;
  `mid/part/ring/index` olarak düzeltildi. shared12'de 85 bağımsız durum
  validate/plan/execute ve model korunumu denetiminden geçti.
- 59–62: yeni uyarı regresyonu. GUI kolon düşümünü kapatır (38,22 m²),
  dış kaynak duvarı siler; mahal `lostEnclosure` ve son sınırla kalır,
  fakat Metraj yeşil onayla eski alanı sunar. 61 ayrıca 18 px dar
  tabloda başlık çakışmasını gösterir. İki gerçek undo 62 ve
  `warning-before-restored.kcad` ile tam önceki modeli geri getirir.
  Kaynaktan birinci nesne silindiğinde KCAD dosyası geçici slot id'lerini
  sıkıştırır; denetim bütün kalıcı UUID'leri ve diğer alanları korur.
  Kusurun son derlemedeki düzeltme kabulü 67–75 ve gerçek CSV dosyalarıyla
  aşağıda kayıtlıdır.
- application13'te bütün ortak durumlar ve 215 entegrasyon testi geçti;
  ek native test yanlış yardımcı çizimde bulunmayan `mahal` katmanını
  kullanıyordu. Kurulum o çizimin var olan `cizim` katmanına düzeltildi;
  application14 51 kütüphane + 216 entegrasyon testini geçti (22 atlanan).
- quantities-ui14 karışık geçerli/kopuk satırın ekran/dosya sütunlarını
  geçti. 162 saniyelik soğuk test süresini pencere gecikmesi saymayın:
  örneklenen stack, test executable'ı `target/debug/deps` altındayken
  macOS CoreFoundation bundle bilgisi dizin taramasında, ilk font sistemi
  kurulumundadır. Ham örnek `testlogs/quantities-ui-sample.txt`; paketli QA `.app`
  ile 63–76 gerçek ekran kabulü ayrıca yapıldı.

## Büyük belge ölçümü — başarı eşiği değildir

`measure.py` public Python → native çağrılarıyla 200 yapı, 400 mahal,
200 kolon, 200 ayırıcı, 400 etiket, 400 satırlı çizelge ve 20.000 ilgisiz
çizgi kullanır. Bağımsız beklenen alanlar, tam KCAD undo, etiket ve tablo
birlikte kontrol edilir. İlk debug/eşzamanlı derleme sonucu
`measure-first.json` içindedir: plan 68,8 ms, execute 3,44 s, bütün metraj
9,75 s, tek ayırıcı/follower 23,4 ms. GPU/release/UI etkileşim süresi değildir.
Metrajın her mahal/yöntemde aynı kaynak ağını yeniden kurması sonraki
performans işidir; ARCH-SPACE-01 ve ARCH-PERF-01 açık kalır.

## Açık kalite sınırı

Genel düzenleme primitive'i desteklenmeyen rol dönüşümünü atomik reddeder.
Bunun bütün üst seviye araçların `validate` ve katalog ret kodlarıyla tam
paritesi ayrıca denetlenecek. Bu denetimde bulunan `cad.curves.edit` →
ToSpline ve `cad.entities.vertex` → width ön kontrol eksikleri düzeltildi:
validate/plan/execute aynı mevcut `edit_not_applicable`/`invalid_polyline`
retlerini verir; model değişmez. Yeni komutun 105 ortak durumunun geçmesi
bütün mevcut araç yüzeyinin tamamlandığı anlamına gelmez. Genel üst araç
denetimi ve performans nedeniyle ana plan [~] kalır.

## Testler

`test-runs.json` kabul edilen 18 koşuyu ve 11 tanısal kaydı ham log/SHA-256
ile eşler. Son application14: 51 kütüphane + 216 entegrasyon başarılı,
22 atlanan; ortak 105 bağımsız durum bunların içindedir. Python14: **300 test**,
SDK kontrolü ve son clippy dahil bütün script başarılıdır. Gerçek MCP host'u
6 test; yeni metraj gösterim/dosya regresyonu 1 test; uygulama/MCP/Python/
masaüstü clippy, desktop build, katalog/SDK/892 satırlık manifest ve WASM
kontrolleri başarılıdır. Önceki interaction9: 63+680 başarılı/1 atlanan;
KCAD10: 6+207; formats10: 113+50 başarılı/7 atlanan; bağımsız KCAD fixture
kontrolü 1170 dosya. Bu koşular kaynaklarının değişmediği sınırlar için
tutulur; desktop9 tek başına tamamen yeşil değildir, düzeltmenin kapsamı
yukarıda açıktır. Aynı testin tekrarları benzersiz test sayısına eklenmez;
atlanan ve sıfır testli koşular başarılı test diye sunulmaz.

## Son derlemede yeniden kabul

- 63–66: gerçek iki nokta girişi önizlemede belgeyi değiştirmez. Tek Enter
  27,36 m² bölümü ve yerelleştirilmiş tek komut geçmişini üretir; tek Undo
  34,22 m² ve ilk dört nesneyi tam geri getirir.
- 67/70/72: kaynak duvar yokken son 38,22 m² korunur, **son geçerli sınır**
  uyarısı hem açık hem koyu 18 px dar pencerede okunur. Başlıklar ayrı kalır;
  yatay kaydırma çubuğunun sağ rayına tıklama sağdaki sayıları tam gösterir.
  68 yatay tekerlek ve 69 thumb sürükleme denemelerinde konum değişmedi;
  bunlar başarılı kaydırma kabulü değildir ve genel girdi denetiminde açıktır.
- 71: gerçek Kaydet diyaloğundan alınan `warning-verified.csv` 13 sütunla
  aynı uyarıyı taşır. 73: duvar geri gelince uyarı kalkar; önceki kolon
  tercihi geri alınınca alan 34,22 m², rapor eski 12 sütundur. 74–75: 13 px
  geniş koyu rapor ve `valid-verified.csv` aynı 34,22/33,40/36/37,82 değerlerini
  verir. CSV'nin yuvarlanmamış FP64 sayıları 1e-9 mutlak toleransla doğrulanır;
  ilk salt eşitlik denetimi bu ikili sayı gösterimi farkında başarısızdı,
  üretim hesap/CSV biçimi bu nedenle değiştirilmedi.
- 76: gerçek Dosya Aç sonrası dört nesne/34,22 m²; `verified-reopened.kcad`
  önceki modelin tamamıyla eşittir. **35 gerçek GUI kaydı ve 76 ham JPEG**,
  bağımsız `verify.py` ile alan/UUID/rol/oda/kaynak/proje ayarları ve
  undo/redo korunumu denetiminden geçer; iki gerçek CSV de aynı denetimdedir.
- `quantity-break-before.json` / `quantity-break-after.json`: yalnız sanal
  4×3 m sınırın kaybında public Python artık `boundaryBreak=lostEnclosure`
  ve son sınır açıklamasını döndürür; son alan 12 m² ve tam Undo korunur.

Tekrar: `python3 docs/baseline/2026-10-25-architecture-space-separators/verify.py`.
Yeni ayarlar için sekiz görünüm tamamdır; bütün mimari araçların tam
boyut/tema/yazı matrisi ve release uçtan uca performans bu dilimle kapanmaz.
