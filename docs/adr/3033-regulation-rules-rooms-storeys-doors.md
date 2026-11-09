# 3033 — Mevzuat denetimi: piyes, kat ve kapı kuralları

Tarih: 2026-11-09. Durum: kabul (M33, ADV-23; plan S23).

## Sorun

- **Denetim yönetmeliği okumuyordu.** Mimari denetim (ADR 0703, 0732) çizimin kendi sorunlarını buluyordu: kırık bağ,
  boşluk, dar mahal eşiği. Bina programı (ADR 0773) tasarım hedeflerini okuyordu ve "yasal sınır değil" diyordu.
- **Bulgular maddeye bağlı değildi.** Planlı Alanlar İmar Yönetmeliği'nin piyes, koridor, kat yüksekliği ve kapı
  ölçülerini mimar elle denetliyordu. 13.8.2025 değişikliği (Madde 39) kapı genişliklerini net temiz geçişe bağladı;
  eski alışkanlıkla çizilen "90'lık" kapılar artık yetmiyor ve bunu söyleyen yoktu.
- **Plan M33'ün kabulü:** "Kural motoru; piyes, kat ve kapı denetimi | Madde ve ölçüyle bulgu".

## Araştırma (9 Kasım 2026)

- **Planlı Alanlar İmar Yönetmeliği** (mevzuat.gov.tr, `MevzuatMetin/yonetmelik/7.5.23722.pdf`; kesitler
  `.run/gui-evidence/mimari-m33/m33-once-sonra.png`):
  - 28/1-c (12.8.2023 değişikliğiyle): konut bölgelerinde zemin ve normal katlarda kat yüksekliği (döşeme üstünden
    döşeme üstüne) en çok 3,60 m; bodrum hariç 3 katı geçmeyen binalarda 4,00 m.
  - 28/4: iskân edilen katların iç yüksekliği en az 2,60 m. 28/5: ıslak hacim, kiler, geçitlerde 2,20 m.
  - 29/1: her müstakil konutta en az oturma odası (dar kenar 3,00 m, net 12,0 m²), yatak odası (2,50 m, 9,00 m²),
    mutfak (1,50 m, 3,30 m²), banyo (1,50 m, 3,00 m²) ve tuvalet (1,00 m, 1,20 m²).
  - 29/2: üç ve daha az odalı konutta banyo ile tuvalet aynı yerde olabilir. 29/3: hol ve koridor en az 1,20 m.
  - 39/1 (13.8.2025, RG 32985): kapı yüksekliği en az 2,10 m. Net (temiz) genişlik bina girişinde 1,50 m (çift kanatta
    bir kanat 1,00 m), bağımsız bölüm girişinde 1,00 m, diğer mahallerde 0,90 m.
- **W4 §4/4–5** (`docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`):
  - Kural; kaynağı, maddesi, Resmî Gazete tarihi ve sayısı, yürürlüğü ve kapsamıyla tutulur.
  - Katmanlar: ulusal, belediye, plan notu.
  - Piyes asgari tablosu; dar kenar en küçük iç genişliktir.
- **KentOS'ta parçalar hazırdı:**
  - Mahalin net alanı (`SpaceSearch::net_area`, iç yüzler) ve net eni (`program_check::width_of`: içindeki en büyük
    dairenin çapı).
  - Kapının net geçişi (`OpeningSizes::clear`, ADR 0772 §2).
  - Açıklığın iki yanındaki mahal (`opening_codes::sides`, ADR 3028).
  - Bina programının bölüm türleri ve niteliği (ADR 0773, 3031).

## Karar

- **Mevzuat grubu** (`ArchAuditCheck::Regulation`, "Mevzuat"; `regulation_audit.rs`).
  - Doors ve Fixtures gibi yalnız adıyla istenince çalışır; böylece eski çağrıların bulguları değişmez.
  - Denetim paneli bütün grupları adıyla istediği için grup panelde görünür.
