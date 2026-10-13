// An independent reference for KentOS's curves (docs/adr/0259): OpenCASCADE
// evaluates what scripts/fixtures/nurbs_cases.py asks, without KentOS code.
//
// Build (OpenCASCADE 7.9 from Homebrew; see tools/occt/README.md):
//   clang++ -std=c++17 -O2 tools/occt/occt_ref.cpp -o .run/occt_ref \
//     -I/opt/homebrew/include/opencascade -L/opt/homebrew/lib \
//     -lTKernel -lTKMath -lTKG2d -lTKG3d -lTKGeomBase -lTKGeomAlgo -Wl,-rpath,/opt/homebrew/lib
//
// Protocol: one request per line on stdin, one answer per line on stdout,
// numbers written with 17 significant digits.
//   nurbs <degree> <n> x1 y1 w1 … xn yn wn <m> k1 … km   (flat knots) — sets the curve
//   ellipse cx cy mx my ratio t0 t1                       — sets an ellipse (DXF form)
//   circle cx cy r a0 a1                                  — sets a circle's arc
//   point u        → x y
//   derivs u       → x y dx dy ddx ddy
//   length a b     → length
//   closest x y    → u x y distance (the global minimum, GeomAPI_ProjectPointOnCurve)
//   split u        → n x1 y1 w1 … m k1 … km  (the part after u; Segment keeps the parameters)
//   domain         → first last
//   intersect      → count u1 … (the current curve with the line given next: lx ly dx dy)

#include <GCPnts_AbscissaPoint.hxx>
#include <Geom2dAdaptor_Curve.hxx>
#include <Geom2dAPI_InterCurveCurve.hxx>
#include <Geom2dAPI_ProjectPointOnCurve.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <Geom2d_Ellipse.hxx>
#include <Geom2d_Circle.hxx>
#include <Geom2d_Line.hxx>
#include <Geom2d_TrimmedCurve.hxx>
#include <GCPnts_AbscissaPoint.hxx>
#include <TColStd_Array1OfInteger.hxx>
#include <TColStd_Array1OfReal.hxx>
#include <TColgp_Array1OfPnt2d.hxx>
#include <gp_Elips2d.hxx>
#include <gp_Circ2d.hxx>
#include <gp_Ax22d.hxx>

#include <cmath>
#include <cstdio>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

static Handle(Geom2d_Curve) curve;
static Handle(Geom2d_BSplineCurve) spline;

static void out(const std::vector<double>& xs) {
  for (size_t i = 0; i < xs.size(); ++i) std::printf(i ? " %.17g" : "%.17g", xs[i]);
  std::printf("\n");
  std::fflush(stdout);
}

static void set_nurbs(std::istringstream& in) {
  int degree, n;
  in >> degree >> n;
  TColgp_Array1OfPnt2d poles(1, n);
  TColStd_Array1OfReal weights(1, n);
  for (int i = 1; i <= n; ++i) {
    double x, y, w;
    in >> x >> y >> w;
    poles(i) = gp_Pnt2d(x, y);
    weights(i) = w;
  }
  int m;
  in >> m;
  std::vector<double> flat(m);
  for (auto& k : flat) in >> k;
  std::vector<double> uniq;
  std::vector<int> mult;
  for (double k : flat) {
    if (!uniq.empty() && k == uniq.back()) ++mult.back();
    else { uniq.push_back(k); mult.push_back(1); }
  }
  TColStd_Array1OfReal knots(1, (int)uniq.size());
  TColStd_Array1OfInteger mults(1, (int)uniq.size());
  for (size_t i = 0; i < uniq.size(); ++i) { knots((int)i + 1) = uniq[i]; mults((int)i + 1) = mult[i]; }
  spline = new Geom2d_BSplineCurve(poles, weights, knots, mults, degree, false);
  curve = spline;
}

