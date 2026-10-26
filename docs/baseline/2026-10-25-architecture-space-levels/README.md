# Mahalın katına bağlı sınır — kabul kaydı

25 Ekim 2026, `codex/architecture-2d-workflows`; [ADR 0670](../../adr/0670-level-bound-architectural-spaces.md).
ARCH-SPACE-01 sürüyor; bu kayıt toplu mahal bulmanın tamamlandığı anlamına gelmez.

## Somut hata ve kural

Aynı XY başlangıcında 200 mm merkez duvarlı üç dikdörtgen vardır: Zemin kat
6×4 m, 1. kat 10×8 m, katı atanmamış 12×10 m. İki tanımlı katın kotu da 0 m'dir;
birbirinden ayrılmaları kotla değil kalıcı kat kimliğiyle yapılmalıdır.

Eski FFI'da `arch.level=kat-1`, seed=(500002, 4400002) çağrısı üst kattaki
9,8×7,8=**76,44 m²** yerine alt katın 5,8×3,8=**22,04 m²** sınırını döndürdü.
`before-engine.json` eski program hash'ini ve bağımsız Decimal alan hesabını tutar.
Yeni hesap açık kata ait duvarları hem yakın hem tam aramada süzer. Kattaki
duvar kaldırılınca başka kat onu kapatmış sayılmaz; son sınır ve lostEnclosure
aynı undo adımında korunur. Mahalın kat düzenleme planı da yeni sınırı hesaplar.

Katı olmayan eski çağrılar ve mahaller bütün çizim davranışını korur. Yeni
aracın **Tüm katlar** seçeneği bunu açıkça söyler. Dosya açılışında sessiz kat
ataması yoktur. Renk, ölçek, birim, render ve KCAD şeması değişmez.

## Otomatik kontroller

- Native: üst üste katlar/eşit kot, m/mm/ft, uzak koordinat, eski katsız çağrı;
  salt okunur plan, tek undo/redo, snapshot açılışı; hedef kat dışında tohum ve
  bilinmeyen katın atomik reddi; kat düzenleme planı; duvar kalınlaşması/silinmesi,
  yalnız ilgili mahalın alanı ve metrajı, lostEnclosure ve undo.
- Etkileşim: görünür kat, Tüm katlar varsayılanı, salt okunur önizleme, tek
  tıklama ve undo; kaldırılan seçimin kaybolmaması ve yazmanın reddi.
- Python: mevcut tipli SDK ile aynı sınır/metraj, plan/execute, kat düzenleme,
  kalıcı UUID'ler, undo/redo ve KCAD kayıt/açılış; atomik ret.
- MCP: gerçek sunucu ve çizim dosyası üzerinden plan/execute/metraj, tek
  undo/redo ve başka katın tohumu kapatmasının reddi.
- Kesirli bağımsız `space_command_cases.py` açık katı aynı kuralla süzer;
  eski 84 durum korunur, 3 yeni kat durumu eklenmiştir. Beklenenler ürün
  çıktısından üretilmez. İlk hedef testte eksik fixture attrs alanı yakalandı;
  test kurulumu düzeltildi, başarısız ham koşu da saklanır.

Son kararlı kaynakta **3210 Rust testi geçti, 249 test atlandı**; ilgili altı
paketin bütün hedeflerinde clippy `-D warnings` geçti. Masaüstü ve Python FFI
derlendi; **282 Python testi geçti**. Katalog/SDK tutarlılığı, 888 komutlu
manifest, şerit ve 47 crate/61 hedef bağımlılık yönü kontrolleri geçti.
Bu dilimde Core/WASM veya web arayüz kodu değişmedi; WASM yeniden derlenmedi.
Yeni veya değişmiş `.ts`/`.tsx` dosyası yoktur.

Başarısız ilk geniş Rust koşusunda eski etkileşim kütüphanesi ile derleme
sırasında değişen test etiketi karıştı; kaynak sabitlenerek tam koşu tekrarlandı.
İlk Python koşusunda kayıt/açılışta sıkıştırılan geçici sayısal nesne numarası
karşılaştırılıyordu. Test yalnız o geçici numarayı eşitleyerek tüm kalıcı içeriği
ve aynı UUID ile erişimi karşılaştırır; iki hedef test ve tam 282 test yeniden
geçti. Ürün kodu bu Python düzeltmesi sırasında değişmedi. İlk ve son ham
koşular `logs/` altında korunur; son sonuç `test-results.json` içindedir.

