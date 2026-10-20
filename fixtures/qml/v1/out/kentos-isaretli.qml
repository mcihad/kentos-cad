<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2-KentOS" styleCategories="Symbology">
  <renderer-v2 type="singleSymbol" symbollevels="0" forceraster="0" enableorderby="0" referencescale="-1">
    <symbols>
      <symbol alpha="1" clip_to_extent="1" force_rhr="0" name="0" type="line">
        <layer class="SimpleLine" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="capstyle" type="QString" value="flat"/>
            <Option name="customdash" type="QString" value=""/>
            <Option name="customdash_unit" type="QString" value="MM"/>
            <Option name="joinstyle" type="QString" value="miter"/>
            <Option name="line_color" type="QString" value="51,102,204,255"/>
            <Option name="line_style" type="QString" value="solid"/>
            <Option name="line_width" type="QString" value="0.6"/>
            <Option name="line_width_unit" type="QString" value="MM"/>
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
        <layer class="MarkerLine" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="interval" type="QString" value="4"/>
            <Option name="interval_unit" type="QString" value="MM"/>
            <Option name="offset" type="QString" value="0"/>
            <Option name="offset_along_line" type="QString" value="1"/>
            <Option name="offset_along_line_unit" type="QString" value="MM"/>
            <Option name="offset_unit" type="QString" value="MM"/>
            <Option name="placements" type="QString" value="Interval"/>
            <Option name="ring_filter" type="QString" value="0"/>
            <Option name="rotate" type="QString" value="1"/>
          </Option>
          <symbol alpha="1" clip_to_extent="1" force_rhr="0" name="@0@1" type="marker">
            <layer class="SimpleMarker" enabled="1" locked="0" pass="0">
              <Option type="Map">
                <Option name="angle" type="QString" value="0"/>
                <Option name="cap_style" type="QString" value="square"/>
                <Option name="color" type="QString" value="255,0,0,255"/>
                <Option name="horizontal_anchor_point" type="QString" value="1"/>
                <Option name="joinstyle" type="QString" value="bevel"/>
                <Option name="name" type="QString" value="triangle"/>
                <Option name="offset" type="QString" value="0,0"/>
                <Option name="offset_unit" type="QString" value="MM"/>
                <Option name="outline_color" type="QString" value="0,0,0,0"/>
                <Option name="outline_style" type="QString" value="no"/>
                <Option name="outline_width" type="QString" value="0"/>
                <Option name="outline_width_unit" type="QString" value="MM"/>
                <Option name="scale_method" type="QString" value="diameter"/>
                <Option name="size" type="QString" value="2"/>
                <Option name="size_unit" type="QString" value="MM"/>
                <Option name="vertical_anchor_point" type="QString" value="1"/>
              </Option>
              <data_defined_properties>
                <Option type="Map">
                  <Option name="name" type="QString" value=""/>
                  <Option name="properties" type="Map">
                    <Option name="size" type="Map">
                      <Option name="active" type="bool" value="true"/>
                      <Option name="expression" type="QString" value="&quot;kat&quot; * 2 + 1"/>
                      <Option name="type" type="int" value="3"/>
                    </Option>
                    <Option name="angle" type="Map">
                      <Option name="active" type="bool" value="true"/>
                      <Option name="expression" type="QString" value="-(&quot;yon&quot;)"/>
                      <Option name="type" type="int" value="3"/>
                    </Option>
                  </Option>
                  <Option name="type" type="QString" value="collection"/>
                </Option>
              </data_defined_properties>
            </layer>
          </symbol>
        </layer>
      </symbol>
    </symbols>
  </renderer-v2>
  <layerGeometryType>1</layerGeometryType>
</qgis>
