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
          <se:PolygonSymbolizer uom="http://www.opengeospatial.org/se/units/pixel"><se:Fill><se:SvgParameter name="fill">#f2e6c9</se:SvgParameter></se:Fill><se:Stroke><se:SvgParameter name="stroke">#8a6d3b</se:SvgParameter><se:SvgParameter name="stroke-width">1.071</se:SvgParameter></se:Stroke></se:PolygonSymbolizer>
        </se:Rule>
        <se:Rule>
          <se:Name>Ada A</se:Name>
          <se:Description><se:Title>Ada A</se:Title></se:Description>
          <ogc:Filter>
            <ogc:PropertyIsEqualTo><ogc:PropertyName>ada</ogc:PropertyName><ogc:Literal>A</ogc:Literal></ogc:PropertyIsEqualTo>
          </ogc:Filter>
          <se:MinScaleDenominator>188.9788</se:MinScaleDenominator>
          <se:MaxScaleDenominator>7559.1504</se:MaxScaleDenominator>
          <se:TextSymbolizer><se:Label><ogc:Function name="Concatenate"><ogc:PropertyName>ada</ogc:PropertyName><ogc:Literal>/</ogc:Literal><ogc:PropertyName>parsel_no</ogc:PropertyName></ogc:Function></se:Label><se:Font><se:SvgParameter name="font-family">Noto Sans</se:SvgParameter><se:SvgParameter name="font-size">12</se:SvgParameter><se:SvgParameter name="font-style">italic</se:SvgParameter><se:SvgParameter name="font-weight">bold</se:SvgParameter></se:Font><se:LabelPlacement><se:PointPlacement><se:AnchorPoint><se:AnchorPointX>0.5</se:AnchorPointX><se:AnchorPointY>0.5</se:AnchorPointY></se:AnchorPoint></se:PointPlacement></se:LabelPlacement><se:Halo><se:Radius>2</se:Radius><se:Fill><se:SvgParameter name="fill">#ffffff</se:SvgParameter></se:Fill></se:Halo><se:Fill><se:SvgParameter name="fill">#aa0000</se:SvgParameter></se:Fill><se:Priority>800</se:Priority></se:TextSymbolizer>
        </se:Rule>
        <se:Rule>
          <se:Name>Diğer</se:Name>
          <se:Description><se:Title>Diğer</se:Title></se:Description>
          <se:TextSymbolizer><se:Label><ogc:PropertyName>parsel_no</ogc:PropertyName></se:Label><se:Font><se:SvgParameter name="font-family">Noto Sans</se:SvgParameter><se:SvgParameter name="font-size">9</se:SvgParameter></se:Font><se:LabelPlacement><se:PointPlacement><se:AnchorPoint><se:AnchorPointX>0.5</se:AnchorPointX><se:AnchorPointY>0.5</se:AnchorPointY></se:AnchorPoint></se:PointPlacement></se:LabelPlacement><se:Fill><se:SvgParameter name="fill">#333333</se:SvgParameter></se:Fill></se:TextSymbolizer>
        </se:Rule>
      </se:FeatureTypeStyle>
    </UserStyle>
  </NamedLayer>
</StyledLayerDescriptor>
