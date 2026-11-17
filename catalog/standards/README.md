# Standart kural paketleri

KentOS'un ortak Python denetim kuralları (plan/bim-plan.md K10, plan/eklenti.md ADIM 20, ADR 8057 §36). Kural
uygulamaya gömülmez: Rust çizimin olgularını bir kez ölçer (`facts.project`), karar bu betiklerdedir. Masaüstünde ve
bulut işinde aynı betik çalışır; idarenin ek kuralı bir betik ve bir profil katmanıdır, uygulama yeniden derlenmez.

Her paket bir KentOS eklentisidir (`manifest.json`, `kentos.extension` 1): `kentos kek verify <klasör>` doğrular.

| Paket | İçerik |
|---|---|
| `dijital-proje/` | Dijital proje yönetmeliğinin denetimleri: dijital teslim kapsamı (`kapsam`, ADR 8063). Tarih ve eşikler `crates/shared/digital-project/data/kapsam-2026.json`'dadır; paket bir kopyasını taşır, Rust testi ikisini bir tutar |
| `mimari-mevzuat/` | Türkiye imar mevzuatının mimari denetimi, başvuru gününde yürürlükteki hâlleriyle (ADR 8078): kural seti (`yururluk`, maddelerin hâlleri `data/surumler.json`), asansör (PAİY 34/1 ve 34/8, Sivas). Masaüstü Denetim ve başsız `cad.arch.audit` çağırır |

- Paketin kuralı KentOS'tan veri alır (kural kataloğu, kararlar); kendi başına çalıştığında (bulut işi, test) kendi
  kopyasını okur.
- Her kuralın bağımsız durumları `fixtures/standards/v1/` altındadır: beklenen sonuçlar kuralın kodundan değil,
  mevzuat metninden elle çıkarılır.
- Sınamalar: `python3 -I catalog/standards/dijital-proje/tests/test_kapsam.py`, `python3 -I catalog/standards/mimari-mevzuat/tests/test_asansor.py`.
