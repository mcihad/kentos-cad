<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2-KentOS" styleCategories="Symbology|Labeling" labelsEnabled="1">
  <renderer-v2 type="singleSymbol" symbollevels="0" forceraster="0" enableorderby="0" referencescale="-1">
    <symbols>
      <symbol alpha="1" clip_to_extent="1" force_rhr="0" name="0" type="line">
        <layer class="SimpleLine" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="capstyle" type="QString" value="flat"/>
            <Option name="customdash" type="QString" value=""/>
            <Option name="customdash_unit" type="QString" value="Pixel"/>
            <Option name="joinstyle" type="QString" value="miter"/>
            <Option name="line_color" type="QString" value="85,85,85,255"/>
            <Option name="line_style" type="QString" value="solid"/>
            <Option name="line_width" type="QString" value="2.143"/>
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
  <labeling type="simple">
   <settings calloutType="simple">
    <text-style fieldName="ad" isExpression="0" fontFamily="Noto Sans" namedStyle="" fontSize="10" fontSizeUnit="Pixel" fontWeight="400" forcedBold="0" fontItalic="0" forcedItalic="0" textColor="31,78,158,255" textOpacity="1">
     <text-buffer bufferDraw="1" bufferSize="1.5" bufferSizeUnits="Pixel" bufferColor="255,255,255,204" bufferOpacity="1"/>
    </text-style>
    <placement placement="3" placementFlags="10" dist="2" distUnits="Pixel" priority="5" repeatDistance="120" repeatDistanceUnits="Pixel" overlapHandling="PreventOverlap" maxCurvedCharAngleIn="25" maxCurvedCharAngleOut="-25" fitInPolygonOnly="0"/>
    <rendering drawLabels="1" scaleVisibility="0" scaleMin="0" scaleMax="0" mergeLines="0" minFeatureSize="0"/>
   </settings>
  </labeling>
  <layerGeometryType>1</layerGeometryType>
</qgis>
