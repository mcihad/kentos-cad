# ADR 8171: Mimari'nin 3B görünümü, kesiti ve eksik araçları

- **Durum:** kabul edildi (2026-11-27). Plan: [plan/mimari-bim.md](../../plan/mimari-bim.md). Araştırma:
  [docs/research/2026-11-27-mimari-3b-kesit-ve-araclar.md](../research/2026-11-27-mimari-3b-kesit-ve-araclar.md).
- **İstek** (sahip, 27 Kasım 2026):
  - "3d bina çizimlerde ciddi problemler var … kolon kirişler düzgün render edilmiyorlar. Kesit esnasında ciddi
    bozulmalar oluyor";
  - "Kesit değiştirirken ve döndürürken üçgenlerde ve mesh yapılarda titremeler ve kayıplar oluyor";
  - mimari projede eksik araçlar; piyasadaki mimari çizim ve BIM araçlarının incelenmesi.
- **Değiştirdikleri:** ADR 8131 K310 (kesit sürüklenirken kapak yok), ADR 8041 §1'in kirişinin ve kolonunun 3B ile IFC
  gövdesi. Kirişin ve kolonun ölçüleri ve kotları değişmez.

## §1 Bulgu

Araştırma raporunun §3'ü. Özetle:

- **B1:** Kolon, kiriş ve döşeme aynı betonu tutuyor. Aynı kottaki yüzler derinlikte yarışıyor (titreme). IFC hacimleri
  birleşimleri iki ya da üç kez sayıyor.
- **B2:** Kesit sürüklenirken kapak bilerek kaldırılıyor; gövdeler içi boş görünüyor.
- **B3:** Kapak halkası 1e-7 m ile zincirleniyor; float32 gövdede kopup kapak hiç çizilmeyebiliyor.
- **B4:** Kenar çizgisi köşeleri bitleri birebir aynıysa birleştiriyor; düz yüzün köşegeni çiziliyor.
- **B5:** Her değişiklikte bütün sahne yeni kimliklerle yeniden yükleniyor; eski parçalar ekran kartında kalıyor.
- **B6:** Keskin dönen kirişin gönyesi bandı kendini kestiriyor; 3B'de uç kayboluyor.

## §2 Karar

### §2.1 Birleşim kuralı (MB-01)

Kesit modeli gövdeleri **birleşim önceliğiyle** ayırır. Hiçbir iki gövde aynı hacmi tutmaz:

1. **Kolon kirişi keser.** Kirişin bandı aynı kattaki ve düşeyde çakışan kolonların ayak izinde kesilir. Revit'in
   betonarme birleşim tablosu da kolonu baskın sayar.
2. **Kolon ve kiriş döşemeyi keser.** Döşemenin betonarme katmanı, tepesi o katmanın içine uzanan kolonun ve kirişin
   ayak izinde açılır. Bu, ADR 8041'in kuralını korur: kolonun ve kirişin tepesi döşemenin betonarme üstüdür; kiriş
   yüksekliği döşemeyi içine alır (TBDY 2018 7.4.1.1). Döşemenin üstündeki şap, yalıtım ve kaplama katmanları kesilmez.
3. **Kiriş–kiriş:** önce çizilen baskındır (Revit'in kuralı). Sonra çizilenin bandı öbürünün bandında kesilir.

**Kapsam:** Kesit modelini kullanan her şey aynı gövdeyi kullanır: 3B görünüm, IFC'nin iki profili, IFC'nin miktar
setleri ve 2B kesit.

**2B kesit:** Aynı malzemeden bitişik kesilen parçalar zaten tek dolgu ve aralarında çizgi yok (ADR 8041 §2,
TS EN ISO 128-3). Birleşim kesitte yeni çizgi doğurmamalı; altın sınamalar bunu denetler.

**Metraj:** Kolonun ve kirişin IFC hacmi birleşimdeki betonu içerir, döşemeninki içermez. Bağlayıcı bir Türk metraj
metni bu turda bulunamadı (rapor §6); toplam beton hacmi kesindir ve iki kez sayılmaz.

### §2.2 Kesit yüzü (MB-02)

- Kapak sürüklenirken de çizilir. K310'un "akıcılık için kapaksız" kararı kaldırıldı: sahibin gördüğü kayıp buydu.
- Kapak, modelin parçalarından ayrı küçük bir katmandır. Kaydırıcı her adımda yalnız onu yeniden kurar; model
  parçaları yeniden yüklenmez.
- Halka zinciri ve komşu kapakların köşe alması gövdenin float32 aralığında birleşir. Bu aralık bugünkü
  `MEET` (0,1 mm) mertebesindedir; çizimin hassasiyetinin çok altındadır.

### §2.3 Kenar çizgisi (MB-03)

Köşeler, parçanın kendi kökenine göre 0,05 mm'lik ızgaraya yuvarlanarak birleştirilir. İki gövde yine hiç
birleştirilmez (ADR 8120).

### §2.4 Kararlı parçalar (MB-04)

- Parçanın kimliği içeriğinden gelir. Aynı içerik aynı kimliği alır ve yeniden yüklenmez; kenarları yeniden
  hesaplanmaz.
- Önceki sahnede olup yenisinde olmayan parça hemen ekran kartından bırakılır.

### §2.5 Keskin köşe (MB-05)

Kiriş bandının gönyesi sınırlanır. Sınırı aşan köşe pahlanır; bant kendini kesmez. Plan, kesit ve IFC aynı bandı
kullanır.

### §2.6 Eksik araçlar (Faz B)

Sıra plandadır: temel, duvar profili, arazi ve kazı-dolgu, 3B görünüm paftada, mahal renk planı, güneş ve gölge,
serbest biçimli eleman, yürüyen merdiven, kazık. Her biri kendi ADR'siyle, kendi araştırmasıyla gelir.

### §2.7 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | 3B görünüm, IFC ve kesit Mimari'nin. |
| Harita, CBS | yok | Bina modeli yok; IFC görüntüleyicisi (`ifc.view`) aynı görüntüleyiciyi kullandığından MB-02…MB-04 onda da geçerli. |
| Yol | kısmen | Yolun 3B görünümü aynı çiziciyi kullanır: MB-03 ve MB-04 orada da geçerli. Birleşim kuralı yolu ilgilendirmez. |
| MCP, Python, eklenti, yapay zekâ ajanı | var | `cad.arch.ifc` aynı kesit modelini yazar; birleşim kuralı onlarda da geçerli. Görüntüleyici bir görünümdür; kanalların ayrı yolu yok. |

## §3 Kanıt

Her madde kendi bölümünü ekler: önce ve sonra resimleri, sınamalar, ölçüler.
