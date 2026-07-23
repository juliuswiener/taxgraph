import os
import resource
import sys
import time

import pytest


class TimingPlugin:
    def pytest_runtest_logreport(self, report):
        if report.when == "call":
            rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
            print(f"[{time.strftime('%H:%M:%S')}] {report.nodeid} {report.outcome} "
                  f"{report.duration:.2f}s rss={rss}kb", flush=True)


t0 = time.time()
rc = pytest.main(["tests/test_paket_b_e2e_http.py", "-q"], plugins=[TimingPlugin()])
print(f"TOTAL {time.time() - t0:.1f}s RC={rc}", flush=True)
sys.exit(rc)
