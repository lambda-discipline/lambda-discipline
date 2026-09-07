#!/usr/bin/env python3

import argparse
import os
import platform
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CONFIG_FILE = ROOT / "profiles.toml"


class Colors:
    RESET = "\033[0m"
    RED = "\033[31m"
    GREEN = "\033[32m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"


def color_enabled():
    if os.environ.get("NO_COLOR") is not None:
        return False

    if os.environ.get("FORCE_COLOR") is not None:
        return os.environ["FORCE_COLOR"] != "0"

    return sys.stdout.isatty()


USE_COLOR = color_enabled()


def log(level: str, message: str):
    colors = {
        "info": Colors.BLUE,
        "error": Colors.RED,
    }

    label = level

    if USE_COLOR:
        label = f"{colors.get(level, '')}{level}{Colors.RESET}"

    print(f"{label}: {message}")


def info(message: str):
    log("info", message)


def error(message: str):
    log("error", message)


def load_profiles():
    try:
        import tomllib
    except ImportError:
        import tomli as tomllib

    try:
        with CONFIG_FILE.open("rb") as file:
            return tomllib.load(file)
    except FileNotFoundError:
        error(f"configuration file not found: {CONFIG_FILE}")
        sys.exit(1)
    except tomllib.TOMLDecodeError as exc:
        error(f"invalid configuration in {CONFIG_FILE}: {exc}")
        sys.exit(1)


CONFIG = load_profiles()


def run(command: list[str], *, cwd: Path = ROOT):
    formatted = " ".join(shlex.quote(str(arg)) for arg in command)
    info(f"running: {formatted}")

    try:
        subprocess.run(
            command,
            cwd=cwd,
            check=True,
        )
    except FileNotFoundError:
        error(f"command not found: {command[0]}")
        sys.exit(1)
    except subprocess.CalledProcessError as exc:
        error(f"command failed with exit code {exc.returncode}")
        sys.exit(exc.returncode)


def require_command(command: str):
    if shutil.which(command) is None:
        error(f"required command not found: {command}")
        sys.exit(1)


def default_toolchain():
    match platform.system():
        case "Windows":
            return "mingw"
        case "Linux":
            return "gcc"
        case "Darwin":
            return "clang"
        case system:
            error(f"unsupported host platform: {system}")
            sys.exit(1)


def python_executable():
    if sys.platform == "win32":
        return VENV_DIR / "Scripts" / "python.exe"

    return VENV_DIR / "bin" / "python"


def build_directory():
    return ROOT / "build" / f"cmake-build-{PROFILE_NAME}-{TOOLCHAIN_NAME}"


def is_multi_config_generator(generator: str) -> bool:
    if not generator:
        return False

    return generator.startswith("Visual Studio") or generator == "Xcode"


def validate_configuration():
    toolchains = CONFIG.get("toolchains", {})
    profiles = CONFIG.get("profiles", {})

    if TOOLCHAIN_NAME not in toolchains:
        error(f"unknown toolchain: {TOOLCHAIN_NAME}")
        error(f"available toolchains: {', '.join(sorted(toolchains))}")
        sys.exit(1)

    if PROFILE_NAME not in profiles:
        error(f"unknown profile: {PROFILE_NAME}")
        error(f"available profiles: {', '.join(sorted(profiles))}")
        sys.exit(1)

    toolchain = toolchains[TOOLCHAIN_NAME]
    profile = profiles[PROFILE_NAME]

    build_type = profile.get("build_type")
    if not build_type:
        error(f"profile '{PROFILE_NAME}' has no 'build_type'")
        sys.exit(1)

    generator = toolchain.get("generator")

    if generator is not None and not isinstance(generator, str):
        error(f"invalid generator for toolchain '{TOOLCHAIN_NAME}'")
        sys.exit(1)


def cmake_configure_command():
    generator = TOOLCHAIN.get("generator")

    command = [
        "cmake",
        "-S",
        str(ROOT),
        "-B",
        str(build_directory()),
    ]

    if generator:
        command.extend(["-G", generator])

    if not is_multi_config_generator(generator):
        command.append(f"-DCMAKE_BUILD_TYPE={PROFILE['build_type']}")

    return command


