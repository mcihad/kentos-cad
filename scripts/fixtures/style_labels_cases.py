#!/usr/bin/env python3
"""Stil dosyalarında etiketlerin ortak durumları ve QGIS 4.2 ile iki yönlü denetimi (docs/adr/0635), KentOS kodu olmadan.

    python3 scripts/fixtures/style_labels_cases.py --files   # QGIS 4.2'nin etiketli SLD ve QML'lerini ve elle yazılanları
                                                             # fixtures/style-labels/v1'e yazar
    python3 scripts/fixtures/style_labels_cases.py           # file.import.{sld,qml,maplibre}.labels.json'u yazar
    python3 scripts/fixtures/style_labels_cases.py --check   # durumlar güncel mi; KentOS'un dosyaları QGIS'te ne oluyor

Okuma kuralları ADR'den, sld_cases.py, qml_cases.py ve maplibre_cases.py'nin okuyucularındadır (xml.etree ve json ile);
etiketlerin ortak kuralı style_labels_ref.py'de. Bu betik onları etiketli dosyalara uygular: QGIS'in yazdığı dört SLD ve dört
QML (nokta, çizgi, alan ve kurallı etiketleme) ile uç durumların elle yazılmış dosyaları.

KentOS'un yazdığı dosyaları (`KENTOS_WRITE_LABELS=1 cargo test -p kentos-headless --test headless style_labels` yazar,
fixtures/style-labels/v1/out) QGIS 4.2 açar: QML'lerin bütün etiket ayarları, SLD'lerinkiler QGIS'in SLD okuyucusunun
aldığı kadar (QGIS'in kendi yazdığı `Concatenate` metnini okumaz) beklenenle denetlenir.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import maplibre_cases as M  # noqa: E402
import qml_cases as Q  # noqa: E402
import sld_cases as S  # noqa: E402
import style_labels_ref as SL  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/style-labels/v1"
CASES = {k: ROOT / f"fixtures/commands/v1/file.import.{k}.labels.json" for k in ("sld", "qml", "maplibre")}
QGIS_PY = "/usr/bin/python3"
QGIS_PATH = "/usr/share/qgis/python"

SE = ('xmlns="http://www.opengis.net/sld" xmlns:ogc="http://www.opengis.net/ogc" xmlns:se="http://www.opengis.net/se" '
      'version="1.1.0"')


def sld(rules: str, name="x") -> str:
    return (f'<?xml version="1.0" encoding="UTF-8"?>\n<StyledLayerDescriptor {SE}><NamedLayer><se:Name>{name}</se:Name>'
            f"<UserStyle><se:FeatureTypeStyle>\n{rules}\n</se:FeatureTypeStyle></UserStyle></NamedLayer>"
            "</StyledLayerDescriptor>\n")


POINT = ('<se:PointSymbolizer><se:Graphic><se:Mark><se:WellKnownName>circle</se:WellKnownName><se:Fill>'
         '<se:SvgParameter name="fill">#d62728</se:SvgParameter></se:Fill></se:Mark><se:Size>6</se:Size></se:Graphic>'
         "</se:PointSymbolizer>")
FILL = ('<se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill">{c}</se:SvgParameter></se:Fill>'
        "</se:PolygonSymbolizer>")


def text(label: str, more: str = "") -> str:
    return f"<se:TextSymbolizer><se:Label>{label}</se:Label>{more}</se:TextSymbolizer>"


def prop(f: str) -> str:
    return f"<ogc:PropertyName>{f}</ogc:PropertyName>"


def font(size, *extra) -> str:
    params = "".join(f'<se:SvgParameter name="{k}">{v}</se:SvgParameter>' for k, v in
                     [("font-family", "DejaVu Sans"), ("font-size", size), *extra])
    return f"<se:Font>{params}</se:Font>"


def rule(name, body, flt=""):
    return f"<se:Rule><se:Name>{name}</se:Name>{flt}{body}</se:Rule>"


def eq(field, value):
    return (f"<ogc:Filter><ogc:PropertyIsEqualTo>{prop(field)}<ogc:Literal>{value}</ogc:Literal>"
            "</ogc:PropertyIsEqualTo></ogc:Filter>")


HAND_SLD = {
    "geoserver-etiket.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.0.0" xmlns="http://www.opengis.net/sld" xmlns:ogc="http://www.opengis.net/ogc">
  <NamedLayer><Name>yol</Name><UserStyle><FeatureTypeStyle>
    <Rule><Name>Yol</Name>
      <LineSymbolizer><Stroke><CssParameter name="stroke">#777777</CssParameter><CssParameter name="stroke-width">3</CssParameter></Stroke></LineSymbolizer>
      <TextSymbolizer>
        <Label>No: <ogc:PropertyName>no</ogc:PropertyName></Label>
        <Font><CssParameter name="font-family">DejaVu Sans</CssParameter><CssParameter name="font-size">11</CssParameter></Font>
        <LabelPlacement><LinePlacement><PerpendicularOffset>4</PerpendicularOffset></LinePlacement></LabelPlacement>
        <Fill><CssParameter name="fill">#003366</CssParameter></Fill>
        <VendorOption name="followLine">true</VendorOption>
        <VendorOption name="repeat">150</VendorOption>
      </TextSymbolizer>
    </Rule>
  </FeatureTypeStyle></UserStyle></NamedLayer>
</StyledLayerDescriptor>
""",
    "yalniz-etiket.sld": sld(rule("Etiket", text(prop("ad"), font(11) + '<se:Halo><se:Radius>1.5</se:Radius><se:Fill>'
                                  '<se:SvgParameter name="fill">#FFFFFF</se:SvgParameter></se:Fill></se:Halo>'
                                  '<se:Fill><se:SvgParameter name="fill">#222222</se:SvgParameter>'
                                  '<se:SvgParameter name="fill-opacity">0.8</se:SvgParameter></se:Fill>'))),
    "kategori-etiket.sld": sld(
        rule("Ada A", FILL.format(c="#e41a1c") + text(prop("ada"), font(12)), eq("ada", "A")) + "\n"
        + rule("Ada B", FILL.format(c="#377eb8") + text(prop("ada"), font(10)), eq("ada", "B"))),
    "strconcat.sld": sld(rule("Parsel", FILL.format(c="#f2e6c9") + text(
        '<ogc:Function name="strConcat"><ogc:Function name="strConcat">' + prop("ada")
        + "<ogc:Literal> / </ogc:Literal></ogc:Function>" + prop("parsel") + "</ogc:Function>", font(9)))),
    "islev-etiket.sld": sld(rule("Parsel", FILL.format(c="#cccccc") + text(
        '<ogc:Function name="strToUpperCase">' + prop("ada") + "</ogc:Function>", font(9)))),
    "metinsiz.sld": sld(rule("Durak", POINT + "<se:TextSymbolizer>" + font(10) + "</se:TextSymbolizer>")),
    "bos-metin.sld": sld(rule("Boş", POINT + text("   ")) + "\n" + rule("Ad", text(prop("ad"), font(10)))),
    "metre-etiket.sld": sld(rule("Etiket", '<se:TextSymbolizer uom="http://www.opengeospatial.org/se/units/metre">'
                                 "<se:Label>" + prop("ad") + "</se:Label>" + font(5) + "</se:TextSymbolizer>")),
    "oncelik-alan.sld": sld(rule("Durak", POINT + text(
        prop("ad"), font(10) + '<se:LabelPlacement><se:PointPlacement><se:Rotation>30</se:Rotation></se:PointPlacement>'
        '</se:LabelPlacement><se:Fill><se:SvgParameter name="fill">' + prop("renk") + "</se:SvgParameter></se:Fill>"
        "<se:Priority>" + prop("onem") + "</se:Priority>"))),
    "diger-etiket.sld": sld(rule("Konut", text(prop("ada"), font(12)), eq("tur", "konut")) + "\n"
                            + rule("Diğer", "<se:ElseFilter/>" + text(prop("ada"), font(9)))),
    "ayni-ad.sld": sld(rule("Ad", text(prop("ada"), font(12)), eq("tur", "konut")) + "\n"
                       + rule("Ad", text(prop("parsel"), font(9)), eq("tur", "ticaret"))),
    "aralik-disi.sld": sld(rule("Etiket", text(prop("ad"), font(500) + '<se:LabelPlacement><se:PointPlacement>'
                                "<se:Displacement><se:DisplacementX>900</se:DisplacementX><se:DisplacementY>0"
                                "</se:DisplacementY></se:Displacement></se:PointPlacement></se:LabelPlacement>"
                                "<se:Halo><se:Radius>20</se:Radius></se:Halo>"))),
    "kisa-renk.sld": sld(rule("Etiket", text(prop("ad"), font(10) + '<se:Halo><se:Radius>2</se:Radius><se:Fill>'
                              '<se:SvgParameter name="fill">red</se:SvgParameter></se:Fill></se:Halo>'
                              '<se:Fill><se:SvgParameter name="fill">#a00</se:SvgParameter></se:Fill>'))),
    "olcek-etiket.sld": sld(rule("Ölçekli", "<se:MinScaleDenominator>500</se:MinScaleDenominator>"
                                 "<se:MaxScaleDenominator>5000</se:MaxScaleDenominator>" + text(prop("ad"), font(10)))),
    "nokta-orta.sld": sld(rule("Etiket", POINT + text(
        prop("ad"), font(10, ("font-weight", "700"), ("font-style", "oblique"))
        + "<se:LabelPlacement><se:PointPlacement><se:AnchorPoint><se:AnchorPointX>0.5</se:AnchorPointX>"
        "<se:AnchorPointY>0.5</se:AnchorPointY></se:AnchorPoint></se:PointPlacement></se:LabelPlacement>"
        "<se:Priority>300</se:Priority>"))),
    "iki-metin.sld": sld(rule("Durak", POINT + text(prop("ad"), font(10)) + text("<ogc:Literal>Durak</ogc:Literal>",
                                                                               font(8)))),
}

