#!/usr/bin/env python3

import argparse
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CONFIG_FILE = ROOT / "profiles.toml"


################################################################################
# Logging
################################################################################


def log(level: str, message: str):
    print(f"[{level}] {message}")


def info(message: str):
    log("INFO", message)


def error(message: str, should_exit=False):
    log("ERROR", message)
    if should_exit:
        sys.exit(1)


################################################################################
# Configuration
################################################################################


def load_profiles():
    try:
        import tomllib
    except ImportError:
        import tomli as tomllib

    with open(CONFIG_FILE, "rb") as f:
        return tomllib.load(f)


CONFIG = load_profiles()


################################################################################
# Helpers
################################################################################


def python():
    if sys.platform == "win32":
        return str(Path(VENV_DIR) / "Scripts" / "python.exe")
    return str(Path(VENV_DIR) / "bin" / "python")


def build_directory():
    return Path("build") / f"cmake-build-{PROFILE_NAME}-{TOOLCHAIN_NAME}"


def default_toolchain():
    match platform.system():
        case "Windows":
            return "mingw"
        case "Linux":
            return "gcc"
        case "Darwin":
            return "clang"

    error(f"Unsupported host platform '{platform.system()}'", True)
    return None


def cmake_configure_command():
    command = [
        "cmake",
        "-S",
        ".",
        "-B",
        str(build_directory()),
    ]

    generator = TOOLCHAIN.get("generator")
    if generator:
        command.extend(["-G", generator])

    command.append(
        f"-DCMAKE_BUILD_TYPE={PROFILE['build_type']}"
    )

    return command


################################################################################
# Commands
################################################################################


def subcommand_setup():
    build_directory().mkdir(parents=True, exist_ok=True)

    subprocess.run(
        [sys.executable, "-m", "venv", VENV_DIR],
        check=True,
    )

    subprocess.run(
        [
            python(),
            "-m",
            "pip",
            "install",
            "-r",
            "requirements.dev.txt",
        ],
        check=True,
    )

    subprocess.run(
        cmake_configure_command(),
        check=True,
    )


def subcommand_build():
    command = [
        "cmake",
        "--build",
        str(build_directory()),
    ]

    if FORCE_BUILD:
        command.append("--clean-first")

    subprocess.run(command, check=True)


def subcommand_test():
    subprocess.run(
        [
            python(),
            "-m",
            "pytest",
            f"--build-dir={build_directory()}",
            "-v",
        ],
        cwd=ROOT,
        check=True,
    )


################################################################################
# Main
################################################################################


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        prog="make.py",
        description="Lambda Discipline build tool",
    )

    parser.add_argument(
        "--venv-dir",
        default="venv",
        help="Python virtual environment",
    )

    parser.add_argument(
        "--toolchain",
        default=default_toolchain(),
        help="Toolchain name",
    )

    parser.add_argument(
        "--profile",
        default=CONFIG["defaults"]["profile"],
        help="Build profile",
    )

    parser.add_argument(
        "--force",
        action="store_true",
        help="Clean before build",
    )

    parser.add_argument(
        "--warnings-as-errors",
        action="store_true",
        help="Treat warnings as errors",
    )

    subparsers = parser.add_subparsers(
        dest="command",
        required=True,
    )

    subparsers.add_parser("setup")
    subparsers.add_parser("build")
    subparsers.add_parser("test")

    args = parser.parse_args()

    VENV_DIR = args.venv_dir
    FORCE_BUILD = args.force

    TOOLCHAIN_NAME = args.toolchain
    PROFILE_NAME = args.profile

    try:
        TOOLCHAIN = CONFIG["toolchains"][TOOLCHAIN_NAME]
    except KeyError:
        error(f"Unknown toolchain '{TOOLCHAIN_NAME}'", True)

    try:
        PROFILE = CONFIG["profiles"][PROFILE_NAME]
    except KeyError:
        error(f"Unknown profile '{PROFILE_NAME}'", True)

    info(f"Toolchain : {TOOLCHAIN_NAME}")
    info(f"Profile   : {PROFILE_NAME}")
    info(f"Build Dir : {build_directory()}")

    match args.command:
        case "setup":
            subcommand_setup()

        case "build":
            subcommand_build()

        case "test":
            subcommand_test()
