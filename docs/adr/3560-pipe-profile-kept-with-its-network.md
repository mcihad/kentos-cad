# 3560 — Hat profilinin ağıyla tutulması ve canlı güncellenmesi; `.kcad` şema 212

Tarih: 2026-11-09. plan/map-cad.md MAP-16, altıncı tur. Durum: taslak. ADR 1860, 2360, 2760 ve 3160'ın “Sonra”larındaki
“profilin canlı güncellenmesi”; planın “Plan veya kot değişince profil ve çakışma denetimi güncellensin” maddesi.

İlgili:
- [ADR 1720](1720-map-project-tree-and-rebuild.md): Harita projesinde proje ağacı ve yeniden üretim;
- [ADR 1860](1860-pipe-run-profile.md): hat profili (`cad.pipes.profile`);
- [ADR 2840](2840-drawing-record-version-and-revision.md): çizim kaydında işlem sürümü ve çıktı revizyonu.

## Dayanak

- **Civil 3D 2024 Kullanıcı Kılavuzu**,
  [About Displaying Pipe Networks in Profile Views](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-AD016B9E-790E-45B0-A4EF-992ADD638E0E.htm)
  (9 Kasım 2026'da okundu):
  - ağda planda yapılan değişiklikler (parçayı taşımak, değiştirmek, silmek, boyutunu değiştirmek) profil görünüşünde
    gösterilen parçalara yansır;
  - profil görünüşünde parçanın X, Y yeri değiştirilemez.
- **Bentley Drainage and Utilities CONNECT Edition yardımı**,
  [Profile Runs](https://docs.bentley.com/LiveContent/web/Drainage%20and%20Utilities%20CONNECT%20Edition%20Help-v5/en/GUID-66692669-F6EB-4AE0-BB1B-47C5A1036834.html)
  (9 Kasım 2026'da okundu): profil koşusunun ağ değişince güncellenip güncellenmediğini söylemez.
- **KentOS bugün:**
  - hat profili düz nesneler olarak çizilir (ADR 1860). Boru ya da baca değişince profil eski kalır ve bunu söyleyen yoktu;
  - yol ve Harita çizimleri kaynaklarının damgalarıyla tutulur. Proje ağacı eskiyeni söyler, `road.project.rebuild` ve
    kendiliğinden yeniden üretim aynı girdiyle yeniden çizer (ADR 0911, 1720). Kaynak türleri yol ayarlarındadır, ağ
    yoktu.
- **Bulunan hata** (bu dilimin Python testinde): tutulan bir çizim yeniden üretilip kaydedilip açılınca “Nesneleri elle
  değişti” görünüyordu. Nesnelerin damgası nesnenin belgedeki sayısını (`id`) da içeriyordu. Dosya yeniden okununca sayılar
  sıklaşır (yeniden çizilenler 370'ten başlıyordu, açılınca 258'den). Aynısı çizimden önceki bir nesne silinip kaydedilince
  de olurdu; bütün tutulan çizimlerde (sıfır hattı, kazı ve dolgu bölgeleri, yol çizimleri) vardı.

## Karar

1. **Ağ bir kaynaktır** (`RoadSourceKind::Network`, `network`):
   - damgası ağın tanımıyla kenar ve düğüm katmanlarındaki nesnelerin (çizim sırasıyla) kanonik JSON'undan, öbür
     kaynaklarınki gibi 16 hane. Katmanın süzgeci uygulanmaz: bugün süzgecin dışında kalan nesne yarın içeride olabilir;
   - adı ve kimliği proje ayarlarındaki ağınki (`settings.networks`). Ağın kendi kapanışı yoktur.
2. **Hat profili ağıyla tutulur** (`cad.pipes.profile`, Harita ve Yol projesinde):
   - ana kaynak ağ (`network`), öbürleri zemini okunan yüzey (`ground.surface`) ve kesişen ağ (`crossing.network`);
   - tek geri alma adımı “Hat profili” kaydı da içerir. CBS projesinde profil tutulmaz (yol ayarları orada yazılmaz):
     ağ değişince profil elle yeniden çizilir.
   - Ağın bir nesnesi (boru, baca, öznitelik) ya da tanımı değişince çizim eskir: “Atıksu” ağı çizildiğinden beri değişti.
     `road.project.rebuild` aynı girdiyle yerinde yeniden çizer; kendiliğinden yeniden üretim açıksa değişikliğin adımında.
3. **Proje ağacı:** profil çizilen her ağ bir satırdır (`RoadNodeKind::Network`, kendi durumu hep güncel). Çizimleri onun
   altındadır; ağaçtaki adı “Hat profili · ‹katman›”. İş grafiğinde de ağ bir düğümdür (`RoadGraphKind::Network`).
   Masaüstünde ağ satırı Boru hattı denetimi'ni açar.
4. **`.kcad` şema 212:**
   - `cad.pipes.profile` çizimi ve `network` kaynağı 212'nindir; 211 ve öncesi ikisini tanımaz (`bad_value`);
   - Rust okuyucu ve yazıcı, bağımsız Python okuyucu (`tools/kcad/kcad.py`) ve başvuru yazıcısı birlikte değişti;
   - örnek `map-pipe-profile.json/.kcad`, bozuk `broken/pipe-profile-drawing-in-schema-211.kcad`, “daha yeni” örneği
     artık `schema-version-213.kcad`.
5. **Nesnelerin damgası `id`'siz** (hatanın düzeltmesi): kalıcı kimlik uid'dir. Önceden sayıyla yazılmış kayıt, sayıyla
   hesaplanan damga tutuyorsa elle değişmemiş sayılır; yeniden çizilince sayısız damgayı alır. Biçim değişmez.
6. **Kapsam işareti:** ADR 1860'taki gibi **Harita + Yol + CBS** (tutma Harita ve Yol'da).

## Kanıt

- **Dosya biçimi:**
  - bağımsız yazıcı ile okuyucu 1712 dosyada aynı sonucu verir (`kcad_v2_reference.py --check`);
  - Rust `kentos-kcad` 242 test geçer, geçerli dosya sayısı 243.
- **Ortak durumlar** (`scripts/fixtures/road_project_cases.py`; KentOS kodu yok; profilin nesne sayısı
  `pipes_profile_cases.py`'nin başvurusundan):
  - query (27): Harita projesinde profil ağının altında güncel; P1'in malzemesi yazılınca güncel değil, nedeniyle;
  - rebuild (24): eskiyen profil ağından yeniden üretilir (112 nesne kaldırılıp 112 yazılır), ağaç güncel olur;
  - edit (22): kendiliğinden yeniden üretme açıkken borunun değiştiği adımda profil yeniden çizilir;
  - hat profili, boru denetimi ve sıfır hattı, kazı-dolgu durumları değişmeden geçer. Rust ve Python geçer.
- **Python** (`python/tests/test_pipe_profile_live.py`): çiz, ağaçta ağın altında, boru değişince güncel değil, yeniden
  üret, `.kcad` olarak yazılıp okununca güncel (düzeltmenin denetimi).
- **MCP** (`crates/native/mcp/tests/pipe_profile_live.rs`): aynı akış; kaydedilen dosya yeniden açılınca güncel.
- **Masaüstü** (`road_project_panel_tests::a_pipe_profile_goes_stale_when_its_network_changes_and_is_drawn_again`):
  - Proje ağacında Atıksu ve altında Hat profili;
  - P1 değişince Güncel değil, Hepsini yeniden üret yerinde çizer (112 nesne);
  - proje şema 212 ile kaydedilir.

  Proje ağacı ve İş grafiği testleri, Boru hattı denetimi testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/hat-profili-canli-3560/` (güncel değil ve yeniden üretildi, açık geniş ve koyu dar).

## Sonra

- Boru hattı denetimi'nin kendi sonucunun ağ değişince kendiliğinden yeniden okunması (pencere açıkken).
- Yüzey kaplaması; kesişen borunun dış duvarı (MAP-16'nın öbür maddeleri).
- Web (TODOS §0.2).
