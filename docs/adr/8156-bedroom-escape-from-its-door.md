# ADR 8156: Otel ve yurt yatak odasının kapısından kaçış uzaklığı — BYKHY 50/4, 50/5

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B22 ("50/4–50/5 odanın
  kapısından merdivene"); ADR [8111](8111-hotel-room-distance-to-its-door.md) §6.
- **Bağlam:** ADR 8035 (yangın kaçışı: mahalin en uzak noktasından en yakın kat çıkışına yürüyüş, Ek-5/B; Kaçış yolları
  çizimi), 8107 (kullanım sınıfı: bina programının kullanımından, yoksa mahal adlarından), 8111 (otel odasının içinde
  kendi kapısına yürüyüş, 50/3).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 50/4, 50/5 (RG 19.12.2007/26735), 32/3, 32/4, 32/6,
  Ek-5/B (değişik RG 9.9.2009/27344, 9.7.2015/29411).
- **Sözleşme:** yeni komut yok. `kentos_domain::escape::RoomEscape::door_exit` (`DoorExit`),
  `kentos_domain::fire_use::{lodging_of, lodging_bedroom}`. `kentos.auditFacts` 1'in `building.spaces[]`'ine `doorExit`
  (`DoorExitFacts`). `tr.kentos.mimari-mevzuat` 0.43.0 (`yatak_kacisi`, `konaklama.sinif`). **`.kcad`:** değişmedi.
  **Web:** dokunulmadı.

## §1. Sorun

Mimari denetimin kaçış uzaklığı (ADR 8035, 32/4) otel ve yurt yatak odasını da en uzak noktasından ölçüyordu. 50/4 bu
odaların kaçış uzaklığını odanın çıkış kapısından başlatır; odanın içi 50/3'ün işidir (ADR 8111). Kapısı merdivene 8 m
olan, en uzak noktası 17,8 m olan otel odası 32/4'ten kalıyordu; 50/4 ölçülmüyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 50/4 (mevzuat.gov.tr derlenmiş metni, SHA-256 d82d06d2…a7d9) | "Kaçış uzaklığı, yatak odası veya süit odanın çıkış kapısından başlayarak bir kaçış merdivenine, dış kaçış geçidine veya dış açık alana açılan çıkış kapısına kadar olan ölçüdür." | Odanın kapısından en yakın kat çıkışına (K400, K401) |
| BYKHY 50/5 | "Koridor boyunca yalnızca tek yönde kaçış imkânı var ise, kaçış uzaklığı en uzaktaki yatak odası çıkış kapısından itibaren ölçülür. İki yönde kaçış sağlanabiliyor ise, kaçış uzaklığı her bir yatak odasının çıkış kapılarından ölçülür." | Her oda kendi kapısından; tek çıkışta bütün yol tek yön sınırına (en uzak kapı en uzun yoldur), iki çıkışta tek yönlü kısım ve bütün (K401) |
| BYKHY 50 başlığı ve 50/1 | "Oteller, moteller ve yatakhaneler"; otellerin, motellerin ve diğer binaların yatakhane olarak kullanılan bölümleri | Otel, motel, pansiyon ve yurt, yatakhane (K401) |
| Ek-5/B (9.7.2015 hâli) | Oteller, Pansiyonlar: tek yön 15 m (yağmurlamalı 20 m), iki yön 30 m (45 m); Yurtlar, Yatakhaneler: 15 m (30 m), 45 m (75 m). Not: dış kaçış geçitlerinde yağmurlamalı binanın uzaklıkları | Binanın satırı, yağmurlama beyanının sütunu (K401); dış koridor kalan (§6) |
| Değişiklik tablosu | Ek-5/B 10.8.2009 (2009/15316) ve 16.3.2015 (2015/7401) kararlarıyla değişti; işaretler endüstri yapılarının dipnotunu ve hastanelerin kaçış merdiveni sütununu gösterir | Kural setinde 9.7.2015 hâli; önceki başvuru uzman incelemesi (K401) |
| ADR 8035, 8111 | Kaçış modülü mahalin en uzak noktasından çıkışa ve kendi kapısına yürür; geçitlerin iki yanı düğümdür | Kapının dış yanından yürüyüş aynı çizgeyle (K400) |
| plan/bim-plan.md K10 | Yeni kural Rust'a yazılmaz; Rust kuralları dokunuldukça betiğe taşınır | 50/4 paketin kuralı; Rust'ın 32/4'ü bu odaları bırakır (K402) |

## §3. Karar

- **K400 — Kapıdan yürüyüş:** kaçış modülü her mahal için kapılarının (ve eşiksiz geçitlerinin) dış yanından en yakın
  kat çıkışına yürüyüşü verir: yürüyüş kapının açıldığı mahalde o mahalin kendi yürüyüşleri gibi sürer, kapılardan en
  yakını. Kaçış merdiveninin mahaline açılan kapı ve zemin katta dışarı açılan kapı (binanın son çıkışı) çıkıştadır
  (0 m, tek çıkış). Eşit iki kapıda dışarı açılan, sonra açıldığı mahalin kalıcı kimliği önce (çizimin sırası sonucu
  değiştirmez). Olgularda `doorExit`: uzunluğu, tek yönlü kısmı (öbür çıkışa giden yolla ortak kısım; tek çıkışta
  bütünü), ulaştığı çıkış sayısı, kapının açıldığı mahal (yok: dışarı).
- **K401 — Kural (`yatak_kacisi`):** binanın 50. maddedeki satırı `konaklama.sinif`'ten (K403): otel ya da yurt. Yatak
  odası adı Yatak, Süit, Konuk, Misafir, Öğrenci odası, Oda ya da Otel odası; banyo, WC, koridor, hol değil. Tek çıkışa
  ulaşan odanın bütün yolu tek yön sınırını, iki çıkışa ulaşanın tek yönlü kısmı tek yön, bütünü iki yön sınırını aşarsa
  kaldı (`bykhy_bedroom_escape_distance`); sınırlar Ek-5/B'nin satırından, yağmurlama beyanının sütunundan. Kapısı öbür
  bir yatak odasına açılan oda (süitin iç odası) o odanın kapısından ölçülür: süit odanın çıkış kapısı. Kapısız oda ve
  kapısından çıkışa yol bulunmayan oda eksik veri (ulaşamayan mahal 33/6-b'nin bulgusudur). Satır 9.7.2015'ten önceki
  başvuruda uzman incelemesi.
- **K402 — Rust'ın 32/4'ü bırakır:** yangın kuralları aynı okumayla (`lodging_of`, `lodging_bedroom`; paketin
  `konaklama.sinif` ve `yatak_kacisi.yatak_odasi`'na birebir) bu odaları en uzak noktalarından ölçmez, bulgusunu paket
  verir. Kaçış yolları çizimi odanın yolunu kapısından, binanın satırına göre çizer (kırmızı, yeşil). 33/6-b değişmedi.
- **K403 — Otel ve yurt:** kullanımında ya da adında otel, motel, pansiyon, konaklama ya da hostel geçen konaklama binası
  oteldir, yalnız yurt ve yatakhane geçen yurttur. `konaklama.otel` önce her konaklama sözcüğünün yanında yurt geçince
  ("Otel ve yurt") binayı yurt sayıyordu; 50/3 de bu okumayı kullanır.

## §4. Uygulama

- `crates/native/domain/src/escape.rs` (`DoorExit`, `door_exit`), `crates/native/domain/src/fire_use.rs` (`lodging_of`,
  `lodging_bedroom`).
- `crates/shared/extensions/src/audit_facts/building.rs` (`DoorExitFacts`),
  `crates/native/processing/src/audit_facts/building.rs` (`door_exit`).
- `crates/native/application/src/arch/fire_audit.rs` (konaklama yatak odası: 32/4'ten çıkar, yolu kapıdan çizilir).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/yatak_kacisi.py` (yeni), `konaklama.py` (`sinif`), `mimari.py`,
  `denetim.py` (`kullanim` katkısı `bykhy_bedroom_` bulgularını da verir), `manifest.json`, `data/surumler.json` (50/4,
  Ek-5/B'nin 9.7.2015 hâliyle).

## §5. Sınamalar

- **Kural** (`test_yatak_kacisi.py`; 50/4, 50/5 ve Ek-5/B'nin metninden elle): 21 bağımsız durum: otelde tek çıkışa
  12 m geçer, 16,50 m kalır, tam 15,00 m geçer; iki çıkışta 28 m (tek yönlü 9 m) geçer, 32 m kalır, tek yönlü kısmı
  17 m kalır; yağmurlamalı otelde 19 m geçer, 21 m kalır; yurtta öğrenci odası 16 m kalır, iki yön 40 m geçer (otelde
  kalırdı), yağmurlamalı 28 m geçer; "Otel ve yurt" otel satırıyla kalır; süitin iç odası salonun kapısından geçer;
  merdivene açılan kapı 0 m geçer; kapısız ve yolsuz oda eksik veri; büro ve yatak odasız otel kapsam dışı; tarihsiz en
  yeni hâl; 2014 başvurusu uzman incelemesi; adlardan otel. 50/3'ün durumlarına "Otel ve yurt" (26).
- **Alan:** `fire_use`'un okuması (kullanımlardan, adlardan, otel ve yurt, yatak odası adları).
- **Olgular:** `mimari.json` altında yalnız 11 `doorExit` eklendi (koridorun dışarı açılan kapısı 0 m, `into`'suz); aynı
  plan her sırayla aynı bayt (eşit iki kapının seçimi kimlikle). SDK şeması yeniden üretildi.
- **Masaüstü** (`lodging_escape_tests`, elle çözülen iki katlı otel): 1. katta batıda kaçış merdiveni, 20 m koridor; Oda
  101'in kapısından 7,84 + 0,40 = 8,24 m, Oda 102'ninkinden 18,82 + 0,40 = 19,22 m, tek çıkış; Oda 101'in en uzak
  noktasından ~17,8 m'ye rağmen 32/4 bulgusu yok; 50/4 kaldı (yalnız Oda 102, "> 15,00 m (otel, tek yön)"); Kaçış
  yolları iki odanın yolunu kapılarından çizer (yeşil, kırmızı); doğuya ikinci merdivenle Oda 102'nin yolu 1,53 m, iki
  çıkış, tek yönlü kısım 0: geçti. Kural seti satırında kapsam dışı maddeler 38 (`regulation_dates_tests`).
- **Resimler:** `.run/gui-evidence/yatak-odasi-kacis-8156/`: 1. katın planında kapılardan çizilen kaçış yolları ve
  Denetim'in 50/4 bulgusu, açık ve koyu.

## §6. Kalanlar

- 50/2 dış koridorla erişilen otel odaları ve Ek-5/B'nin notu (dış kaçış geçidinde yağmurlamalı binanın uzaklıkları):
  dış mekân mahalleri kaçış çizgesinde değil.
- Süitin salonu yatak odası adı taşımıyorsa (Salon) iç oda kendi kapısından ölçülür (salondan geçen yürüyüş de sayılır).
- Otelin içindeki konut bölümünün (lojman) yatak odası pakette 50/4'le okunur: olgularda mahalin bölümü yok; Rust onu 48
  ile ölçer.
- 32/4'ün öbür mahalleri, 48 ve Kaçış yolları çizimi Rust'ta (K10: dokunuldukça betiğe; konaklama yatak odaları taşındı).
- Ek-5/B'nin 9.7.2015'ten önceki hâli; 50/6 duman kesicileri.
