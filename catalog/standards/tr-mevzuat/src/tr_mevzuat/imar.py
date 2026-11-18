"""İmar durumu: emsal (KAKS) ve TAKS (PAİY md. 4/1-tt, 4/1-cccc, 5/5, 5/6, 5/8).

- 4/1-tt: “Katlar alanı katsayısı (KAKS) (Emsal): Yapının inşa edilen tüm kat alanlarının toplamının imar parseli
  alanına oranını,”
- 4/1-cccc: “Taban alanı kat sayısı (TAKS): Taban alanının imar parseli alanına oranını (Taban alanı kat sayısı, arazi
  eğimi nedeniyle tabii veya tesviye edilmiş zeminin üzerinde kalan tüm bodrum katlar ile zemin kat izdüşümü birlikte
  değerlendirilerek hesaplanır.),”
- 5/5 (RG 25.7.2019): “Taban alanı ve emsal hesabı; net imar parseli alanı üzerinden yapılır.”
- 5/6 (RG 1.7.2026/33297): “Ayrık veya blok nizam olan yerlerde, uygulama imar planında açıkça belirlenmemiş ise TAKS
  %40'ı geçemez. … Yapı yaklaşma mesafeleri ile KAKS verilip TAKS verilmeyen parsellerde de, TAKS %60'ı geçmemek
  şartıyla, yapı yaklaşma mesafelerine göre uygulama yapılır.”
- 5/8 (RG 30.9.2017): “… emsal harici tüm alanların toplamı; parselin toplam emsale esas alanının % 30'unu aşamaz.”

Ölçü KentOS Mimari'nin Emsal ve TAKS tablosunun okuduğudur (olguların ``zoning``'i): katlar alanı, %30'u aşan emsal
harici alan eklenmiş olarak; taban alanı, zemin katın sayılan mahalleri. Oranlar projenin İmar durumundaki net parsel
alanı üzerinden. Planın verdiği değer aşılırsa hata; İmar durumu emsalin ya da TAKS'ın nereden okunduğunu söylüyorsa
(imar durum belgesi ya da alındığı plan kararı) bulgu onu anar. TAKS verilmemişse 5/6: yapı nizamı (olguların
``zoning.layout``'u, 4/1-pppp) ayrık, blok ya da ikizse (ikiz blok sayılır) %40'ı, bitişikse %60'ı aşan hata; nizam
yazılmamışsa %60'ı aşan hata, %40 ile %60 arası uyarı (plan nizam belirlemiyorsa ayrık uygulanır, 8/5: müellif yazar).
Emsal verilmeyen parselde katlar alanı TAKS × kat adedidir (21/4); kat adedi olgularda yok, söylenmez. İmar durumu
yazılmamış projede kural bir şey söylemez.
"""

from __future__ import annotations

from kentos import ext

from .bina import PAY, metrekare

MADDE_EMSAL = "PAİY md. 4/1-tt ve 5/5; imar durumu"
MADDE_TAKS = "PAİY md. 4/1-cccc ve 5/5; imar durumu"
MADDE_TAKS_VERILMEYEN = "PAİY md. 5/6 (RG 1.7.2026/33297 ile değişik)"
MADDE_OTUZ = "PAİY md. 5/8 (RG 30.9.2017 ile değişik)"

#: Yapı nizamı (4/1-pppp), olguların yazdığından; ikiz blok sayılır.
NIZAM = {"detached": "ayrık", "attached": "bitişik", "block": "blok", "twin": "ikiz (blok sayılır)"}


def oran(v: float) -> str:
    """Bir oran, iki basamakla: “1.50”."""
    return f"{v:.2f}"


def kaynakli(madde: str, kaynak: str | None) -> str:
    """Maddenin anışı, değer İmar durumu'na bir yerden alınmışsa o yerle: “…; imar durumu (UIP-1 …, karar K1)”."""
    return f"{madde} ({kaynak})" if kaynak else madde


