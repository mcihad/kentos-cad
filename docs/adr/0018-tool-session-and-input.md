# ADR 0018: Araç oturumu, değer girişi ve etkileşim izleri

- **Durum:** kabul edildi (2026-09-25). Karar, web'in bugünkü davranışını iki platformun ortak hedefi yapar. Sondaki açık sorular sahibin kararını bekler.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** TODOS.md §5 (`UX-01`, `UX-03`, `UX-04`, `UX-06`), §22.1 madde 3; CLAUDE.md §4.6–4.7; ADR 0010, 0017

## Bağlam

Web'in araç akışı TS'tedir:
- `ToolManager` ve `Tool` arayüzü (`tools/`);
- nokta alan araçların ailesi `PointInputTool`;
- imleç yanındaki değer alanı `ui/shell/CursorInput.ts`;
- komut satırı `ui/bottom/CommandLine.ts`;
- tuş eşlemi `core/keymap.ts`.

Masaüstü aynı alışkanlıkları karşılayacak. DOM ile Iced'in bileşen ağacı aynı olamaz; ortak olan davranıştır. Bu davranışın yazılı ve sınanabilir bir referansı yoktu. TODOS §5'in kabul izi yalnız bir cümleydi.

İz yazılırken web'de iki eksik bulundu (25 Eylül; koddan ve tarayıcıda izle doğrulandı):
- **`+` ve `-`:** her zaman yakınlaştırmaya bağlıydı.
  - Komut çalışırken `-4` yazınca eksi görünümü uzaklaştırıyor, değer alanı `4` ile açılıyordu.
  - Nokta imlecin yönüne 4 m konuyordu. Kullanıcı ters yön istemişti ve bir uyarı almıyordu.
- **Tab:** değer alanında odağı alandan çıkarıyordu. Yazılan değer uyarısız kayboluyordu.

## Karar

### Araç oturumu (`UX-01`)

İki platformda aynı durumlar ve geçişler:

| Durum | Anlamı |
|---|---|
| Boşta | Seçim aracı, komut yok. Enter son komutu yeniden başlatır |
| Çalışıyor | Araç bir adım bekler: nokta, sayı ya da seçenek. İstem adımı ve seçenekleri söyler: `Araç: adım [Seçenek (TUŞ) / …]`. Bu metin sözleşmesi `UX-02`'de tipli `PromptSpec`'e dönecek |
| Önizleme | Çalışırken imleç hareketi taslağı çizer; belge değişmez |
| Onay | Enter, boş değer alanında Enter ya da kısa sağ tık. Poligon en az üç noktayla tek belge işlemi ve tek geri alma adımı olarak yazılır. Araç açık kalır, yeni nesneye hazırdır |
| İptal | Esc. Bitmemiş taslak bırakılır; belgeye ve geri alma geçmişine bir şey yazılmaz. Araç kapanır |
| Askıda | Şeffaf bir araç (nokta hesaplayıcı) çalışırken ana araç durumunu korur. Hesaplayıcının noktası ana araca tıklanmış gibi gider |

**Gözlenebilir durum.** İzler bunları okur; masaüstü aynı adlarla verir:
- etkin araç ve aracın aldığı nokta sayısı (`Tool.pointCount`);
- istemdeki seçenek tuşları;
- değer alanının metni ve komut satırının metni;
- nesne sayısı ve en yeni nesne;
- geri al, yinele ve kaydedilmemiş değişiklik bayrakları;
- son iletinin düzeyi;
- görünüm ölçeği.

### Klavyenin yolu (`UX-03`'ün ilk kısmı, `UX-04`)

Bir tuş aşağıdaki sırayla ilk sahibine gider:

1. **Açık bir metin alanı ya da pencere:** değer alanı, komut satırı ya da iletişim kutusu. Orada yalnız izinli kısayollar çalışır (örneğin Ctrl+S).
2. **IME birleştirmesi:** birleştirme süren tuş kısayollara bakmaz.
3. **Seçenek harfi:** çalışan aracın istemdeki harfi (`(G)`) araç kısayolundan önce gelir. Poligonda G “Geri”dir, aracı yeniden başlatmaz.
4. **Kısayollar.**
5. **Değer başlatan karakter:** rakam, `.` ve `@`; komut çalışırken `-` ve `+` da.
   - İmleç çizim alanındaysa ve tercih açıksa değer alanı imlecin yanında açılır; değilse komut satırı odaklanır.
   - **İlk karakter alana tam bir kez girer.** Web'de tuş işleyicisi alanı odaklar, tarayıcı karakteri yeni odağa yazar.
   - Masaüstünde karakter, tuş olayının ürettiği metinden alınır, fiziksel tuş kodundan değil. Türkçe Q ve F klavye böylece korunur (CLAUDE.md §4.6).

Komut yokken `+` ve `-` görünümü 1,5 kat yakınlaştırır ve uzaklaştırır.

### Tuş ve fare anlamları (`UX-06`)