## Gerçek pencere kabulü

Son derlenmiş test uygulaması, bu kabul için hazırlanmış çizim ve ayrı ayar
diziniyle çalıştırıldı. `gui-binary-before/after.json` program ve kaynak
hash'lerini tutar. `shots/` içindeki **25 ham JPEG**, gerçek CUA fare/klavye
denemeleridir; yeniden boyutlandırılmış kanıt kullanılmadı. `gui-events.json`
elle kaydedilen adım dizisidir, uygulama olay kaydedicisi değildir.

1. Eski programda üç duvar ve kat seçimi olmayan Mahal aracı kaydedildi (01–02).
2. Yeni programda görünür Kat, 1. kat seçimi, yazmayan **76,44 m²** önizleme,
   tek tıklamayla mahal, tek undo/redo ve her aşamada kayıt denendi (03–09).
3. Üst kat duvarı seçilip silindi: başka kattaki duvarlar kaybı kapatmadı;
   son sınır ve kırmızı kopukluk işareti kaldı. Undo duvarı ve ilişkiyi geri
   getirdi (10–13). İçeriden ilk seçim denemesi nesneyi seçmedi (11); etiket
   tıklaması seçti (12). Bu sınırlama başarılı seçim gibi raporlanmaz.
4. Önceki test penceresi başka bir çizim gösterince o pencereye işlem
   gönderilmedi; aynı doğrulanmış programla ayrı `KentOS Space Level QA`
   uygulaması ve ayrı ayar dizininde devam edildi.
5. Özelliklerden kat değişimi **76,44 → 22,04 m²**, aynı mahalın etiket ve sınır
   hareketi; tek undo/redo **76,44 → 22,04** ve kayıt doğrulandı (14–18).
6. Çizim kapatıldı, dosya aç penceresinde tam yol doğrulanarak yeniden açıldı
   ve **22,04 m²** korundu (19–20). Native Quit denemeleri süreç kapanışını
   kanıtlamadı; burada doğrulanan işlem çizim kapat/açtır.
7. Gerçek iç boyutu **1100×650**, ham görüntü **2200×1364** olan dar pencerede
   kat listesi, 1. katın **76,44 m²** önizlemesi ve Escape ile yazmadan çıkış
   denendi (21–25). Geniş görüntüler yaklaşık 1440×828–831 iç boyuttadır;
   1440×900 kabulü olarak sayılmaz. Tema koyu, metin 18 px'dir.

`verify.py`, ürün SDK/geometri hesabını kullanmadan `tools/kcad` ile dosyaları
okur; uzak koordinatları yerelleştirip Decimal alan hesabı yapar. Üç özgün
duvarın bütün içeriği ve UUID'leri, proje ayarları, aynı mahal UUID'si, alanlar,
oluştur/kat değiştir undo-redo, kayıp sınır/undo, kapat-aç ve dar pencere iptali
doğrulandı. `python3 docs/baseline/2026-10-25-architecture-space-levels/verify.py`
ile tekrar çalıştırılabilir.

## Açık kapsam

Toplu bulma/tekrar eleme, bölge kapsamı, açık sınırı sahnede vurgulama, gerçek
kolonların alan çıkarımı ve kalıcı sanal sınır burada uygulanmış sayılmaz.
Tam gerçek pencere/tema/font matrisi de ayrıca açıktır. Net/Eksen/Brüt mevcut
KentOS hesabıdır; bir uluslararası veya yerel alan standardına sertifika iddiası yoktur.
Elle çizilen mahalda katsız seçimin metni, seçili mahal özelliklerindeki
“Yok” metniyle birlikte daha geniş kapsam/kat UX çalışmasında ayrıca ele
alınmalıdır; bu dilim yeni kalıcı sınır-kaynak kapsamı getirmez.
