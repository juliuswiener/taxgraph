"""Startet `produkt/haut/server.py` mit festem `store._now()` (Differenz-Harness `api_http_paritaet`).

Jedes abgeleitete Event traegt `_now()`; seine Kennung haengt am Zeitstempel. Zwei Server, die
dasselbe tun, kaemen sonst auf zwei Kennungen. `TAXGRAPH_JETZT` ist der feste Text, den auch der
Rust-Testbau (`--features festzeit`) liest. Aufruf wie `server.py`: `server.py <port>`.
"""
import os
import runpy
import sys

repo = os.getcwd()
for d in ("produkt/store", "produkt/auth", "produkt/haut"):
    sys.path.insert(0, os.path.join(repo, d))
import store  # noqa: E402

FEST = os.environ["TAXGRAPH_JETZT"]
store._now = lambda: FEST  # noqa: SLF001
sys.argv = ["produkt/haut/server.py", *sys.argv[1:]]
runpy.run_path("produkt/haut/server.py", run_name="__main__")