@ext.audit("imar-durumu")
def imar_durumu(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Projenin İmar durumu varsa emsal (katlar alanı / net parsel) ve TAKS (taban alanı / net parsel) planın verdiği
    değerlere: aşan hata. TAKS verilmemişse 5/6: ayrık, blok, ikiz nizamda %40'ı, bitişikte %60'ı aşan hata; nizam
    yazılmamışsa %60'ı aşan hata, %40-%60 arası uyarı. %30'u aşan emsal harici alan katlar alanına eklendi: bilgi."""
    a = facts.project.architecture
    z = a.zoning if a is not None else None
    if z is None or z.parcel <= 0:
        return []
    findings: list[ext.AuditFinding] = []
    emsal = z.floors / z.parcel
    taks = z.footprint / z.parcel
    nizam = getattr(z, "layout", None)
    if z.far is not None and z.floors > z.far * z.parcel + PAY:
        findings.append(
            ext.AuditFinding(
                message=(
                    f"Emsal {oran(emsal)} > {oran(z.far)}: katlar alanı {metrekare(z.floors)}, imar durumuna göre en "
                    f"çok {metrekare(z.far * z.parcel)} (net parsel {metrekare(z.parcel)}) — "
                    f"{kaynakli(MADDE_EMSAL, getattr(z, 'far_source', None))}."
                ),
                severity="error",
            )
        )
    if z.coverage is not None:
        if z.footprint > z.coverage * z.parcel + PAY:
            findings.append(
                ext.AuditFinding(
                    message=(
                        f"TAKS {oran(taks)} > {oran(z.coverage)}: taban alanı {metrekare(z.footprint)}, imar durumuna "
                        f"göre en çok {metrekare(z.coverage * z.parcel)} (net parsel {metrekare(z.parcel)}) — "
                        f"{kaynakli(MADDE_TAKS, getattr(z, 'coverage_source', None))}."
                    ),
                    severity="error",
                )
            )
    elif nizam in ("detached", "block", "twin") and z.footprint > 0.40 * z.parcel + PAY:
        findings.append(
            ext.AuditFinding(
                message=(
                    f"TAKS {oran(taks)}: imar durumu TAKS vermiyor; yapı nizamı {NIZAM[nizam]}, 5/6'ya göre %40'ı "
                    f"geçemez (taban alanı en çok {metrekare(0.40 * z.parcel)}) — {MADDE_TAKS_VERILMEYEN}."
                ),
                severity="error",
            )
        )
    elif z.footprint > 0.60 * z.parcel + PAY:
        findings.append(
            ext.AuditFinding(
                message=(
                    f"TAKS {oran(taks)}: imar durumu TAKS vermiyor; 5/6'ya göre %60'ı geçemez (taban alanı en çok "
                    f"{metrekare(0.60 * z.parcel)}) — {MADDE_TAKS_VERILMEYEN}."
                ),
                severity="error",
            )
        )
    elif nizam is None and z.footprint > 0.40 * z.parcel + PAY:
        findings.append(
            ext.AuditFinding(
                message=(
                    f"TAKS {oran(taks)}: imar durumu TAKS vermiyor; ayrık ya da blok nizamda %40'ı geçemez, bitişik "
                    f"nizamda %60; yapı nizamı yazılmamış, karar verilemedi (Proje ayarları › Mimari › İmar durumu; "
                    f"plan nizam belirlemiyorsa ayrık uygulanır, PAİY 8/5) — {MADDE_TAKS_VERILMEYEN}."
                ),
                severity="warning",
            )
        )
    if z.over > PAY and z.cap is not None:
        findings.append(
            ext.AuditFinding(
                message=(
                    f"Emsal harici alanlar {metrekare(z.exempt)}, sınır {metrekare(z.cap)} (emsale esas alanın %30'u): "
                    f"aşan {metrekare(z.over)} katlar alanına eklendi — {MADDE_OTUZ}."
                ),
                severity="info",
            )
        )
    return findings