QML_HEAD = '<!DOCTYPE qgis PUBLIC \'http://mrcc.com/qgis.dtd\' \'SYSTEM\'>\n'
MARKER = ('<renderer-v2 type="singleSymbol"><symbols><symbol name="0" type="marker" alpha="1">'
          '<layer class="SimpleMarker" enabled="1"><Option type="Map"><Option name="name" type="QString" value="circle"/>'
          '<Option name="color" type="QString" value="214,39,40,255"/><Option name="size" type="QString" value="2"/>'
          '</Option></layer></symbol></symbols></renderer-v2>')
FILLR = ('<renderer-v2 type="singleSymbol"><symbols><symbol name="0" type="fill" alpha="1">'
         '<layer class="SimpleFill" enabled="1"><Option type="Map"><Option name="color" type="QString" '
         'value="242,230,201,255"/><Option name="style" type="QString" value="solid"/>'
         '<Option name="outline_style" type="QString" value="no"/></Option></layer></symbol></symbols></renderer-v2>')
LINER = ('<renderer-v2 type="singleSymbol"><symbols><symbol name="0" type="line" alpha="1">'
         '<layer class="SimpleLine" enabled="1"><Option type="Map"><Option name="line_color" type="QString" '
         'value="85,85,85,255"/><Option name="line_width" type="QString" value="0.6"/></Option></layer></symbol>'
         "</symbols></renderer-v2>")


def settings(field, expr="0", size="10", unit="Pixel", extra_style="", placement="", rendering="", inner=""):
    return (f'<settings calloutType="simple"><text-style fieldName="{field}" isExpression="{expr}" fontFamily="DejaVu Sans"'
            f' fontSize="{size}" fontSizeUnit="{unit}" textColor="0,0,0,255" textOpacity="1"{extra_style}>{inner}'
            f"</text-style><placement{placement}/><rendering{rendering}/></settings>")


def qml(body, geom, labels_enabled="1"):
    return (QML_HEAD + f'<qgis version="4.2.3" labelsEnabled="{labels_enabled}">{body}'
            f"<layerGeometryType>{geom}</layerGeometryType></qgis>\n")


HAND_QML = {
    "yalniz-etiket.qml": qml('<labeling type="simple">' + settings(
        "ad", placement=' placement="1" priority="7"') + "</labeling>", 0),
    "kapali-etiket.qml": qml(FILLR + '<labeling type="simple">' + settings("ada") + "</labeling>", 2, "0"),
    "cizilmez.qml": qml(FILLR + '<labeling type="simple">' + settings("ada", rendering=' drawLabels="0"')
                        + "</labeling>", 2),
    "harita-birimi.qml": qml(MARKER + '<labeling type="simple">' + settings("ad", size="5", unit="MapUnit")
                             + "</labeling>", 0),
    "punto.qml": qml(MARKER + '<labeling type="simple">' + settings(
        "ad", size="9", unit="Point", placement=' placement="6" dist="1" distUnits="MM" priority="5"',
        inner='<text-buffer bufferDraw="1" bufferSize="0.5" bufferSizeUnits="MM" bufferColor="255,255,255,255"/>')
        + "</labeling>", 0),
    "bilinmeyen-etiket.qml": qml(FILLR + '<labeling type="pal"/>', 2),
    "suzgec-bozuk.qml": qml(FILLR + '<labeling type="rule-based"><rules key="{0}">'
                            '<rule key="{1}" description="Büyük" filter="regexp_match(&quot;ada&quot;, \'^A\')">'
                            + settings("ada") + '</rule><rule key="{2}">' + settings("parsel")
                            + "</rule></rules></labeling>", 2),
    "qt5-kalin.qml": qml(MARKER + '<labeling type="simple">' + settings(
        "ad", extra_style=' fontWeight="75" fontItalic="1"').replace('textOpacity="1"', 'textOpacity="0.5"')
        + "</labeling>", 0),
    "kural-olcek.qml": qml(FILLR + '<labeling type="rule-based"><rules key="{0}">'
                           '<rule key="{1}" description="Yakın" scalemindenom="500" scalemaxdenom="5000">'
                           + settings("ada") + "</rule></rules></labeling>", 2),
    "alan-disi.qml": qml(FILLR + '<labeling type="rule-based"><rules key="{0}">'
                         '<rule key="{1}" description="Dışarıda">' + settings("ada", placement=' placement="8"')
                         + '</rule><rule key="{2}" description="Çevre">'
                         + settings("parsel", placement=' placement="7"') + "</rule></rules></labeling>", 2),
    "nokta-yerlesim.qml": qml(MARKER + '<labeling type="simple">' + settings("ad", placement=' placement="5"')
                              + "</labeling>", 0),
    "ifade-bozuk.qml": qml(MARKER + '<labeling type="simple">' + settings("regexp_replace(&quot;ad&quot;, 'a', 'b')",
                                                                          expr="1")
                           + "</labeling>", 0),
    "islevli-metin.qml": qml(MARKER + '<labeling type="simple">' + settings(
        "upper(&quot;ad&quot;) || ' (' || to_string(round(&quot;kot&quot;, 1)) || ')'", expr="1") + "</labeling>", 0),
    "islevli-suzgec.qml": qml(FILLR + '<labeling type="rule-based"><rules key="{0}">'
                              '<rule key="{1}" description="Okul" filter="length(&quot;ad&quot;) &gt; 3 AND '
                              'lower(&quot;tur&quot;) = \'okul\'">' + settings("coalesce(&quot;ad&quot;, '?')", expr="1")
                              + "</rule></rules></labeling>", 2),
    "aralik-disi.qml": qml(LINER + '<labeling type="simple">' + settings(
        "ad", placement=' placement="3" repeatDistance="5" repeatDistanceUnits="Pixel" maxCurvedCharAngleIn="120"')
        + "</labeling>", 1),
    "ic-ice.qml": qml(FILLR + '<labeling type="rule-based"><rules key="{0}">'
                      '<rule key="{1}" description="Konut" filter="&quot;tur&quot; = \'konut\'">'
                      + settings("ada") + '<rule key="{2}" description="Büyük konut" filter="&quot;alan&quot; &gt; 1000">'
                      + settings("parsel") + '</rule></rule><rule key="{3}" description="Kapalı" active="0">'
                      + settings("ada") + "</rule></rules></labeling>", 2),
}


