# ADR 8085: Bölge — kat yükseklikleri planın kullanımına göre (PAİY 28/1-a, b, c, ç)

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 ("bölge türü"); ADR
  [8033](8033-regulation-rules-rooms-storeys-doors.md) Sonraki adımlar ("Projede … bölge türü (ticaret, karma, konut)"),
  [8034](8034-regulation-stairs-shafts-elevators-local-rules.md) §Sonraki ("Sivas 28/1-a, b, ç ticaret ve karma bölge kat
  yükseklikleri"), [8083](8083-dated-storey-height-and-doors.md) §6.
- **Bağlam:** ADR [8083](8083-dated-storey-height-and-doors.md) (kat yüksekliği pakette, tarihli), [8084](8084-plan-note-layer.md)
  (plan notu katmanı), [8072](8072-buildings-shared-basement-location.md) §5 (parselin plan kararları: `Kullanim`),
  [8040](8040-vertical-model.md) §3 (asma kat adıyla okunur).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-d (asma kat), 4/1-bbbbb (zemin kat), 4/1-ııı (normal kat), 28/1-a, b,
  c, ç, d (RG 3.7.2017/30113; ç 12.8.2023/32277 ile, d 9.10.2021/31623 ile); Sivas Belediyesi İmar Yönetmeliği 28/1 (RG
  10.11.2022/32009).
- **Web:** dokunulmadı (ADR 0228); TS türleri yeniden üretildi (`ArchSettings`, `Zone`). **`.kcad`:** belge şeması
  değişmedi; mimari verisi sürüm 12 (`settings.arch.zone`). **Sözleşme:** `ArchSettings.zone`; `kentos.auditFacts` 1'in
  `building` bölümüne `zone`, katlara `dwellingRooms` ve `mezzanine`.

## §1. Araştırma (17 Kasım 2026)

Metin bu bilgisayardaki derlenmiş PAİY'den (mevzuat.gov.tr PDF'i, `pdftotext`; ADR 8083 §1'de de kullanıldı), 2017
aslı Lexpera'nın Resmî Gazete metninden, Sivas'ınki Resmî Gazete'nin eski sayısından okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 28/1-a | Ticaret bölgelerinde zemin katlarda 4,50, asma katlı zemin katlarda 5,50, diğer katlarda 4,00 m | `commercial`; değişiklik işareti yok |
| PAİY 28/1-b | Ticaretin de yapılabildiği karma alanlarda zemin 4,50, asma katlı zemin 5,50; diğer katlar konut ise 3,60, konut harici ise 4,00 m | `mixed`; konut mahali taşıyan kat konut katı |
| PAİY 28/1-c | Konut bölgelerinde zemin ve normal katlarda 3,60; 12.8.2023'ten bodrum hariç 3 katı geçmeyen binalarda 4,00 m | `residential` (ADR 8083) |
| PAİY 28/1-ç | Zemin katında ticaret yapılabilen konut bölgelerinde zemin 4,50, asma katlı zemin 5,5 (RG 12.8.2023/32277 "mülga ibare"), diğer katlar 3,60 m | `residentialShops`; 12.8.2023'ten asma katlı zemin kat da 4,50 |
| PAİY 28/1-d (RG 9.10.2021) | Konut bölgelerinde geleneksel mimari öğeli, kesme taş yığma, zemin dahil 3 katı geçmeyen binada 4,50 m | Taşıyıcı sistem beyanı yok: okunmaz (§6) |
| PAİY 4/1-d | Asma kat: zemin katı ticari olarak kullanılmayan konut alanları dışında, zemin katta ait olduğu bağımsız bölümü tamamlayan, ondan bağlantılı, iç yüksekliği 2,40 m'den az olmayan kat | Asma kat zemin katın parçası: "asma katlı zemin kat" |
| PAİY 4/1-bbbbb, ııı | Zemin kat: kat adedine göre en altta, su basman kotunun üzerindeki kat; normal kat: bodrum, zemin, tesisat ve çatı arası dışındakiler | Zemin kat ±0,00 ve üstündeki katların en altı |
| Sivas 28/1 | a) zemin 5,00, asma katlı 6,50, diğer 4,00; b) zemin 5,00, asma katlı 6,50, diğer konut 3,60, konut harici 4,00; c) 3,60; ç) zemin 4,50, asma katlı 5,50, diğer 3,60; d) 4,50 | Yerel katman; zeminin 5,00'ı PAİY'nin 4,50'siyle çelişki |
| `imar-plani` profili, `plan-kullanim` | Kararın kullanımı: konut, ticaret, ticaret-konut, sanayi, eğitim, sağlık … | Bölgenin seçenekleri; karar Parselin plan kararları'nda gösterilir, alana yazılmaz (ADR 8072 §5) |

## §2. Karar

- **K93 — Bölge projenin verisidir** (`ArchSettings.zone`, `Zone`): Konut alanı (c), Zemin katında ticaret yapılabilen
  konut alanı (ç), Ticaret + konut (karma) alanı (b), Ticaret alanı (a), Öbür kullanım (28/1 kat yüksekliği vermez).
  İmar durumu'nda seçilir; CBS'in kararı yazmaz (bir kesişim hukuki karar değildir).
- **K94 — Bölgenin bendi okunur:** zemin katta zemin ya da asma katlı zemin değeri, öbür katlarda diğer katların değeri;
  karma alanda konut mahali (bağımsız bölümü konut olan) taşıyan kat konut katıdır. c bendi ADR 8083'teki gibi.
