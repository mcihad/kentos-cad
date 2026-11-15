# Değişiklikler

## 0.2.0 — 16 Kasım 2026

- Yeni kural: EK-3 ve EK-4 (`ek34`, docs/adr/8064). Mahal numarasının kalıbı, kat kodu, sırası ve saat yönü; mahal adının
  Tablo 3.2'deki yazımı; katman adı (4.11, Tablo 4.1–4.3), "0" katmanı, tarama katmanı ("-T"), emsal katmanı, aks çizgisi
  (noktalı kesikli), ölçü yazısının yazı tipi (4.21.2) ve blok adı (Tablo 4.4). Olgular KentOS'un `cad` bölümünden; kural
  verisi `data/ek34-2026.json` (KentOS'un kural kataloğundan `scripts/mevzuat/ek34_veri.py` yazar). Düzeltmeler tek adım:
  mahalleri saat yönünde yeniden numaralamak, mahalin adını ya da numarasını düzeltmek, katmanı yeniden adlandırmak, aks
  katmanının çizgi tipini noktalı kesikli yapmak.

## 0.1.0 — 15 Kasım 2026

- İlk kural: dijital teslim kapsamı (`kapsam`, docs/adr/8063). Bu projede yönetmeliğin kapsamı, 2B çizim ve belgelerin
  PDF/A'sı (Md. 4/2–3), IFC modeli (Md. 4/5), modelden üretilen çizimlerin PDF/A'sı (Md. 9/1) ve e-PYS'de teslim
  (e-PYS Md. 4) ne zaman, hangi maddeyle başlar. Tarih ve eşikler `data/kapsam-2026.json`'da (KentOS'un kural
  kataloğunun kopyası); eksik girdi sonucu değiştirebiliyorsa "teyit gerekli".
