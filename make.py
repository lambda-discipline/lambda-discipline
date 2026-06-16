import argparse
import subprocess
import sys
from pathlib import Path


def log(level, message):
    print(f"[{level}] {message}")


def error(message, should_exit=False):
    log("ERROR", message)
    if should_exit:
        sys.exit(1)


def info(message):
    log("INFO", message)


ROOT = Path(__file__).resolve().parent


def python():
    venv_dir = Path(VENV_DIR)
    if sys.platform == "win32":
        return str(venv_dir / "Scripts" / "python.exe")
    else:
        return str(venv_dir / "bin" / "python")


def subcommand_setup():
    build_dir = Path(BUILD_DIR)
    venv_dir = Path(VENV_DIR)
    build_dir.mkdir(parents=True, exist_ok=True)

    subprocess.run([sys.executable, "-m", "venv", str(venv_dir)], check=True)
    subprocess.run([python(), "-m", "pip", "install", "-r", "requirements.dev.txt"], check=True)

    try:
        subprocess.run(["cmake", "-S", ".", "-B", str(build_dir)], check=True)
    except subprocess.CalledProcessError as e:
        error(f"CMake configure failed with exit code {e.returncode}")


def subcommand_build():
    build_dir = Path(BUILD_DIR)
    subprocess.run(["cmake", "--build", str(build_dir)], check=True)


def subcommand_test():
    subprocess.run([python(), "-m", "pytest", f'--build-dir={BUILD_DIR}', "-v"], cwd=ROOT, check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(prog="make.py", description="Lambda Discipline build tool")
    parser.add_argument("--build-dir", default="cmake-build-debug", help="CMake build directory (default: %(default)s)")
    parser.add_argument("--venv-dir", default="venv",
                        help="Python virtual environment directory (default: %(default)s)")

    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("setup", help="Create venv, install dependencies, and configure CMake")
    subparsers.add_parser("build", help="Build the project")
    subparsers.add_parser("test", help="Run tests")

    args = parser.parse_args()
    BUILD_DIR = args.build_dir
    VENV_DIR = args.venv_dir

    match args.command:
        case "setup":
            subcommand_setup()
        case "build":
            subcommand_build()
        case "test":
            subcommand_test()
