# ADR 0284: Komut geçmişinde çağrılar, Python kodu ve tekrar oynatma girdisi

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-20`: “Komut geçmişi; kullanıcıya okunabilir
  işlem, seçenekler, hata ve uyarıyı taşısın; istenirse karşılık gelen Python kodu ve replay girdisi alınabilsin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0114 (masaüstünün günlüğü ve alt paneli), 0131 (Python SDK'sı), 0132 (Python konsolu), 0253 (komut protokolü:
  `kentos.calls/1` kaydı, `kentos.replay/1`, tekrar oynatma).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Alt panelin Komut geçmişi aracın söylediklerini (adı, tıklanan noktalar, yazılan değerler, sonucu) satır satır gösteriyordu; hangi ürün
komutunun hangi girdiyle çalıştığını, hangi uyarıyı ya da hatayı hangi kodla verdiğini göstermiyordu. Çizimin kaydı (`kentos.calls/1`,
ADR 0253 §3) her komutun girdisini, sonucunu ve uyarılarını zaten tutuyor; ama yalnız Python'dan, MCP'den ve izlerden okunuyordu. Reddedilen
çağrı (hiçbir şey yazmayan) kayda hiç girmiyordu. Aynı işi Python'da yapmak isteyen kullanıcı da girdiyi kendisi yazmak zorundaydı.

## Karar

1. **Komut geçmişi kaydı okur:** her mesajdan sonra açık çizimin yeni kayıt satırları ve reddedilen çağrıları, günlüğün o anki son
   satırından sonra Komut geçmişi'ne girer (`history/`, `follow_history`). Satırlar araçların yaptığından yeniden türetilmez; satırın
   durumu (geri alındı, yinelendi) çizildiği anda çizimden okunur. Geçmişi temizle onları da siler; 500'den eskiler düşer.
2. **Okunur çağrı:** komutun Türkçe başlığı (kataloğun; tablo testle kataloğa bağlı), adı, sürümü ve geri alma adımı; girdisi düz
   Türkçe (“Katman: «Yapı» · Merkez: 487020, 4420015 · Yarıçap: 7.5”: alan adları Türkçe, katman adıyla, nesneler sayıyla, noktalar
   Y, X sırasıyla, iç içe ve etiketli girdiler bir düzeyde, `expectedRevision` yazılmaz); ne yazdığı (eklenen, değişen, silinen nesneler,
   projenin parçaları, yazılan dosyalar; geri alındıysa “geri alındı”); uyarıları kodlarıyla; reddedilen çağrıda nasıl bittiği (Reddedildi,
   Çakışma, Girdi eksik, Vazgeçildi), hatanın kodu ve iletisi. Aynı geri alma adımının çağrıları birbirinin altında, saatsiz “↳” ile.
   Komutsuz düzenleme satırı da görünür, Python karşılığı olmadığını söyler.
3. **Reddedilen çağrılar belgede:** `call::recorded` yazmadan dönen her çağrıyı (`Failed`, `Conflict`, `NeedsInput`, `Cancelled`)
   girdisi ve hatasıyla belgeye not eder (`Document::note_refusal`, `refused_calls`; oturumun, dosyaya ve kayda yazılmaz, son 200).
   Okuma kopyası (plan, iş) kendi notlarını tutar, çizime taşımaz.
4. **Python kodu:** `kentos_native_application::python_call` komutu ve girdisini SDK'nın çağrısına çevirir:
   `command = cad.circle.create` ve `command.execute_input(doc, command.input_type.from_json({...}))`. Girdi kataloğun JSON'u olarak
   verilir, komutun kendi girdi türü okur: iç içe ve etiketli girdiler tek yoldan, tam olarak. Yol SDK'nın üretecinin kuralıyla (`cad.`
   düşer, Python sözcüğü olan parça `_` alır: `cad.file.import_.dxf`). `expectedRevision` yazılmaz: kod çalıştırıldığı çizime yazar.
   `doc` Python konsolundaki açık çizimdir. Çağrının “Python” düğmesi kodu altında gösterir; kopyala düğmesi panoya yazar.
5. **Tekrar oynatma girdisi:** çağrının satırı `kentos.replay/1` olarak (projenin bağlamıyla tek satır; reddedilen çağrı için girdisiyle
   kurulan satır) panoya kopyalanır ya da `.json` dosyasına kaydedilir; Python'da `doc.replay(...)`, MCP'de ve başsız sunucuda aynı
   tekrar oynatmayla çalışır.

## Sonuçlar

- Araçlar, pencereler, Python konsolu ve ajan bağlantısı aynı kayda yazdığı için hepsinin çağrıları aynı yerde görünür.
- Yeni ürün komutu yok; yeni tuş ya da fare hareketi yok (düğmeler ipuçlarıyla).
- Python kodu ve tekrar oynatma girdisi web'e de aynı çekirdekten verilebilir (TODOS.md §0.2).
- Komutsuz düzenlemeler (manifestteki `geometry` ve `none` borçları) geçmişte “Komutsuz düzenleme” olarak görünür; borcun kendisi görünür
  olur.

## Doğrulama

- `cargo test -p kentos-native-application --lib python_call` (yol, Python değişmezleri, satır kırma, `expectedRevision`);
  `cargo test -p kentos-native-application --test all python_calls`: ortak komut durumlarının (fixtures/commands/v1) tamamlanan 1 132
  çağrısının kodu `python3`'ün `ast`'iyle ayrıştırılır, iç içe ve etiketli girdi `ast.literal_eval` ile geri okunur.
- `KENTOS_PYTHON_CALLS_OUT=… cargo test -p kentos-native-application --test all python_calls` ve
  `.run/py/bin/python scripts/python/history_calls.py …`: KentOS'un Rust'ı olmadan SDK'yla her kodun sarmalayıcısı `kentos.cad.COMMANDS`'tan
  doğrulanır, değişmez girdiye eşittir, girdi türü okuyup aynısını yazar (1 132 çağrı, 0 sorun).
- `cargo test -p kentos-desktop history::`: Daire aracının komutu geçmişte girdisi, sonucu, Python'u ve tekrar oynatma girdisiyle; geri alınınca
  “geri alındı”; tekrar oynatma girdisi aynı çizimde `same`; reddedilen çağrı kodu ve Python'uyla, tekrar oynatılınca yine `failed`;
  kaydetme ve Geçmişi temizle.
- Resimler: `KENTOS_SHOTS_ONLY=gecmis-python,gecmis-geri-alindi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-gecmis-*`).
