<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor xmlns="http://www.opengis.net/sld" xmlns:ogc="http://www.opengis.net/ogc" xmlns:se="http://www.opengis.net/se" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="1.1.0" xsi:schemaLocation="http://www.opengis.net/sld http://schemas.opengis.net/sld/1.1.0/StyledLayerDescriptor.xsd">
  <NamedLayer>
    <se:Name>egri</se:Name>
    <UserStyle>
      <se:Name>egri</se:Name>
      <se:FeatureTypeStyle>
        <se:Rule>
          <se:Name>küçük</se:Name>
          <se:Description><se:Title>küçük</se:Title></se:Description>
          <ogc:Filter>
            <ogc:And>
              <ogc:PropertyIsGreaterThanOrEqualTo><ogc:PropertyName>alan</ogc:PropertyName><ogc:Literal>0</ogc:Literal></ogc:PropertyIsGreaterThanOrEqualTo>
              <ogc:PropertyIsLessThan><ogc:PropertyName>alan</ogc:PropertyName><ogc:Literal>500</ogc:Literal></ogc:PropertyIsLessThan>
            </ogc:And>
          </ogc:Filter>
          <se:PolygonSymbolizer uom="http://www.opengeospatial.org/se/units/pixel"><se:Fill><se:SvgParameter name="fill">#fee5d9</se:SvgParameter></se:Fill><se:Stroke><se:SvgParameter name="stroke">#222222</se:SvgParameter><se:SvgParameter name="stroke-width">1</se:SvgParameter></se:Stroke></se:PolygonSymbolizer>
        </se:Rule>
        <se:Rule>
          <se:Name>orta</se:Name>
          <se:Description><se:Title>orta</se:Title></se:Description>
          <ogc:Filter>
            <ogc:And>
              <ogc:PropertyIsGreaterThanOrEqualTo><ogc:PropertyName>alan</ogc:PropertyName><ogc:Literal>500</ogc:Literal></ogc:PropertyIsGreaterThanOrEqualTo>
              <ogc:PropertyIsLessThan><ogc:PropertyName>alan</ogc:PropertyName><ogc:Literal>1000</ogc:Literal></ogc:PropertyIsLessThan>
            </ogc:And>
          </ogc:Filter>
          <se:PolygonSymbolizer uom="http://www.opengeospatial.org/se/units/pixel"><se:Fill><se:SvgParameter name="fill">#fcae91</se:SvgParameter></se:Fill><se:Stroke><se:SvgParameter name="stroke">#222222</se:SvgParameter><se:SvgParameter name="stroke-width">1</se:SvgParameter></se:Stroke></se:PolygonSymbolizer>
        </se:Rule>
        <se:Rule>
          <se:Name>büyük</se:Name>
          <se:Description><se:Title>büyük</se:Title></se:Description>
          <ogc:Filter>
            <ogc:And>
              <ogc:PropertyIsGreaterThanOrEqualTo><ogc:PropertyName>alan</ogc:PropertyName><ogc:Literal>1000</ogc:Literal></ogc:PropertyIsGreaterThanOrEqualTo>
              <ogc:PropertyIsLessThanOrEqualTo><ogc:PropertyName>alan</ogc:PropertyName><ogc:Literal>5000</ogc:Literal></ogc:PropertyIsLessThanOrEqualTo>
            </ogc:And>
          </ogc:Filter>
          <se:PolygonSymbolizer uom="http://www.opengeospatial.org/se/units/pixel"><se:Fill><se:SvgParameter name="fill">#de2d26</se:SvgParameter></se:Fill><se:Stroke><se:SvgParameter name="stroke">#222222</se:SvgParameter><se:SvgParameter name="stroke-width">1</se:SvgParameter></se:Stroke></se:PolygonSymbolizer>
        </se:Rule>
      </se:FeatureTypeStyle>
    </UserStyle>
  </NamedLayer>
</StyledLayerDescriptor>
