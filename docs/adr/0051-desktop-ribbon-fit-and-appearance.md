# ADR 0051: Masaüstünde şeridin pencereye sığması ve Görünüm sekmesi

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** DESIGN.md §7.3.1; CLAUDE.md §4.4, §8; ADR 0016 (KentOS UI), 0017 (masaüstü kabuğu), 0023 (tipli ayarlar)
- **Sahibin istekleri (26 Eylül):**
  - “Ribbon butonlarını daha da iyi hale getirelim, mesela bazı sekmelerde buton çok ve sağa doğru scroll kayması yapıyor, daha rafine ve kalite algısı yüksek bir görüntü ver.”
  - “Showcase içerisindeki ribbon menüde görünüm kategorisi vardı, o kategoriyi uygulamaya getir, arayüzü ayarlamak için güzel bir menüydü, gereksizleri çıkar, aynı fontlar renkler ve temaları getir.”

## Bağlam

- DESIGN.md §7.3.1 şeridin pencereye sığmasını ister: paneller sağdan sola, her biri bir adım inerek küçülür (büyük → küçük etiketli → yalnız ikon → panelin adını taşıyan tek düğme); 1100 px'te hiçbir düğme kesilmez ya da kaydırmaya kalmaz. Web bunu yapar (`ui/ribbon/Ribbon.ts` `fit`, `panels.ts` seviyeleri).
- KentOS UI'nin şeridi (`crates/ui/src/widget/ribbon/`) sığmayan grupları yatay kaydırıyordu. Masaüstünde 1440 px'te bile Giriş'in sağı kesiliyordu.
- Web'in seyrek araçları (panel başlığındaki ▾: Halka, Revizyon bulutu, Uzat-kısalt, Köşe ekle/sil, Çizgiye çevir) masaüstü şeridinde hiç yoktu; yalnız komut satırından çalışıyorlardı.
- Masaüstünde yalnız koyu ve açık tema vardı. KentOS UI'nin vitrininde dört tema, sekiz vurgu rengi ve özel renk, üç arayüz ve iki eş aralıklı yazı ailesi, 11–18 px yazı boyutu ve harita zemini seçenekleri bir Görünüm sekmesinde duruyordu; bunlar uygulamada kullanılmıyordu.

## Karar

### Şerit pencereye sığar (KentOS UI)

