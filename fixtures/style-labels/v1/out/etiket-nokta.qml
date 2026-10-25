<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2-KentOS" styleCategories="Symbology|Labeling" labelsEnabled="1">
  <renderer-v2 type="singleSymbol" symbollevels="0" forceraster="0" enableorderby="0" referencescale="-1">
    <symbols>
      <symbol alpha="1" clip_to_extent="1" force_rhr="0" name="0" type="marker">
        <layer class="SimpleMarker" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="angle" type="QString" value="0"/>
            <Option name="cap_style" type="QString" value="square"/>
            <Option name="color" type="QString" value="214,39,40,255"/>
            <Option name="horizontal_anchor_point" type="QString" value="1"/>
            <Option name="joinstyle" type="QString" value="bevel"/>
            <Option name="name" type="QString" value="circle"/>
            <Option name="offset" type="QString" value="0,0"/>
            <Option name="offset_unit" type="QString" value="MM"/>
            <Option name="outline_color" type="QString" value="0,0,0,0"/>
            <Option name="outline_style" type="QString" value="no"/>
            <Option name="outline_width" type="QString" value="0"/>
            <Option name="outline_width_unit" type="QString" value="Pixel"/>
            <Option name="scale_method" type="QString" value="diameter"/>
            <Option name="size" type="QString" value="7.143"/>
            <Option name="size_unit" type="QString" value="Pixel"/>
            <Option name="vertical_anchor_point" type="QString" value="1"/>
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
    <text-style fieldName="'Kot ' || &quot;kot&quot;" isExpression="1" fontFamily="Noto Sans" namedStyle="" fontSize="10" fontSizeUnit="Pixel" fontWeight="400" forcedBold="0" fontItalic="0" forcedItalic="0" textColor="0,0,0,255" textOpacity="1">
     <text-buffer bufferDraw="0"/>
    </text-style>
    <placement placement="6" placementFlags="0" dist="3" distUnits="Pixel" priority="6" repeatDistance="0" repeatDistanceUnits="Pixel" overlapHandling="PreventOverlap" maxCurvedCharAngleIn="25" maxCurvedCharAngleOut="-25" fitInPolygonOnly="0"/>
    <rendering drawLabels="1" scaleVisibility="0" scaleMin="0" scaleMax="0" mergeLines="0" minFeatureSize="0"/>
   </settings>
  </labeling>
  <layerGeometryType>0</layerGeometryType>
</qgis>
