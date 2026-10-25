<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor xmlns="http://www.opengis.net/sld" version="1.1.0" xmlns:ogc="http://www.opengis.net/ogc" xmlns:se="http://www.opengis.net/se" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.opengis.net/sld http://schemas.opengis.net/sld/1.1.0/StyledLayerDescriptor.xsd">
 <NamedLayer>
  <se:Name>parsel</se:Name>
  <UserStyle>
   <se:Name>parsel</se:Name>
   <se:FeatureTypeStyle>
    <se:Rule>
     <se:Name>Single symbol</se:Name>
     <se:PolygonSymbolizer>
      <se:Fill>
       <se:SvgParameter name="fill">#f2e6c9</se:SvgParameter>
      </se:Fill>
      <se:Stroke>
       <se:SvgParameter name="stroke">#232323</se:SvgParameter>
       <se:SvgParameter name="stroke-width">1</se:SvgParameter>
       <se:SvgParameter name="stroke-linejoin">bevel</se:SvgParameter>
      </se:Stroke>
     </se:PolygonSymbolizer>
    </se:Rule>
    <se:Rule>
     <ogc:Filter xmlns:ogc="http://www.opengis.net/ogc">
      <ogc:PropertyIsEqualTo>
       <ogc:PropertyName>ada</ogc:PropertyName>
       <ogc:Literal>A</ogc:Literal>
      </ogc:PropertyIsEqualTo>
     </ogc:Filter>
     <se:TextSymbolizer>
      <se:Label>
       <ogc:PropertyName>ada</ogc:PropertyName>
      </se:Label>
      <se:Font>
       <se:SvgParameter name="font-family">DejaVu Sans</se:SvgParameter>
       <se:SvgParameter name="font-size">12</se:SvgParameter>
       <se:SvgParameter name="font-weight">bold</se:SvgParameter>
      </se:Font>
      <se:LabelPlacement>
       <se:PointPlacement>
        <se:AnchorPoint>
         <se:AnchorPointX>0.5</se:AnchorPointX>
         <se:AnchorPointY>0.5</se:AnchorPointY>
        </se:AnchorPoint>
       </se:PointPlacement>
      </se:LabelPlacement>
      <se:Fill>
       <se:SvgParameter name="fill">#e41a1c</se:SvgParameter>
      </se:Fill>
     </se:TextSymbolizer>
    </se:Rule>
    <se:Rule>
     <se:ElseFilter xmlns:se="http://www.opengis.net/se"/>
     <se:TextSymbolizer>
      <se:Label>
       <ogc:PropertyName>parsel</ogc:PropertyName>
      </se:Label>
      <se:Font>
       <se:SvgParameter name="font-family">DejaVu Sans</se:SvgParameter>
       <se:SvgParameter name="font-size">9</se:SvgParameter>
      </se:Font>
      <se:LabelPlacement>
       <se:PointPlacement>
        <se:AnchorPoint>
         <se:AnchorPointX>0.5</se:AnchorPointX>
         <se:AnchorPointY>0.5</se:AnchorPointY>
        </se:AnchorPoint>
       </se:PointPlacement>
      </se:LabelPlacement>
      <se:Fill>
       <se:SvgParameter name="fill">#377eb8</se:SvgParameter>
      </se:Fill>
     </se:TextSymbolizer>
    </se:Rule>
   </se:FeatureTypeStyle>
  </UserStyle>
 </NamedLayer>
</StyledLayerDescriptor>
