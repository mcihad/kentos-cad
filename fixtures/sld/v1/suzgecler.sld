<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc">
  <NamedLayer><se:Name>parsel</se:Name><UserStyle><se:FeatureTypeStyle>
    <se:Rule><se:Name>Boş not</se:Name>
      <ogc:Filter><ogc:PropertyIsNull><ogc:PropertyName>not</ogc:PropertyName></ogc:PropertyIsNull></ogc:Filter>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill">#EEEEEE</se:SvgParameter></se:Fill></se:PolygonSymbolizer>
    </se:Rule>
    <se:Rule><se:Name>Ka ile başlayan</se:Name>
      <ogc:Filter><ogc:PropertyIsLike wildCard="*" singleChar="?" escapeChar="\"><ogc:PropertyName>ada</ogc:PropertyName><ogc:Literal>Ka*</ogc:Literal></ogc:PropertyIsLike></ogc:Filter>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill">#AADDAA</se:SvgParameter></se:Fill></se:PolygonSymbolizer>
    </se:Rule>
    <se:Rule><se:Name>Orta</se:Name>
      <ogc:Filter><ogc:Or>
        <ogc:PropertyIsBetween><ogc:PropertyName>alan</ogc:PropertyName><ogc:LowerBoundary><ogc:Literal>100</ogc:Literal></ogc:LowerBoundary><ogc:UpperBoundary><ogc:Literal>200.5</ogc:Literal></ogc:UpperBoundary></ogc:PropertyIsBetween>
        <ogc:Not><ogc:PropertyIsEqualTo><ogc:PropertyName>ada</ogc:PropertyName><ogc:Literal>O'Neil</ogc:Literal></ogc:PropertyIsEqualTo></ogc:Not>
      </ogc:Or></ogc:Filter>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill">#5599FF</se:SvgParameter></se:Fill></se:PolygonSymbolizer>
    </se:Rule>
  </se:FeatureTypeStyle></UserStyle></NamedLayer>
</StyledLayerDescriptor>
