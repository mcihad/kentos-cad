# Dijital proje denetimleri

Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik ile e-PYS Yönetmeliğinin
(RG 5.8.2026/33331) KentOS denetimleri. Kurallar Python'dadır; ölçü KentOS'un (plan/bim-plan.md K10).

## Dijital teslim kapsamı (`kapsam`, ADR 8063)

Bu proje için her yükümlülüğün durumu, başladığı gün ve maddesi:

| Satır | Dayanak |
|---|---|
| Yönetmeliğin kapsamı | Md. 1, 16; Geçici 1/2 |
| 2B çizimler ve belgeler PDF/A | Md. 4/2–3; Geçici 1/1-a; Geçici 1/1-b son cümle |
| BIM modeli IFC | Md. 4/5; Geçici 1/1-b |
| Modelden üretilen çizimler PDF/A | Md. 9/1 |
| e-PYS'de teslim | e-PYS Md. 1, 4/4, 8 |

Durum dörttür: uygulanır, uygulanmaz (maddesiyle), henüz başlamadı (tarihiyle), teyit gerekli (eksik bilgisiyle).

- **Girdiler** ruhsat künyesindendir (Ruhsat takımı penceresinin "Dijital teslim kapsamı" bölümü): başvuru ya da
  ihale tarihi, plan alanı, yapı türü, kullanım, müstakil olup olmadığı, tarım yapısı, il ve belediye nüfusu ile
  kaynağı, İmar Kanunu istisnası, Bakanlığın e-PYS kararı. Toplam yapı inşaat alanı ve bodrum dışındaki kat adedi
  boşsa çizimden ölçülür.
- **Tahmin yok:** eksik ya da okunamayan bir bilgi sonucu değiştirebiliyorsa "teyit gerekli"dir; değiştirmiyorsa sonuç
  yazılır ve eksik bilgi not edilir.
- **Okuma kararları** K23–K28 (ADR 8063): "geçen"/"fazla" kesin büyüktür; "… tarihine kadar uygulanmaz" o günden
  başlar; başvuru ile ihale tarihinin erkeni esastır; konut dışı binada 4/2–3 kendi başına başlamaz; hangi belediyenin
  nüfusu ve nüfusun yılı idareye sorulur; tadilat kapsam dışıdır, ilave teyit ister.

Python'dan:

```python
from dijital_proje import kapsam
sonuc = kapsam.karar({"particulars": {"kullanim": "konut", "il_nufusu": "650000", ...}}, "2026-11-15")
```

## Çizimin ve modelin kuralları

Olgular KentOS'un `cad` bölümündendir (`kentos.auditFacts` 1); kural yalnız karar verir.

| Modül | Ne | Dayanak | ADR |
|---|---|---|---|
| `ek34` | Mahal numarası ve adı, katman, tarama, aks çizgisi, ölçü yazısı, blok adı | Md. 7; EK-3, EK-4 | 8064 |
| `turetim` | Eski kesit ve görünüş, kaynağı silinmiş çizim, model katmanında serbest çizim, kopmuş etiket ve ölçü | Md. 9/1; EK-9 satır 21 | 8065 |
| `paket` | PDF/A, bir dosyada bir pafta, A4 ve katları, EK-2 dosya adı | Md. 4/2, 6, 8 | 8066 |
| `kalite` | Aynı kattaki mahaller örtüşmez (ortak alan 0,01 m²'den küçükse yuvarlama sayılır) | EK-9 Tablo 9.1 satır 13 | 8067 |
| `zorunlu` | Modelin zorunlu alanları: yazılacak modelde sınıfının her nesnesinde değeri olmayan satır, nesneleriyle | Md. 12/1, 13; EK-6, EK-7; EK-9 satır 9 | 8068 |

`mimari.denetle` masaüstünün Denetim sekmesi için `ek34`, `turetim`, `kalite` ve `zorunlu`'yu; `ruhsat.denetle` ruhsat yayınının
kapısı için `turetim` ve `paket`'i birlikte verir.

## Model kalite kontrol formu, yazılmış dosyadan (`form`, ADR 8074)

EK-9 Tablo 9.1'in 21 sorusu KentOS'un yazdığı IFC dosyasının olgularından (`kentos.ifcFacts` 1: nesneler, EK-7 kayıtları,
zorunlu alan eksikleri, set denetimi, EK-5 kodları, kaynak yazıları, gövdeler) ve ev sahibinin eklediklerinden
(`expected`: kaynak modelin nesneleri; `export`: dışa aktarımın bıraktıkları) cevaplanır. Her satır iki şey yazar:
formun resmî cevabı (Evet, Hayır, boş) ve KentOS'un kanıtı (geçti, kaldı, inceleme); inceleme Evet'e çevrilmez.

- `form.cevapla(olgular)`: 21 satır, her biri cevabı, kanıtı, gerekçesi ve bulgularıyla.
- `form.denetle(olgular)`: kalan satırların bulguları (Denetim'in Mevzuat grubu); her bulgu nesnelerini GlobalId'leri ve
  çizimdeki kimlikleriyle, maddeyi, ek satırını ve sayfayı söyler.
- `form.tablo(olgular)`: formun çizimdeki tablosu (Ruhsat takımı'nın FR/MKF paftası).

Dosyanın olguları verilince `mimari.denetle(cizim, dosya)` EK-9'un sorularını dosyadan cevaplar; `kalite` ve `zorunlu`
çizimin olgularıyla yalnız dosya yokken çalışır. Okuma kararları K48 (kullanıcı tanımlı rol), K49 (açıklaması boş ad
incelemedir), K50 (proje tipi yapı türünden).
