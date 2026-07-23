import sys
import time

import pytest


class TimingPlugin:
    def pytest_runtest_logreport(self, report):
        if report.when == "call":
            print(f"[{time.strftime('%H:%M:%S')}] {report.nodeid} {report.outcome} {report.duration:.2f}s", flush=True)


sys.exit(pytest.main(["tests/test_paket_b_e2e_http.py", "-q"], plugins=[TimingPlugin()]))
