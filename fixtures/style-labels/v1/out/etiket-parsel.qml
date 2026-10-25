<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2-KentOS" styleCategories="Symbology|Labeling" labelsEnabled="1">
  <renderer-v2 type="singleSymbol" symbollevels="0" forceraster="0" enableorderby="0" referencescale="-1">
    <symbols>
      <symbol alpha="1" clip_to_extent="1" force_rhr="0" name="0" type="fill">
        <layer class="SimpleFill" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="color" type="QString" value="242,230,201,255"/>
            <Option name="joinstyle" type="QString" value="bevel"/>
            <Option name="offset" type="QString" value="0,0"/>
            <Option name="offset_unit" type="QString" value="MM"/>
            <Option name="outline_color" type="QString" value="0,0,0,0"/>
            <Option name="outline_style" type="QString" value="no"/>
            <Option name="outline_width" type="QString" value="0"/>
            <Option name="outline_width_unit" type="QString" value="MM"/>
            <Option name="style" type="QString" value="solid"/>
          </Option>
          <data_defined_properties>
            <Option type="Map">
              <Option name="name" type="QString" value=""/>
              <Option name="properties"/>
              <Option name="type" type="QString" value="collection"/>
            </Option>
          </data_defined_properties>
        </layer>
        <layer class="SimpleLine" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="capstyle" type="QString" value="flat"/>
            <Option name="customdash" type="QString" value=""/>
            <Option name="customdash_unit" type="QString" value="Pixel"/>
            <Option name="joinstyle" type="QString" value="miter"/>
            <Option name="line_color" type="QString" value="138,109,59,255"/>
            <Option name="line_style" type="QString" value="solid"/>
            <Option name="line_width" type="QString" value="1.071"/>
            <Option name="line_width_unit" type="QString" value="Pixel"/>
            <Option name="offset" type="QString" value="0"/>
            <Option name="offset_unit" type="QString" value="MM"/>
            <Option name="ring_filter" type="QString" value="0"/>
            <Option name="use_custom_dash" type="QString" value="0"/>
          </Option>
          <data_defined_properties>
            <Option type="Map">
              <Option name="name" type="QString" value=""/>
              <Option name="properties"/>
              <Option name="type" type="QString" value="collection"/>
            </Option>
          </data_defined_properties>
        </layer>
      </symbol>
    </symbols>
  </renderer-v2>
  <labeling type="rule-based">
   <rules key="{00000000-0000-0000-0000-000000000900}">
    <rule key="{00000000-0000-0000-0000-000000000901}" description="Ada A" filter="&quot;ada&quot; = 'A'">
     <settings calloutType="simple">
      <text-style fieldName="&quot;ada&quot; || '/' || &quot;parsel_no&quot;" isExpression="1" fontFamily="Noto Sans" namedStyle="" fontSize="12" fontSizeUnit="Pixel" fontWeight="700" forcedBold="1" fontItalic="1" forcedItalic="0" textColor="170,0,0,255" textOpacity="1">
       <text-buffer bufferDraw="1" bufferSize="2" bufferSizeUnits="Pixel" bufferColor="255,255,255,255" bufferOpacity="1"/>
      </text-style>
      <placement placement="4" placementFlags="0" dist="2" distUnits="Pixel" priority="8" repeatDistance="0" repeatDistanceUnits="Pixel" overlapHandling="PreventOverlap" maxCurvedCharAngleIn="25" maxCurvedCharAngleOut="-25" fitInPolygonOnly="0"/>
      <rendering drawLabels="1" scaleVisibility="1" scaleMin="188.9788" scaleMax="7559.1504" mergeLines="0" minFeatureSize="0"/>
     </settings>
    </rule>
    <rule key="{00000000-0000-0000-0000-000000000902}" description="Diğer">
     <settings calloutType="simple">
      <text-style fieldName="parsel_no" isExpression="0" fontFamily="Noto Sans" namedStyle="" fontSize="9" fontSizeUnit="Pixel" fontWeight="400" forcedBold="0" fontItalic="0" forcedItalic="0" textColor="51,51,51,255" textOpacity="1">
       <text-buffer bufferDraw="0"/>
      </text-style>
      <placement placement="4" placementFlags="0" dist="2" distUnits="Pixel" priority="5" repeatDistance="0" repeatDistanceUnits="Pixel" overlapHandling="PreventOverlap" maxCurvedCharAngleIn="25" maxCurvedCharAngleOut="-25" fitInPolygonOnly="0"/>
      <rendering drawLabels="1" scaleVisibility="0" scaleMin="0" scaleMax="0" mergeLines="0" minFeatureSize="0"/>
     </settings>
    </rule>
    <rule key="{00000000-0000-0000-0000-000000000903}" description="İşlevli" filter="length(&quot;ada&quot;) &gt; 0">
     <settings calloutType="simple">
      <text-style fieldName="upper(&quot;ada&quot;) || '-' || to_string(&quot;parsel_no&quot;)" isExpression="1" fontFamily="Noto Sans" namedStyle="" fontSize="8" fontSizeUnit="Pixel" fontWeight="400" forcedBold="0" fontItalic="0" forcedItalic="0" textColor="0,0,0,255" textOpacity="1">
       <text-buffer bufferDraw="0"/>
      </text-style>
      <placement placement="4" placementFlags="0" dist="2" distUnits="Pixel" priority="5" repeatDistance="0" repeatDistanceUnits="Pixel" overlapHandling="PreventOverlap" maxCurvedCharAngleIn="25" maxCurvedCharAngleOut="-25" fitInPolygonOnly="0"/>
      <rendering drawLabels="1" scaleVisibility="0" scaleMin="0" scaleMax="0" mergeLines="0" minFeatureSize="0"/>
     </settings>
    </rule>
   </rules>
  </labeling>
  <layerGeometryType>2</layerGeometryType>
</qgis>
