<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor xmlns="http://www.opengis.net/sld" xmlns:ogc="http://www.opengis.net/ogc" xmlns:se="http://www.opengis.net/se" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="1.1.0" xsi:schemaLocation="http://www.opengis.net/sld http://schemas.opengis.net/sld/1.1.0/StyledLayerDescriptor.xsd">
  <NamedLayer>
    <se:Name>parsel</se:Name>
    <UserStyle>
      <se:Name>parsel</se:Name>
      <se:FeatureTypeStyle>
        <se:Rule>
          <se:Name>parsel</se:Name>
          <se:Description><se:Title>parsel</se:Title></se:Description>
          <se:PolygonSymbolizer uom="http://www.opengeospatial.org/se/units/pixel"><se:Fill><se:SvgParameter name="fill"><ogc:PropertyName>renk</ogc:PropertyName></se:SvgParameter></se:Fill><se:Stroke><se:SvgParameter name="stroke"><ogc:PropertyName>kenar</ogc:PropertyName></se:SvgParameter><se:SvgParameter name="stroke-width">1</se:SvgParameter></se:Stroke></se:PolygonSymbolizer>
        </se:Rule>
      </se:FeatureTypeStyle>
    </UserStyle>
  </NamedLayer>
</StyledLayerDescriptor>
