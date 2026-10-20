<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2"><renderer-v2 type="RuleRenderer"><rules key="{0}">
<rule key="{1}" label="Seçili adalar" filter="&quot;ada&quot; IN ('A', 'C') AND &quot;alan&quot; BETWEEN 100 AND 900" symbol="0">
 <rule key="{2}" label="Tapusuz" filter="&quot;tapulu&quot; IS NULL OR NOT (&quot;tapulu&quot; = 1)" symbol="1"/>
</rule>
<rule key="{3}" label="İşlevli" filter="upper(&quot;ada&quot;) = 'B'" symbol="1"><rule key="{4}" label="Alt" symbol="0"/></rule>
<rule key="{5}" label="Kapalı" filter="&quot;ad&quot; LIKE 'Ç%'" symbol="0" checkstate="0" scalemaxdenom="5000"/>
<rule key="{6}" filter="ELSE" symbol="1"/>
</rules><symbols>
<symbol name="0" type="fill" alpha="1"><layer class="SimpleLine" enabled="1" locked="0" pass="0"><Option type="Map">
 <Option name="line_color" type="QString" value="0,0,0,255"/><Option name="line_width" type="QString" value="0.5"/>
 <Option name="line_width_unit" type="QString" value="MM"/></Option></layer></symbol>
<symbol name="1" type="fill" alpha="1"><layer class="SimpleFill" enabled="1"><Option type="Map">
<Option name="color" type="QString" value="#33a02c"/><Option name="outline_style" type="QString" value="no"/></Option></layer></symbol>
</symbols></renderer-v2></qgis>
