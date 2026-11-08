# 2480 — Nesnenin projedeki rolüne göre bağlamsal araçlar

Tarih: 2026-11-08. plan/map-cad.md MAP-01, dördüncü tur. Durum: taslak. Plan satırı: “Harita işinin nesneleri … türüne
göre bağlamsal araçlarla düzenlensin.”

İlgili:
- [ADR 1201](1201-map-work-relative-files-relink-and-contextual-tools.md) §5: seçime göre bağlamsal araçlar (Harita
  işi paneli);
- [ADR 2080](2080-map-work-content-revision-and-approval-stamp.md). “Sonra”sındaki “nesne türüne göre bağlamsal araçlar
  (yüzey, plan alanı, altyapı)” maddesini bu ADR kapatır; tipli katman profili ve onaydan sonra değişen nesneler kalır;
- [ADR 0622](0622-interface-arrangement.md) §4: seçimin bağlamsal sekmesi.

## Dayanak

- **Autodesk Civil 3D, About the Autodesk Civil 3D Ribbon**
  ([belge](https://help.autodesk.com/cloudhelp/2019/ENG/Civil3D-UserGuide/files/GUID-CEC80555-4DB7-4841-B63A-1F1E0743F270.htm),
  8 Kasım 2026'da okundu):
  - bağlamsal sekme bir nesne seçilince ya da nesneye özgü bir komut çağrılınca kendiliğinden açılır;
  - seçili nesneyle ilgili komutları taşır;
  - birden çok türden nesne seçilince “Multiple” sekmesi açılır;
  - seçim kalkınca sekmelerin çoğu kapanır.
- **KentOS bugün:**
  - bağlamsal sekme nesnenin çizim türüne göre açılıyordu (Nokta, Çizgi, Alan …; ADR 0622 §4);
  - Civil 3D'nin yüzeyi, boru ağı, parseli kendi nesneleridir. KentOS'ta bunlar çizim nesneleridir; “türleri”
    katmanlarının projedeki rolüdür: bir yüzeyin kaynak katmanı, bir altyapı ağının boru ya da yapı katmanı, imar
    planı profilinin katmanları;
  - yüzey noktası seçen kullanıcı Pafta eğrileri'ni ya da Yüzey analizi'ni Arazi sekmesinde arıyordu. Pencereler de
    seçimden habersiz, ilk yüzey ya da ilk ağla açılıyordu.

## Karar

1. **Seçimin rolleri** (`selection_roles.rs`). Bir rol, seçili nesnelerin **hepsinde** varsa seçimindir:
   - `surface`: nesne bir arazi yüzeyinin kaynağıdır. Ya yüzeyin noktalarını ya da kırık çizgilerini aldığı bir
     katmandadır (grubun altındakiler dahil), ya da kalıcı kimliğiyle yüzeyin kaynağıdır;
   - `network`: nesne bir altyapı ağının boru (kenar) ya da yapı (düğüm) katmanındadır. Yol ağı sayılmaz;
   - `zoning`: nesne `imar-plani` profilinin plan sınırı, karar alanı ya da plan notu katmanındadır
     (`zoning_check::plan_layers`).

   Roller seçim ya da çizim değişince yeniden hesaplanır. Seçimin hepsini tutan ilk yüzey ve ağ da bilinir.
2. **Rol panelleri** (`ribbon.json`, panelin yeni `roles` alanı). Panel, istediği rol seçimde yoksa görünmez:
   - **Yüzey** (Nokta ve Çizgi sekmeleri; Harita ve Yol; `surface`): Arazi yüzeyleri, Pafta eğrileri, Yüzey analizi;
   - **Altyapı** (Nokta ve Çizgi; Harita ve Yol; `network`): Boru hattı denetimi;
   - **İmar** (Alan ve karışık seçimin Seçim sekmesi; Harita; `zoning`): İmar denetimi, Senaryo ölçüleri.

   Paneller Harita işi panelinden önce durur. Simgeleri komutlarınkidir; yeni simge ve komut yoktur. Dar pencerede
   adlı düğmelerine katlanırlar (başlık her zaman görünür).
3. **Pencereler seçimin kaynağıyla açılır:**
   - Arazi yüzeyleri seçimin yüzeyinin satırını seçer;
   - Pafta eğrileri o yüzeyle açılır;
   - Yüzey analizi onu birinci yüzey yapar, karşılaştırmayı başka bir yüzeyden seçer;
   - Boru hattı denetimi seçimin ağıyla açılır.

   Seçim bir yüzeyde ya da ağda birleşmiyorsa eskisi gibi ilk yüzey ya da ağ.
4. **Kapsam:** CBS projesinde rol paneli yoktur (`modes: ["cad"]`); imar paneli yalnız Harita'dadır.
5. **Kapsam işareti:** **Harita + Yol** (yüzey, altyapı) ve **Harita** (imar).

## Kanıt

- **Masaüstü** (`selection_roles_tests.rs`), ortak durumların çizimleri:
  - Boru hattı denetiminin çizimi: içme suyu hatları seçilince Çizgi sekmesinde Altyapı. Boru hattı denetimi
    “su” ağıyla açılır, ilk ağ olan atıksuyla değil. Bir atıksu ve bir su hattında panel yoktur, pencere ilk ağla
    açılır;
  - Pafta eğrilerinin çizimi: Çukur yüzeyinin noktaları seçilince Nokta sekmesinde Yüzey (Altyapı yok). Pafta
    eğrileri ve Arazi yüzeyleri t-3'le açılır; Yüzey analizi t-3'ü birinci, t-1'i karşılaştırma yapar. Noktayla
    birlikte bir bina seçilince yüzey rolü yoktur;
  - İmar planı denetiminin çizimi: karar alanları seçilince Alan sekmesinde İmar. Plan sınırı ve plan notu karışık
    seçilince Seçim sekmesinde de İmar. Başka bir çizimin binası seçilince İmar yoktur;
  - CBS projesinde rol paneli yoktur.
- **Çapraz denetimler:** şerit, modül, katalog, ayarlar ve etkilenen pencerelerin testleri, 167 geçer. Resimler
  `.run/gui-evidence/baglamsal-roller-2480/`.
- **MCP ve Python:** komut değişmedi. Bu dilim yalnız masaüstünün şeridi ve pencerelerinin açılışıdır.

## Sonra

- Tipli katman profili (MAP-01'in kalanı).
- Onaydan sonra hangi nesnelerin değiştiği (`.kcad` şema 207: onay damgasında nesne başına özet).
- Parsel ve ada katmanları için rol (bugün kadastro katmanını tanıyan bir proje ayarı yok).
- Web'in şeridinde aynı roller (TODOS §0.2).
