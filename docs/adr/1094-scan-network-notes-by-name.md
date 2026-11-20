# 1094 — Tarama ağı uyarıları taramaları adıyla söyler

Tarih: 2026-11-20. gis-usage-report.md GIS-037. Durum: kabul edildi.

İlgili: ADR 4720 (Tarama ağı dengelemesi), ADR 5100 (dış döngü, Nesnede tut).

## Sorun

Ağ dengelemesinin özetindeki zayıf yatay geometri uyarıları her bağı ve taramayı iki uzun iç kimlikle (UUID) yazıyordu;
üst bölüm dosya adlarını gösterdiği halde hangi uyarının hangi taramaya ait olduğu görülmüyordu. Aynı dosya adlı iki
bulut üst listede de ayırt edilemiyordu.

## Karar

1. Komut (`cad.pointclouds.network`) notlarında kimliği taşımaya devam eder: betik ve MCP kimlikle çalışır; pencere
   her notta pencerenin bildiği taramaların kimliğini görünen adıyla değiştirir. Komutun sözleşmesi ve paylaşılan
   örnekler değişmez.
2. Görünen ad ilk dosyanın adıdır; aynı adı paylaşan taramalar katmanları farklıysa katman adıyla (“ag-e.xyz
   (Tarama-1)”), değilse nesne numarasıyla (“ag-e.xyz #5”) ayrılır. Sabit tarama, Uygulanacak tarama ve tablolar aynı
   adları kullanır.
3. Bir taramayı ya da bağı anan not uyarı simgesiyle yazılır ve tıklanabilir: iki tarama anıyorsa Bağlar tablosu,
   biriyse Taramalar tablosu açılır ve satırın başına “▸” konur. Yeni çözüm işareti kaldırır.

## Sınır

Notun kimlikli özgün metni pencerede gösterilmez; kopyalama eylemi bu pencerede yoktur (kimlik betikten okunur).

## Kanıt

`apps/desktop/src/tests/scan_network_window_tests.rs`: düz zeminin üç taramasında notlarda kimlik kalmaz, bağ notu
tıklanınca Bağlar'da tek satır işaretlenir, yeni çözümde işaret kalkar; aynı adlı iki dosya numarayla ayrılır; resim
`.run/gui-evidence/cbs-kullanim/gis-037-ag-uyarilari.png`.
