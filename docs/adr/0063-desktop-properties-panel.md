# ADR 0063: Masaüstünde Öznitelikler paneli, düzenlenebilir

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.4, §4.10, §7; DESIGN.md §7.5; TODOS.md `UI-09`, `UI-11`; ADR 0029 (seçim), 0058 (paneller)
- **Sahibin yönü (26 Eylül):** web'deki araçlar, düzenleyiciler ve menüler masaüstüne birebir taşınır.
- **Web ajanının tarifi (26 Eylül):** `ui/properties/PropertiesPanel.ts` ve `ui/widgets/PropertyGrid.ts` koddan okundu. Web'in aynı günkü düzeltmeleri de alındı:
  - kilitli ya da karışık seçimde renk ve sembol adıyla gösterilir (`b643fa8`);
  - alınmayan değer gösterilen değere döner; Metin kırpılır;
  - gizli katmana taşımada uyarı verilir (`36884fa`);
  - renk listesi Siyah ile başlar.

## Bağlam

- Masaüstünün özellikler paneli salt okunurdu:
  - seçimin özeti;
  - ağaçta seçili katmanın bilgisi (yalnız masaüstünde);
  - projenin ayarları.
- Web'in Öznitelikler paneli ise düzenlenir: katman, renk ve sembol; noktanın, yazının, ölçünün ve taramanın değerleri; öznitelikler.

## Karar

### Panel (`apps/desktop/src/properties/`)

- **Adı:** "Öznitelikler", web'deki gibi.
- **Başlığın meta yazısı:** tek nesnede "#4", çoklu seçimde "3 nesne".
- **Seçim yokken:**
  - "Seçili nesne yok" ve "Özelliklerini görmek için çizimde bir nesneye tıklayın. Birden fazla nesne için sürükleyerek seçin.";
  - altında "Çizim" bölümü: Dosya, Koordinat sistemi, SRID, Çizim ölçeği, Nesne sayısı, Etkin katman.
- **Tek nesne:**
  - Özette türü, etiketi (vurgu renginde, ör. "101/7") ve katmanının renk örneğiyle yolu yazar.
  - "Genel" bölümü: Tür, Katman ▾, Renk ▾ ve Sembol ▾ (yazı ve ölçüde Sembol yok).
  - "Geometri" bölümü: satırları web'deki gibi türe göredir.
    - Uzunluklar projenin basamağı ve m ile yazılır.
    - Semt projenin açı birimindedir; geometrik açılar derece ve 4 basamaktır (yazı ve tarama Açısı 2 basamak).
    - Alan birimi m² değilse iki satır: m² ve projenin birimi.
  - "Öznitelik bilgileri" bölümü, varsa: her anahtar bir satırdır; sayıya benzeyen değer eş aralıklı yazılır.
- **Çoklu seçim:**
  - Özette "3 nesne seçili" ve türlerin dökümü ("1 kapalı alan, 1 daire, 1 çizgi").
  - "Ortak özellikler": ortak katman, renk ve sembol, ya da "Çeşitli".
  - "Toplamlar": toplam uzunluk ve alan, geometri deposundan.
