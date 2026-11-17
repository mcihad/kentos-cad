# ADR 8108: Yağmurlama zorunluluğu — kullanıma, yüksekliğe ve alana göre (BYKHY 96/2)

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B22 ("Konut dışı kullanımlar", BIM-13),
  ikinci dilim. Her bent için geçen ve kalan bağımsız durum, gerçek çizimde Denetim (§4).
- **Bağlam:** ADR [8107](8107-use-class-escape-articles.md) (kullanım sınıfının okunması), [8035](8035-fire-escape-routes-distances-load-exits.md)
  (projenin yağmurlama beyanı `settings.arch.sprinklers`; Ek-5/B'nin yağmurlamalı sütunları), [8081](8081-fire-compartments.md)
  ve [8082](8082-fire-resistance-of-elements.md) (Ek-4'ün ve Ek-3/C'nin yağmurlama notları), plan K10.
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Komut:** değişmedi. **Kural paketi:** mimari-mevzuat 0.23.0
  (`yagmurlama`). **Olgular:** değişmedi.

## §1. Araştırma (17 Kasım 2026)

Kaynak ADR 8107 §1'deki aynı derlenmiş metin (SHA-256 `d82d06d2…a7d9`), 96 ncı madde. Kural setine yazılan metin
resmî metinle birebir denetlendi.

| Bent | Ne diyor | KentOS kararı |
|---|---|---|
| a (ilk metin) | Yapı yüksekliği 30,50 m'den fazla olan konut haricindeki bütün binalarda | Bütün bölümleri konut olmayan bina konut haricidir (karma da; sıkı olan, BYKHY 18) |
| b (2009) | Yapı yüksekliği 51,50 m'yi geçen konutlarda | Bina programı yalnız konut ya da konut sayısı bölüm sayısına eşit |
| c (2009) | Alanlarının toplamı 600 m²'den büyük kapalı otoparklarda ve 10'dan fazla aracın asansörle alındığı kapalı otoparklarda | Adı Otopark ya da Garaj olan mahallerin alanlarının toplamı; araç asansörü modelde yok |
| ç (ilk metin) | Birden fazla katlı binada yatılan oda sayısı 100'ü veya yatak sayısı 200'ü geçen otel, yurt, pansiyon, misafirhanelerde ve yapı yüksekliği 21,50 m'den fazla bütün yataklı tesislerde | Oda sayısı yatak odası adlı mahallerden; yatak sayısı çizimde yok: odası 100'ü geçmeyen katlı konaklama yapısı eksik veri. Yataklı tesis: konaklama ve sağlık |
| d (2009) | Toplam alanı 2000 m²'nin üzerinde olan katlı mağazalarda, alışveriş, ticaret ve eğlence yerlerinde | Yalnız ticaret ya da eğlence binasında bütün mahaller; karma binada ticaret ve eğlence adlı mahaller ve "katlı" olması onların katlarından |
| e (ilk metin) | Toplam alanı 1000 m²'den fazla, kolay alevlenici ve parlayıcı madde üretilen veya bulundurulan yapılarda | 17'nin saydıkları adından (LPG, akaryakıt, barut, dinamit, mermi, patlayıcı, parlayıcı); maddeyi çizim söylemez: uzman incelemesi |

Bentlerin 2009 öncesi metinleri (b, c, d) kural setinde yok: 9.9.2009'dan önceki başvuru uzman incelemesi.

## §2. Karar

- **K194 — Beyan:** proje yağmurlamayı beyan etmişse (`settings.arch.sprinklers`) 96/2 geçer; satır hangi bendin istediğini
  söyler. Beyan yoksa isteyen her bent bir bulgudur (`bykhy_sprinklers_required`, kuralı `BYKHY 96/2-<bent>`).
- **K195 — Alanlar:** c, d ve e'nin alanları olguların mahal alanlarıdır (`spaces[].area`): yönetmeliğin alanlarının
  ölçtüğü dış kontur (PAİY 4/1-ğ-1, brüt), Mimari denetimin ve Metrekare cetvelinin saydığı. Mahali çizilmemiş yer (dolu
  duvarların arası, şaft) sayılmaz.
- **K196 — Durum:** gerektiren bentlerden biri kaldıysa satır kaldı; hepsi eksik veriyse eksik veri; yalnız e'yse uzman
  incelemesi.

## §3. Uygulama

`catalog/standards/mimari-mevzuat/src/mimari_mevzuat/yagmurlama.py`; kural sırası, eklenti denetimi yüzü (`yagmurlama`),
`manifest.json`, `data/surumler.json` (96/2), 0.23.0, CHANGELOG; `cad.arch.audit`'in açıklaması yeni kodu sayar.

## §4. Sınamalar

- **Bağımsız durumlar** (`fixtures/standards/v1/yagmurlama.json`, 26 durum, `scripts/fixtures/yagmurlama_cases.py`): 33 m
  büro kalır, beyanla geçer; 24 m büro ve 40 m konut kapsam dışı; 54 m konut ve 32 m karma kalır; 650 m² otopark (tek ya
  da iki mahal) kalır, 550 m² kapsam dışı; 110 odalı katlı otel kalır, 40 odalı eksik veri, tek katlı kapsam dışı; 24 m otel
  ve hastane kalır, 15 m hastane kapsam dışı; iki katlı 2500 m² alışveriş merkezi kalır, tek katlı ve 1800 m² kapsam dışı;
  karma binada iki kata yayılan 2400 m² mağaza kalır, tek kattaki kapsam dışı; 1200 m² LPG tesisi uzman incelemesi, 600 m²
  kapsam dışı; 33 m büronun 700 m² otoparkı iki bulgu; 1.6.2008 başvurusu uzman incelemesi; tarihsiz en yeni hâl; iki
  katlı 2200 m² sinema kalır.
- **Gerçek çizimde Denetim** (`use_class_tests::a_33_metre_office_asks_for_sprinklers_until_they_are_declared`): komutlarla
  çizilen on bir katlı büro (33 m) yağmurlamasız kalır, `settings.arch.sprinklers` ile geçer. Konut binasının kural seti
  satırı "24 madde kapsam dışı" (`regulation_dates_tests`).

## §5. Arayüz

Değişmedi: Denetim'in Mevzuat grubunda yeni bulgu, kural setinde yeni madde; yağmurlama beyanı Proje ayarları › Mimari ›
Yangın güvenliği'ndedir.

## §6. Sıradaki adımlar

- Araç asansörlü otopark (c'nin ikinci yarısı), yatak sayısının beyanı (ç).
- Yağmurlamanın kat ya da kompartıman bazında beyanı (bugün bina için tek beyan).