def ml(*layers):
    return {"version": 8, "name": "etiket", "sources": {"s": {"type": "geojson", "data": "x.geojson"}},
            "layers": list(layers)}


CIRCLE = {"id": "durak", "type": "circle", "source": "s", "paint": {"circle-color": "#d62728", "circle-radius": 4}}
PFILL = {"id": "parsel", "type": "fill", "source": "s", "paint": {"fill-color": "#f2e6c9"}}
LLINE = {"id": "yol", "type": "line", "source": "s", "paint": {"line-color": "#555555", "line-width": 2}}


def sym(ident, layout, paint=None, **extra):
    v = {"id": ident, "type": "symbol", "source": "s", "layout": layout, **extra}
    if paint is not None:
        v["paint"] = paint
    return v


HAND_ML = {
    "kentos-etiket.json": ml(CIRCLE, sym("durak/2", {"text-field": ["to-string", ["get", "ad"]],
                                                     "text-font": ["Noto Sans Regular"], "text-size": 11,
                                                     "text-variable-anchor": ["bottom-left", "top-left"],
                                                     "text-radial-offset": 0.2},
                                         {"text-color": "#222222", "text-halo-color": "#ffffff",
                                          "text-halo-width": 1.5},
                                         metadata={"kentos:layer": "durak", "kentos:label": "Etiket"})),
    "karisik.json": ml(PFILL, sym("parsel/2", {"text-field": ["get", "ada"], "text-size": 12},
                                  {"text-color": "#000000"}, filter=["==", ["geometry-type"], "Polygon"],
                                  metadata={"kentos:layer": "parsel", "kentos:label": "Ad"}),
                       sym("parsel/3", {"text-field": ["get", "ada"], "text-size": 12, "text-anchor": "center"},
                           {"text-color": "#000000"}, filter=["==", ["geometry-type"], "Point"],
                           metadata={"kentos:layer": "parsel", "kentos:label": "Ad"})),
    "eski-belirtec.json": ml(CIRCLE, sym("durak/ad", {"text-field": "{ad} ({kot})", "text-size": 10})),
    "concat.json": ml(PFILL, sym("parsel/no", {"text-field": ["concat", ["get", "ada"], "/",
                                                              ["to-string", ["get", "parsel"]]], "text-size": 9},
                                 {"text-color": "rgb(51, 51, 51)"})),
    "format.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["format", ["get", "ad"], {"font-scale": 1.2}]})),
    "zoom-boy.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["get", "ad"],
                                                 "text-size": ["interpolate", ["linear"], ["zoom"], 10, 8, 16, 14]})),
    "renkler.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["get", "ad"]}, {"text-color": ["get", "renk"]}),
                       sym("durak/kot", {"text-field": ["get", "kot"]},
                           {"text-color": "boya", "text-halo-color": "rgba(255,255,255,0.5)", "text-halo-width": 2})),
    "cizgi-etiket.json": ml(LLINE, sym("yol/ad", {"text-field": ["get", "ad"], "symbol-placement": "line",
                                                  "text-offset": [0, -1.2], "symbol-spacing": 250, "text-size": 10,
                                                  "text-max-angle": 30}, {"text-color": "#1f4e9e"})),
    "cevre.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["get", "ad"], "text-size": 10,
                                              "text-variable-anchor": ["top", "bottom"], "text-radial-offset": 0.5,
                                              "symbol-sort-key": 3, "text-allow-overlap": True})),
    "zoom-etiket.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["get", "ad"]}, minzoom=12, maxzoom=18)),
    "gizli-etiket.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["get", "ad"], "visibility": "none"})),
    "suzgec-etiket.json": ml(CIRCLE, sym("durak/zoom", {"text-field": ["get", "ad"]},
                                         filter=["==", ["zoom"], 3]),
                             sym("durak/okul", {"text-field": ["get", "ad"]}, filter=["==", ["get", "tur"], "okul"])),
    "bilinmeyen-anahtar.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["get", "ad"],
                                                           "text-transform": "uppercase", "text-letter-spacing": 0.1},
                                              {"text-translate": [0, 2]})),
    "kalin-egik.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["get", "ad"],
                                                   "text-font": ["Open Sans Bold Italic"]})),
    "yalniz-etiket.json": ml({"id": "yol/zemin", "type": "background", "paint": {"background-color": "#ffffff"}},
                             sym("yol/ad", {"text-field": ["get", "ad"], "symbol-placement": "line"})),
    "isi.json": ml({"id": "durak/1", "type": "heatmap", "source": "s",
                    "paint": {"heatmap-radius": 25, "heatmap-weight": ["/", ["to-number", ["get", "w"], 0], 40],
                              "heatmap-color": ["interpolate", ["linear"], ["heatmap-density"], 0, "rgba(0,0,255,0)",
                                                0.5, "#ffff00", 1, "red"], "heatmap-opacity": 0.7}}),
    "isi-karisik.json": ml({"id": "durak/1", "type": "heatmap", "source": "s",
                            "paint": {"heatmap-weight": ["*", ["get", "w"], 2], "heatmap-intensity": 2,
                                      "heatmap-color": ["interpolate", ["linear"], ["heatmap-density"], 0, "blue",
                                                        0.2, "lime", 1, "red"]}},
                           {"id": "durak/2", "type": "heatmap", "source": "s", "paint": {}}, CIRCLE),
    "kentos-simge.json": ml(sym("durak/1", {"icon-image": "kentos-star-0a1b2c3d", "icon-rotate": 30,
                                            "icon-allow-overlap": True, "icon-ignore-placement": True},
                                {"icon-opacity": 0.8},
                                metadata={"kentos:layer": "durak", "kentos:symbol": "marker",
                                          "kentos:marker": {"shape": "star", "size": 15.118, "unit": "px",
                                                            "fill": "#ffcc00", "stroke": "#7a5c00",
                                                            "strokeWidth": 1.134}})),
    "yabanci-simge.json": ml(CIRCLE, sym("durak/hastane", {"icon-image": "hospital-15", "icon-size": 1.5})),
    "aralik-disi.json": ml(CIRCLE, sym("durak/ad", {"text-field": ["get", "ad"], "text-size": 300},
                                       {"text-halo-width": 20, "text-halo-color": "#ffffff"})),
}

