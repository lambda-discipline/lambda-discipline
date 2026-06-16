from pathlib import Path
import pytest


def pytest_addoption(parser):
    parser.addoption("--build-dir", type=str, action="store", default="cmake-build-debug",
                     help="CMake build directory (default: %(default)s)")


@pytest.fixture
def build_dir(request) -> Path:
    return Path(request.config.getoption("--build-dir"))