| Girdi | Anlamı |
|---|---|
| Enter | Değer alanında değeri uygular. Alan boşsa ya da kapalıysa aracı onaylar (poligonu bitirir). Komut yokken son komutu yeniden başlatır |
| Esc | Değer alanı açıksa yalnız alanı kapatır. Komut satırında yazı varsa onu siler. Yoksa komuttan çıkar; taslak bırakılır |
| Backspace | Değer alanında son karakteri siler |
| Tab | Değer alanında değeri korur, odağı taşımaz |
| Kısa sağ tık | Enter. Kısa, 300 ms'den az basılı tutmaktır |
| Basılı sağ tık | Komut menüsü; seçim aracında seçim menüsü |
| Shift + sağ tık | Tek seferlik kenet menüsü |
| Çift tık | İkinci bir nokta eklemez; aynı yere ikinci tıklama yok sayılır |
| Orta tuşla sürükleme | Kaydırma |
| Orta tuşla çift tık | Bütün çizimi gösterir |
| Seçenek G (Geri) | Son noktayı geri alır; komut sürer |
| Üçten az noktayla onay | Uyarı verir, belgeye yazmaz, aracı ilk noktaya döndürür |

### Etkileşim izi

- **Biçim:** `kentos.interaction-trace` sürüm 1, [`fixtures/interaction/README.md`](../../fixtures/interaction/README.md).
  - Adımlar platformdan bağımsızdır: komut, tuş, yazılan metin, dünya koordinatında fare.
  - Beklentiler gözlenebilir durumdur.
  - Tıklanan noktalar bir piksel toleransla, yazılan değerler tam eşitlikle karşılaştırılır.
- **İlk üç iz:**
  - `polygon-accept`: TODOS §5 kabul izi. Tıkla, `12` yaz (alan açılır, `12` görünür), Enter, `@0,8`, kapat, tek adımda geri al, yinele, kaydet ve aç.
  - `polygon-signs`: `UX-04`.
  - `polygon-keys`: `UX-06`.
- **Web:** izleri gerçek tarayıcıda oynatır: `pnpm e2e:interaction` (`make e2e-interaction`). Oynatıcı istemi uygulamanın kendi ayrıştırıcısıyla okur, belgeyi ve kamerayı geliştirme yüzeyinden (`window.kentos`) okur.
- **Masaüstü:** çizim alanı (`REN-01..07`) ve poligon komutu (`CMD-04..07`) gelince aynı dosyaları değiştirmeden oynatır. **Masaüstünün ilk davranış hedefi budur.**
- **Kural:** iz web'in bugünkü davranışını yazar. Davranış değişecekse sıra şudur:
  1. karar;
  2. iki uygulamada değişiklik;
  3. izin güncellenmesi.

  Beklenen değer hataya göre yenilenmez.

## Sonuçlar

- **Web'de değişenler:**
  - Komut çalışırken `+` ve `-` değer başlatıyor (`app/keybindings.ts`, `ui/bottom/CommandLine.ts`).
  - Tab değer alanında değeri koruyor (`ui/shell/CursorInput.ts`).
  - `Tool.pointCount` eklendi.
  - Seçim aracındaki davranış değişmedi.
- **Ters deneme:** iki düzeltme geri alınınca `polygon-signs` ve `polygon-keys` tam o adımlarda düşüyor.
  - `-4` yazınca nokta `[4, 0]`'a gidiyor.
  - Tab'dan sonra alan kapanıyor ve değer kayboluyor.

  İzler davranışı gerçekten tutuyor.
- **Henüz izde olmayan varyantlar:** yüksek DPI, Türkçe klavye ve IME. Odak başka alandayken yazma varyantının ilk örneği `polygon-keys`'te.
- **`UX-03`:** öncelik sırası yazıldı. Değer alanı ile iletişim kutusu arasındaki ayrıntılar ve IME testi açık.

## Açık sorular (sahibin kararı)

Dördü de 25 Eylül'de tarayıcıda denenerek doğrulandı:

1. **İlk köşeye tıklamak.**
   - Bugün: ilk noktanın aynısı yeni bir köşe olarak ekleniyor. Enter'dan sonra poligon, ilk ve son köşesi aynı olan dört köşeyle yazılıyor; sıfır uzunlukta bir kenar kalıyor.
   - Öneri: ilk köşeye, kenet toleransı içinde tıklamak alanı kapatıp bitirsin; yinelenen köşe hiç yazılmasın.
2. **Boşluk tuşu.**
   - Bugün: web'de komut satırına götürüyor; ardından yazılan değer komut satırına gidiyor. AutoCAD'de Boşluk, Enter'dır.
   - Hangisi kalsın?
3. **Komut çalışırken Ctrl+Z.**
   - Bugün: bitmemiş poligonun noktaları yerinde kalıyor, bir önceki bitmiş nesne siliniyor.
   - Öneri: taslakta nokta varken Ctrl+Z son noktayı geri alsın (G gibi); taslak boşken belgeyi geri alsın.
4. **Tab ile uzunluk ve açı.**
   - Bugün tek alan var.
   - AutoCAD'deki gibi kilitlenebilir iki alan (uzunluk, açı) ve aralarında Tab ile geçiş istenir mi? Bu, `UX-08` ile birlikte ele alınır.
