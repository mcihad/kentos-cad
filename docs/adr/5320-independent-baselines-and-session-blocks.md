# 5320 — Oturumun bağımsız bazları (n − 1) ve pencerede oturumun blokları

Tarih: 2026-11-12. plan/map-cad.md MAP-03, on birinci tur. Durum: taslak. MAP-03'ün sıradaki işi (ADR 4920'nin “Sonra”sı):
bağımsız baz önerisi (oturum başına n − 1); oturumun bloklarının pencerede gösterilmesi.

İlgili:
- [ADR 4920](4920-gnss-sessions-new-point-velocity-kept-limits.md): GVX oturumları `bo-N`, oturum korelasyonlarıyla
  dengeleme;
- [ADR 1740](1740-gnss-network-from-archive.md): bazlar, dengelemeye girme seçimi (`adjust`).

## Dayanak

12 Kasım 2026'da okundu; not `.run/research/map-cad/uygulama/MAP-03.11.md`.

- **BÖHHBÜY:**
  - C1 ağı en çok 30 km'lik bağımsız bazlardan;
  - C2 ve C3 noktaları en az iki bağımsız bazla.

  Bağımsız bazın seçimi yazılmaz.
- **Trimble Business Center yardımı:**
  - ağ dengelemesi bağımsız vektörlere dayanır;
  - n alıcının eş zamanlı oturumunda bağımsız baz n − 1, olası baz n(n − 1)/2;
  - bağımlılar devre dışı bırakılır.
- **Penn State GEOG 862:** aynı oturumun bazları aynı gözlemlerdendir, yalnız onlardan kurulan halka sıfır kapanır. En
  kısaların seçilmesi yaygındır, kesin kural değildir.

## Karar

1. **Bağımsız ve bağımlı** (`map_survey_sessions::independent`):
   - bir baz oturumunun arşivde olan, dışlanmamış bazları uzunluğa göre sıralanır (eşitse kimliğin numarası küçük
     olan önce);
   - işaretleri henüz birbirine bağlanmamış iki işareti bağlayan baz bağımsızdır (Kruskal'ın en kısa yayılan ağacı); n
     işaretli bağlı oturumda n − 1;
   - gerisi bağımlıdır;
   - işaret adları yazıldığı gibi karşılaştırılır.
2. **`map.survey.query`:** oturumdaki dışlanmamış bazın satırında `independent` (true / false). Dışlanan baz ve oturumu
   olmayan baz için alan yok.
3. **`map.survey.edit` `independent`** (`session` verilmezse bütün oturumlar; tek geri alma adımı “Bağımsız bazlar”):
   - bağımsızlar dengelemeye girer, bağımlılar girmez; dışlananlara dokunulmaz;
   - **retler:** değişen yoksa `nothing_to_set`; bilinmeyen oturum `unknown_survey_record` (`operation.session`); arşivde
     oturum yoksa `unknown_survey_record` (`operation`).
4. **Masaüstü** (GNSS ağ dengelemesi; yeni şerit düğmesi yok):
   - **Bazlar:** **Bağımsız** sütunu (Bağımsız / Bağımlı).
   - **Oturumlar** (yeni sekme): her bloğun oturumu, adı, satır ve sütun bazı, dokuz korelasyon satır satır, en büyük
     |ρ|.
   - **Bağımsız bazları seç:** işlemi bütün oturumlara uygular ve ağı yeniden çözer.
5. **Kapsam:** Harita + CBS (ADR 4920 gibi).

## Sınırlar

- **Seçim kuralı:** en kısa yayılan ağaçtır. İşleme istatistiği (ratio, RMS) ve ortak uydu sayısıyla seçim yok; kullanıcı
  `adjust` ile başka seçebilir.
- **Oturumsuz bazlar:** eş zamanlılığı zamanlarından çıkarılmaz; yalnız GVX oturumları.
- **Yönetmelik denetimi:** “en az iki bağımsız baz” (C2/C3) ve 30/15 km sınırı denetlenmez.
- **Kabul:** gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Kanıt

- **Bağımsız başvuru** (`map_survey_cases.py`, `map_survey_network_cases.py`; QGIS'in Python'uyla, KentOS kodu yok):
  - kural betikte ayrıca yazıldı (`independent_of`);
  - üç alıcılı oturum (bz-1, bz-5, bz-3 üçgen; en uzun bz-3 bağımlı);
  - edit 138'den 144'e: bir oturum, bütün oturumlar, değişecek bir şey yok, bilinmeyen oturum, oturumsuz arşiv, sorgu
    satırları.
  - Bozma denemesi: bir bazın `independent`'i ve bir `adjust` değiştirilince koşucu tam o ikisini yakalar.
- **Python** (`python/tests/test_independent_baselines.py`): testin kendi kuralı ve bağımsız `.kcad` okuyucusuyla;
  Python ortak durumları geçer.
- **MCP** (`crates/native/mcp/tests/independent_baselines.rs`).
- **Masaüstü** (`independent_baselines_tests`): bağımlı baz tek adımda çıkar, geri alınır. Çapraz denetimler 152 test.
  Resimler `.run/gui-evidence/bagimsiz-baz-5320/`.

## Sonra

- İşleme istatistiğiyle (ratio, RMS, ortak uydu) ağırlıklı seçim.
- Oturumsuz bazların eş zamanlılığının gözlem zamanlarından bulunması.
- C2/C3 “en az iki bağımsız baz” ve uzunluk sınırının denetimi.
- Canlı RTK/NTRIP ve gerçek alıcı kabulü (sahip/cihaz).
- Web (TODOS §0.2).