def cmake_build_command():
    generator = TOOLCHAIN.get("generator")

    command = [
        "cmake",
        "--build",
        str(build_directory()),
    ]

    if is_multi_config_generator(generator):
        command.extend(
            [
                "--config",
                PROFILE["build_type"],
            ]
        )

    if FORCE_BUILD:
        command.append("--clean-first")

    return command


def subcommand_setup():
    require_command("cmake")

    build_directory().mkdir(parents=True, exist_ok=True)

    venv = python_executable()

    if not venv.exists():
        info(f"creating virtual environment: {VENV_DIR}")

        run(
            [
                sys.executable,
                "-m",
                "venv",
                str(VENV_DIR),
            ]
        )
    else:
        info(f"using existing virtual environment: {VENV_DIR}")

    if not venv.exists():
        error(f"virtual environment was not created: {venv}")
        sys.exit(1)

    run(
        [
            str(venv),
            "-m",
            "pip",
            "install",
            "--upgrade",
            "pip",
        ]
    )

    run(
        [
            str(venv),
            "-m",
            "pip",
            "install",
            "-r",
            str(ROOT / "requirements.dev.txt"),
        ]
    )

    run(cmake_configure_command())


def subcommand_build():
    require_command("cmake")

    if not build_directory().exists():
        error(f"build directory does not exist: {build_directory()}")
        error("run the 'setup' command first")
        sys.exit(1)

    run(cmake_build_command())


def subcommand_test():
    venv = python_executable()

    if not venv.exists():
        error(f"virtual environment not found: {VENV_DIR}")
        error("run the 'setup' command first")
        sys.exit(1)

    if not build_directory().exists():
        error(f"build directory does not exist: {build_directory()}")
        error("run the 'setup' command first")
        sys.exit(1)

    run(
        [
            str(venv),
            "-m",
            "pytest",
            f"--build-dir={build_directory()}",
            "-v",
        ]
    )


def main():
    parser = argparse.ArgumentParser(
        prog="make.py",
        description="Lambda Discipline build tool",
    )

    parser.add_argument(
        "--venv-dir",
        default=ROOT / "venv",
        type=Path,
        help="Python virtual environment directory",
    )

    parser.add_argument(
        "--toolchain",
        default=default_toolchain(),
        help="toolchain name",
    )

    parser.add_argument(
        "--profile",
        default=CONFIG["defaults"]["profile"],
        help="build profile",
    )

    parser.add_argument(
        "--force",
        action="store_true",
        help="clean before building",
    )

    subparsers = parser.add_subparsers(
        dest="command",
        required=True,
    )

    subparsers.add_parser(
        "setup",
        help="create the environment and configure CMake",
    )

    subparsers.add_parser(
        "build",
        help="build the project",
    )

    subparsers.add_parser(
        "test",
        help="run the test suite",
    )

    args = parser.parse_args()

    global VENV_DIR
    global FORCE_BUILD
    global TOOLCHAIN_NAME
    global PROFILE_NAME
    global TOOLCHAIN
    global PROFILE

    VENV_DIR = args.venv_dir.resolve()
    FORCE_BUILD = args.force

    TOOLCHAIN_NAME = args.toolchain
    PROFILE_NAME = args.profile

    validate_configuration()

    TOOLCHAIN = CONFIG["toolchains"][TOOLCHAIN_NAME]
    PROFILE = CONFIG["profiles"][PROFILE_NAME]

    info(f"toolchain: {TOOLCHAIN_NAME}")
    info(f"profile: {PROFILE_NAME}")
    info(f"build directory: {build_directory()}")
    info(f"virtual environment: {VENV_DIR}")

    match args.command:
        case "setup":
            subcommand_setup()

        case "build":
            subcommand_build()

        case "test":
            subcommand_test()

        case _:
            pass


if __name__ == "__main__":
    main()
