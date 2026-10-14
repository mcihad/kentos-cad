# ADR 0282: AutoCAD takma adları profili

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-18`: “İngilizce AutoCAD alışkanlıkları için
  desteklenen alias profili sun; KentOS'un Türkçe adları ve klavye sözleşmesi korunmalı.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0017 ve 0018 (komut satırı, tek tuş kısayolları, yazılan ad), 0023 (tipli ayarlar), 0147 §7 (yöntemin takma
  adı), 0228 §4 (yeni arayüz kayıtları `catalog/v1`'de), CLAUDE.md §4.5–§4.6.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

KentOS'un komut satırı Türkçe adları (`CIZGI`, `DIKDORTGEN`), Netcad adlarını ve AutoCAD'in en bilinen kısaltmalarının çoğunu (L, PL, C,
M, CO, RO, TR, EX, O, F, H, X, DLI …) zaten tanıyor. AutoCAD'den gelen kullanıcının elinde kalanlar ise ya hiç çalışmıyor (`RECTANG`,
`LA`, `DT`, `U`, `UNDO`, `QSAVE`, `JOG` …) ya da KentOS'ta başka bir iş yapıyor: `T` KentOS'ta Yazı (AutoCAD'de çok satırlı yazı), `S`
Eğri'nin tuşu (AutoCAD'de Esnet), `P` Çoklu çizgi'nin tuşu (AutoCAD'de Kaydır), `CP` Çokgenle seç (AutoCAD'de Kopyala), `POLYGON` Kapalı
alan (AutoCAD'de düzgün çokgen). Bunları KentOS'un adlarına eklemek Türkçe kullanıcının bildiği anlamları bozar; hiç eklememek AutoCAD
alışkanlığını cezalandırır.

## Karar

1. **Profil veridir:** `catalog/v1/aliases-autocad.json` (`kentos.aliasProfile/1`). Her satır yazılan ad (`name`), AutoCAD'de karşılığı
   olan komut (`autocad`: `LA` → `LAYER`) ve başlattığı KentOS komutu (`command`); bir aracın yöntemiyse şeridin menüsündeki seçeneği
   (`option`: `JOG` Ölçülendirme'yi Kırıklı yarıçap olarak, `I`, açar). Kaynak AutoCAD'in `acad.pgp`'sindeki varsayılan kısaltmalar
   ve komut adlarıdır; yalnız KentOS'ta karşılığı olan ve masaüstünde çalışan komutlar (57 satır). KentOS'un aynı işe zaten verdiği
   adlar dosyada yoktur.
2. **Tercih, kapalı başlar:** `drafting.autocadAliases` (kullanıcı kapsamı, canlı uygulanır; şimdilik yalnız masaüstü). Uygulama
   ayarları › Görünüm'ün Komut satırı bölümünde “AutoCAD takma adları”.
3. **KentOS'un adları ve tuşları önce gelir:** profil yüklenirken KentOS'un bir anlam verdiği ad dışarıda kalır: bir komutun takma adı,
   başlığı ya da kimliği, bir yöntemin takma adı ya da tek tuş kısayolu (`kentos_meaning`). Bugün dışarıda kalanlar sekiz ad: `B` (Kır'ın
   tuşu), `CP` (Çokgenle seç), `D` (Ölçülendirme'nin tuşu), `I` (Dik in'in tuşu), `P` (Çoklu çizgi'nin tuşu), `POLYGON` (Kapalı alan),
   `S` (Eğri'nin tuşu), `T` (Yazı). Dosyada kalırlar ki hangi AutoCAD alışkanlığının KentOS'ta başka anlama geldiği görülsün; KentOS'a
   yeni bir ad eklenirse onunla çakışan profil adı kendiliğinden düşer ve test listenin değiştiğini söyler. Kullanıcının modellerinin
   adları da önce gelir.
4. **Yalnız yazılan ad:** profil komut satırına yazılıp Enter ya da Boşluk'la gönderilen adı etkiler (`run_typed`'in başında
   `run_profile_name`; profil açık değilse ya da ad profilde yoksa satır eskisi gibi sürer). Çizim alanındaki tek tuş kısayolları,
   çalışan aracın seçenek harfleri, değer girişi ve Komut ara değişmez. Profil adı, KentOS'un adı gibi Türkçe katlamayla eşlenir
   (`rectang` de olur); ↑ yazılanı geri getirir; araç başlarken kendi adını söyler.
5. **Komut satırının listesi:** profil açıkken her komutun satırında profilin adları da görünür ve eşleşir (`LA` yazınca Katman ve
   öznitelik paneli ilk sırada, LAYISO'lardan önce). Kendi takma adı olmayan komut kimliğiyle değil, dosyadaki ilk AutoCAD adıyla
   listelenir. Yöntem adları kendi satırındadır (`JOG` — Ölçülendirme › Kırıklı yarıçap), çünkü aracın satırı seçilince aracın adı
   çalışır ve yöntem kaybolur.
6. **Kısayollar penceresi (F1):** “AutoCAD adları (komut satırı)” grubu; profil açıkken her KentOS komutu ve AutoCAD adları, kapalıyken
   nerede açılacağı.

## Sonuçlar

- AutoCAD'den gelen kullanıcı `RECTANG`, `LA`, `DT`, `U`, `UNDO`, `REDO`, `QSAVE`, `JOG`, `MEA`, `LI`, `W` gibi adları yazabilir; Türkçe
  kullanıcı için hiçbir şey değişmez (profil kapalı başlar, açıkken de KentOS'un adları aynı anlamdadır).
- Yeni ad eklemek dosyaya bir satırdır; çalışmayan komut, olmayan yöntem, iki kez yazılmış ad ya da KentOS'un aynı işe verdiği ad testte
  düşer.
- Profil arayüzün yoludur, ürün komutu değildir: Python ve MCP komutları kendi kimlikleriyle çağırır; profil onları etkilemez.
- Başka bir profil (ör. MicroStation ya da Netcad'in başka adları) aynı biçimle `catalog/v1/aliases-*.json` olarak eklenebilir; bugün
  yalnız AutoCAD'inki var.
- Komut ara (Alt+Q) profil adlarını aramaz; komut satırı AutoCAD kullanıcısının yazdığı yerdir.

## Doğrulama

- `cargo test -p kentos-desktop alias_profile`: her adın masaüstünde çalışan bir komutu ve varsa yöntemi, adların tekliği ve AutoCAD
  yazımı, dışarıda kalanların tam listesi ve KentOS'taki anlamları, dosyada KentOS'un aynı işe verdiği adın olmaması; kapalıyken `JOG`
  bilinmez; açıkken `rectang` Dikdörtgen'i, `JOG` Ölçülendirme'yi Kırıklı yarıçap olarak başlatır, `T` Yazı'dır, `DİKDÖRTGEN` çalışır;
  komut satırının listesinde `JOG` ve `LA` satırları yalnız profil açıkken; listeden `RECTANG` Dikdörtgen'i başlatır.
- `KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings` (şema), `cargo test -p kentos-desktop settings_view` (her ayar tek
  bölümde).
- Resimler: `KENTOS_SHOTS_ONLY=autocad-ayar,autocad-satir-jog,autocad-satir-la,autocad-kisayollar cargo test -p kentos-desktop
  tools_screens -- --ignored --nocapture` (`.run/shots/arac-autocad-*`).
