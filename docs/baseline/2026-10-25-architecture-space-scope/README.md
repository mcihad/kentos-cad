# Atanmamış mahal kaynakları ve görünür iç nokta — kabul kaydı

25 Ekim 2026; dal `codex/architecture-2d-workflows`, başlangıç `8a6cf0b3a`.
[ADR 0645](../../adr/0645-unassigned-space-sources.md) kapsamındaki bu dilim
uygulandı ve aşağıdaki sınırlar içinde doğrulandı. ARCH-SPACE-01 bütünü [~]
kalır: seçilmiş kalıcı kaynak listesi, sanal ayırıcı ve genel performans/tam
iş akışı matrisi bu kabulün parçası değildir.

## Kaynak ve bağımsız beklenti

Başlangıç dosyası SDK ile hazırlanmıştır; elle çizildiği iddia edilmez.
`before.kcad`: 10×8 m eksenli kapalı dış duvar, kalınlık 200 mm; Zemin kata
atanmış x=5 m bölmesi; atanmamış 2×2 m kolon (3,4); başka katın 2×2 m kolonu
(8,4). İki katın kotu aynıdır. Başlangıç E=500000/N=4400000 m; dosya origin'i
sıfır, birim mm, mevcut çizim ölçeği 1:50 korunur. Kapsam görünürlük veya
kilitten tahmin edilmez; kaynakların mimari kat kimliğinden belirlenir.

- Net/Tüm katlar sol oda: (4,8×7,8)−4 = **33,44 m²**.
- Net/Yalnız atanmamış, bölme hâlâ Zemin kattayken: (9,8×7,8)−4 = **72,44 m²**.
- Bölme de atanmamış yapılıp iç nokta sağa seçilince: 4,8×7,8 = **37,44 m²**.
  Başka kata atanmış kolon bu kapsamda düşmez.

`verify.py`, bağımsız `tools/kcad/kcad.py` okuyucusu ve uzak origin'i çıkaran
Decimal çokgen hesabıyla **29 gerçek GUI kaydını** sınar. Alanların yanında
kaynak UUID/öznitelikleri, proje ayarları, kat tanımları, aktif katman, delikler,
önizleme/iptal/boş tekrar, atomik ret, undo/redo ve kayıt/aç tam model eşitliği
kontrol edilir. Native geometri veya Python uzantısı kullanılmaz.

## Gerçek GUI akışı

Bütün işlemler gerçek KentOS test penceresinde CUA fare/klavyeyle yapıldı.
Kullanıcının çizimi yerine ayrı QA çalışma dosyası/profili kullanıldı. JPEG'ler
ham yakalama baytlarıdır; kırpma veya yeniden çizim yoktur. Zaman/build eşlemesi
`gui-events.json`, çalıştırılan binary ve kaynak hash'leri `gui-*-binary-*.json`
içindedir. İlk ve son binary ayrı tutulur. Son binary açılış girdisi
`first-reseed-redo.kcad` dosyasının çalışma kopyasıdır. Sonraki tek test eklemesi
runtime kodunu değiştirmez.

| Ham görüntü | Doğrulanan işlem |
| --- | --- |
| 01–03 | Önceki sürümde dört kaynak, Tüm katlar 33,44 m² ve atanmamış seçeneğinin yokluğu. Escape/kayıt tam modeli korur. |
| 04–06 | Önceki aramada iç nokta aracı yok. Eski Yalnız net kolon tercihiyle eksen 40 m², brüt yaklaşık 42,64 m²; mevcut davranış korunur. |
| 07–15 | Yeni seçenek, tek mahal 72,44 m²; Öznitelikler'den Tüm katlar → 33,44 m²; kapsam geri alması, oluşturma undo/redo. |
| 16–19 | Toplu önizleme yazmaz; tek Enter oluşturur; undo/redo; ikinci bulmada 0 yeni/1 mevcut, boş tekrar undo eklemez. |
| 20 | Bölme üstünde kalan toplu seed ile Tüm katlar'a geçiş atomik ret; kayıt öncesi modelle tam eşit. |
| 21–24 | Kaynak bölmenin katını Yok yapınca son 72,44 m² korunur, açık uyarı çıkar; Öznitelikler/Sınır/İç nokta seç görünür; duvar gövdesi tıklaması reddedilir. |
| 25–30 | Sol oda önizlemesi kayıt modelini değiştirmez; sağ menüde tek Onayla/bitir ile aynı UUID 33,44 m²'ye iyileşir. Escape, kaynak katı ve iç nokta undo/redo doğrulanır. |
| 31–49 | Son binary'de kayıt/aç, aşağıdaki tema/yazı/boyut tablosu; seçili Kat değeri eksiksiz Atanmamış, menüde tam Yalnız atanmamış. |
| 50–56 | Son binary'de Mahal bağlamsal sekmesinden İç nokta seç; sağ oda önizlemesi; tek sağ menü onayı 37,44 m²; tek undo 33,44 m², redo aynı sağ oda; Escape tam modeli korur. |

