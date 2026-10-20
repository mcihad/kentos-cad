<?xml version="1.0" encoding="utf-8" ?>
<ogr:FeatureCollection
     gml:id="aFeatureCollection"
     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
     xsi:schemaLocation="http://ogr.maptools.org/ geri-yaylar.xsd"
     xmlns:ogr="http://ogr.maptools.org/"
     xmlns:gml="http://www.opengis.net/gml/3.2">
  <gml:boundedBy><gml:Envelope srsName="urn:ogc:def:crs:EPSG::5256"><gml:lowerCorner>4420000 487000</gml:lowerCorner><gml:upperCorner>4420035 487015</gml:upperCorner></gml:Envelope></gml:boundedBy>
  <ogr:featureMember>
    <ogr:kenar gml:id="kenar.0">
      <gml:boundedBy><gml:Envelope srsName="urn:ogc:def:crs:EPSG::5256"><gml:lowerCorner>4420000 487000</gml:lowerCorner><gml:upperCorner>4420010 487015</gml:upperCorner></gml:Envelope></gml:boundedBy>
      <ogr:geometryProperty><gml:CompositeCurve srsName="urn:ogc:def:crs:EPSG::5256" gml:id="kenar.geom.0"><gml:curveMember><gml:LineString gml:id="kenar.geom.0.0"><gml:posList srsDimension="2">4420000 487000 4420000 487010</gml:posList></gml:LineString></gml:curveMember><gml:curveMember><gml:Curve gml:id="kenar.geom.0.1"><gml:segments><gml:ArcString><gml:posList srsDimension="2">4420000 487010 4420005 487015 4420010 487010</gml:posList></gml:ArcString></gml:segments></gml:Curve></gml:curveMember><gml:curveMember><gml:LineString gml:id="kenar.geom.0.2"><gml:posList srsDimension="2">4420010 487010 4420010 487000</gml:posList></gml:LineString></gml:curveMember></gml:CompositeCurve></ogr:geometryProperty>
      <ogr:ad>üç kesim</ogr:ad>
    </ogr:kenar>
  </ogr:featureMember>
  <ogr:featureMember>
    <ogr:kenar gml:id="kenar.1">
      <gml:boundedBy><gml:Envelope srsName="urn:ogc:def:crs:EPSG::5256"><gml:lowerCorner>4420020 487000</gml:lowerCorner><gml:upperCorner>4420035 487010</gml:upperCorner></gml:Envelope></gml:boundedBy>
      <ogr:geometryProperty><gml:MultiCurve srsName="urn:ogc:def:crs:EPSG::5256" gml:id="kenar.geom.1"><gml:curveMember><gml:LineString gml:id="kenar.geom.1.0"><gml:posList srsDimension="2">4420020 487000 4420020 487010</gml:posList></gml:LineString></gml:curveMember><gml:curveMember><gml:Curve gml:id="kenar.geom.1.1"><gml:segments><gml:ArcString><gml:posList srsDimension="2">4420030 487000 4420035 487005 4420030 487010</gml:posList></gml:ArcString></gml:segments></gml:Curve></gml:curveMember></gml:MultiCurve></ogr:geometryProperty>
      <ogr:ad>çok eğri</ogr:ad>
    </ogr:kenar>
  </ogr:featureMember>
</ogr:FeatureCollection>
