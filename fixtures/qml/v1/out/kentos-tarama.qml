<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2-KentOS" styleCategories="Symbology">
  <renderer-v2 type="singleSymbol" symbollevels="0" forceraster="0" enableorderby="0" referencescale="-1">
    <symbols>
      <symbol alpha="1" clip_to_extent="1" force_rhr="0" name="0" type="fill">
        <layer class="SimpleFill" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="color" type="QString" value="255,255,255,255"/>
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
        <layer class="LinePatternFill" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="angle" type="QString" value="30"/>
            <Option name="clip_mode" type="QString" value="during_render"/>
            <Option name="coordinate_reference" type="QString" value="feature"/>
            <Option name="distance" type="QString" value="2.5"/>
            <Option name="distance_unit" type="QString" value="MM"/>
            <Option name="offset" type="QString" value="0.5"/>
            <Option name="offset_unit" type="QString" value="MM"/>
          </Option>
          <symbol alpha="1" clip_to_extent="1" force_rhr="0" name="@0@1" type="line">
            <layer class="SimpleLine" enabled="1" locked="0" pass="0">
              <Option type="Map">
                <Option name="capstyle" type="QString" value="flat"/>
                <Option name="customdash" type="QString" value=""/>
                <Option name="customdash_unit" type="QString" value="MM"/>
                <Option name="joinstyle" type="QString" value="miter"/>
                <Option name="line_color" type="QString" value="51,102,153,255"/>
                <Option name="line_style" type="QString" value="solid"/>
                <Option name="line_width" type="QString" value="0.2"/>
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
          </symbol>
        </layer>
        <layer class="SimpleLine" enabled="1" locked="0" pass="0">
          <Option type="Map">
            <Option name="capstyle" type="QString" value="flat"/>
            <Option name="customdash" type="QString" value=""/>
            <Option name="customdash_unit" type="QString" value="MM"/>
            <Option name="joinstyle" type="QString" value="miter"/>
            <Option name="line_color" type="QString" value="170,0,0,255"/>
            <Option name="line_style" type="QString" value="solid"/>
            <Option name="line_width" type="QString" value="0.5"/>
            <Option name="line_width_unit" type="QString" value="MM"/>
            <Option name="offset" type="QString" value="0"/>
            <Option name="offset_unit" type="QString" value="MM"/>
            <Option name="ring_filter" type="QString" value="0"/>
            <Option name="use_custom_dash" type="QString" value="0"/>
          </Option>
          <data_defined_properties>
            <Option type="Map">
              <Option name="name" type="QString" value=""/>
              <Option name="properties" type="Map">
                <Option name="outlineWidth" type="Map">
                  <Option name="active" type="bool" value="true"/>
                  <Option name="expression" type="QString" value="&quot;alan&quot; / 1000"/>
                  <Option name="type" type="int" value="3"/>
                </Option>
              </Option>
              <Option name="type" type="QString" value="collection"/>
            </Option>
          </data_defined_properties>
        </layer>
      </symbol>
    </symbols>
  </renderer-v2>
  <layerGeometryType>2</layerGeometryType>
</qgis>
