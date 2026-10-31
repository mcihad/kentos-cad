# ADR 0795: Bölüm çizelgesi: bina programının bölümleri satır satır

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-04'ün üçüncü teslimi (ADV-04c) **[~]**.
- **Bağlam:** ADR 0784'ün “Kapsam dışı”ı: bağımsız bölüm çizelgesi türü (`of: units`); o güne dek bölüm toplamı mahal
  çizelgesinin Bölüm ile gruplanmasıyla okunuyordu (her mahalde bölümün payı tekrar eder, toplanmaz). Çizelgeler ADR 0344
  §3, 0733 (başlık, ara toplam, Dosyaya yaz, `cad.schedules.query`); bina programı ADR 0773, pay ADR 0784
  (`kentos.share/1`).
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'ye yazılacak.
- **Şema:** `.kcad` 151 (opt-in; dalda geçici 159'du, birleşmede 151 oldu), `FORMATS_VERSION`
  değişmez (dosya alışverişi değişmedi).

## Araştırma (üreticilerin resmî belgeleri; yerelde çalıştırılmadı)

- [Revit — Area Schemes](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-194A8E37-06C5-426D-964C-D9148A1FEFE3.htm),
  [Use Area Schemes](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-Model/files/GUID-828E8162-206D-4FB3-9866-30257CAE726F.htm),
  [Create an Area Plan](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-Model/files/GUID-D237E6DA-CC5B-4BBE-86E1-3C8AC28CF8AF.htm),
  API [Creating a Schedule](https://help.autodesk.com/cloudhelp/2016/ENU/Revit-API/files/GUID-7DB8EEA6-2C4A-4BC3-A783-1C85B6776D53.htm)
  (`ViewSchedule.CreateSchedule`'un alan şemalı biçimi): alan çizelgesi bir alan şemasınındır; satırı bir **Area**
  öğesidir (oda değil), alanı alan planındaki sınırdan; başka şemanın çizelgesine kopyalanmaz. Satır birden çok odayı
  toplamaz; bir dairenin odalarını toplamak için oda çizelgesi daire parametresiyle gruplanır.
- [Archicad — About Schedules](https://help.graphisoft.com/AC/20/INT/AC20Help/03_2_Views_Virtual_Building/03_2_Views_Virtual_Building-95.htm),
  [Scheme Settings Dialog Box (Interactive Schedule)](https://help.graphisoft.com/AC/24/INT/_AC24_Help/130_UserInterfaceDialogBoxes/130_UserInterfaceDialogBoxes-68.htm):
  Interactive Schedule bölgeleri (Zone) listeler ve **düzenlenebilir**: Scheme Settings'te düzenlenebilen alanlar mavi,
  hesaplananlar siyah gösterilir; düzenleme öğenin kendisine yazılır. Bölgenin üstünde bir “daire” öğesi yoktur; daire
  toplamı bölgelerin bir özelliğe göre gruplanmasıdır.
- Sonuç: iki üründe de daire (bağımsız bölüm) bir satır değil, gruplamanın sonucudur; ortak alan payı, kayıtlı alan ve
  tasarım hedefi daire düzeyinde okunmaz. KentOS'ta bina programının bölümü zaten bir kayıttır (ADR 0773 §1): bölüm
  çizelgesinin satırı **bölümdür**, sayıları mahallerden ve ADR 0784'ün payından hesaplanır; Archicad'deki gibi
  düzenlenen ile hesaplanan sütunlar ayrı görünür (hesaplanan gri), düzenlenen hücre nesneye değil **programa** yazılır.

## Karar

### §1 Tür ve satırlar

- `ScheduleOf::Units` (`of: "units"`, “Bölüm çizelgesi”): bina programının grupları (bağımsız bölüm, ortak alan, dış
  alan) **programın sırasıyla**, her biri bir satır. Satırın `members`'ı grubun mahalleri (çizimin sırasıyla), yanında
  belgenin yazdığı `units` satırın gruplarının kimlikleri (gruplu satırda birkaç, başlık ve ara toplamda boş).
  Mahali olmayan bölüm de veri satırıdır (`members` boş, `units` dolu); bu yüzden bölüm çizelgesinde veri satırını
  `units` söyler.
- Sütunlar (`ScheduleField::reads`): No, Ad, Bölüm türü (`unitKind`), Bina (`building`), Kanat (`wing`), Kat (mahallerin
  durduğu katlar, projenin sırasıyla “, ” ile), Alan (odaları: şaft ve dış mahal dışı, `Document::program_rooms` ile
  aynı toplam; mahalli ama odasız grupta 0, mahalsiz grupta boş), Kayıtlı alan (`recordedArea`), Pay ağırlığı
  (`shareWeight`, yazıldığı gibi), Pay ve Brüt (`kentos.share/1`, yalnız bağımsız bölümün; kural yoksa boş), Tasarım
  hedefi ve Hedef farkı, Adet (başlığı “Mahal sayısı”: grubun bütün mahalleri). Öbür alanlar (evre, öznitelik, kullanım …)
  bölüm çizelgesinde okunmaz (`invalid_schedule`), yeni beş alan başka çizelgede okunmaz.
- **Hedef:** bölümün hedefli odalarının (boşluk ve dış mahal değil, hedefi alan aralığı veren) aralıklarının toplamı;
  bir odanın alt ya da üst sınırı yoksa o yan açıktır (“≥ 32.00 m²”). Fark: bu odaların alanlarının toplamının o aralığın
  dışında kalan kadarı (ADR 0773 §2'nin kuralı, toplam üstünde). Hedefli odası olmayan bölümde ikisi de boş.
- Varsayılan sütunlar: No, Ad, Bölüm türü, Bina, Kat, Alan, Pay, Brüt, Kayıtlı alan. Toplanan sütunlar: Adet, Alan ve bölüm
  çizelgesinde Pay, Brüt, Kayıtlı alan (`ScheduleField::summed_in`; mahal çizelgesinde bölümün payı her mahalde tekrar
  ettiği için toplanmaz, ADR 0784 §3).
- Sıralama, “aynı satırları birleştir”, grup başlığı, ara toplam ve genel toplam bütün çizelgelerin kuralıyla
  (ADR 0733 §4); sıralanmamış çizelge programın sırasındadır.

### §2 Süzgeç

- Bölüm çizelgesinin süzgeci: `levels` (bir mahali o katta olan grup), `buildings`, `wings` (programın kimlikleri) ve
  `kinds` (`unit`, `common`, `outdoor`). Bölüm çizelgesinde `phases`, `forms`, `layers`, `types`; başka çizelgede
  `buildings`, `wings`, `kinds` `invalid_schedule`. Programda olmayan bina ya da kanat **`building_not_found`** (yeni kod;
  `level_not_found`'dan sonra, `filter.buildings[i]` / `filter.wings[i]`).

### §3 Hücre programa yazılır

- Bölüm çizelgesinde düzenlenen sütunlar No, Ad, Kayıtlı alan ve Pay ağırlığıdır (`ScheduleField::edits_in`); öbürleri
  hesaplanır (`cell_computed`). `cad.schedules.edit`'in `cell`'i satırın gruplarını programda değiştirir ve programı
  `cad.program.edit`'in yazdığı yoldan (`Document::set_program`) tek adım “Çizelgeyi düzenle” olarak yazar; çizelge,
  etiketler ve öbür çizelgeler aynı adımda izler. Cevabın `units`'i yazılan grupların kimlikleridir.
- Kayıtlı alan projenin alan biriminde yazılır (nokta ya da virgül; m²'ye çevrilir), boş yazı kaldırır; pay ağırlığı
  virgül noktaya çevrilerek yazıldığı gibi; ad boşsa kalkar; numara boş olamaz. Programın kendi kuralını kıran değer
  (aynı numara iki bölümde, sıfır kayıtlı alan, okunamayan ağırlık) programın sözüyle **`invalid_cell_value`**.
- Gruplu satır birkaç bölümse ad, kayıtlı alan ve ağırlık hepsine yazılır; numaralar ayrı olduğundan numara sütunlu
  satır hep tek bölümdür, birkaç bölümlü satıra numara yazılmaz (`schedule_group_number`). Başlık ve ara toplam satırı
  `schedule_summary_row`.
- Programın değişmesi bir ayar değişikliğidir: katman kilidi sorulmaz (nesneye yazılmıyor).

### §4 Çizelge izler

- Bölüm çizelgesi bir mahal eklendiğinde, değiştiğinde ya da silindiğinde, programın (`Op::BuildingProgram`) ve proje
  ayarının (`Op::TemplateSettings`: katlar) değiştiği adımda yeniden yazılır (`table_cells::reads_settings`). Aynı kuralla
  program alanlı (bölüm, hedef, pay, brüt) mahal çizelgesi de artık programın değiştiği adımda izler (önceden yalnız bir
  mahal değişince).

### §5 Sorgu

- `cad.schedules.query`: bölüm çizelgesinde başlığı “Mahal sayısı”, toplanan ve düzenlenen sütunlar bu ADR'nin kuralıyla;
  veri satırının `units`'i (grupların kimlikleri) `members`'ın yanında; mahali olmayan bölümün satırı da `data`.

### §6 Arayüz

- Mimari › Belgele › Çizelge ▾'de **Bölüm çizelgesi** (`tool.scheduleUnits`, ikon `scheduleUnits`, takma adlar
  BOLUMCIZELGESI, UNITSCHEDULE): imleçten yerleştirir, Tür, Kat, Grup, Genel toplam, Ara toplam, Sırala görünür
  ayarlardır; sıralama varsayılanı kapalıdır (programın sırası). Grupta Bölüm türü, Bina, Mahal sayısı, Alan, Pay, Brüt.
- Çizelge ayarları: bölüm çizelgesinde süzgeç Kat, Bina, Kanat ve Bölüm türü; sütunlar “programa yazılır” ya da
  “hesaplanır” der. Tabloyu düzenle bölüm çizelgesinin hücresinde “Bina programına yazılır: 2 Daire 2 …” der;
  kaynağı “Bölüm çizelgesi: n satır, bina programından”.

### §7 Şema 151

- Yazıcı 151'i yalnız bir çizelgede `of: units`, beş yeni alandan biri, süzgecin `buildings`/`wings`/`kinds`'ı ya da
  satırların `units`'i varken yazar (`schedule_units`). Önceki şemada bunlar `unknown_field`; bölüm çizelgesi olmayanda ya
  da `members`'tan başka uzunlukta `units`, bölüm çizelgesinde evre süzgeci `bad_value`. Örnek
  `fixtures/kcad/v2/unit-schedules.{json,kcad}`; bozuklar `unit-schedules-in-schema-148` (`unknown_field`),
  `-in-schema-158` (henüz şema değil, `schema_version`), `-on-spaces`, `-units-short`, `-phase-filter` (`bad_value`);
  `schema-version-160`.

## Kapsam dışı (açık)

- Resmî alan profili ve arsa payı (ADR 0784 §4): kayıtlı alanın kuralı yine söylenmez.
- Bölümün dış alanı (balkon) ayrı sütun olarak; şimdilik Alan yalnız odaları toplar.
- Grup türünü (bağımsız bölüm ↔ ortak alan) ve binayı/kanadı hücreden değiştirmek: pay ve bina bağı etkilendiğinden Bina
  programı penceresinde kalır.
- Çok sayıda bölüm çizelgesinde her çizelge payı bir kez hesaplar; ölçülmedi. Gerçek pencerede el ile kabul.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/unit_schedule_cases.py` (KentOS kodu olmadan; plan ve paylar
  `program_share_cases.py`'nin kesirleriyle): `cad.schedules.create.units.json` 25, `cad.schedules.edit.units.json` 25,
  `cad.schedules.query.units.json` 20 ortak durum (satırlar, alanlar, katlar, hedef ve açık yanı, ağırlıkla pay, kuralsız
  pay, mahalsiz bölüm, dört süzgeç, sıra, grup, başlık ve ara toplam, plan ve doğrulama; hücrenin programa yazılması,
  geri alma, ağırlıkla payın aynı adımda değişmesi, gruplu satır, programın ve mahallerin izlenmesi, retler). Rust ve
  Python koşucuları geçer.
- KCAD 151: Rust kodeği ve `tools/kcad/kcad.py` aynı sonucu verir (`kcad_v2_reference.py --check`).
- Masaüstü kabul testi `program_scenes::tests::the_units_schedule_reads_the_program_and_writes_back_to_it`; resimler
  `arac-bolum-cizelgesi-{plan,arac,ayarlar,duzenle}` (1440×900, 1100×650; koyu ve açık).

## Belgelere eklenecek satırlar

- **CLAUDE.md §0** (ADR 0784'ün maddesinden sonra):
  - **Bölüm çizelgesi (31 Ekim 2026; [ADR 0795](docs/adr/0795-unit-schedule.md), ADV-04c [~]).** Çizelgenin
    `of: units`'i bina programının bölümlerini programın sırasıyla satır satır yazar (`members` mahalleri, `units`
    grupları; mahalsiz bölüm de satır): No, Ad, Bölüm türü, Bina, Kanat, Kat, Alan (odalar bir kez), Kayıtlı alan, Pay
    ağırlığı, Pay ve Brüt (`kentos.share/1`), hedefli odaların toplam hedefi ve farkı, Mahal sayısı; süzgeç kat, bina, kanat,
    tür (`building_not_found`); Pay, Brüt, Kayıtlı alan toplanır. No, Ad, Kayıtlı alan, Pay ağırlığı hücresi
    `cad.schedules.edit` ile programa yazılır (programın kuralı `invalid_cell_value`), çizelge program ve mahallerle aynı
    adımda izler (program alanlı mahal çizelgesi de). Mimari › Çizelge › Bölüm çizelgesi, Çizelge ayarları'nda bölüm
    süzgeci. `.kcad` şema 151 (opt-in). 25 + 25 + 20 ortak durum (`unit_schedule_cases.py`).
- **CLAUDE.md §2** (çalıştırma listesi):
  `uv run --no-project --with shapely python3 scripts/fixtures/unit_schedule_cases.py --check; KENTOS_CASES_ONLY=cad.schedules cargo test -p kentos-native-application --test all every_command_case; cargo test -p kentos-desktop --lib the_units_schedule_reads   # bölüm çizelgesinin 25 + 25 + 20 ortak durumu KentOS kodu olmadan ve masaüstü kabulü; resimler KENTOS_SHOTS_ONLY=bolum-cizelgesi-plan,bolum-cizelgesi-arac,bolum-cizelgesi-ayarlar,bolum-cizelgesi-duzenle cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture (ADR 0795)`
- **plan/architecture-advanced.md ADV-04 durum satırı:** “… tam payı … yapıldı; bölüm çizelgesi (`of: units`, hücresi
  programa yazılır, ADR 0795) yapıldı. Açık: resmî alan profili ve arsa payı, gerçek pencerede elle kabul.”
- **TODOS.md §0.2 (Web'e ertelenenler):** ADR 0795 — web'in çizelge aracında Bölüm çizelgesi, `ScheduleOf::Units`'in
  satırları ve hücresinin programa yazılması (`cad.schedules.edit`), Çizelge ayarları'nda bina/kanat/tür süzgeci.