# QGIS writes these (--files), each SLD and QML.
QGIS_CODE = r'''
import sys
from qgis.core import *
from qgis.PyQt.QtGui import QColor, QFont
from qgis.PyQt.QtXml import QDomDocument
app = QgsApplication([], False)
app.initQgis()
out = sys.argv[1]
R = Qgis.RenderUnit


def fmt(size, unit, color, bold=False, italic=False, buf=None, bufunit=R.Pixels):
    f = QgsTextFormat()
    fo = QFont("DejaVu Sans")
    fo.setBold(bold)
    fo.setItalic(italic)
    f.setFont(fo)
    f.setForcedBold(bold)
    f.setSize(size)
    f.setSizeUnit(unit)
    f.setColor(QColor(color))
    if buf:
        b = QgsTextBufferSettings()
        b.setEnabled(True)
        b.setSize(buf[0])
        b.setSizeUnit(bufunit)
        b.setColor(QColor(buf[1]))
        f.setBuffer(b)
    return f


def layer(geom, fields, name, color):
    l = QgsVectorLayer(f"{geom}?crs=EPSG:5256&" + "&".join(f"field={f}" for f in fields), name, "memory")
    s = QgsSymbol.defaultSymbol(l.geometryType())
    s.setColor(QColor(color))
    l.setRenderer(QgsSingleSymbolRenderer(s))
    return l


def save(l, name):
    d = QDomDocument()
    l.exportSldStyle(d, "")
    open(f"{out}/{name}.sld", "w", encoding="utf-8").write(d.toString(1))
    l.saveNamedStyle(f"{out}/{name}.qml")


# A point's label: an expression, bold and leaning, a buffer, around the point, a priority and a scale range.
l = layer("Point", ["ad:string", "kot:double"], "durak", "#d62728")
s = QgsPalLayerSettings()
s.fieldName = '"ad" || \' (\' || "kot" || \')\''
s.isExpression = True
s.setFormat(fmt(12, R.Pixels, "#aa0000", True, True, (2, "#ffffff")))
s.placement = Qgis.LabelPlacement.OrderedPositionsAroundPoint
s.dist = 2
s.distUnits = R.Pixels
s.priority = 8
s.scaleVisibility = True
s.minimumScale = 5000
s.maximumScale = 500
l.setLabeling(QgsVectorLayerSimpleLabeling(s))
l.setLabelsEnabled(True)
save(l, "qgis-etiket-nokta")

# A line's label: a field, in points, curved below the line, repeated, lines merged.
l = layer("LineString", ["ad:string"], "yol", "#555555")
s = QgsPalLayerSettings()
s.fieldName = "ad"
s.setFormat(fmt(9, R.Points, "#1f4e9e", buf=(0.5, "#ffffff"), bufunit=R.Millimeters))
s.placement = Qgis.LabelPlacement.Curved
ls = s.lineSettings()
ls.setPlacementFlags(Qgis.LabelLinePlacementFlag.BelowLine | Qgis.LabelLinePlacementFlag.MapOrientation)
ls.setMergeLines(True)
s.setLineSettings(ls)
s.repeatDistance = 100
s.repeatDistanceUnit = R.Pixels
l.setLabeling(QgsVectorLayerSimpleLabeling(s))
l.setLabelsEnabled(True)
save(l, "qgis-etiket-cizgi")

# An area's label: level inside, only where it fits, a background and a shadow (KentOS says them lost).
l = layer("Polygon", ["ada:string", "parsel:integer"], "parsel", "#f2e6c9")
s = QgsPalLayerSettings()
s.fieldName = "ada"
f = fmt(10, R.Pixels, "#333333")
bg = QgsTextBackgroundSettings()
bg.setEnabled(True)
f.setBackground(bg)
sh = QgsTextShadowSettings()
sh.setEnabled(True)
f.setShadow(sh)
s.setFormat(f)
s.placement = Qgis.LabelPlacement.Horizontal
s.fitInPolygonOnly = True
l.setLabeling(QgsVectorLayerSimpleLabeling(s))
l.setLabelsEnabled(True)
save(l, "qgis-etiket-alan")

# Rule-based labels: a filter and its own scale, an ELSE rule.
l = layer("Polygon", ["ada:string", "parsel:integer"], "parsel", "#f2e6c9")
root = QgsRuleBasedLabeling.Rule(None)
a = QgsPalLayerSettings()
a.fieldName = "ada"
a.setFormat(fmt(12, R.Pixels, "#e41a1c", True))
a.placement = Qgis.LabelPlacement.Horizontal
root.appendChild(QgsRuleBasedLabeling.Rule(a, 0, 0, '"ada" = \'A\'', "Ada A"))
b = QgsPalLayerSettings()
b.fieldName = "parsel"
b.setFormat(fmt(9, R.Pixels, "#377eb8"))
b.placement = Qgis.LabelPlacement.Horizontal
root.appendChild(QgsRuleBasedLabeling.Rule(b, 0, 0, "", "Diğer", True))
l.setLabeling(QgsRuleBasedLabeling(root))
l.setLabelsEnabled(True)
save(l, "qgis-etiket-kurallar")

# Thematic renderers (docs/adr/0635 §5), QML only: a heat map, clusters, displaced points, inverted areas.
l = layer("Point", ["ad:string", "w:double"], "durak", "#d62728")
h = QgsHeatmapRenderer()
h.setRadius(12)
h.setRadiusUnit(R.Pixels)
h.setWeightExpression('"w"')
h.setMaximumValue(50)
h.setRenderQuality(3)
h.setColorRamp(QgsGradientColorRamp(QColor("#0000ff"), QColor("#ff0000"), False,
                                    [QgsGradientStop(0.5, QColor("#ffff00"))]))
l.setRenderer(h)
l.saveNamedStyle(f"{out}/qgis-isi.qml")
l = layer("Point", ["ad:string"], "durak", "#d62728")
c = QgsPointClusterRenderer()
c.setTolerance(20)
c.setToleranceUnit(R.Pixels)
c.setEmbeddedRenderer(QgsSingleSymbolRenderer(QgsMarkerSymbol.createSimple({"color": "#ff0000", "size": "2"})))
c.setClusterSymbol(QgsMarkerSymbol.createSimple({"color": "#00aa00", "size": "6"}))
l.setRenderer(c)
l.saveNamedStyle(f"{out}/qgis-kume.qml")
d = QgsPointDisplacementRenderer()
d.setTolerance(5)
d.setToleranceUnit(R.Millimeters)
d.setPlacement(QgsPointDisplacementRenderer.Placement.ConcentricRings)
d.setCircleWidth(0.5)
d.setCircleColor(QColor("#808080"))
d.setCircleRadiusAddition(1)
d.setEmbeddedRenderer(QgsSingleSymbolRenderer(QgsMarkerSymbol.createSimple({"color": "#ff0000", "size": "2"})))
l.setRenderer(d)
l.saveNamedStyle(f"{out}/qgis-yayma.qml")
l = layer("Polygon", ["ada:string"], "parsel", "#000000")
inv = QgsInvertedPolygonRenderer(QgsSingleSymbolRenderer(QgsFillSymbol.createSimple({"color": "#000000",
                                                                                      "outline_style": "no"})))
inv.setPreprocessingEnabled(True)
l.setRenderer(inv)
l.saveNamedStyle(f"{out}/qgis-ters.qml")

# QGIS's other symbol layers (docs/adr/0635 §7): an ellipse and a filled marker, a shapeburst fill, an arrow.
reg = QgsApplication.symbolLayerRegistry()
l = layer("Point", ["ad:string"], "durak", "#d62728")
sym = QgsMarkerSymbol()
sym.deleteSymbolLayer(0)
sym.appendSymbolLayer(reg.symbolLayerMetadata("EllipseMarker").createSymbolLayer({
    "symbol_name": "rectangle", "symbol_width": "3", "symbol_height": "2", "symbol_width_unit": "MM",
    "symbol_height_unit": "MM", "color": "255,0,0,255", "outline_color": "0,0,0,255", "outline_width": "0.2"}))
fm = reg.symbolLayerMetadata("FilledMarker").createSymbolLayer({"name": "star", "size": "4"})
fm.setSubSymbol(QgsFillSymbol.createSimple({"color": "#00aa00", "outline_color": "#003300", "outline_width": "0.3"}))
sym.appendSymbolLayer(fm)
l.setRenderer(QgsSingleSymbolRenderer(sym))
l.saveNamedStyle(f"{out}/qgis-katmanlar-nokta.qml")
l = layer("Polygon", ["ada:string"], "parsel", "#000000")
sym = QgsFillSymbol()
sym.deleteSymbolLayer(0)
sym.appendSymbolLayer(reg.symbolLayerMetadata("ShapeburstFill").createSymbolLayer({
    "color": "0,0,255,255", "color2": "255,255,255,255"}))
l.setRenderer(QgsSingleSymbolRenderer(sym))
l.saveNamedStyle(f"{out}/qgis-katmanlar-alan.qml")
l = layer("LineString", ["ad:string"], "yol", "#000000")
sym = QgsLineSymbol()
sym.deleteSymbolLayer(0)
sym.appendSymbolLayer(QgsSimpleLineSymbolLayer(QColor("#444444"), 0.5))
sym.appendSymbolLayer(reg.symbolLayerMetadata("ArrowLine").createSymbolLayer({}))
l.setRenderer(QgsSingleSymbolRenderer(sym))
l.saveNamedStyle(f"{out}/qgis-ok.qml")
print("ok")
'''


