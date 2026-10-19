<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc" xmlns:xlink="http://www.w3.org/1999/xlink">
  <NamedLayer><se:Name>durak</se:Name><UserStyle>
    <se:FeatureTypeStyle>
      <se:Rule><se:Name>Durak</se:Name>
        <se:PointSymbolizer><se:Graphic>
          <se:ExternalGraphic><se:OnlineResource xlink:type="simple" xlink:href="durak.svg"/><se:Format>image/svg+xml</se:Format></se:ExternalGraphic>
          <se:Mark><se:WellKnownName>shape://vertline</se:WellKnownName><se:Fill><se:SvgParameter name="fill">#0055AA</se:SvgParameter></se:Fill></se:Mark>
          <se:Size>12</se:Size>
        </se:Graphic></se:PointSymbolizer>
        <se:TextSymbolizer><se:Label><ogc:PropertyName>ad</ogc:PropertyName></se:Label></se:TextSymbolizer>
      </se:Rule>
      <se:Rule><se:Name>Kutuda</se:Name>
        <ogc:Filter><ogc:BBOX><ogc:PropertyName>geom</ogc:PropertyName></ogc:BBOX></ogc:Filter>
        <se:PointSymbolizer><se:Graphic><se:Mark><se:WellKnownName>circle</se:WellKnownName></se:Mark></se:Graphic></se:PointSymbolizer>
      </se:Rule>
    </se:FeatureTypeStyle>
    <se:FeatureTypeStyle><se:Rule><se:PointSymbolizer><se:Graphic><se:Mark/></se:Graphic></se:PointSymbolizer></se:Rule></se:FeatureTypeStyle>
  </UserStyle></NamedLayer>
</StyledLayerDescriptor>
