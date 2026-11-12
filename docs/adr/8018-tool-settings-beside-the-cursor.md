# 8018 — Aracın seçenekleri imlecin yanında

Tarih: 2026-11-08. Durum: kabul (M18, ADV-11; plan S11, S13).

## Sorun

- **İmlecin yanındaki değer kutusu yalnız noktayı ve kilitleri alıyordu** (ADR 0018, 0166 §6). Bir rakam kutuyu açıyor,
  Tab uzunluğu kilitleyip açıya geçiyordu.
- **Duvarın kalınlığı ve hizası panelde ya da komut satırındaydı.** Öznitelikler'in "Etkin aracın ayarları" ve komut
  çubuğunun "Kalınlık: 0.250 m E" çipi. Değiştirmek için el çizimden ayrılıyordu.
- **Dikdörtgen duvarın boyutları için de öyle:** "4,20 × 3,60 bir yatak odası" (ADV-11'in kullanıcı sonucu) Boyutlar (B)
  seçeneğiyle, komut satırından yazılıyordu.
- **Boş bir kilit alanında Tab hiçbir şey yapmıyordu:** uzunluk yazılmadan açıya geçilemiyordu.

## Araştırma (8 Kasım 2026)

- **AutoCAD yardımı, *Dynamic Input*:**
  - "Some commands have multiple input fields. Press the Tab key to move from one to another."
  - "When the down arrow icon appears in a prompt, press the down arrow on your keyboard to see a list of options for that
    command."
- **ADV-11'in örnekleri (W3 B):** SketchUp Measurements, Archicad Tracker, Vectorworks Data bar. Seçenekler imlecin yanında
  yazılır, kutuya tıklanmaz.
- **KentOS'ta hazır yol:** her aracın ayarları (`Tool::settings`, panelin satırları) ve onları yazan `Tool::configure`
  vardır. Panel bunlarla yazıyor; değer kutusu da aynı yoldan yazarsa iki ayrı doğrulama olmaz.

## Karar

- **Araç, değer kutusunun sunacağı ayarlarını söyler** (`Tool::cursor_settings`, Tab sırasıyla):
  - Duvar (yol aracı, duvar biçiminde): Kalınlık (E), Hiza (H).
  - Dikdörtgen duvar: Boyutlar (B), Kalınlık (E), Boy referansı (R).
  - Stilin sabitlediği kalınlık gibi düzenlenemeyen satır sunulmaz (`Session::cursor_settings`).
- **Tab sırası:** Uzunluk → Açı → aracın ayarları → (bir kilit varsa) Uzunluk.
  - Boş bırakılan alan kilitlenmeden geçilir, AutoCAD'deki gibi. Önce boş alanda Tab hiçbir şey yapmıyordu.
  - Hiçbir şey yazılmamış nokta kutusunda Tab doğrudan ilk ayara gider.
- **↓ çizimin üstünde kutuyu ilk ayarda açar** (AutoCAD'in "aşağı ok: seçenekler"i): Dikdörtgen duvarda ↓, "4.2,3.6",
  Enter. Tab çizimin üstünde kenet adaylarının kalır (ADR 0271). Izgara açıkken neredeyse her yerde aday olduğu için Tab
  kutuyu açamazdı.
- **Yazma panelin yoludur** (`Session::configure`). Enter yazar ve kutuyu kapatır, çizim sürer (ilk nokta kalır). Tab yazar
  ve sonrakine geçer. Yanlış değer söylenir ve kutu açık kalır.
- **Seçenekli ayar** (Hiza: Sol, Orta, Sağ; Boy referansı: İç net, Dış ölçü, Eksen) kelimenin başıyla seçilir: so → Sol,
  o → Orta, sa → Sağ. Türkçe büyük-küçük harf kuralıyla karşılaştırılır (İ → i, I → ı). İki değere uyan başlangıç ("s")
  "bir harf daha yazın" der. ↑ ve ↓ öncekini ya da sonrakini hemen yazar.
- **Kutu ayarın adını gösterir** (Kalınlık, Hiza, Boyutlar). İpucu şimdiki değeri ("Şimdi 0.250 m"), seçenekleri ve tuşları
  söyler. Nokta kutusunun ipucu ayarları anar: "… · Tab: Kalınlık, Hiza".

## Ölçüm

- **Gerçek pencere** (`fixtures/interaction/real/imlec-secenekleri.txt`, 1440×900, sahibin ayarlarının kopyası, KentOS Mimari'nin
  Dış duvar 25'i):
  - Duvar: ilk nokta, "3", Tab (Açı), Tab (Kalınlık, "Şimdi 0.250 m"), Tab (Hiza), "sa" Enter. Öznitelikler'in Hiza'sı Orta → Sağ.
    Komut satırına odak 0 (önce Hiza için çip ya da panel: 1 odak değişimi ve fare).
  - Dikdörtgen duvar: ↓ (Boyutlar), "4.2,3.6", Tab, Enter, köşe ve yön için iki tık. MM-DUVA-TUGLA'ya 4,20 × 3,60 m'lik oda
    yazıldı, etiketi "Boy referansı: İç net 4.200 × 3.600 m".
  - Resimler: `.run/gui-evidence/mimari-m18/once/`, `sonra/`, `m18-once.png`, `m18-sonra.png`.
- **Testler** (`input.rs` `cursor_settings_tests`):
  - Tab'ın kilitlerden Kalınlık ve Hiza'ya ve yeniden uzunluğa dolaşması; Kalınlık panelin yoluyla yazılır.
  - "sa" Enter → Sağ, kutu kapanır, duvar sürer.
  - "s" iki değere uyar ve söylenir; "so" Sol; ↓ sonrakini hemen yazar.
  - ↓ çizimin üstünde kutuyu Kalınlık'ta açar.
  - Masaüstünün izleri (traces) değişmeden geçer: boş alandaki Tab'ın yeni davranışı hiçbir izi bozmadı.

## Sonraki adımlar

- Öbür araçlar: kapı ve pencerenin genişliği ve yerleşimi, merdivenin basamakları, Ölçülü bölmenin uzaklığı.
- 2+1 kabul izini imleç yanı seçeneklerle yeniden yazmak (ADV-11'in ilk kabulü: etkileşim 157 → ≤ 80, odak 15 → ≤ 3); Oda aracı
  (M21) ile birlikte.
- Web: aynı Tab sırası ve ↓ (TODOS.md).
