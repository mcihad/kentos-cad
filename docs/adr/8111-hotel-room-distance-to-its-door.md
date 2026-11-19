# ADR 8111: Otel yatak odasının içindeki kaçış uzaklığı (BYKHY 50/3) ve mahalin kendi kapısına yürüyüşü

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B22 ("Konut dışı kullanımlar", BIM-13),
  dördüncü dilim. Her durum için bağımsız durumlar ve gerçek çizimde Denetim (§4).
- **Bağlam:** ADR [8035](8035-fire-escape-routes-distances-load-exits.md) (kaçış yolları: en uzak nokta duvarlardan 0,40 m
  önde, 32/6; iç köşelerden 0,40 m'den dönülür; konut dairesinde daire kapısına yürüyüş, 48/2), [8107](8107-use-class-escape-articles.md)
  (konaklama sınıfı, 50/1-a ve 50/1-b), [8108](8108-sprinklers-required-by-use.md) (yağmurlama beyanı), plan K10.
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Komut:** değişmedi (`cad.arch.audit`'in açıklaması yeni kodu
  sayar). **Kural paketi:** mimari-mevzuat 0.25.0 (`konaklama`). **Olgular:** bina olgularının mahallerine `toDoor`.

## §1. Araştırma (18 Kasım 2026)

Binaların Yangından Korunması Hakkında Yönetmelik, ADR 8107 §1'deki aynı derlenmiş metin (SHA-256 `d82d06d2…a7d9`), 50 nci
madde. Kural setine yazılan metin `pdftotext` çıktısında aynen geçer.

| Fıkra | Ne diyor | KentOS kararı |
|---|---|---|
| 50 (başlık), 50/1 | Oteller, moteller ve yatakhaneler; (1) otellerin, motellerin ve yatakhane olarak kullanılan bölümlerin | Konaklama sınıfı ADR 8107'nin okumasıyla |
| 50/2, 50/3 | (2) dış koridorla erişilen otel yatak odaları; (3) **otel yatak odasında veya süit odada** en uzak noktadan çıkış kapısına uzaklık 15 m'yi aşmıyorsa tek kaçış kapısı yeter. Ancak (a) aşıyorsa birbirinden uzakta en az 2 çıkış kapısı; (b) tamamı yağmurlamalı otelin yatak odalarında ya da süitlerinde en uzak noktadan kapıya uzaklık 20 m'yi aşmaz | Otel, motel ve pansiyon; yalnız yurt ya da yatakhane olan bina kapsam dışı (fıkra otel yatak odasını söyler). (b) yağmurlamalı otelde tek kapının sınırı 20 m; 20 m'yi aşan iki kapılı oda uzman incelemesi |
| 50/4, 50/5 | Kaçış uzaklığı odanın çıkış kapısından kaçış merdivenine, dış kaçış geçidine ya da dış açık alana açılan kapıya; tek yönlü koridorda en uzak odanın kapısından | Bu işte değil: Mimari denetimin kaçış uzaklıkları (ADR 8035) en uzak noktadan ölçer (§6) |
| 32/6 | En uzak nokta mekânın içinde, duvarlardan 40 cm önde | Yürüyüşün başı |

Yabancı kurallar (ikincil kaynaklar, yalnız bağlam): otel odası içindeki yürüyüş bazı kurallarda kaçış yolunun ortak kısmı
sayılmaz, ayrı bir sınırı vardır (yağmurlamasız ve yağmurlamalı iki değer); süitin yangına dayanıklı ayrılmışsa süitin
kapısından ölçüldüğüne dair bir itiraz kurulu kararı. BYKHY'nin değerleri (15 m, 20 m) bunlardan farklıdır; bağlayıcı olan
BYKHY'dir, değerler ona dayanır.

## §2. Karar

- **K204 — Mahalin kendi kapısına yürüyüş (alan):** `escape::RoomEscape.door`: her mahal için en uzak noktasından (ADR
  8035'in başları: dış köşeler 0,40 m içeri) en yakın kapısına ya da eşiksiz boşluğuna (kapının mahal yanındaki noktası,
  duvar yüzünden 10 cm önde), yalnız mahalin içinden, iç köşelerden 0,40 m'den dönerek. Kapısı olmayan mahalde yok.
  Kaçış yollarının aynı çizgesi; mahal başına bir kısa yol araması.
- **K205 — Olgu:** bina olgularının mahallerinde `toDoor` (m, milimetreye). Mimari olgularda değişiklik yok.
- **K206 — 50/3:** konaklama sınıfında, yalnız yurt ya da yatakhane olmayan binada; oda adı otel odası, yatak odası, süit,
  konuk ya da misafir odası ya da Oda (banyo, WC, koridor, hol, antre değil). Sınır yağmurlama beyanı varsa 20 m, yoksa 15 m.
  `toDoor` sınırı aşan odada kapı (olguların kapıları) 2'den azsa kaldı (`bykhy_hotel_room_distance`, kuralı 50/3-a ya da
  50/3-b); yağmurlamalı otelde 20 m'yi aşan iki kapılı oda uzman incelemesi; kapısı çizilmemiş oda eksik veri. Kapıların
  "birbirinden uzakta" olması ölçü değildir: idarenin takdiri, satırın notu söyler.
