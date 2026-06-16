import re
import subprocess
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
TESTS = ROOT / "tests"

def discover_tests():
    return sorted(TESTS.rglob("*.test.toml"))


def make_test_id(path: Path) -> str:
    return str(path.relative_to(TESTS))


def lbd(build_dir: Path) -> str:
    candidates = [
        build_dir / "lbd",
        build_dir / "Debug" / "lbd",
        build_dir / "Debug" / "lbd.exe",
        build_dir / "lbd.exe",
    ]

    exe = next((p for p in candidates if p.exists()), None)
    if exe is None:
        raise FileNotFoundError("Could not locate lbd executable")
    return str(exe)


@pytest.mark.parametrize(
    "manifest_path",
    discover_tests(),
    ids=make_test_id,
)
def test_program(manifest_path: Path, build_dir: Path):
    manifest = tomllib.loads(manifest_path.read_text())

    source = manifest_path.parent / manifest["file"]

    result = subprocess.run(
        [lbd(build_dir), "run", str(source)],
        capture_output=True,
        text=True,
    )

    expect = manifest["expect"]

    if "exit_code" in expect:
        assert result.returncode == expect["exit_code"]

    if "stdout" in expect:
        assert result.stdout == expect["stdout"]

    if "stderr" in expect:
        assert result.stderr == expect["stderr"]

    if "stdout_contains" in expect:
        values = expect["stdout_contains"]
        if isinstance(values, str):
            values = [values]

        for value in values:
            assert value in result.stdout

    if "stderr_contains" in expect:
        values = expect["stderr_contains"]
        if isinstance(values, str):
            values = [values]

        for value in values:
            assert value in result.stderr

    if "stdout_regex" in expect:
        assert re.search(expect["stdout_regex"], result.stdout)

    if "stderr_regex" in expect:
        assert re.search(expect["stderr_regex"], result.stderr)
