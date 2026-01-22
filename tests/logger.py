import sys


def useColor() -> bool:
    return sys.stdout.isatty()


def logInternal(ansiColor: str, logLevel: str, message: str):
    if useColor():
        print(f"[{ansiColor}{logLevel}\033[0m] {message}")
    else:
        print(f"[{logLevel}] {message}")


def error(message: str):
    logInternal("\033[31m", "ERROR", message)


def info(message: str):
    logInternal("\033[34m", "INFO", message)


def warning(message: str):
    logInternal("\033[33m", "WARNING", message)


def success(message: str):
    logInternal("\033[32m", "SUCCESS", message)
