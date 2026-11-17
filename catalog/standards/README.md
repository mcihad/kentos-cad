# Standart kural paketleri

KentOS'un ortak Python denetim kuralları (plan/bim-plan.md K10, plan/eklenti.md ADIM 20, ADR 8057 §36). Kural
uygulamaya gömülmez: Rust çizimin olgularını bir kez ölçer (`facts.project`), karar bu betiklerdedir. Masaüstünde ve
bulut işinde aynı betik çalışır; idarenin ek kuralı bir betik ve bir profil katmanıdır, uygulama yeniden derlenmez.

Her paket bir KentOS eklentisidir (`manifest.json`, `kentos.extension` 1): `kentos kek verify <klasör>` doğrular.

| Paket | İçerik |
|---|---|
| `dijital-proje/` | Dijital proje yönetmeliğinin denetimleri: dijital teslim kapsamı (`kapsam`, ADR 8063). Tarih ve eşikler `crates/shared/digital-project/data/kapsam-2026.json`'dadır; paket bir kopyasını taşır, Rust testi ikisini bir tutar |
| `mimari-mevzuat/` | Türkiye imar mevzuatının mimari denetimi, başvuru gününde yürürlükteki hâlleriyle (ADR 8078, 8079): kural seti (`yururluk`, maddelerin hâlleri `data/surumler.json`), asansör (PAİY 34/1 ve 34/8, Sivas), kaçış yolu kapıları (BYKHY 47), emsal sınırlarının koşulları (PAİY 22/1-ç, -f; 44/1), merdiven altı (PAİY 31/6; ADR 8080), yangın kompartımanları (BYKHY 24; ADR 8081), yangına dayanım süreleri (BYKHY Ek-3; ADR 8082), kat yüksekliği ve kapı ölçüleri (PAİY 28/1-c, 39/1, Sivas; ADR 8083), parselin plan notu katmanı (ADR 8084), bölgeye göre kat yükseklikleri (ADR 8085). Masaüstü Denetim ve başsız `cad.arch.audit` çağırır |

- Paketin kuralı KentOS'tan veri alır (kural kataloğu, kararlar); kendi başına çalıştığında (bulut işi, test) kendi
  kopyasını okur.
- Her kuralın bağımsız durumları `fixtures/standards/v1/` altındadır: beklenen sonuçlar kuralın kodundan değil,
  mevzuat metninden elle çıkarılır.
- Sınamalar: `python3 -I catalog/standards/tests/dijital-proje/test_kapsam.py`, `python3 -I catalog/standards/tests/mimari-mevzuat/test_asansor.py` (`test_kapilar.py`, `test_emsal.py`, `test_merdiven_alti.py`, `test_kompartiman.py`, `test_dayanim.py` de).
