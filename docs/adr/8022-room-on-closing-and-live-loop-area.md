# 3022 — Kapanınca mahal ve canlı döngü alanı

Tarih: 2026-11-08. Durum: kabul (M22, ADV-11; plan S11, S13).

## Sorun

- **Duvarla kapatılan oda mahal değildi.** Duvar aracında Kapat (K) kapalı duvarı yazıyordu. Mahal için Mahal aracına geçip
  içine tıklamak gerekiyordu: bir araç değişimi ve bir geri alma adımı daha.
- **Çizerken odanın alanı görünmüyordu.** Etiket yalnız son kenarın uzunluğunu ve açısını söylüyordu. "Bu oda kaç metrekare
  olacak" ancak kapatıp mahal yapınca okunuyordu.
- **ADV-11:** "döngü kapanınca isteğe bağlı mahal oluşur; çift tık da mahal yapar (Sweet Home 3D, Rayon, Cedreo). Sürüklerken
  canlı alan ve program hedefiyle fark görünür."

## Araştırma (8 Kasım 2026)

- **ADV-11'in örnekleri:**
  - Sweet Home 3D: oda duvarların içine çift tıklanarak oluşur.
  - Rayon ve Cedreo: duvar döngüsü kapanınca oda kendiliğinden gelir; alan çizerken görünür.
- **KentOS'ta parçalar hazırdı:**
  - Mahal tohumdan sınırını bulur (`cad.spaces.create`).
  - Tek geri alma için `begin_group` var (ADR 8021).
  - Kutup noktası (`labels::geom::pole`) çokgenin içinde kenarlardan en uzak noktadır; L biçimli bir odada ağırlık merkezi
    odanın dışında kalabilir.

## Karar

- **Duvar aracına "Kapanınca mahal" ayarı** (`Memory::wall_room`, harfi M):
  - Panelde aç-kapa düğmesi, komut çubuğunda "Kapanınca mahal: Açık/Kapalı" çipi. Çizerken M tuşu değiştirir.
  - İlk değeri Kapalı: var olan izler ve alışkanlık değişmez. Uygulama açık kaldıkça ayar kalır, duvarın öbür ayarları gibi.
  - Yalnız Duvar aracında. Dikdörtgen duvar, Ölçülü bölme ve Çizgiden duvar ortak satırlarını (`wall_rows`) değiştirmeden
    kullanır.
- **Açıkken Kapat** (`close_wall`, `write_loop_room`):
  - Kapalı duvar ve mahali tek geri alma adımıdır ("Duvar ve mahal"). Mahal reddedilirse grup iptal edilir, duvar da gider.
  - Tohum, döngünün kutup noktasıdır. Mahal net sütun kuralıyla, Etkin kat'ta, mahallerin katmanında (ADR 8008) yazılır.
  - İleti: "Kapalı duvar ve mahali eklendi: duvar 39,42 m, mahal …".
- **Canlı döngü alanı:** Duvar çizerken üçüncü noktadan sonra etikete "Döngü alanı {alan}" satırı gelir.
  - Alan, noktalardan imlece ve imleçten ilk köşeye kapanan halkanın alanıdır: Kapat'ın kapatacağı. Duvarın yolu üzerinden
    ölçülür, net değildir; net alanı mahal söyler.
- **Çift tık ve program hedefiyle fark** bu maddede yapılmadı (sonraki adımlar).

## Ölçüm

- **Test** (`building_context_tests::duvar_shows_the_loop_s_area_and_kapat_writes_its_room_when_asked`):
  - Kapanınca mahal açıldı, 0,0 · 50,0 · 50,40 yazıldı, imleç (2, 38)'de: etiket "Döngü alanı 1910,00 m²".
  - 0,40 ve K: 1 duvar, 1 mahal, 0 etiket; bir geri alma: hiçbiri.
- **Gerçek pencere** (`fixtures/interaction/real/kapaninca-mahal.txt`, 1440×900, sahibin ayarlarının kopyası):
  - Duvar ▾ › Duvar, M (Kapanınca mahal Açık), üç köşe tıklandı; imleç dördüncüde: "Döngü alanı 96,71 m²".
  - Dördüncü köşe ve K: kapalı duvar ve mahali yazıldı, MM-XXXX-MAHAL açıldı.
  - Resim: `.run/gui-evidence/mimari-m22/m22-once-sonra.png`.
- **Test ortamı notu:** testte çizim alanının boyu verilmezse (`camera.set_size`) kameranın ölçeği oturmuyor ve kenetin
  menzili bütün odayı kaplıyordu: imleç ilk köşeye kenetleniyordu.

## Sonraki adımlar

- Odaya çift tık ile mahal (Sweet Home 3D): boş bir yerde, Mimari'de, çevresinde duvar varken.
- Canlı alanın yanında program hedefiyle fark (ADV-04): odanın bölümü ve kullanımı seçildiğinde.
- Çokgen oda (ADR 8021'in sonraki adımı): Oda aracının Duvar yoluyla çizilen hâli; bu madde ona temel oldu.