def qgis_env():
    return dict(os.environ, QT_QPA_PLATFORM="offscreen", PYTHONPATH=QGIS_PATH)


def qgis_ready() -> bool:
    return Path(QGIS_PATH, "qgis").is_dir() and Path(QGIS_PY).exists()


def write_files() -> None:
    DIR.mkdir(parents=True, exist_ok=True)
    for name, t in {**HAND_SLD, **HAND_QML}.items():
        (DIR / name).write_text(t, encoding="utf-8")
    for name, v in HAND_ML.items():
        (DIR / name).write_text(json.dumps(v, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    if not qgis_ready():
        sys.exit("QGIS 4.2 yok (/usr/share/qgis/python); QGIS'in dosyaları yazılmadı.")
    r = subprocess.run([QGIS_PY, "-c", QGIS_CODE, str(DIR)], env=qgis_env(), capture_output=True, text=True,
                       timeout=300)
    if "ok" not in r.stdout:
        sys.exit(f"QGIS yazamadı: {r.stderr[-800:]}")
    print(f"{len(HAND_SLD) + len(HAND_QML) + len(HAND_ML)} elle yazılmış ve 15 QGIS dosyası: {DIR}")


# ─── The cases ─────────────────────────────────────────────────────────────────────────────────────────

ROOT_PATH = "$fixture:style-labels/v1/"


def warn(owner, lost):
    return [] if not lost else [{"code": "import_losses", "message": f"{owner} bazı parçaları alınamadı: "
                                 + "; ".join(lost) + ".", "path": "path"}]


def xml_import(mod, owner, ident, name, changed=True):
    """(renderer, got) and the command's output and warnings for an SLD or a QML, by its module's reader."""
    got_all = mod.read((DIR / name).read_text(encoding="utf-8"))
    assert not isinstance(got_all, str), (name, got_all)
    renderer, lost, rules, got = got_all
    output = {"layer": ident, "kind": renderer["type"] if renderer else "", "rules": rules, "renderer": renderer,
              "changed": changed, "revision": "$current", **SL.output_of(got)}
    if lost:
        output["lost"] = lost
    return (renderer, got), output, warn(owner, lost)


def xml_ok(mod, owner, title, ident, name):
    (renderer, got), output, warnings = xml_import(mod, owner, ident, name)
    return {"name": title, "steps": [{
        "op": "execute", "input": {"layer": ident, "path": ROOT_PATH + name},
        "result": {"status": "completed", "output": output, "warnings": warnings},
        "expect": {"ids": [1, 2, 3], "canUndo": True, "dirty": True, "revision": "changed",
                   "layers": mod.layer_after(ident, renderer, got)},
    }]}


def failed(title, inp, code, message, path):
    return {"name": title, "steps": [{
        "op": "execute", "input": inp,
        "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
        "expect": {"ids": [1, 2, 3], "canUndo": False, "dirty": False, "revision": "same"},
    }]}


def xml_more(mod, owner, both, labels_only, setup_layers):
    """Plan, undo and redo, twice, refusals of the layer, an old revision: for SLD and QML alike."""
    cases = []
    (renderer, got), output, warnings = xml_import(mod, owner, "parsel", both)
    cases.append({"name": "Plan okur, hiçbir şey yazmaz", "steps": [
        {"op": "plan", "input": {"layer": "parsel", "path": ROOT_PATH + both},
         "result": {"status": "completed", "output": output, "warnings": warnings},
         "expect": {"ids": [1, 2, 3], "canUndo": False, "dirty": False, "revision": "same",
                    "layers": setup_layers}}]})
    cases.append({"name": "Görünüş ve etiketler tek adımda geri alınır ve yinelenir", "steps": [
        {"op": "execute", "input": {"layer": "parsel", "path": ROOT_PATH + both},
         "result": {"status": "completed", "output": output, "warnings": warnings}},
        {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": setup_layers}},
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False,
                                  "layers": mod.layer_after("parsel", renderer, got)}}]})
    cases.append({"name": "Aynı dosya ikinci kez: değişmez, adım yazılmaz", "steps": [
        {"op": "execute", "input": {"layer": "parsel", "path": ROOT_PATH + both},
         "result": {"status": "completed", "output": output, "warnings": warnings},
         "expect": {"canUndo": True, "revision": "changed"}},
        {"op": "execute", "input": {"layer": "parsel", "path": ROOT_PATH + both},
         "result": {"status": "completed", "output": dict(output, changed=False), "warnings": warnings},
         "expect": {"revision": "same", "layers": mod.layer_after("parsel", renderer, got)}}]})
    cases += [
        failed("Yalnız etiketli dosya servis katmanına: etiketleme nesneleri olan katmanın",
               {"layer": "altlik", "path": ROOT_PATH + labels_only}, "service_layer",
               "“OpenStreetMap” servisten çizilir ve nesne tutmaz; etiketleme nesneleri olan katmanın olur.", "layer"),
        failed("Yalnız etiketli dosya katman grubuna", {"layer": "imar", "path": ROOT_PATH + labels_only},
               "not_a_layer", "“İmar” bir katman grubu; etiketleme yalnız katmanın olur. Grubun bir katmanını verin.",
               "layer"),
        failed("Yalnız etiketli dosya olmayan katmana", {"layer": "yok", "path": ROOT_PATH + labels_only},
               "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
               "layer"),
    ]
    cases.append({"name": "Eski sürüme göre hazırlanmış istek: hiçbir şey yazılmaz", "steps": [
        {"op": "execute", "input": {"layer": "durak", "path": ROOT_PATH + labels_only, "expectedRevision": "999"},
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki "
                        "hâline göre yeniden hazırlayın.",
             "path": "expectedRevision", "revision": "$current"}},
         "expect": {"canUndo": False, "revision": "same"}}]})
    return cases


