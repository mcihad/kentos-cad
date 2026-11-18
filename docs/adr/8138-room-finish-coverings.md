# ADR 8138: Mahal kaplamalarının malzemesi — BYKHY 20/3, 26/2, 26/4

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (Kalan: "mahal kaplamalarının
  malzemesi"), sıra B15; ADR [8091](8091-material-reaction-to-fire.md) §6 ("Duvar ve tavan kaplamaları: mahal
  kaplamasının malzemesi modelde yok").
- **Bağlam:** ADR [8091](8091-material-reaction-to-fire.md) (malzemenin TS EN 13501-1 sınıfı, Ek-2/Ç, kaplama nesneleri
  26/2 ve 26/4), [8038](8038-room-list-and-graphic-joinery-list.md) (mahal listesinin kaplamaları), [8134](8134-roof-covering-fire-class.md)
  (çatı katmanının malzemesi adıyla). Kurallar Python'dadır, Rust olguları verir (bim-plan K10).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 20/3 (16.3.2015-2015/7401 ile değişik, RG 9.7.2015/29411),
  26/2, 26/4, Ek-2/Ç.
- **Sözleşme:** belge ve komut değişmedi (`cad.arch.audit`'in açıklaması kaplama kodlarını sayar: 8091'inkiler de
  eksikti). `kentos.auditFacts` 1'e `building.spaces[].finishes` (kısım, ad, malzeme, sınıfı). **`.kcad`:** değişmedi.
  **Web:** değişmedi; Öznitelikler'in notları TODOS.md §0.2'de.

## §1. Sorun

Mahalin döşeme, duvar ve tavan kaplaması (mahal listesi, 8038) yalnız bir addır ("Seramik", "Saten boya"): yangın
kuralları onun malzemesini ve yanıcılık sınıfını okuyamıyordu. 26/2 ve 26/4 yalnız kaplama nesnelerini (8091: döşeme
kaplaması, asma tavan) okuyordu; duvar iç kaplamasının maddesi (20/3) hiç okunmuyordu.

## §2. Araştırma (18 Kasım 2026)

Yönetmeliğin metni derlenmiş resmî metinden (pdftotext; ADR 8081'in kopyası) okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 20/3 (Değişik: 16/3/2015-2015/7401) | "Duvarlarda iç kaplamalar ile içte uygulanacak ısı ve ses yalıtımları; en az normal alevlenici, yüksek binalarda ve kapasitesi 100 kişiden fazla olan sinema, tiyatro, konferans ve düğün salonu gibi yerlerde ise en az zor alevlenici malzemeden yapılır." | Mahallerin duvar kaplaması (K343); 9.7.2015 öncesi metin kural setinde yok |
| BYKHY 26/2 | "Döşeme kaplamaları en az normal alevlenici, yüksek binalarda ise en az zor alevlenici malzemeden yapılır." | Kaplama nesnelerinin yanında mahallerin döşeme kaplaması (K344) |
| BYKHY 26/4 | "Ayrık nizamda müstakil konutlar dışındaki binaların tavan kaplamaları ve asma tavanlarının malzemesinin en az zor alevlenici olması gerekir." | Asma tavanın yanında mahallerin tavan kaplaması; tek konutlu binada uzman incelemesi (8091) |
| BYKHY 36, 41 | Korunumlu koridor ve kaçış merdiveninde kaplamanın yanıcılığına dair hüküm yok (36/1-b-3 ve 41: kaymayı önleyen kaplama) | 8091 §6'nın "kaçış yolunun kaplamaları" bugünkü metinde yok: okunmaz |
| ADR 8134 | Çatı katmanının malzemesi projenin malzemesine kimliğiyle ya da adıyla (büyük-küçük harf bir) bağlanır | Kaplamanın adı da aynı yolla malzemedir (K342) |

## §3. Karar

- **K342 — Kaplamanın malzemesi adıdır:** mahalin döşeme, duvar ve tavan kaplamasının adı, kimliği ya da adı (büyük-küçük
  harf bir) aynı olan proje malzemesidir; sınıfı o malzemenin beyan edilen TS EN 13501-1 sınıfı (Malzemeler, 8091).
  Belge değişmez: kaplama adı mahal listesinin yazdığıdır. Olgular `spaces[].finishes`'te (`part` floor, wall, ceiling;
  `name`, `material`, `reaction`). Süpürgelik, denizlik, basamak ve korkuluk kaplamaları okunmaz (maddeleri yok).
- **K343 — 20/3 (`kaplamalar.duvar`, `bykhy_wall_lining_reaction`):** mahallerin duvar kaplaması en az normal alevlenici;
  yüksek binada (4/ccc) ve kişi yükü 100'ü aşan, adında sinema, tiyatro, konferans ya da düğün geçen mahalde en az zor
  alevlenici. 100'ü aşan başka adlı mahalde ("gibi yerler") normal alevlenici kaplama uzman incelemesi. İçte uygulanan
  ısı ve ses yalıtımı okunmaz (§6).
- **K344 — 26/2 ve 26/4:** kaplama nesneleri gibi mahallerin döşeme ve tavan kaplamaları da okunur. Aynı adlı kaplama
  (aynı istekle) tek bulgudur, mahalleriyle: "“Duvar kağıdı” duvar kaplaması (2 mahal): malzemesi “Duvar kağıdı” F,
  kolay alevlenici; 20/3 en az normal alevlenici ister". Malzemeler'de o adla malzeme yoksa ya da sınıfı beyan
  edilmemişse eksik veri.
- **K345 — Öznitelikler:** mahalin Döşeme, Duvar ve Tavan kaplaması satırlarının altında not: "Malzemelerden: F.",
  "Malzemelerde; yanıcılık sınıfı beyan edilmemiş." ya da "Malzemeler'de bu adla malzeme yok."; kuralın ölçüsü panelde
  hesaplanmaz.

## §4. Uygulama

- `crates/shared/extensions/src/audit_facts/building.rs` (`SpaceFacts.finishes`, `FinishFacts`);
  `crates/native/processing/src/audit_facts/building.rs` (`finish_facts`).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/kaplamalar.py` (`_ogeler`, `_bulgu`, `duvar`), `data/surumler.json`
  (20/3, 9.7.2015); paket 0.30.0.
- `apps/desktop/src/cad/properties/rows/space.rs` (notlar).

## §5. Sınamalar

- **Python** (`test_kaplamalar.py`, 30 durum; 14'ü yeni, 20/3, 26/2, 26/4 ve Ek-2/Ç'den elle): mahal döşemesi Seramik
  A1fl geçer; malzemesi olmayan Laminat iki mahalde tek eksik veri bulgusu; yüksek binada Dfl-s1 kalır; tavan D-s2,d0
  kalır, A1 geçer; duvar B-s1,d0 geçer, F üç mahalde tek bulguyla kalır, yüksek binada D-s2,d0 kalır; 150 kişilik düğün
  salonunda kalır, spor salonunda uzman incelemesi, 80 kişilik konferans salonunda geçer; sınıfsız malzeme eksik veri;
  1.5.2014 başvurusu 20/3 için uzman incelemesi; üç kısım birlikte geçer. Eski 16 durum 20/3'ün kapsam dışı satırını
  aldı (2008 başvurusu almaz: o gün metin yok, duvar kaplaması da yok).
- **Masaüstü** (`finish_reaction_tests`): dört daireli binada Salon ve Oda'nın duvarına Öznitelikler'den Duvar kağıdı,
  döşemesine Seramik: olgularda adlarıyla, malzemesiz; not "Malzemeler'de bu adla malzeme yok."; Denetim 20/3 ve 26/2
  eksik veri, 20/3'ün tek bulgusu iki mahalle; Malzemeler'de Duvar kağıdı F, Seramik A1fl: notlar "Malzemelerden: F.",
  20/3 kalır, 26/2 geçer; B-s1,d0 ile 20/3 geçer. `regulation_dates_tests`'in satırında 20/3 kapsam dışı (30 madde).
  Ortak süzgeçlerle (modül, şerit, geçmiş, katalog, ayarlar, Öznitelikler, mevzuat, kaplamalar, çatılar) 251 sınama
  yeşil; olguların altını değişmedi (örnek binanın mahallerinde kaplama yok).
- **Resimler:** `.run/gui-evidence/mahal-kaplamasi-8138/` (`finish_reaction_tests::screens`): Salon'un Öznitelikler'i,
  Denetim'de 20/3'ün bulgusu; açık ve koyu.

## §6. Kalanlar

- İçte uygulanan ısı ve ses yalıtımı (20/3): duvar stilinin iç yüzdeki yalıtım bileşeni katman katman okunabilir.
- Kaplamanın adı yerine malzemeye bağlanması (duvar bileşeni gibi `materialId`): mahal listesinin sözleşmesini değiştirir.
- 26/4'ün nizamı: binanın nizamı İmar durumu'na yazılırsa tek konutlu binanın uzman incelemesi kalkar — yapıldı: ADR [8141](8141-building-layout-nizam.md) (Proje ayarları › Mimari › İmar durumu › Yapı nizamı).
