<?xml version="1.0" encoding="UTF-8"?>
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
