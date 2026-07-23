import faulthandler
import sys

import pytest

dumpf = open("/tmp/diag_hang_dump.log", "w", buffering=1)
faulthandler.dump_traceback_later(15, repeat=True, file=dumpf)
sys.exit(pytest.main(["tests/test_paket_b_e2e_http.py", "-q", "-x", "-v"]))
