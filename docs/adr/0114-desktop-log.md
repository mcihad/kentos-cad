# ADR 0114: Masaüstünde günlük: alt panelin satırları, Uyarılar rozeti ve durum çubuğunun iletisi

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** DESIGN.md §7.6, §7.7; ADR 0058 (alt panel), 0113 (bulut hücreleri).
- **Kaynak:** web'in `ui/bottom/logPlan.ts`, `warnings.ts`, `BottomPanel.ts`, `CommandLine.ts`, `ui/statusbar/StatusBar.ts` ve `app/state.ts`'in `MessageLog`'u; ortak durumlar `fixtures/shell/v1/log.json`.

## Bağlam

Masaüstünün günlüğü KentOS UI komut satırının kendi geçmişiydi:

- Komut satırının üstünde son üç satır soluklaşarak dururdu.
- Alt panelin Komut geçmişi, komut satırının açılmış geçmişiydi. Satırlarda saat yoktu.
- Uyarılar sekmesi okunmamışları başlığında “Uyarılar (2)” diye sayardı.
- Günlük sınırsız büyürdü.
- Durum çubuğunda son iletiyi gösteren hücre yoktu.

Web'de ise durum başkaydı:

- Komut satırı tek satırdır.
- Günlük alt paneldedir. Her satırda saat (`09:05:07`), düzeyin simgesi ve metin vardır.
- En çok 500 satır tutulur.
- Uyarılar sekmesi görülmeyenleri amber bir rozetle sayar.
- Son ileti durum çubuğunda birkaç saniye görünür.

DESIGN.md §7.6 ve §7.7 de web'i anlatır.

## Karar

Kurallar ve sözler `log_plan.rs`'tedir. Bu, web'in `logPlan.ts`'inin karşılığıdır. İki platform `fixtures/shell/v1/log.json`'u oynatır (`log_plan_tests.rs`). Günlüğün uygulamadaki yeri `message_log.rs`'tedir.

### Günlük

- Uygulamanın tek günlüğü `Log`'dur. Her satırın kimliği, düzeyi, metni ve zamanı vardır.
- Kimlikler programın çalışması boyunca büyür; Geçmişi temizle'den sonra da sürer.
- En çok 500 satır tutulur; fazlası en eskiden atılır.
- `say` bir satır yazar. Uyarılar sekmesi ekrandayken yazılan satır görülmüş sayılır.
- Çalışan komuta verilen değer ya da seçenek komut düzeyinde `› 12` diye yazılır.
- Başlayan araç adıyla yazılır (“Çizgi”), web'in `ToolManager.activate`'i gibi. Yapıştır “Yapıştır” diye yazılır.
- Komut satırına yazılan komut adı ayrıca yazılmaz; araç kendi adını söyler, komut da yaptığını.
- Bilinmeyen ad web'in sözüyle hatadır: ““ABC” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın.”

### Komut satırı

