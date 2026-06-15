from pathlib import Path
import subprocess
import re
import tomllib
import pytest

ROOT = Path(__file__).resolve().parent.parent
TESTS = ROOT / "tests"
LBD = ROOT / "cmake-build-debug" / "lbd"


def discover_tests():
    return sorted(TESTS.rglob("*.test.toml"))


def make_test_id(path: Path) -> str:
    return str(path.relative_to(TESTS))


@pytest.mark.parametrize(
    "manifest_path",
    discover_tests(),
    ids=make_test_id,
)
def test_program(manifest_path: Path):
    manifest = tomllib.loads(manifest_path.read_text())

    source = manifest_path.parent / manifest["file"]

    result = subprocess.run(
        [str(LBD), "run", str(source)],
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
