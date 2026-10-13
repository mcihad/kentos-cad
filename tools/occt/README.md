# OpenCASCADE başvurusu

`occt_ref.cpp`, KentOS'un eğrilerini KentOS kodu olmadan OpenCASCADE'le değerlendiren küçük bir araçtır (ADR 0259):
`scripts/fixtures/nurbs_cases.py` ve `scripts/fixtures/curves_occt.py` onu satır satır sorar. İlk çalıştırmada
`nurbs_cases.py` kendisi derler; elle derlemek için (Homebrew'un OpenCASCADE 7.9'uyla):

```bash
clang++ -std=c++17 -O2 tools/occt/occt_ref.cpp -o .run/occt_ref -I/opt/homebrew/include/opencascade -L/opt/homebrew/lib -lTKernel -lTKMath -lTKG2d -lTKG3d -lTKGeomBase -lTKGeomAlgo -Wl,-rpath,/opt/homebrew/lib
```

İstek ve cevapların biçimi dosyanın başındadır.
