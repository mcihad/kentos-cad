# ADR 0017: Masaüstü kabuğu: vitrinin kalıpları, web'in komut envanteri

- **Durum:** kabul edildi (2026-09-25). Sahibin talimatları:
  - “showcase sayfalarına bakarak ilk arayüzü kur”;
  - “komut sistemini ayarlarken web tarafındaki tüm özellikleri adım adım masaüstüne de taşıyacağız”.

  Dosya pencereleri için `rfd` onaylandı.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** TODOS.md `UI-11`, §5, §22.1; ADR 0010 (platform sınırı), 0011 (`.kcad`), 0013 (ürün komutları), 0016 (UI taşıması)

## Karar

- **Uygulama:**
  - `apps/desktop`: paket `kentos-desktop`, ikili `kentos-cad`.
  - Native Iced ve KentOS UI bileşenleri. Yerleşim vitrinden gelir: şerit, yüzdürülebilen yuvada Katmanlar ve Özellikler, komut satırı, durum çubuğu.
  - Ortadaki çizim alanı, saf wgpu hattı (`REN-01..07`) gelene kadar belgenin özetini gösteren bir yer tutucudur.
- **Alan modeli vitrinin örnek modeli değildir:**
  - Belge, `kentos-contracts` üzerinden okunan ve yazılan `.kcad` v1'dir (`DocumentSnapshotV1`). Masaüstü web'in yazdığını okur, web'in okuyacağı biçimde yazar: tek satır JSON ve satır sonu.
  - Kayıt önce geçici dosyaya yazılıp yerine taşınır (`FILE-16`).
  - `dirty` bayrağı revizyonla korunur: kayıt sürerken yapılan değişiklik “kaydedildi” sayılmaz (CLAUDE.md §4.8).
  - Kaydedilmemiş değişiklik açma ve pencereyi kapatma öncesinde sorulur.
  - Vitrinin LonLat/Mercator modeli ve örnek verisi kullanılmaz (ADR 0016).
- **Komutlar web envanterinden gelir.**
  - `docs/inventory/web.json` derlemede gömülür. Masaüstü, web'in her komutunu aynı kimlik, ad, kısa ad, takma adlar ve kısayollarla gösterir.
  - Şeridin sekmeleri, panelleri ve hızlı erişim düğmeleri web'deki sırayla aynıdır.
  - `catalog::PORTED` masaüstünün çalıştırdığı komutların listesidir:
    - Öbürleri kütüphanenin devre dışı düğmesiyle soluk çizilir. İpucu “Web'de var; masaüstüne henüz taşınmadı” der; komut satırından ya da kısayoldan çağrılınca aynı iletiyi yazar. Sessiz düğme yoktur (CLAUDE.md §4.5).
    - Web'de de bekleyen komutlar kendi notlarını gösterir (“Yakında”, “Geliştirme aşamasında”).
    - Hiçbir üyesi taşınmamış bölmeli ya da menülü düğmeler de soluk ve menüsüzdür.
- **Taşımanın izlenmesi:**
  - `apps/desktop/ported.json`'ı bir test `PORTED` ile eşit tutar. Envanter onu okuyup masaüstü sütununu doldurur (`docs/inventory/web.md` → “Masaüstü”).
  - İlk dilimde 11 / 163 komut taşındı: aç, kaydet, farklı kaydet, tema (üç komut), şeridi daralt, komut satırına git, hakkında, klavye kısayolları, tüm katmanları göster.
  - Bir komutu taşımak dört adımdır:
    1. `App::run`'a işleyici yazılır;
    2. komut `PORTED`'a eklenir;
    3. `KENTOS_WRITE_PORTED=1 cargo test -p kentos-desktop ported` çalıştırılır;
    4. `pnpm inventory` çalıştırılır.

    Bir test, `PORTED`'daki her komutun işleyicisi olduğunu denetler.
- **Ürün komutlarıyla ilişki (ADR 0013):**
  - Masaüstünün arayüz komutları web'in arayüz komutlarının karşılığıdır.
  - Bir web komutu ürün komutuna vardığında masaüstü işleyicisi aynı ürün komutunun native gerçekleştirmesini çağırır. Örnek: poligon aracının onayı `cad.polygon.create` olur.
  - İki taraf aynı fixture'dan aynı sonucu verir (`CMD-04..07`).
- **Klavye:**
  - Ctrl ya da Alt'lı akorlar ve F tuşları web'in tuş eşlemesinden gelir.
  - Başka bir şey yazmak komut satırını açar (CAD alışkanlığı).
  - Web'in tek harfli araç kısayolları (L, P, G …) araçlarla birlikte taşınır.
- **Dosya pencereleri:** `rfd` 0.17.2 (MIT). Linux'ta xdg-desktop-portal, Windows ve macOS'ta sistemin penceresi. Getirdiği tek yeni paket `pollster`'dır ([bağımlılık kaydı](../deps/README.md)).
- **Görsel denetim:** `kentos-cad snapshot çıktı.png [çizim.kcad] [--sekme <id>] [--tema acik]` pencereyi açmadan görüntüler (KentOS UI'ın ekransız çizicisi). `make desktop-snapshot` aynısını yapar.

## Sonuçlar

- `make desktop` ve `make stop-desktop` uygulamayı arka planda başlatır ve kapatır. `pnpm rust:test:desktop` masaüstü testlerini ve clippy'yi de kapsar.
- Masaüstü derlemesi web envanterine bağlıdır. Web'de komut değişince `pnpm inventory` çalıştırılır; `inventory:check` bunu denetler, masaüstü de yeni hâli gömer.
- **Sıradaki işler:**
  - saf wgpu çizim alanı (`REN-01..07`);
  - poligon kabul izi ve araçlar (`UX-01/04/06`, `CMD-04..07`);
  - ayarlar (`SET-*`);
  - bulut durumu.
  - Web'in “Yeni proje” akışı standart katman şablonuna ve koordinat sisteminin çalışma alanı merkezine dayanır. Bu ikisi TS işlevi olarak kopyalanmadan, önce paylaşılan veri (sözleşme ya da fixture) olmalıdır.
- **Açık:**
  - erişilebilirlik ve IME denetimleri, Windows/macOS matrisi (`UI-12`);
  - dosya penceresinin KDE/GNOME portalında elle denenmesi;
  - masaüstünde ayar ve yerleşimin kalıcılığı (`SET-04`).