- KentOS UI komut satırı `lines(0)` ile yalnız giriş satırıdır (yeni seçenek). Kendi geçmişini çizmez.
- ↑ ile geri gelenler yalnız komut satırına yazılanlardır: komut adları ve çalışan komuta yazılan değerler. Web'in `CommandLine.remember`'ı gibi, art arda aynısı bir kez tutulur. Şeritten başlatılan araç ve seçeneğin harfi geri gelmez.
- Satırın Geçmiş düğmesi paneli kaldığı sekmede açar ya da kapatır (web'in düğmesi gibi).

### Alt panel

- Komut geçmişi ve Uyarılar web'in `.log` listesidir:
  - eş aralıklı yazı, küçük boy, 1,5 satır yüksekliği;
  - saat sütunu 64 px, simge sütunu 18 px, gerisi metin;
  - metin boşluklarını korur ve alt satıra geçer;
  - komut satırı metnin renginde ve kalın yazılır;
  - başarının simgesi yeşil; uyarının simgesi ve metni amber, hatanınki kırmızı;
  - satır üzerine gelince aydınlanır.
- Saat, satırın yazıldığı andaki yerel duvar saatidir.
- Liste açılınca sonundadır. Yeni satır listeyi sonuna yalnız liste sonuna 24 px'ten yakınken götürür; okunan yer yerinde kalır.
- Boş listeler web'in sözlerini söyler.
- Sekmelerin simgeleri web'inkilerdir.
- Uyarılar rozeti web'in `.badge`'idir: amber zeminde kalın küçük rakam, KentOS UI `Tab::badge`. Sayı, Uyarılar sekmesi son ekrandayken görülmemiş uyarı ve hatalardır.
- Panelin içi web'deki gibi panellerin zeminindedir, komut satırı alanın zeminindedir.

### Durum çubuğunun iletisi

Koordinatlardan sonra, kalan yeri kaplayan hücredir (web'in `status__flash`'ı).

- Günlüğün durum çubuğunun gösterdiği en yeni satırını gösterir: düzeyin simgesi ve metin. Komut satırı ve bir öncekini sürdüren girintili satır gösterilmez.
- Uyarı ve hata 9 s, gerisi 5 s kalır. 160 ms'de belirir ve kaybolur; solma sırasında pencerenin kareleri çizilir (`window::frames`).
- Ekrandaki iletinin yerine gelen ileti hemen görünür.
- Sığmayan metin “…” ile kısalır. Yer, tahminle değil KentOS UI'ın yeni `Elided` bileşeniyle bulunur: metin gerçek yazı tipiyle dizilip ölçülür, karakter sınırından kesilir.
- Pencere daralınca hücreler, iletiye yazı boyunun 15 katı yer kalana dek web'in sırasıyla yer açar.

Durum çubuğunun dizilişi web'inkidir: koordinat │ ileti │ seçim │ çizim yardımcıları │ ekran ölçeği │ çalışma modu │ koordinat sistemi │ kayıt │ sunucu │ çizim motoru.

- Masaüstüne özgü “1:1000” (pafta ölçeği) ve “13 nesne” hücreleri kalktı. Ölçek şeridin Özellikler panelinde, nesne sayısı Öznitelikler'dedir.
- Koordinat sistemi hücresi web'deki gibi yalnız adını yazar; EPSG ipucundadır.

## Sonuçlar

- Komut satırının üstündeki soluk satırlar kalktı. Son ileti durum çubuğunda, bütün günlük F2 ile alt paneldedir.
- `App.history`, `warnings_total` ve `seen_warnings` kalktı. Testler günlüğün satırlarını okur, izlerin oynatıcısı da adımın satırlarını kimliklerden bulur.
- Web'in komut satırındaki “Komut geçmişini aç” düğmesinin masaüstündeki karşılığı KentOS UI komut satırının Geçmiş düğmesidir. Sözü aynı değildir.
- Komut satırı web'deki 500 ağırlığı yerine yarı kalın yazılır; gömülü yazı tiplerinde 500 yoktur.

## Doğrulama

- **`log_plan_tests`:** `log.json`'un bütün bölümleri: sekmeler ve sözler, düzeyler, saatler, yankı, ileti, saklanan satırlar, rozet.
- **`message_log_tests`:**
  - iletinin seçimi, süreleri ve solması;
  - aynı anda yazılan satırlardan hangisinin gösterildiği;
  - ↑'nun geri getirdikleri;
  - komut satırının düğmesi.
- **`bottom::tests`:**
  - rozetin sayması ve görülmesi;
  - Geçmişi temizle.
- **`app::tests`:**
  - adıyla yazılan araç;
  - bilinmeyen ad;
  - değerin yankısı.
- **KentOS UI:** `widget::elided` (uzun metin sığacak kadar kesilir, kısa olan kalır) ve `widget::command_line` (satırsız komut satırı yalnız giriş satırıdır).
- **Görüntüler:** `message_log_tests::screens` (`.run/shots/gunluk-*`), web'in `layout` resimleriyle karşılaştırıldı: Komut geçmişi ve rozet, Uyarılar, kapalı panel ve uzun uyarı; koyu ve açık tema, 1440×900 ve 1100×650.