int main() {
  std::string line;
  while (std::getline(std::cin, line)) {
    std::istringstream in(line);
    std::string op;
    in >> op;
    try {
      if (op == "nurbs") {
        set_nurbs(in);
        out({});
      } else if (op == "ellipse") {
        double cx, cy, mx, my, ratio, t0, t1;
        in >> cx >> cy >> mx >> my >> ratio >> t0 >> t1;
        double a = std::hypot(mx, my);
        gp_Ax22d ax(gp_Pnt2d(cx, cy), gp_Dir2d(mx, my), gp_Dir2d(-my, mx));
        Handle(Geom2d_Ellipse) e = new Geom2d_Ellipse(gp_Elips2d(ax, a, a * ratio));
        if (t1 <= t0) t1 += 2 * M_PI;
        curve = new Geom2d_TrimmedCurve(e, t0, t1);
        spline.Nullify();
        out({});
      } else if (op == "circle") {
        double cx, cy, r, a0, a1;
        in >> cx >> cy >> r >> a0 >> a1;
        Handle(Geom2d_Circle) c = new Geom2d_Circle(gp_Circ2d(gp_Ax22d(gp_Pnt2d(cx, cy), gp_Dir2d(1, 0), gp_Dir2d(0, 1)), r));
        if (a1 <= a0) a1 += 2 * M_PI;
        curve = new Geom2d_TrimmedCurve(c, a0, a1);
        spline.Nullify();
        out({});
      } else if (op == "point") {
        double u; in >> u;
        gp_Pnt2d p = curve->Value(u);
        out({p.X(), p.Y()});
      } else if (op == "derivs") {
        double u; in >> u;
        gp_Pnt2d p; gp_Vec2d d1, d2;
        curve->D2(u, p, d1, d2);
        out({p.X(), p.Y(), d1.X(), d1.Y(), d2.X(), d2.Y()});
      } else if (op == "length") {
        double a, b; in >> a >> b;
        Geom2dAdaptor_Curve ad(curve);
        out({GCPnts_AbscissaPoint::Length(ad, a, b, 1e-14)});
      } else if (op == "closest") {
        double x, y; in >> x >> y;
        Geom2dAPI_ProjectPointOnCurve pr(gp_Pnt2d(x, y), curve);
        if (pr.NbPoints() == 0) { out({}); continue; }
        gp_Pnt2d p = pr.NearestPoint();
        out({pr.LowerDistanceParameter(), p.X(), p.Y(), pr.LowerDistance()});
      } else if (op == "domain") {
        out({curve->FirstParameter(), curve->LastParameter()});
      } else if (op == "split") {
        double u; in >> u;
        Handle(Geom2d_BSplineCurve) c = Handle(Geom2d_BSplineCurve)::DownCast(spline->Copy());
        c->Segment(u, spline->LastParameter());
        std::vector<double> r{(double)c->NbPoles()};
        for (int i = 1; i <= c->NbPoles(); ++i) {
          r.push_back(c->Pole(i).X()); r.push_back(c->Pole(i).Y()); r.push_back(c->Weight(i));
        }
        TColStd_Array1OfReal flat(1, c->NbPoles() + c->Degree() + 1);
        c->KnotSequence(flat);
        r.push_back(flat.Length());
        for (int i = flat.Lower(); i <= flat.Upper(); ++i) r.push_back(flat(i));
        out(r);
      } else if (op == "intersect") {
        double lx, ly, dx, dy; in >> lx >> ly >> dx >> dy;
        Handle(Geom2d_Line) l = new Geom2d_Line(gp_Pnt2d(lx, ly), gp_Dir2d(dx, dy));
        Geom2dAPI_InterCurveCurve ix(curve, l, 1e-12);
        std::vector<double> r{(double)ix.NbPoints()};
        for (int i = 1; i <= ix.NbPoints(); ++i) {
          gp_Pnt2d p = ix.Point(i);
          r.push_back(p.X()); r.push_back(p.Y());
        }
        out(r);
      } else {
        std::printf("error unknown %s\n", op.c_str());
        std::fflush(stdout);
      }
    } catch (Standard_Failure& f) {
      std::printf("error %s\n", f.GetMessageString());
      std::fflush(stdout);
    }
  }
}
