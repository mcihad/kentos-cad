# ADR 0050: Masaüstünde uygulama menüsü, Başlangıç ekranı ve son dosyalar

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** DESIGN.md §7.1.1; CLAUDE.md §2, §4.4, §4.5, §21.2; ADR 0017 (masaüstü kabuğu), 0023 (tipli ayarlar), 0041 (masaüstünün bulut arayüzü), 0049 (yeni proje ve proje ayarları)
- **Sahibin isteği (26 Eylül):** “Web'de ribbon görünümünde sol üstte harika bir ribbon ana menü var, masaüstünde de olmalı bu menü.” Genel yön: masaüstü web ile aynı düzeye getirilir.

## Bağlam

- Web'de KentOS logosuna tıklayınca uygulama menüsü açılır (`ui/appmenu/AppMenu.ts`, DESIGN.md §7.1.1): solda dosya komutları, sağda üzerine gelinen satırın içeriği ya da açık çizim ve son dosyalar, altta ayarlar, kısayollar ve hakkında.
- Web'de Başlangıç ekranı (`file.start`, `ui/start/StartScreen.ts`) açılışta ya da Dosya menüsünden gelir: yeni proje, dosya aç, bulut, DXF, son dosyalar, çizime devam.
- İkisi de son dosyalar listesini kullanır (`app/recentFiles.ts`; IndexedDB'de dosya tutamaçları, en çok 10).
- Masaüstünde üçü de yoktu. Şeridin marka düğmesi yalnız ilk sekmeye geçiyordu. KentOS UI'da bir uygulama menüsü bileşeni vardı (`crates/ui/src/widget/app_menu.rs`), masaüstü onu kullanmıyordu.

## Karar

### Son dosyalar (`apps/desktop/src/recent.rs`)

- Masaüstü, açılan ya da kaydedilen çizimlerin yolunu kendi dosyasında tutar: `$XDG_STATE_HOME/kentos-cad/son-dosyalar.json` (yoksa `~/.local/state/kentos-cad/`).
  - Bu, programın geçmişidir: ayar değildir (`ayarlar.json`'a girmez), çizim değildir, çizimden bir şey taşımaz.
  - Biçim: `{"format": "kentos.recent-files", "version": 1, "files": [{path, name, at, info}]}`. Okunamayan dosya boş liste sayılır ve sonraki kayıtta yeniden yazılır.
  - Dosya bütün yazılır, sonra yerine taşınır; çökme eski ya da yeni listeyi bırakır.
- Web'in kuralları: en yeni başta, en çok 10; aynı dosya yeniden açılınca başa geçer; satırda dosyanın adı, o anki nesne sayısı ve koordinat sistemi, ne zaman (“az önce”, “5 dakika önce”…; masaüstünün öbür listeleriyle aynı söz).
- Listeye giren: bir dosyadan açılan çizim (v1 dahil) ve Kaydet ya da Farklı kaydet ile yazılan dosya. Kurtarma kopyası ve bulut projesi girmez (dosyaları yoktur). Kayıt sürerken başka çizim açıldıysa o kayıt listeye girmez: satırın bilgisi ekrandaki çizimden yazılırdı.
- Son dosyayı açmak, Aç gibi önce ekrandaki çizimden ayrılır (`Then::OpenRecent`): kaydedilmemiş iş sorulur, bulut projesinin gönderilmemiş işi taslağa yazılır. Vazgeç hiçbir şeyi açmaz. Yeri boşalmış dosya açılmaz, listeden çıkar ve bu söylenir.

### Uygulama menüsü (`apps/desktop/src/app_menu/`)

- Şeridin KentOS CAD düğmesi menüyü açar ve kapatır; düğme açıkken koyulaşır. Menü dışına tıklamak, Esc ya da herhangi bir komutun çalışması (satır, kısayol, şerit) menüyü kapatır; web'deki `executed` gibi.
- **Sol sütun** web'in dokuz satırıdır, aynı sözlerle: Yeni, Aç, Kaydet, Farklı kaydet, İçe aktar ›, Dışa aktar ›, Bulut ›, Yazdır ve pafta, Proje ayarları.
  - Satırın altındaki çizgi durumu söyler: Kaydet'te dosyanın adı ya da “İlk kayıtta yer sorulur” (bulut projesinde kendiliğinden kayıt ya da revizyon), Bulut'ta hesap, açık proje ya da “Sunucuya ulaşılamıyor”.
  - Kısayol başlığın satırındadır; açıklama satırın bütün genişliğini alır.
- **Sağ bölme:**
  - Bu çizim: adı ve nerede durduğu, yongalar (koordinat sistemi, çalışma modu, ölçek, çizim yazı tipi), nesne ve katman sayısı, kayıt durumu (kaydedilmemişse Kaydet), dört hızlı karo, son beş dosya.
  - İçe ve Dışa aktar: web'in biçim listesi, rozet, ad ve ne aldığı ya da verdiği.
  - Bulut: oturum yoksa giriş ve bu cihazdaki projeler; oturum varsa hesap kartı ve Çıkış, açık proje kayıt lambasıyla, düğmeler ve hesabın sunucudaki son beş projesi (`CatalogView::Recent`; bölme ilk gösterildiğinde bir kez sorulur, yüklenirken iskelet satırlar, geç gelen eski yanıt düşer).
- **Masaüstünde henüz olmayan komut**, şeritteki gibi soluktur ve nedenini söyler: “Geliştirme aşamasında” (web'de de yok), “Web'de var; masaüstüne henüz taşınmadı” ya da “Açık çizim yok”. Biçim satırında “Yakında” hapı vardır. Sessiz düğme yoktur (CLAUDE.md §4.5).
- **Klavye:** menü açıkken bütün tuşlar ona gider. ↑ ↓ satırlar arasında (soluk satırlar atlanır), → sağ bölmeye, ← geri, Enter ya da Boşluk satırı çalıştırır, Esc kapatır.
- **Son bulut projesi** tek tıkla açılır; açılış penceresi projenin adını gösterir. Web aynı satırda Bulut projesi aç penceresini proje seçili açar; masaüstü doğrudan açar (fare önceliği, tek tıkla yeniden açılan son dosyalarla aynı).
- **KentOS UI bileşeni** DESIGN.md §7.1.1'e göre genişletildi:
  - isteğe bağlı başlık (logo, ürün adı, çizimin adı);
  - genişlik (masaüstünde 780);
  - kısayol;
  - klavye vurgusu (`current`);
  - 34 px simge karosu (`style::container::tile`);
  - soluk satır;
  - alt şeritte eylemler solda, not sağda.

  Marka işareti `ribbon::logo_mark` olarak açıldı. Vitrin aynı bileşeni kullanır.

### Başlangıç ekranı (`apps/desktop/src/start.rs`)

- Web'in ekranı, aynı sözlerle: marka, dört eylem (kısayoluyla), açık çizim varsa “Çizime devam et”, son dosyalar (hepsi), “Açılışta göster” anahtarı.
- Açılışta çıkar:
  - tercih açıksa (`appearance.startScreen`, varsayılan açık; artık masaüstünde de, Uygulama ayarları → Görünüm);
  - komut satırında çizim verilmemişse;
  - önce kurtarma sorusu yoksa. Çöken bir oturumun işi varsa o sorulur, Başlangıç ekranı onun üstünü kapatmaz.
- Dosya → Başlangıç ekranı (`file.start`) ve menüdeki satır yeniden açar. Esc, arka plan ve “Çizime devam et” kapatır.

## Bu dilimde olmayanlar

- Çalışma modunun şeridi ve menüyü süzmesi (`workspace.*`): sonraki dilim.
- Masaüstünde GeoJSON ve Shapefile (ADR 0046'nın okuyucuları hazır) ve onaylanan `.zip` desteği (`FMT-07`).
- Yazdır ve pafta, NCZ, PDF: web'de de yok.

## Doğrulama (26 Eylül 2026, Linux)

- `recent`: en yeni başta, en çok 10, yeniden açılan başa geçer, liste programı aşar; bozuk dosya boş liste sayılır ve yeniden yazılır; bellekteki liste dosya yazmaz.
- `app_menu::tests`:
  - düğme açar ve kapatır; dışarı tıklama, Esc ve başka yerden çalışan komut kapatır;
  - satırlar komutlarını çalıştırır;
  - olmayan komut soluk ve nedeniyle;
  - satır çizgileri;
  - klavye (sarma, soluk satırı atlama, → ←, Enter);
  - son dosya açılır, kaybolan listeden çıkar;
  - kaydedilmemiş iş önce sorulur, Vazgeç açmaz;
  - açılan ve kaydedilen dosya başa geçer;
  - bulut bölmesi: oturumsuz düğmeler, istek bir kez, geç yanıt düşer, satır projeyi adıyla açar, hata söylenir.
- `start::tests`: açılışta çıkar, kapalıysa ya da kurtarma sorusu varsa çıkmaz; eylemler, Çizime devam et, Esc; anahtar tercihi yazar; unutulan dosya listeden çıkar.
- Bilerek beş kez bozuldu, beşini de testler yakaladı: komutun menüyü kapatması, kaydın listeye yazılması, son dosyada kaydedilmemiş iş sorusu, klavyenin soluk satırı atlaması, kurtarma sorusunun önceliği.
- Görüntüler (koyu, açık): `cargo test -p kentos-desktop app_menu::tests::screens -- --ignored`, `.run/shots/menu-*`, `baslangic-*`.