- **Kural tablosu:** her kural bir `Article`: madde ve Resmî Gazete ("PAİY 39/1-c · RG 13.8.2025/32985
  değişikliğiyle").
  - Bulgunun yeni `rule` alanı bunu taşır; ajan ve Python bulguyu maddesine göre süzebilir.
  - İletinin sonu kısa adıdır. Değişiklikle gelen maddede değişikliğin tarihi de yazılır ("— PAİY 39/1-c
    (13.8.2025)"): geçen yıl uyan bir çizim yeni kurala takılabilir.
  - Kural seti `RULE_SET`: PAİY'nin 3.7.2017/30113 aslı ve 14.1.2026/33137'ye kadarki değişiklikleri.
- **Piyesler (29/1):**
  - Mahal adı ya da kullanımıyla tanınır (`kind_of`). Islak hacimler önce gelir: "Ebeveyn banyosu" banyodur.
  - Sonra mutfak, hol/koridor/antre, salon/oturma; yatak odası "Yatak", "Ebeveyn", "Çocuk" ya da yalnız "Oda N" adıyla
    tanınır. "Çalışma odası" yatak odası değildir.
  - Net alan iç yüzlerdendir (mahal hangi yöntemle ölçülürse ölçülsün). Dar kenar, net halkanın içindeki en büyük
    dairenin çapıdır; dikdörtgende kısa kenar.
  - Niteliği daire, mesken, konut ya da ev olmayan bölümün mahalleri okunmaz (büro, dükkân). Nitelik verilmemişse
    konut sayılır. Programsız projede mahaller ölçülür.
- **Eksik piyes (29/1, 29/2):** konut bölümünde oturma odası, yatak odası, mutfak, banyo ve tuvalet aranır. Üç ve daha
  az odada (oturma ve yatak odaları) banyo tuvaleti karşılar. Eklentinin mahalleri bölümüne sayılır.
- **Hol ve koridor (29/3):** net en 1,20 m. L biçimli koridorda iç daire köşede biraz büyür (en çok %17): ilk sürümde
  böyle kabul edildi.
- **Kat yüksekliği (28/1-c):**
  - Yalnız proje açıkça konutsa okunur: konut nitelikli bölüm ya da yatak odası olmalı.
  - Sınır 3,60 m; ±0,00 ve üstündeki kat sayısı üçü geçmiyorsa 4,00 m. Yükseklik katın kendi yüksekliği, yoksa üst
    katın kotudur.
  - Bölge türü (ticaret, karma) henüz projede yazılmıyor; ileti "konut bölgesi" diyerek varsayımını söyler.
- **Kapılar (39/1):**
  - Yükseklik üst kot eksi eşiktir. Net genişlik kapının net geçişidir: kanat açıkken kanat yüzünden karşı kasaya.
  - Bir yanında mahal olmayan kapı giriştir: binada ortak alan ya da birden çok bağımsız bölüm varsa bina girişi
    (1,50 m), yoksa tek bölümün girişi (1,00 m). Müstakil evde bina girişi ile bölüm girişi aynı kapıdır; 1,00 m
    seçildi.
  - Bir bölümün mahali ile ortak alan arasındaki kapı bölüm girişidir (1,00 m). Öbürleri 0,90 m.
- **Yeni kodlar** (uyarı): `paiy_room_area_below`, `paiy_room_width_below`, `paiy_room_missing`, `paiy_corridor_narrow`,
  `paiy_storey_height_over`, `paiy_door_height_below`, `paiy_door_width_below`.
- **Yeni ikon `regulation`:** kıvrık köşeli sayfa ve onay işareti (Denetim panelinde grubun simgesi).
- Bulgu yokluğu ruhsat uygunluğu değildir; planın notları ve belediyenin yönetmeliği ayrıca okunur.

## Ölçüm

- **Test** (`regulation_audit::tests`): adlar: Ebeveyn banyosu → banyo, WC → tuvalet, Oda 1 ve Çocuk odası → yatak
  odası, Giriş holü → hol, Depo ve Çalışma odası → yok.
- **Test** (`regulation_audit_tests::the_apartment_s_doors_are_narrower_than_the_regulation_asks`): kabul dairesi.
  - Mahalleri 29/1'e uyuyor.
  - Dört iç kapı "Kapının net genişliği 0.76 m < 0.90 m — PAİY 39/1-c (13.8.2025)".
  - Giriş "Bağımsız bölüm giriş kapısının net genişliği 0.86 m < 1.00 m".
  - Grubun adı verilmeyen çağrıda bulgu yok.
- **Test** (`regulation_audit_tests::a_small_dwelling_s_rooms_corridor_kitchen_and_storey_are_said`): tek katlı
  (4,20 m) konut, programı "Daire".
  - "“1” bölümünde mutfak yok — PAİY 29/1".
  - "Salon net alanı 11.11 m² < 12.00 m²"; yatak odası 8,2… m² < 9 ve dar kenarı 2,3… m < 2,50.
  - "Koridor net eni 0.8… m < 1.20 m — PAİY 29/3".
  - "“Zemin kat” kat yüksekliği 4.20 m > 4.00 m (konut bölgesi, 1 kat) — PAİY 28/1-c (12.8.2023)".
  - Bölüm "Büro" olunca hiç bulgu yok.
- **Gerçek pencere** (1440 × 900):
  - `fixtures/interaction/real/mevzuat-denetimi.txt`, kabul dairesi: Denetim, beş "Mevzuat" bulgusu.
  - `fixtures/interaction/real/mevzuat-konut.txt`, küçük konut: Bina programı bulgularının ardından Mevzuat bulguları.
  - Resim: `.run/gui-evidence/mimari-m33/m33-once-sonra.png`.
- **Fikstürler:** 19 280 komut vakası değişmedi (grup adıyla istenir).

## Sonraki adımlar

- Projede ruhsat başvuru tarihi ve bölge türü (ticaret, karma, konut); kural seti tarihe göre seçilsin.
- İç yükseklik (28/4, 28/5): kat yüksekliği eksi döşeme kalınlığı, ıslak hacimde 2,20 m.
- Çift kanatlı bina girişinde bir kanadın 1,00 m'si (39/1-b); balkon kapısı 0,90 m (39/5); havalandırma penceresi
  (39/6).
- 29/4: birleşik mekânda en az alanların toplamı. Belediye yönetmeliği ve plan notu katmanları; çelişkilerin
  gösterilmesi (M34).