def sld_cases():
    ok = lambda title, ident, name: xml_ok(S, "SLD'nin", title, ident, name)  # noqa: E731
    cases = [
        ok("QGIS'in nokta etiketi: Concatenate ifadesi, kalın ve eğik, hale, noktanın çevresi, öncelik, ölçek aralığı",
           "durak", "qgis-etiket-nokta.sld"),
        ok("QGIS'in çizgi etiketi: punto boyu, çizgi boyunca", "yol", "qgis-etiket-cizgi.sld"),
        ok("QGIS'in alan etiketi: alanın ortasında", "parsel", "qgis-etiket-alan.sld"),
        ok("QGIS'in kurallı etiketleri: süzgeçli kural, ELSE kuralı söylenir", "parsel", "qgis-etiket-kurallar.sld"),
        ok("GeoServer'ın SLD 1.0'ı: karışık metin, CssParameter, kaydırılmış çizgi etiketi, followLine eğri boyunca, "
           "bilinmeyen seçenek söylenir", "yol", "geoserver-etiket.sld"),
        ok("Yalnız etiket: görünüş değişmez, “Etiket” adlı tek kural tek etiket olur, yarı saydam renk", "durak",
           "yalniz-etiket.sld"),
        ok("Kategorili görünüş ve kuralların etiketleri: her kural kendi süzgeciyle bir sınıf", "parsel",
           "kategori-etiket.sld"),
        ok("İç içe strConcat ve boşluklu yazı", "parsel", "strconcat.sld"),
        ok("Bilinmeyen işlevli metin söylenir, etiket alınmaz, görünüş alınır", "parsel", "islev-etiket.sld"),
        ok("Metni olmayan etiket söylenir", "durak", "metinsiz.sld"),
        ok("Boşluktan ibaret metin söylenir, öbür kuralın etiketi alınır", "durak", "bos-metin.sld"),
        ok("Metre birimli yazı söylenir, boyu piksel alınır", "durak", "metre-etiket.sld"),
        ok("Veriye bağlı renk ve öncelik, döndürme söylenir", "durak", "oncelik-alan.sld"),
        ok("“Diğerleri” etiketi söylenir, her nesneye alınır", "parsel", "diger-etiket.sld"),
        ok("Aynı adlı iki kural: ikincisi “Ad (2)”", "parsel", "ayni-ad.sld"),
        ok("Aralık dışı boy, uzaklık ve hale KentOS'un aralığına çekilir ve söylenir", "durak", "aralik-disi.sld"),
        ok("Üç basamaklı renk yazılır, okunamayan hale rengi söylenir", "durak", "kisa-renk.sld"),
        ok("Kuralın ölçek aralığı etiketin ölçeği olur", "durak", "olcek-etiket.sld"),
        ok("Ortadaki nokta etiketi, 700 kalınlık, oblique eğik, öncelik 3", "durak", "nokta-orta.sld"),
        ok("Bir kuralda iki etiket: iki sınıf, ikincinin adı “Durak (2)”", "durak", "iki-metin.sld"),
    ]
    return cases + xml_more(S, "SLD'nin", "kategori-etiket.sld", "yalniz-etiket.sld", S.SETUP["layers"])


def qml_cases():
    ok = lambda title, ident, name: xml_ok(Q, "QML'in", title, ident, name)  # noqa: E731
    cases = [
        ok("QGIS'in nokta etiketi: ifade, kalın ve eğik, tampon, sıralı yerleşim, öncelik, ölçek aralığı", "durak",
           "qgis-etiket-nokta.qml"),
        ok("QGIS'in çizgi etiketi: punto boyu, milimetre tampon, eğri, çizginin altı, yineleme, çizgileri birleştir",
           "yol", "qgis-etiket-cizgi.qml"),
        ok("QGIS'in alan etiketi: yatay, yalnız içine; zemin ve gölge söylenir", "parsel", "qgis-etiket-alan.qml"),
        ok("QGIS'in kurallı etiketleri: süzgeç, ELSE kuralı söylenir", "parsel", "qgis-etiket-kurallar.qml"),
        ok("Yalnız etiket: görünüş değişmez, noktanın üstünde, öncelik 7", "durak", "yalniz-etiket.qml"),
        ok("Etiketleri kapalı QML: etiketler alınmaz, söylenmez", "parsel", "kapali-etiket.qml"),
        ok("Çizilmeyen etiket alınmaz", "parsel", "cizilmez.qml"),
        ok("Harita biriminde yazı boyu söylenir", "durak", "harita-birimi.qml"),
        ok("Punto ve milimetre piksele çevrilir", "durak", "punto.qml"),
        ok("Bilinmeyen etiketleme söylenir", "parsel", "bilinmeyen-etiket.qml"),
        ok("Çevrilemeyen süzgeçli kural söylenir, adsız kural “Etiket 1” olur", "parsel", "suzgec-bozuk.qml"),
        ok("Qt 5'in kalınlığı ve yarı saydam yazı", "durak", "qt5-kalin.qml"),
        ok("Kuralın ölçek aralığı etiketin ölçeği olur", "parsel", "kural-olcek.qml"),
        ok("Alanın dışına yerleşim söylenir, eğri çevre yerleşimi alınır", "parsel", "alan-disi.qml"),
        ok("Noktada KentOS'ta olmayan yerleşim söylenir", "durak", "nokta-yerlesim.qml"),
        ok("Ortak olmayan işlevli ifade söylenir, görünüş alınır", "durak", "ifade-bozuk.qml"),
        ok("İşlevli metin ifadesi KentOS'un işlevleriyle alınır", "durak", "islevli-metin.qml"),
        ok("İşlevli süzgeç ve metin: length, lower, coalesce", "parsel", "islevli-suzgec.qml"),
        ok("Aralık dışı yineleme ve açı KentOS'un aralığına çekilir", "yol", "aralik-disi.qml"),
        ok("İç içe kurallar: alt kuralın koşulu üstününkiyle birleşir, kapalı kural söylenir", "parsel", "ic-ice.qml"),
        ok("QGIS'in ısı haritası: yarıçap, ağırlık, en büyük değer, kalite, renk rampası", "durak", "qgis-isi.qml"),
        ok("QGIS'in kümelemesi: uzaklık, iç görünüş ve kümenin simgesi", "durak", "qgis-kume.qml"),
        ok("QGIS'in yayması: milimetre uzaklık piksele, iç içe halkalar, halka çizgisi ve aralığı", "durak",
           "qgis-yayma.qml"),
        ok("QGIS'in ters alanı: birleştirerek, iç simgesi", "parsel", "qgis-ters.qml"),
        ok("QGIS'in elips ve dolgulu işareti KentOS'un şekil işaretleri olur; elipsin farklı kenarları söylenir",
           "durak", "qgis-katmanlar-nokta.qml"),
        ok("QGIS'in kenardan yayılan degradesi ilk rengiyle düz dolgu olur ve söylenir", "parsel",
           "qgis-katmanlar-alan.qml"),
        ok("QGIS'in ok çizgisi söylenir, düz çizgisi alınır", "yol", "qgis-ok.qml"),
    ]
    return cases + xml_more(Q, "QML'in", "qgis-etiket-kurallar.qml", "yalniz-etiket.qml", Q.SETUP["layers"])


def ml_ok(title, name, layer_id=None, frm=None):
    M.DIR = DIR
    res = M.run_import(name, layer_id, frm)
    assert isinstance(res[0], dict), (name, res)
    out, warnings, renderers = res
    inp = {"path": ROOT_PATH + name}
    if layer_id is not None:
        inp["layer"] = layer_id
    if frm is not None:
        inp["from"] = frm
    return {"name": title, "steps": [{
        "op": "execute", "input": inp,
        "result": {"status": "completed", "output": out, "warnings": warnings},
        "expect": {"ids": [1, 2, 3], "canUndo": True, "dirty": True, "revision": "changed",
                   "layers": M.layers_after(renderers)},
    }]}


