<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.0.0" xmlns="http://www.opengis.net/sld" xmlns:ogc="http://www.opengis.net/ogc">
  <NamedLayer><Name>imar</Name>
    <UserStyle><Title>İmar</Title>
      <FeatureTypeStyle>
        <Rule><Title>Konut</Title>
          <ogc:Filter><ogc:PropertyIsEqualTo><ogc:PropertyName>kullanim</ogc:PropertyName><ogc:Literal>Konut</ogc:Literal></ogc:PropertyIsEqualTo></ogc:Filter>
          <PolygonSymbolizer><Fill><CssParameter name="fill">#FFFF99</CssParameter><CssParameter name="fill-opacity">0.6</CssParameter></Fill>
            <Stroke><CssParameter name="stroke">#996633</CssParameter><CssParameter name="stroke-width">0.5</CssParameter></Stroke></PolygonSymbolizer>
        </Rule>
        <Rule><Title>Ticaret</Title>
          <ogc:Filter><ogc:PropertyIsEqualTo><ogc:PropertyName>kullanim</ogc:PropertyName><ogc:Literal>Ticaret</ogc:Literal></ogc:PropertyIsEqualTo></ogc:Filter>
          <PolygonSymbolizer><Fill><CssParameter name="fill">#FF6666</CssParameter></Fill></PolygonSymbolizer>
        </Rule>
        <Rule><Title>Öteki</Title><ElseFilter/>
          <PolygonSymbolizer><Fill><CssParameter name="fill">#CCCCCC</CssParameter></Fill></PolygonSymbolizer>
        </Rule>
      </FeatureTypeStyle>
    </UserStyle>
  </NamedLayer>
</StyledLayerDescriptor>