- **K95 — Asma kat zemin katın parçasıdır:** adı Asma kat olan kat (`mezzanine`, 28/4'ün okuduğu gibi) ayrı kat sayılmaz;
  üstünde asma kat olan zemin kat asma katlı zemin kattır, yüksekliği asma katın üstündeki ilk kata kadardır (yoksa
  kendisinin ve asma katlarının yükseklikleri toplamı). Bulgu zemin katın ve asma katın mahallerini seçer.
- **K96 — Bölge yazılmamışsa:** konut projesinde konut alanı sayılır (ADR 8033'ün varsayımı; satırın notu söyler);
  konut projesi olmayan binada sınır planın kullanımına bağlıdır: tek bir "eksik veri" bulgusu ("Bölge yazılmamış …") ve
  "PAİY 28/1" satırı eksik veri. Plan notunun bağladığı katlar yine okunur (ADR 8084).
- **K97 — Öbür kullanım:** 28/1 sınır vermez; "PAİY 28/1" kapsam dışı. Plan notu varsa o bağlar.
- **Bölge bir projenin bütün binalarınındır** (tek parsel); birden çok bloklu parselde blok başına bölge sıradaki iş.

## §3. Uygulama

- **Sözleşme:** `Zone` (`ALL`, `label`, `name`), `ArchSettings::has_zone`; `cad.settings.set` belgesi ve kataloğu.
- **Dosya:** `kentos_kcad::ARCH_WITH_ZONE`, `uses_zone`, `arch_version`; okuyucunun reddi.
- **Olgular:** `BuildingFacts.zone`, `StoreyHolds.dwelling_rooms` (bağımsız bölümü konut olan mahaller),
  `StoreyHolds.mezzanine` (adında ASMA); `fixtures/audit-facts/v1/mimari.json` yeniden yazıldı.
- **Paket** (`tr.kentos.mimari-mevzuat` 0.9.0): `kat_yuksekligi` (`BOLGE_MADDESI`, `zemin_yuksekligi`, `sinir`,
  `nitelik`, `parcalar`); `data/surumler.json`'a PAİY 28/1-a, b, ç (ç'nin 2017 ve 2023 hâlleri) ve Sivas 28/1-a, b, ç.
- **Masaüstü:** İmar durumu'nda "Bölge" seçimi (Yerel imar yönetmeliği'nin altında) ve açıklaması.

## §4. Sınamalar

- **Sözleşme:** `the_zone_is_written_by_its_name`. **Dosya:** `the_zone_round_trips_under_arch_data_12`.
- **Paket** (`fixtures/standards/v1/kat-yuksekligi.json`, 12 yeni durum, metinlerden elle): ticaret alanında zemin 4,80
  kalır, 4,00 tutar, 4,20 kalır; asma katlı zemin kat 5,50 tutar, 6,00 kalır (mahalleri asma katınkilerle); karma alanda
  konut katı 3,60'a kalır, konut dışı kat 4,00'ı tutar; zemin katı ticaretli konut alanında asma katlı zemin 5,20 m 2022'de
  geçer, 2026'da 4,50'ye kalır, tarihsiz eksik veri; Sivas ticaret zemininde çelişki; konut alanı yazılı; konut alanındaki
  büro binası 28/1-c'den okunur; öbür kullanım kapsam dışı, plan notuyla bağlanır. 8083 ve 8084'ün 4 durumunun beklentisi
  değişti (büro ve programsız yatak odasız çizim artık "eksik veri"; notsuz katlı büro).
- **Masaüstü** (`zone_tests`, `dated_sizes_tests`'in evi büro, zemin 4,80 m, 1. kat 4,00 m, 2.7.2026): bölge yokken eksik
  veri; Ticaret alanı seçilip kaydedilince zemin kat 28/1-a'nın 4,50'sinden kalır, 1. kat 4,00'ı tutar; Sivas'la çelişki.
  `regulation_audit_tests`'in büro kısmı bölgesiz eksik veriyi söyler.
- **Resimler:** `.run/gui-evidence/bolge-8085/` (İmar durumu'nda Bölge ve Denetim; açık ve koyu).

## §5. Arayüz

- Proje ayarları › Mimari › İmar durumu: "Bölge" seçimi (Yazılmamış — konut projesinde konut alanı sayılır; Konut alanı;
  Zemin katında ticaret yapılabilen konut alanı; Ticaret + konut (karma) alanı; Ticaret alanı; Öbür kullanım).
- Denetim: "“Zemin kat” kat yüksekliği 4.80 m > 4.50 m (ticaret alanı, zemin kat) — PAİY 28/1-a (1.10.2017)".

## §6. Sıradaki adımlar

- **Blok başına bölge** (birden çok bloklu parsel, ADR 8072); **CBS kararından öneri:** parselin tek kararının
  kullanımı bölgeyi önersin, onayla yazılsın.
- **28/1-d** (geleneksel kesme taş yığma, 4,50 m) taşıyıcı sistem beyanıyla; **28/2** (planın bina yüksekliği kat
  yüksekliğiyle verilmemişse) ve **28/3** (tesisat katı).
- **Asma katın kendi koşulları** (4/1-d: bölümün 1/3'ü, iç yükseklik 2,40, cepheye 3,00 m; konut alanında asma kat).
