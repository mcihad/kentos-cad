<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc" xmlns:xlink="http://www.w3.org/1999/xlink">
  <NamedLayer><se:Name>parsel</se:Name><UserStyle><se:FeatureTypeStyle>
    <se:Rule><se:Name>İşlevli</se:Name>
      <se:PolygonSymbolizer>
        <se:Fill><se:SvgParameter name="fill"><ogc:Function name="Recode"><ogc:PropertyName>tür</ogc:PropertyName>
          <ogc:Literal>konut</ogc:Literal><ogc:Literal>#FF0000</ogc:Literal><ogc:Literal>3</ogc:Literal><ogc:Literal>#00FF00</ogc:Literal>
        </ogc:Function></se:SvgParameter></se:Fill>
        <se:Stroke><se:SvgParameter name="stroke"><ogc:Function name="Categorize"><ogc:PropertyName>alan</ogc:PropertyName>
          <ogc:Literal>#111111</ogc:Literal><ogc:Literal>100</ogc:Literal><ogc:Literal>#555555</ogc:Literal><ogc:Literal>500</ogc:Literal><ogc:Literal>#999999</ogc:Literal>
        </ogc:Function></se:SvgParameter><se:SvgParameter name="stroke-width">2</se:SvgParameter></se:Stroke>
      </se:PolygonSymbolizer>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill"><ogc:Function name="strConcat"><ogc:PropertyName>a</ogc:PropertyName><ogc:Literal>b</ogc:Literal></ogc:Function></se:SvgParameter></se:Fill></se:PolygonSymbolizer>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill"><ogc:Function name="Recode"><ogc:PropertyName>tür</ogc:PropertyName><ogc:PropertyName>başka</ogc:PropertyName></ogc:Function></se:SvgParameter></se:Fill></se:PolygonSymbolizer>
      <se:PointSymbolizer><se:Graphic>
        <se:Mark><se:WellKnownName>circle</se:WellKnownName>
          <se:Fill><se:SvgParameter name="fill"><ogc:PropertyName>renk</ogc:PropertyName></se:SvgParameter><se:SvgParameter name="fill-opacity">0.5</se:SvgParameter></se:Fill>
          <se:Stroke><se:SvgParameter name="stroke"><ogc:PropertyName>kenar</ogc:PropertyName></se:SvgParameter></se:Stroke></se:Mark>
        <se:Size>8</se:Size></se:Graphic></se:PointSymbolizer>
    </se:Rule>
  </se:FeatureTypeStyle></UserStyle></NamedLayer>
</StyledLayerDescriptor>