- Düğmelerle kurulan grup (`Group::tool`) ya da kendi görünüşünü veren grup (`Group::custom`) uyarlanır. Şerit bütün grupları uyarlanırsa panelini Iced'in `responsive`'iyle, o anki genişlikte kurar.
- Seviyeler web'inkidir: 0 tasarlandığı gibi, 1 bütün düğmeler küçük etiketli (sütunda üçer), 2 yalnız ikon, 3 panelin adını taşıyan tek düğme. Kendi görünüşü olan grubun ara seviyesi yoktur: ya tam ya katlanmış.
- Sığdırma (`ribbon::fit`) web'in `Ribbon.fit` kuralıdır:
  - hiçbir panel iki adım inmeden her panel bir adım iner, önce en sağdaki;
  - `keep` paneller (Giriş'te Çizim ve Değiştir) öbürleri yalnız ikona inene kadar büyük düğmelerini korur, yalnız tek düğmeye katlanmak onlardan önce gelir;
  - genişlik kazandırmayan adım atlanır.
- Genişlikler yazı ölçümünden hesaplanır (`typography::text_width`) ve her panel bu genişlikte kurulur; en küçük hâli de sığmayan şerit (çok dar pencere) yine kaydırılır.
- Katlanmış panel tek düğmedir (panelin ikonu, adı ve ▾); tıklayınca panelin araçları menüde açılır: aileler alt menü, seyrek araçlar “Diğer araçlar” alt menüsü. Web paneli açılır pencerede gösterir; KentOS UI'de ortak açılır menü bileşeni vardır.
- Panel başlığı web'inki gibidir: ortada ad, seyrek araçlar varsa ▾ ile menü; varsa sağda pencere açıcı (↘): başka bir sekme ya da komut (Katman stili, Proje ayarları, Uygulama ayarları).
- Düğmeler web'in ölçüleriyle rafine edildi:
  - satır 24 px, büyük ikon 28 px ve sabit 1,55 px çizgi (`Glyph::weight`; ikon büyüyünce ağırlaşmaz);
  - büyük düğme 50–100 px, etiket iki satıra kadar;
  - ikon dinlenirken ikincil renkte, üzerine gelince ana renkte; etiket kendi rengini taşır (`style::button::ribbon`);
  - çalışan araç dolu vurgu (ailesinin bölünmüş düğmesi de), açık anahtar (Kenet, Orto, Kutupsal izleme) yumuşak vurgu zemini;
  - bölünmüş düğmenin eylemi ve oku ayrı aydınlanır.
- Vitrin eski grupları (`Group::push`: alanlar, galeriler) kullanır; onlar olduğu gibi kalır ve o sekme eski davranışla kaydırılır.

### Masaüstü şeridi

- Paneller envanterden gelir; envanter panelin `keep`'ini ve pencere açıcısını da yazar (`apps/web/scripts/inventory/collect.mjs`). Katalog panelin ikonunu, seyrek araçlarını (`overflow`), `keep`'ini ve açıcısını okur.
- Masaüstünde olmayan komut şeritte olduğu gibi soluk durur; açıcının hedefi masaüstünde yoksa açıcı gösterilmez.

### Görünüm sekmesi (masaüstü)

- Web'in Görünüm panellerinden sonra vitrinin grupları gelir:
  - **Tema:** dört tema karosu (Koyu, Aydınlık, Gece, Yüksek karşıtlık), sekiz vurgu rengi ve özel renk;
  - **Çizim zemini:** temaya uy, arduvaz, siyah, kâğıt;
  - **Yazı tipi:** IBM Plex Sans, Inter, Plus Jakarta Sans;
  - **Eş aralıklı:** IBM Plex Mono, JetBrains Mono;
  - **Yazı boyutu:** 11–18 px; Büyüt, Küçült, Varsayılan.
- Gereksizler çıkarıldı: vitrinin Harita, Pafta, Pencereler ve Paneller grupları (uygulamanın kendi komutları var), zemin karşılaştırması; web'in Görünüş panelindeki Tema menüsü (Tema grubu onun yerine geçer) ve Çizim motoru menüsü (WebGL2/WebGPU seçimi; masaüstünde anlamı yok).
- Seçimler kullanıcı tercihidir (§4.4), ayar dosyasına yazılır ve hemen uygulanır. Anahtarlar yalnız masaüstündedir: `appearance.theme` (dört tema), `appearance.accentColor` (hazır ad ya da `#RRGGBB`), `appearance.typeface`, `appearance.monoTypeface`, `appearance.textSize`, `appearance.drawingBackground`. Web kendi görünüm anahtarlarını (vurgu, yazı tipi, ölçek) korur; seçenek kümeleri farklı olduğu için ortak anahtar kurulmadı.
- Uygulama ayarları → Görünüm aynı seçimleri gösterir; özel renk orada `#RRGGBB` yazılır, okunmayan renk kaydedilmez ve söylenir.
- Çizim alanı temadan ayrı bir tuvalle çizilir (`viewport::Canvas`): tema kendi tuvalini seçer (koyu → arduvaz, aydınlık → kâğıt, gece → kısık gece zemini, yüksek karşıtlık → siyah); Çizim zemini bunu temadan bağımsız değiştirir.
- Yazı ayarı KentOS UI'de bütün arayüzündür; uygulama onu yalnız değiştiğinde kurar.

## Bu dilimde olmayanlar

- Katlanmış panelin web'deki gibi açılır pencerede panel olarak gösterilmesi (masaüstünde menü).
- Şeritte klavye gezinmesi ve tuş ipuçları (KeyTips), Komut ara kutusu, hızlı erişime ekleme.
- Giriş'in Katmanlar ve Özellikler panelleri (web'de canlı alanlar; masaüstünde yan panellerde).
- Çalışma modunun şeridi süzmesi (`workspace.*`, sıradaki dilim).

## Doğrulama (26 Eylül 2026, Linux)

- `ribbon_tests`:
  - her sekme 1100 px'e kaydırmadan sığar, geniş pencerede hepsi tasarlandığı gibidir;
  - paneller sağdan küçülür, Giriş'in tutulan panelleri en son;
  - Görünüm sekmesi web panellerine ek beş grup taşır.
- `appearance::tests`: seçimler tercih olarak yazılır ve hemen uygulanır (tema, tuval, vurgu, yazı tipi, boyut sınırları); tema komutları dört temayı bilir; özel renk okunur, bozuğu varsayılana döner.
- Bilerek iki kez bozuldu, ikisini de testler yakaladı: katlanmanın kapatılması (1100 px sığmadı), tutulan panel önceliğinin kaldırılması.
- `pnpm rust:test:desktop` (masaüstü, KentOS UI, vitrin; clippy) geçti.
- Görüntüler (1440 ve 1100 px, koyu ve açık, gece, yüksek karşıtlık, büyük yazı): `cargo test -p kentos-desktop ribbon_tests::screens -- --ignored`, `.run/shots/serit-*`.
