#!/usr/bin/env python3

from __future__ import annotations
import argparse
import shutil
import subprocess
from pathlib import Path
import sys

APP_ID = "fun.ninth.lambda-discipline"
EXECUTABLE = "lbd"


def run(*args: str) -> None:
    subprocess.run(args, check=True)


def copy_file(src: Path, dst: Path) -> None:
    if not src.exists():
        sys.exit(f"[ERROR] Path '{src}' does not exist")

    shutil.copy2(src, dst)


def get_appimagetool() -> str:
    appimagetool = shutil.which("appimagetool")

    if appimagetool is None:
        appimagetool = shutil.which("appimagetool-x86_64.AppImage")

    if appimagetool is None:
        sys.exit(
            "[ERROR] Could not find 'appimagetool' or 'appimagetool-x86_64.AppImage' in PATH."
        )

    return appimagetool


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create an AppImage from an existing CMake build.",
        epilog=(
            "Example:\n  build-appimage.py build\n  build-appimage.py build-release"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    parser.add_argument(
        "build_dir",
        type=Path,
        metavar="BUILD_DIR",
        help="Path to the CMake build directory.",
    )

    parser.add_argument(
        "release_dir",
        type=Path,
        metavar="RELEASE_DIR",
        help="Directory where the AppDir and AppImage will be created.",
    )

    args = parser.parse_args()

    build_dir = args.build_dir.resolve()
    release_dir = args.release_dir.resolve()

    executable = build_dir / EXECUTABLE
    appdir = release_dir / "AppDir"

    if not executable.exists():
        sys.exit(f"[ERROR] Executable '{executable}' not found")

    if appdir.exists():
        shutil.rmtree(appdir)
    appdir.mkdir(parents=True, exist_ok=True)

    packaging = Path("packaging/appimage")

    copy_file(packaging / "AppRun", appdir / "AppRun")
    copy_file(packaging / f"{APP_ID}.desktop", appdir / f"{APP_ID}.desktop")
    copy_file(packaging / f"{APP_ID}.png", appdir / f"{APP_ID}.png")
    copy_file(executable, appdir / EXECUTABLE)

    (appdir / "AppRun").chmod(0o755)
    (appdir / EXECUTABLE).chmod(0o755)

    run(
        get_appimagetool(),
        str(appdir),
        str(release_dir / "Lambda_Discipline-x86_64.AppImage")
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