- **K207 — Süit:** birden çok mahal olarak çizilmiş süitte her mahal kendi en yakın kapısına ölçülür; süitin çıkış
  kapısına yürüyüş ölçülmez. Süit bina programında bir grup olarak çizilince konut dairesindeki gibi grubun kapısına
  yürüyüş sıradaki iş.

## §3. Uygulama

- Alan: `crates/native/domain/src/escape.rs` (`RoomEscape.door`, mahallerin kapı düğümleri `own_doors`).
- Olgular: `crates/shared/extensions/src/audit_facts/building.rs` (`SpaceFacts.to_door`, yuvarlama),
  `crates/native/processing/src/audit_facts/building.rs` (`escape::escape`'ten), SDK şeması; mimari olguların sınama
  dosyası `fixtures/audit-facts/v1/mimari.json` (planın odalarının `toDoor`'u; fark yalnız yeni alan).
- Kural paketi: `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/konaklama.py`, kural sırası, `data/surumler.json`
  (50/3), 0.25.0, CHANGELOG; Denetim yüzü kullanım sınıfınınki (`bykhy_hotel_`).

## §4. Sınamalar

- **Bağımsız durumlar** (`fixtures/standards/v1/konaklama.json`, 25 durum; `scripts/fixtures/konaklama_cases.py`): 8 m
  geçer; 17 m tek kapı kalır, iki kapı geçer; yağmurlamalı 17 m geçer, 22 m tek kapı kalır, iki kapı uzman incelemesi;
  tam 15,00 geçer, 15,01 ve 15,005 kalır ("15,005 m"); yağmurlamalı tam 20,00 geçer; kapısız eksik veri; süit, "Otel
  odası 12", motel, pansiyon kalır; öğrenci yurdu, konut, büro ve odasız otel katı kapsam dışı; iki odadan uzak olan;
  süitin banyosu sayılmaz; otel ve büro karma; tarihsiz en yeni hâl; 1.6.2007 başvurusu uzman incelemesi; programsız
  binada mahal adından.
- **Gerçek çizimde Denetim** (`hotel_room_tests::a_deep_hotel_room_with_one_door_fails_and_a_second_door_passes`):
  komutlarla 20 cm duvarlar içinde Koridor ve 18 m derin Oda 101, kapısı koridor duvarında: olgunun `toDoor`'u 17,365 m,
  50/3 kaldı; uzak duvara ikinci kapı: yürüyüş 15 m'nin altında, geçti. Konut binasının kural seti satırı "27 madde
  kapsam dışı".
- Alanın kaçış sınamaları, olguların sınamaları, başsız `cad.arch.audit`'in tarih sınamaları geçti.
- **Resim** (`.run/gui-evidence/otel-odasi-8111/`): derin odanın bulgusu Denetim'de (açık ve koyu).

## §5. Arayüz

Değişmedi: Denetim'in Mevzuat grubunda yeni bulgu, kural setinde yeni madde.

## §6. Sıradaki adımlar

- Süitin grubunun kapısına yürüyüş (K207).
- 50/4 ve 50/5: odanın çıkış kapısından kaçış merdivenine uzaklık, tek yönlü koridorda en uzak odanın kapısından —
  yapıldı: ADR [8156](8156-bedroom-escape-from-its-door.md) (yurt yatak odası da; süitin iç odası süitin çıkış
  kapısından).
- 50/2 dış koridorlu otel odaları (dış koridor çizimde ayrı okunmuyor), 50/6 duman kesicileri (koridorun havalandırması
  beyanı).