İlk 10/16/19 görüntülerde seçili uzun kapsam adı kesiliyordu. Bu bir kabul
kusuruydu: alandaki değer **Atanmamış** yapıldı, menü/komut satırındaki tam ad
korundu. 31 sonrası son binary bu düzeltmeyi doğrular; ilk kusurlu görüntüler
başarılı son görünüm sayılmaz. İlk native denemede bulunan 1,56 m² duvar gövdesi
sorunu da beklenti değiştirilerek geçirilmedi: yeni açık kapsamın strict oda
sorgusu eklendi. Eski genel `found` sözleşmesi korunur.

## Son binary görünüm matrisi

| Tema | Yazı | Pencere | Tek mahal | Toplu mahal |
| --- | --- | --- | --- | --- |
| Açık pafta | 18 px | Geniş | 32 | 49 |
| Açık pafta | 18 px | Dar | 34 | 35 |
| Koyu grafit | 18 px | Geniş | 39 | 40 |
| Koyu grafit | 18 px | Dar | 37 | 38 |
| Koyu grafit | 13 px | Geniş | 41 | 42 |
| Koyu grafit | 13 px | Dar | 43 | 44 |
| Açık pafta | 13 px | Dar | 45 | 46 |
| Açık pafta | 13 px | Geniş | 47 | 48 |

Dar ham yakalama 2200×1364, 2× DPI'da başlık hariç **1100×650** istemcidir.
Geniş son yakalamalar 2880×1714, başlık hariç **1440×825** civarındadır;
1440×900 kabulü değildir. Panelin boyutu gerçek sürüklemelerle değişir.
Bu tablo yeni kapsam seçimi ve iki önizlemenin okunurluğunu sınar; her araç,
uyarı ve gezinmenin sekiz varyantta tam kabulü sayılmaz. 18 px'te bazı eski
sayısal Öznitelikler değerleri/diğer alanlar hâlâ kısalabilir; genel
ARCH-BASE-01 kabulü açık kalır. Yeni kapsam metni kesilmez.

## Otomatik ve bağımsız kapılar

Komut, zaman, çıkış kodu ve tam log hash'i `test-records.json` içindedir.
Atlanan testler aşağıdaki başarılı sayılara katılmaz.

- `scripts/fixtures/space_scope_cases.py --check`: bağımsız **90** durum;
  5 birim × Net/Eksen/Brüt × legacy/project/unassigned × yakın/uzak origin.
  Aynı ortak fixture native uygulayıcıda `KENTOS_CASES_ONLY=cad.spaces` ile geçti.
- Native tam `all`: **202 başarılı / 22 atlanan**. Son test eklemesiyle odaklı
  `space_scope::`: **5 başarılı**; split/merge sonrası çizilmiş/project/ignore,
  38,22+34,22=72,44 m² ve iki undo ile özgün ilişki/kimlik geri gelir.
- Etkileşim tam çalıştırma: birim **63**, bütünleşik **674 başarılı / 1 atlanan**;
  örneklerde 5 atlanan. Son görünür metin düzeltmesinden sonra bütünleşik
  674 yeniden geçti. Gerçek Session.cancel da sınanır.
- KCAD: **6 birim + 207 bütünleşik başarılı**; 2 ölçüm testi atlandı.
  Bağımsız referans `--check` 1153 yönetilen dosyada geçti. Toplam referansta
  117 geçerli / 1039 bozuk dosya vardır; dış dosyalar yeniden üretilmez.
- MCP tam çalıştırması ve yeni açık kapsam plan/execute/quantities testi geçti;
  protokol, kabul ve diğer mevcut bütünleşik kümeler de kayıttadır.
- Geometri çekirdeği: 524 birim + 259 ana bütünleşik başarılı; ayrı küçük
  kümeler de kayıttadır. İlk birleşik çalıştırmanın son codec sayım koruması
  116→117 beklediği için düştü; düzeltilmiş codec tam çalıştırması geçti.
  İlk başarısız birleşik koşu bütün olarak başarılı gösterilmez.
- Masaüstü Öznitelikler: **38 başarılı / 4 atlanan**. Son masaüstü derlemesi,
  native/interaction/desktop clippy `-D warnings`, ortak sözleşme/geometri
  wasm32 denetimi geçti. Son test eklemesinin clippy sonucu ayrıca kaydedilir.
- Gerçek native uzantı yeniden derlendi; Python bütün küme **292 başarılı**.
  Üç yeni test kapsam, ortak komutlar, kalıcılık ve atomik reddi kapsar.
- Katalog/SDK/manifest tutarlı (**890 satır**), ribbon üretimi tutarlı;
  yeni TS/TSX kaynak dosyası yok. Üretilen TypeScript test çıktıları `.run/`
  altındadır ve Git'e girmez.

KCAD **104**, yalnız yeni kalıcı unassigned kapsamı varsa yazılır. Project
varsayılanı alanı yazmaz; eski şemalar/baytlar frozen testlerle korunur.
Korunmuş önceki 103 okuyucusu gerçek `accepted-reopened.kcad` dosyasını açıkça
reddetti (`old-reader-refusal.json`); sessiz kapsam kaybı yoktur. Web arayüzü
ADR 0228 gereği ertelenmiştir; Python/MCP ortak sözleşmeyi kullanır.

Tekrar: `python3 docs/baseline/2026-10-25-architecture-space-scope/verify.py`.