- **Düzenleme** (web'in kuralları):
  - Noktanın Y ve X'i, ölçünün ötelenmesi, yazı yüksekliği ve yazısı, taramanın deseni, açısı ve aralığı, yazının metni, yüksekliği ve açısı, öznitelikler düzenlenir.
  - Sayılar JavaScript'in `parseFloat`'ı gibi okunur: ilk virgül ondalıktır, "12abc" 12'dir.
  - Alınmayanlar çizimi değiştirmez: sayı olmayan, sıfırdan büyük olması gereken yükseklik ve aralık, boş metin.
  - Ölçünün boş yazısı ölçülen değere döner. Yazı açısı [0, 360) aralığına getirilir.
  - Öznitelik yazıldığı gibi alınır, boş da olur. "Parsel" ya da "Ada" değişince etiket eski değere eşitse onu izler.
- **Katman ▾:**
  - Her katmanın yolu ve renk örneği listelenir; kilitli katman seçilemez.
  - Hedef katman gizliyse şu söylenir: "“{ad}” katmanı gizli; taşınan nesneler görünmeyecek." Nesneler seçili kalır.
- **Renk ▾:** "Katmana göre", sonra web'in sekiz rengi, örnekleriyle.
- **Sembol ▾:** "Katman stiline göre" (`style.clearSymbol`), "Kitaplıktan seç…" (`style.assign`), "Katman stili…" (`style.layerStyle`). Bu komutlar masaüstünde henüz yok; seçilince bunu söylerler.
- **Kilitli katman:** nesnesi ya da onu içeren seçim düzenlenmez.
  - Katman satırı "{yol} (kilitli)" ya da "Kilitli katman içeriyor" yazar.
  - Renk ve Sembol adlarıyla görünür.
- **Adımlar:**
  - Katman için "Katman değiştir", renk için "Renk değiştir", öbürleri için "Değiştir".
  - Bunları web gibi belgeye doğrudan yazar. Web ajanının önerisi (D ve C8) `cad.entities.setProperties` v1 ve `cad.entities.edit`'in "properties" işlemidir. İkisi gelince iki panel de onlardan yazacak.
- **Bölümler:** başlıklarına tıklanınca kapanır, uygulama açık kaldıkça kapalı kalır.

### KentOS UI

- **`EditCell`:** düzenlenene dek yazı gibi görünen hücre.
  - Üzerine gelince ince kenar belirir; düzenlenirken zemini ve vurgu kenarı vardır.
  - Enter ya da hücreden çıkmak (başka yere tıklamak) değişen metni gönderir. Esc vazgeçer ve uygulamaya ulaşmaz.
  - Hücre her zaman uygulamanın değerini gösterir.
- **`PropertySheet`:** Öznitelikler ızgarası.
  - Katlanan bölümler; adı %42, değeri kalan genişlikte satırlar.
  - Değer düz yazı (`property_grid::value`), `EditCell` ya da renk örnekli açılır liste (`property_grid::choice`) olabilir.
- **`Menu::radio` ve `Menu::swatch`:** web'in `radio` ve `swatch` öğeleri.
  - Seçiliyse vurgu renginde nokta, adın önünde renk örneği.
  - Örneği olan menüde bütün adlar aynı hizadan başlar.
  - Katman ağacının renk, çizgi tipi ve kalınlık menüleri de web'deki gibi bunları kullanır.
- Vitrine "Öznitelikler ızgarası" örneği eklendi.

## Web'den ayrılanlar

- **Sembolün adı:** projenin stillerinden okunur. Sistem kitaplığı (MPYY) masaüstünde henüz yok; oradan bir sembol kimliğiyle gösterilir (ör. `mpyy.uip.konut.konut-alani`). Kitaplık, stil düzenleyicileriyle gelecek.
- **Özniteliklerin sırası:** anahtar sırasıdır. Masaüstü belgesi öznitelikleri sıralı tutar; web dosyadaki sırayı korur.
- **Ağaçta seçili katmanın bilgisi** artık panelde gösterilmiyor, web'deki gibi.

## Doğrulama

- `properties::tests` (10 test; değerler elle hesaplandı):
  - seçimsiz çizim özeti;
  - parselin satırları ve Katman ▾ listesi;
  - katman ve renk adımları, gizli katman uyarısı;
  - noktanın `parseFloat` gibi okunan koordinatları;
  - öznitelik ve Parsel etiketi;
  - yazı, ölçü ve taramanın alınan ve alınmayan değerleri;
  - kilitli nesne;
  - çoklu seçim ve toplamlar;
  - bölümün kapanması;
  - sayı okuma.
- KentOS UI:
  - `edit_cell::interaction`: Enter, aynı değer, Esc, başka yere tıklama;
  - `context_menu::tests`: örnek sütunu ve radyo.
- `pnpm rust:test`, `pnpm rust:test:desktop` (clippy temiz), `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm e2e:interaction`, `pnpm inventory:check`.
- Görüntüler (`properties::tests::screens`, `.run/shots/oznitelik-*`), koyu ve açık, 1440×900 ve 1100×650:
  - seçimsiz;
  - parsel;
  - çoklu seçim;
  - ölçü;
  - düzenlenen yazı (Genel kapalı);
  - açık Katman ▾.
