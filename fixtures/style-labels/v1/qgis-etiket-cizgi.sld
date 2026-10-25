<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor xmlns="http://www.opengis.net/sld" version="1.1.0" xmlns:ogc="http://www.opengis.net/ogc" xmlns:se="http://www.opengis.net/se" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.opengis.net/sld http://schemas.opengis.net/sld/1.1.0/StyledLayerDescriptor.xsd">
 <NamedLayer>
  <se:Name>yol</se:Name>
  <UserStyle>
   <se:Name>yol</se:Name>
   <se:FeatureTypeStyle>
    <se:Rule>
     <se:Name>Single symbol</se:Name>
     <se:LineSymbolizer>
      <se:Stroke>
       <se:SvgParameter name="stroke">#555555</se:SvgParameter>
       <se:SvgParameter name="stroke-width">1</se:SvgParameter>
       <se:SvgParameter name="stroke-linejoin">bevel</se:SvgParameter>
       <se:SvgParameter name="stroke-linecap">square</se:SvgParameter>
      </se:Stroke>
     </se:LineSymbolizer>
    </se:Rule>
    <se:Rule>
     <se:TextSymbolizer>
      <se:Label>
       <ogc:PropertyName>ad</ogc:PropertyName>
      </se:Label>
      <se:Font>
       <se:SvgParameter name="font-family">DejaVu Sans</se:SvgParameter>
       <se:SvgParameter name="font-size">11</se:SvgParameter>
      </se:Font>
      <se:LabelPlacement>
       <se:LinePlacement>
        <se:Repeat>true</se:Repeat>
        <se:Gap>100</se:Gap>
        <se:GeneralizeLine>true</se:GeneralizeLine>
       </se:LinePlacement>
      </se:LabelPlacement>
      <se:Halo>
       <se:Radius>1</se:Radius>
       <se:Fill>
        <se:SvgParameter name="fill">#ffffff</se:SvgParameter>
       </se:Fill>
      </se:Halo>
      <se:Fill>
       <se:SvgParameter name="fill">#1f4e9e</se:SvgParameter>
      </se:Fill>
      <se:VendorOption name="followLine">true</se:VendorOption>
      <se:VendorOption name="maxAngleDelta">25</se:VendorOption>
      <se:VendorOption name="repeat">100</se:VendorOption>
      <se:VendorOption name="group">yes</se:VendorOption>
     </se:TextSymbolizer>
    </se:Rule>
   </se:FeatureTypeStyle>
  </UserStyle>
 </NamedLayer>
</StyledLayerDescriptor>
