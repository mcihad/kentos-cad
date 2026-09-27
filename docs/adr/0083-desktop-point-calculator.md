# ADR 0083: Masaüstünde Nokta hesapla ve araç oturumunda askıya alma

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.7; TODOS.md `UX-01`, `UX-07`; ADR 0018 (araç oturumu, "Askıda"), 0021 (masaüstü araç oturumu), 0059 (çizim alanının sağ tık menüleri), 0082 (bilgi kartı)
- **Kaynak:** web'in `tools/pointCalc.ts`'i, `ToolManager.nest/unnest`, `ui/shell/calcMenu.ts`, `CommandLine.ts`, `CommandBar.ts`. Web ajanının `226ea13`'ü (iz ve hesaplayıcının kendi Ctrl+Z'si) ve `d0a358c`'si (her adımda kendi sözüyle ret) bu dilimde birleşti.

## Bağlam

- **Web'de:** Netcad'in "Koordinat hesap makinası"nın karşılığı var. Bir komut nokta beklerken altı yapıdan biri komutu bitirmeden onun üstünde çalışır:
  - Yan nokta,
  - Kenar kesişimi,
  - Doğru kesişimi,
  - Hat üzerinde nokta,
  - Açı ve mesafe,
  - İki nokta ortası.
- **Hesaplayıcının akışı:** referanslarını çizimde, kenetle alır; değerleri yazdırır; hesapladığı noktayı komuta tıklanmış gibi verir.
- **Masaüstünde:** araç oturumunun "Askıda" durumu yoktu (ADR 0018'in tablosu), hesaplayıcı da yoktu. Çizim alanının komut menüsündeki "Nokta hesapla" alt menüsü bunu bekliyordu (ADR 0059).
- **Web ajanının işi:** ortak iz `fixtures/interaction/v1/point-calc.json` (83 adım), iz biçimine iki anahtar ve hesaplayıcının kendi sözüyle ret.

## Karar

### Araç oturumu (`kentos-interaction`)

- **`Session::nest`** çalışan komutu askıya alır; komut yoksa taşınan bir tutamacı askıya alır. Hesaplayıcı üstte çalışır; komut satırı "Nokta hesabı: <yapı>" der.
- **Hesaplayıcı bitince** noktası askıdaki komuta tıklanmış gibi gider (`Tool::accept_point`). Komut o adımda nokta almıyorsa web'in iletisi söylenir: "Çalışan araç şu adımda nokta beklemiyor; hesaplanan nokta kullanılmadı."
- **Esc** komutu olduğu yerden sürdürür.
- **Ctrl+Z** hesaplayıcının kendi adımını geri alır: önce iki çözüm arasındaki seçimi, yoksa son referansı. Askıdaki komutun altındaki çizimi hiç geri almaz.
- **Durumlar ve izler:**
  - `tool_id` askıdaki komutun kimliğidir (izlerin `tool`'u);
  - `point_count` ve istem hesaplayıcınındır;
  - başka bir komut başlarsa ikisi de biter.
- **`can_calc_point`** (web'in `canCalcPoint`'i) hesaplayıcının ne zaman çalışabileceğini söyler:
  - nokta alan ve kenetlenen bir komut çalışırken;
  - ya da taşınan bir tutamaç varken;
  - başka bir hesaplayıcının üstünde çalışmaz.
- **`Tool` arayüzüne üç yöntem eklendi:**
  - `accepts_points` ve `accept_point`: web'in `acceptPoint`'i;
  - `computed`: üstte çalışan aracın sonucu.
- **Hesaplanan noktayı alanlar**, web'de `acceptPoint`'i olan araçlarla aynıdır:
  - çizim araçlarının hepsi, Yazı ve Ölçülendirme dahil;
  - seçimden önce seçen değiştirme tabanı, ama seçim sürerken değil;
  - dik araçları (referans varken);
  - Esnet (taban ya da hedef beklerken);
  - Yapıştır;
  - çizimden nokta alma;
  - seçim aracının tutamacı.
- **Nokta almayanlar:** kenar seçen araçlar, Tarama, İçine tıklayarak alan ve Sil (web'deki gibi).
- **Hesaplayıcı (`point_calc`):**
  - Web'in metinleri birebir: yapılar, istemler, açıklamalar, takma adlar (YAN, KKES, DKES, HAT, AM, ORTA), "Hesaplanan nokta: Y …  X …" ve bütün uyarılar.
  - Yazılan her şey onundur. Okuyamadığını adımın beklediğini söyleyerek reddeder ("“abc” anlaşılamadı. Dik ayak ve dik boyu yazın: absis,ordinat (ör. 12.5,3).").
  - Değerleri web'in düzenli ifadeleriyle aynı kuralla okur: çift, oran ve sayı.
  - Hesap ortak çekirdekten gelir: `side_point`, `distance_intersection`, `line_intersection`, `along_line`, `along_ratio`, `calc_polar`, `midpoint`, `nearest_of`.
  - Önizleme web'in `draw`'ı gibi:
    - kenet renginde halkalar ve harfler (A, B, C, D; S, R; 1, 2; iki çözüm için “?”);
    - görünüm boyunca uzayan kesikli referans doğruları;
    - imleçteki canlı ölçüler, kenet renginde bir etikette (Dik ayak, Dik boy; A’dan; Açı, Mesafe; A’ya, B’ye).

### Masaüstü

- **Takma ad:** komut satırına yazılan takma ad, komut onu okuyamazsa hesaplayıcıyı başlatır. İmleç yanındaki değer alanında başlatmaz (web'deki gibi).
- **Komut menüsü:** "Nokta hesapla ▸", "Tek seferlik kenet"ten önce gelir. Menü bir başlıkla başlar ("Komut nokta beklerken ölçülerden nokta hesaplar"). Her yapının satırında simgesi, ne yaptığını anlatan ikinci satır ve sağda takma adı vardır.
- **"Nokta hesabı" çipi:** komut satırında, şerit üstü komut çubuğu kapalıyken çıkar ve menüyü üstünde açar. Komut çubuğu açıkken aynı düğme orada, seçeneklerin yanındadır.
- **KentOS UI'ya eklenenler:**
  - `Menu::detail`: iki satırlı satır (web'in `menu__label--2`'si). Satır, yazısının boyundadır.
  - Açık menünün fare bulması ve alt menünün yeri artık çizilen satırlardan okunur (`row_at`, `row_top`). Böylece satırların yüksekliği değişebilir.
  - `LinePrompt::menu`: yazıyla seçilmeyen, menü açan çip.
  - `CommandLine`, giriş satırının açılır katmanlarını iletir; önceden hiçbirini iletmiyordu.
- **İz oynatıcısı:**
  - `prompt`: istemin tam metni;
  - `logged`: adımın yazdıkları arasında, tam ve sırasıyla;
  - "/" tuşu: US'te Slash, Türkçe Q'da Shift+7;
  - ızgarası açık iz artık oynatılır. Nesne izleme henüz yok.

## Doğrulama

- **İz:** `point-calc` masaüstünde üç varyantta (US, Türkçe Q, 2× ekran) geçiyor: 83 adım, istemler ve iletiler tam metinle. Öbür bütün izler de geçiyor.
- **`kentos-interaction`:** değerlerin okunuşu, büyük harfe çevirme ve takma adlar sınanıyor.
- **Masaüstü testleri:**
  - Hesaplayıcı yalnız nokta bekleyen komutun üstünde çalışıyor; komut kalıyor, Esc onu geri getiriyor.
  - Komut satırındaki çip menüyü açıyor; ↓ ↓ Enter Kenar kesişimi'ni başlatıyor.
- **Görüntüler** (`point_calc::screens`: `nokta-hesapla-*`):
  - Kenar kesişimi'nin iki çözümü;
  - Yan nokta'nın canlı etiketi;
  - açık alt menüsüyle komut menüsü;
  - çipin menüsü ve komut çubuğunun düğmesi.
  - 1440×900 ve 1100×650'de, koyu ve açıkta incelendi.