def maplibre_cases():
    cases = [
        ml_ok("KentOS'un yazdığı etiket: kentos:label, tek etiket olur, hale ve çevresinde uzaklık", "kentos-etiket.json",
              "durak"),
        ml_ok("Bir sınıfın iki geometri katmanı tek sınıf olur: alanın içi ve noktanın üstü", "karisik.json", "parsel"),
        ml_ok("Eski belirteçli metin alanlar ve yazılar olur", "eski-belirtec.json", "durak"),
        ml_ok("concat, get ve to-string; rgb() rengi", "concat.json", "parsel"),
        ml_ok("format ifadesi söylenir, etiket alınmaz", "format.json", "durak"),
        ml_ok("Yakınlaştırmaya göre yazı boyu söylenir, 16 piksel alınır", "zoom-boy.json", "durak"),
        ml_ok("İfadeyle ve okunamayan renk söylenir, rgba hale rengi alınır", "renkler.json", "durak"),
        ml_ok("Çizgi boyunca etiket: üstünde uzaklık, yineleme, en çok açı", "cizgi-etiket.json", "yol"),
        ml_ok("Değişken çapalar ve uzaklık, sıralama anahtarı öncelik olur, çakışabilir", "cevre.json", "durak"),
        ml_ok("Yakınlaştırma aralığı etiketin ölçeği olur", "zoom-etiket.json", "durak"),
        ml_ok("Gizli etiket katmanı söylenir", "gizli-etiket.json", "durak"),
        ml_ok("Çevrilemeyen süzgeç söylenir, öbür etiketin koşulu alınır", "suzgec-etiket.json", "durak"),
        ml_ok("Bilinmeyen özellikler sırayla söylenir", "bilinmeyen-anahtar.json", "durak"),
        ml_ok("Kalın eğik yazı tipi: kalınlık ve eğiklik alınır, ailesi söylenir", "kalin-egik.json", "durak"),
        ml_ok("Yalnız etiketli stil: görünüş değişmez, arka plan söylenir", "yalniz-etiket.json", "yol"),
        ml_ok("Aralık dışı boy ve hale KentOS'un aralığına çekilir", "aralik-disi.json", "durak"),
        ml_ok("Başka adlı stil katmanları from ile alınır", "concat.json", "durak", "parsel"),
        ml_ok("KentOS'un sprite simgesi şekil işareti olur: döndürme ve saydamlık", "kentos-simge.json", "durak"),
        ml_ok("Başka bir sprite'ın simgesi söylenir, daire alınır", "yabanci-simge.json", "durak"),
        ml_ok("Isı haritası katmanı: yarıçap, en büyüğe bölünen ağırlık, yoğunluğa göre renk, saydamlık", "isi.json",
              "durak"),
        ml_ok("Isı haritası: alan olmayan ağırlık, eşit aralıksız duraklar, ikinci ısı haritası ve öbür katman, "
              "bilinmeyen özellik söylenir", "isi-karisik.json", "durak"),
    ]
    M.DIR = DIR
    out, warnings, renderers = M.run_import("kentos-etiket.json", "durak")
    inp = {"layer": "durak", "path": ROOT_PATH + "kentos-etiket.json"}
    cases.append({"name": "Plan okur, hiçbir şey yazmaz", "steps": [
        {"op": "plan", "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
         "expect": {"ids": [1, 2, 3], "canUndo": False, "dirty": False, "revision": "same",
                    "layers": M.SETUP["layers"]}}]})
    cases.append({"name": "Görünüş ve etiketler tek adımda geri alınır ve yinelenir", "steps": [
        {"op": "execute", "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings}},
        {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": M.SETUP["layers"]}},
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False, "layers": M.layers_after(renderers)}}]})
    same = dict(out, changed=False, layers=[dict(l, changed=False) for l in out["layers"]])
    cases.append({"name": "Aynı stil ikinci kez: değişmez, adım yazılmaz", "steps": [
        {"op": "execute", "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
         "expect": {"canUndo": True, "revision": "changed"}},
        {"op": "execute", "input": inp, "result": {"status": "completed", "output": same, "warnings": warnings},
         "expect": {"revision": "same", "layers": M.layers_after(renderers)}}]})
    cases.append(failed("Servis katmanına stil alınmaz", {"layer": "altlik", "path": ROOT_PATH + "kentos-etiket.json"},
                        "service_layer", "“OpenStreetMap” servisten çizilir ve nesne tutmaz; görünüş nesneleri olan "
                                         "katmanın olur.", "layer"))
    return cases


def head(command, title, setup, kind):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": f"ADR 0635. Etiketli ve tematik {kind} dosyaları (fixtures/style-labels/v1): QGIS 4.2'nin yazdıkları ve elle "
                    "yazılmış uç durumlar; etiketler ADR'nin okuma kuralıyla burada kurulur. Bağımsız başvuru: "
                    "scripts/fixtures/style_labels_cases.py; elle düzenlemeyin.",
            "setup": setup}


def build():
    return {
        "sld": dict(head("file.import.sld", "SLD'den görünüş al: etiketler", S.SETUP, "SLD"), cases=sld_cases()),
        "qml": dict(head("file.import.qml", "QML'den görünüş al: etiketler ve tematik görünüşler", Q.SETUP, "QML"), cases=qml_cases()),
        "maplibre": dict(head("file.import.maplibre", "MapLibre stilinden görünüş al: etiketler ve ısı haritası", M.SETUP, "MapLibre"),
                         cases=maplibre_cases()),
    }


# ─── KentOS's files in QGIS ─────────────────────────────────────────────────────────────────────────────

QGIS_READ = r'''
import json, sys
from qgis.core import *
from qgis.PyQt.QtGui import QColor
app = QgsApplication([], False)
app.initQgis()
geoms = {"parsel": "Polygon?field=ada:string&field=parsel_no:integer", "yol": "LineString?field=ad:string",
         "nokta": "Point?field=ad:string&field=kot:double"}


def info(s):
    f = s.format()
    return {"field": s.fieldName, "expr": s.isExpression, "size": round(f.size(), 3), "pixels": f.sizeUnit() == Qgis.RenderUnit.Pixels,
            "color": f.color().name(), "bold": f.font().bold() or f.forcedBold(), "italic": f.font().italic(),
            "buffer": f.buffer().enabled(), "bufsize": round(f.buffer().size(), 3), "placement": int(s.placement),
            "priority": s.priority, "dist": round(s.dist, 3), "minScale": round(s.minimumScale, 2),
            "maxScale": round(s.maximumScale, 2), "repeat": round(s.repeatDistance, 3)}


out = {}
for name, g in (("tema-isi", geoms["nokta"]), ("tema-kume", geoms["nokta"]), ("tema-ters", geoms["parsel"])):
    l = QgsVectorLayer(g + "&crs=EPSG:5256", name, "memory")
    msg, ok = l.loadNamedStyle(f"{sys.argv[1]}/{name}.qml")
    r = l.renderer()
    v = {"ok": bool(ok), "type": r.type() if r else None}
    if isinstance(r, QgsHeatmapRenderer):
        v.update(radius=r.radius(), weight=r.weightExpression(), max=r.maximumValue(), quality=r.renderQuality(),
                 colors=[r.colorRamp().color1().name(QColor.NameFormat.HexArgb),
                         r.colorRamp().color2().name(QColor.NameFormat.HexArgb)])
    if isinstance(r, QgsPointClusterRenderer):
        v.update(tolerance=r.tolerance(), inner=r.embeddedRenderer().type())
    if isinstance(r, QgsInvertedPolygonRenderer):
        v.update(merge=r.preprocessingEnabled(), inner=r.embeddedRenderer().type())
    out[f"{name}.qml"] = v
for name, g in geoms.items():
    for ext in ("sld", "qml"):
        l = QgsVectorLayer(g + "&crs=EPSG:5256", name, "memory")
        path = f"{sys.argv[1]}/etiket-{name}.{ext}"
        msg, ok = l.loadSldStyle(path) if ext == "sld" else l.loadNamedStyle(path)
        lab = l.labeling()
        v = {"ok": bool(ok), "enabled": l.labelsEnabled()}
        if isinstance(lab, QgsVectorLayerSimpleLabeling):
            v["simple"] = info(lab.settings())
        elif isinstance(lab, QgsRuleBasedLabeling):
            v["rules"] = [{"name": r.description(), "filter": r.filterExpression(), **info(r.settings())}
                          for r in lab.rootRule().children() if r.settings()]
        out[f"etiket-{name}.{ext}"] = v
print(json.dumps(out))
'''

