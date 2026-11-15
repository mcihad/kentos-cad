"""Kuralların ortak okuduğu: binanın katları ve konutları (BYKHY 48/1'in muafiyeti), adların ve ölçülerin yazımı."""

from __future__ import annotations

from dataclasses import dataclass

from kentos import ext

#: Ölçülerin payı: olgular alanı 0.0001 m²'ye, uzunluğu 1 mm'ye yuvarlar; sınırdaki ölçü sınırı aşmış sayılmaz.
PAY = 1e-6


def metre(v: float) -> str:
    """Bir uzunluk, iki basamakla: “22.52 m”."""
    return f"{v:.2f} m"


def metrekare(v: float) -> str:
    """Bir alan, iki basamakla: “12.00 m²”."""
    return f"{v:.2f} m²"


def mahal(r: ext.RoomFacts) -> str:
    """Bir mahal, numarası ve adıyla: “Z02 Salon”."""
    parts = [p for p in (r.number, r.name) if p]
    return " ".join(parts) if parts else "Adsız mahal"


def bolum(u: ext.UnitFacts) -> str:
    """Bir bağımsız bölüm, numarası ve niteliğiyle: “1 numaralı bağımsız bölüm (Daire)”."""
    text = f"{u.number} numaralı bağımsız bölüm"
    return f"{text} ({u.nature})" if u.nature else text


def kat_adlari(a: ext.ArchitectureFacts) -> dict[str | None, str]:
    """Katların adları, kimlikleriyle; katsız mahallerin “katsız”."""
    names: dict[str | None, str] = {s.id: s.name or s.id for s in a.levels}
    names[None] = "katsız"
    return names


def konutlar(a: ext.ArchitectureFacts) -> dict[str, ext.UnitFacts]:
    """Konut olan bağımsız bölümler, kimlikleriyle (olguların ``dwelling``'i; eklenti ve ortak alan değil)."""
    return {u.id: u for u in a.units if u.kind == "unit" and u.dwelling}


@dataclass(frozen=True)
class Bina:
    """BYKHY 48/1'in okuduğu: katların sayısı (zemin ve üstü, mahali olan bodrumlar) ve konut bağımsız bölümleri."""

    katlar: int
    konutlar: int

    @property
    def kacis_mesafesi_aranmaz(self) -> bool:
        """BYKHY 48/1: bodrum katlar dahil 4 katı geçmeyen konutlarda ve tek evlerde kaçış mesafesi aranmaz."""
        return self.konutlar > 0 and (self.katlar <= 4 or self.konutlar == 1)


def bina(a: ext.ArchitectureFacts) -> Bina:
    """Binanın katları ve konutları: ±0.00 ve üstündeki her kat, altındakilerden mahali olanlar (KentOS Mimari'nin
    yangın denetiminin saydığı gibi); konut, olguların konut dediği bağımsız bölüm."""
    used = {r.level for r in a.rooms}
    storeys = sum(1 for s in a.levels if s.elevation >= -PAY or s.id in used)
    return Bina(max(storeys, 1), len(konutlar(a)))
