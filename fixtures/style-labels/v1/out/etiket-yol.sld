<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor xmlns="http://www.opengis.net/sld" xmlns:ogc="http://www.opengis.net/ogc" xmlns:se="http://www.opengis.net/se" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="1.1.0" xsi:schemaLocation="http://www.opengis.net/sld http://schemas.opengis.net/sld/1.1.0/StyledLayerDescriptor.xsd">
  <NamedLayer>
    <se:Name>yol</se:Name>
    <UserStyle>
      <se:Name>yol</se:Name>
      <se:FeatureTypeStyle>
        <se:Rule>
          <se:Name>yol</se:Name>
          <se:Description><se:Title>yol</se:Title></se:Description>
          <se:LineSymbolizer uom="http://www.opengeospatial.org/se/units/pixel"><se:Stroke><se:SvgParameter name="stroke">#555555</se:SvgParameter><se:SvgParameter name="stroke-width">2.143</se:SvgParameter></se:Stroke></se:LineSymbolizer>
        </se:Rule>
        <se:Rule>
          <se:Name>Etiket</se:Name>
          <se:Description><se:Title>Etiket</se:Title></se:Description>
          <se:TextSymbolizer><se:Label><ogc:PropertyName>ad</ogc:PropertyName></se:Label><se:Font><se:SvgParameter name="font-family">Noto Sans</se:SvgParameter><se:SvgParameter name="font-size">10</se:SvgParameter></se:Font><se:LabelPlacement><se:LinePlacement><se:IsRepeated>true</se:IsRepeated><se:Gap>120</se:Gap></se:LinePlacement></se:LabelPlacement><se:Halo><se:Radius>1.5</se:Radius><se:Fill><se:SvgParameter name="fill">#ffffff</se:SvgParameter><se:SvgParameter name="fill-opacity">0.8</se:SvgParameter></se:Fill></se:Halo><se:Fill><se:SvgParameter name="fill">#1f4e9e</se:SvgParameter></se:Fill><se:VendorOption name="followLine">true</se:VendorOption></se:TextSymbolizer>
        </se:Rule>
      </se:FeatureTypeStyle>
    </UserStyle>
  </NamedLayer>
</StyledLayerDescriptor>