RULE_A = {"name": "Ada A", "filter": "\"ada\" = 'A'", "field": "\"ada\" || '/' || \"parsel_no\"", "expr": True,
          "size": 12.0, "pixels": True, "color": "#aa0000", "bold": True, "italic": True, "buffer": True,
          "bufsize": 2.0, "placement": 4, "priority": 8, "minScale": 7559.15, "maxScale": 188.98}
RULE_B = {"name": "Diğer", "filter": "", "field": "parsel_no", "expr": False, "size": 9.0, "color": "#333333",
          "bold": False, "buffer": False, "placement": 4}
WANT = {
    "etiket-parsel.qml": {"ok": True, "enabled": True, "rules": [RULE_A, RULE_B, {
        "name": "İşlevli", "filter": "length(\"ada\") > 0", "field": "upper(\"ada\") || '-' || to_string(\"parsel_no\")",
        "expr": True, "size": 8.0}]},
    "etiket-yol.qml": {"ok": True, "enabled": True, "simple": {
        "field": "ad", "expr": False, "size": 10.0, "pixels": True, "color": "#1f4e9e", "buffer": True,
        "bufsize": 1.5, "placement": 3, "repeat": 120.0}},
    "etiket-nokta.qml": {"ok": True, "enabled": True, "simple": {
        "field": "'Kot ' || \"kot\"", "expr": True, "size": 10.0, "color": "#000000", "placement": 6,
        "priority": 6, "dist": 3.0}},
    # QGIS's SLD reader takes a PropertyName label only (not its own Concatenate): the rules come, the joined text
    # does not; the line's label is curved (followLine).
    "etiket-parsel.sld": {"ok": True, "enabled": True, "rules": [
        {"name": "Ada A", "filter": "ada = 'A'"},
        {"name": "Diğer", "field": "parsel_no", "size": 9.0, "pixels": True, "color": "#333333"}]},
    "etiket-yol.sld": {"ok": True, "enabled": True, "simple": {
        "field": "ad", "size": 10.0, "pixels": True, "color": "#1f4e9e", "buffer": True, "bufsize": 1.5,
        "placement": 3}},
    "etiket-nokta.sld": {"ok": True, "enabled": False},
    "tema-isi.qml": {"ok": True, "type": "heatmapRenderer", "radius": 15.0, "weight": "\"kot\"", "max": 100.0,
                     "quality": 3, "colors": ["#002b83ba", "#ffd7191c"]},
    "tema-kume.qml": {"ok": True, "type": "pointCluster", "tolerance": 25.0, "inner": "singleSymbol"},
    "tema-ters.qml": {"ok": True, "type": "invertedPolygonRenderer", "merge": True, "inner": "singleSymbol"},
}


def sub_problems(name, path, got, want):
    out = []
    if isinstance(want, dict):
        for k, v in want.items():
            out += sub_problems(name, f"{path}.{k}", (got or {}).get(k) if isinstance(got, dict) else None, v)
    elif isinstance(want, list):
        if not isinstance(got, list) or len(got) != len(want):
            return [f"{name}: QGIS'te {path} {json.dumps(got, ensure_ascii=False)}, beklenen {len(want)} öğe"]
        for i, (g, w) in enumerate(zip(got, want)):
            out += sub_problems(name, f"{path}[{i}]", g, w)
    elif got != want:
        out.append(f"{name}: QGIS'te {path} {json.dumps(got, ensure_ascii=False)}, beklenen "
                   f"{json.dumps(want, ensure_ascii=False)}")
    return out


def check_out():
    out_dir = DIR / "out"
    missing = [n for n in [*WANT, "etiket-parsel.json", "etiket-yol.json", "etiket-nokta.json", "tema-sprite.json",
                           "tema-sprite-sprite.json", "tema-sprite-sprite.png", "tema-sprite-sprite@2x.json",
                           "tema-sprite-sprite@2x.png"]
               if not (out_dir / n).exists()]
    if missing:
        return [f"{n} yok: önce KENTOS_WRITE_LABELS=1 ile KentOS'un testini çalıştırın" for n in missing]
    if not qgis_ready():
        print("QGIS 4.2 yok; KentOS'un dosyaları QGIS'te denetlenmedi.")
        return []
    r = subprocess.run([QGIS_PY, "-c", QGIS_READ, str(out_dir)], env=qgis_env(), capture_output=True, text=True,
                       timeout=300)
    line = [x for x in r.stdout.splitlines() if x.startswith("{")]
    if not line:
        return [f"QGIS okuyamadı: {r.stderr[-800:]}"]
    got = json.loads(line[-1])
    problems = []
    for name, want in WANT.items():
        problems += sub_problems(name, "", got.get(name), want)
    # The MapLibre styles through MapLibre's own validator.
    if M.ensure_spec():
        for name in ("etiket-parsel.json", "etiket-yol.json", "etiket-nokta.json", "tema-sprite.json"):
            ok, said = M.validate(out_dir / name)
            if not ok:
                problems.append(f"{name}: MapLibre'nin doğrulayıcısı: {said}")
    else:
        print("MapLibre'nin doğrulayıcısı kurulamadı; etiketli stiller doğrulanmadı.")
    return problems


def main() -> None:
    if "--files" in sys.argv:
        write_files()
        return
    data = build()
    texts = {k: json.dumps(v, ensure_ascii=False, indent=1) + "\n" for k, v in data.items()}
    if "--check" in sys.argv:
        problems = [f"{CASES[k].relative_to(ROOT)} güncel değil" for k, t in texts.items()
                    if not CASES[k].exists() or CASES[k].read_text(encoding="utf-8") != t]
        problems += [f"{k}: {len(v['cases'])} durum, en az 20 olmalı" for k, v in data.items() if len(v["cases"]) < 20]
        problems += check_out()
        if problems:
            sys.exit("\n".join(problems))
        print(", ".join(f"{k} {len(v['cases'])}" for k, v in data.items())
              + " durum güncel; KentOS'un etiketli SLD ve QML'leri QGIS 4.2'de beklenen ayarlarla, MapLibre stilleri "
              "resmî doğrulayıcıda geçerli.")
        return
    for k, t in texts.items():
        CASES[k].write_text(t, encoding="utf-8")
    print(", ".join(f"{k} {len(v['cases'])}" for k, v in data.items()) + " durum yazıldı.")


if __name__ == "__main__":
    main()
