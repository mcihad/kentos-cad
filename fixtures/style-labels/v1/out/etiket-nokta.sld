<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor xmlns="http://www.opengis.net/sld" xmlns:ogc="http://www.opengis.net/ogc" xmlns:se="http://www.opengis.net/se" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="1.1.0" xsi:schemaLocation="http://www.opengis.net/sld http://schemas.opengis.net/sld/1.1.0/StyledLayerDescriptor.xsd">
  <NamedLayer>
    <se:Name>nokta</se:Name>
    <UserStyle>
      <se:Name>nokta</se:Name>
      <se:FeatureTypeStyle>
        <se:Rule>
          <se:Name>nokta</se:Name>
          <se:Description><se:Title>nokta</se:Title></se:Description>
          <se:PointSymbolizer uom="http://www.opengeospatial.org/se/units/pixel"><se:Graphic><se:Mark><se:WellKnownName>circle</se:WellKnownName><se:Fill><se:SvgParameter name="fill">#d62728</se:SvgParameter></se:Fill></se:Mark><se:Size>7.143</se:Size></se:Graphic></se:PointSymbolizer>
        </se:Rule>
        <se:Rule>
          <se:Name>Etiket</se:Name>
          <se:Description><se:Title>Etiket</se:Title></se:Description>
          <se:TextSymbolizer><se:Label><ogc:Function name="Concatenate"><ogc:Literal>Kot </ogc:Literal><ogc:PropertyName>kot</ogc:PropertyName></ogc:Function></se:Label><se:Font><se:SvgParameter name="font-family">Noto Sans</se:SvgParameter><se:SvgParameter name="font-size">10</se:SvgParameter></se:Font><se:LabelPlacement><se:PointPlacement><se:AnchorPoint><se:AnchorPointX>0</se:AnchorPointX><se:AnchorPointY>0.5</se:AnchorPointY></se:AnchorPoint><se:Displacement><se:DisplacementX>3</se:DisplacementX><se:DisplacementY>0</se:DisplacementY></se:Displacement></se:PointPlacement></se:LabelPlacement><se:Fill><se:SvgParameter name="fill">#000000</se:SvgParameter></se:Fill><se:Priority>600</se:Priority></se:TextSymbolizer>
        </se:Rule>
      </se:FeatureTypeStyle>
    </UserStyle>
  </NamedLayer>
</StyledLayerDescriptor>
