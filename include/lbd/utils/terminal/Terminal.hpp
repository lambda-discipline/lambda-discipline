#pragma once

#ifdef _WIN32
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <io.h>
#include <windows.h>
#else
#include <unistd.h>
#endif

#include <cstdio>

namespace lbd::utils::terminal
{
  inline bool enableVirtualTerminal() noexcept
  {
#if defined(_WIN32)
    const HANDLE output_handle = GetStdHandle(STD_OUTPUT_HANDLE); // NOLINT(*-misplaced-const)
    if (output_handle == INVALID_HANDLE_VALUE)
    {
      return false;
    }
    DWORD dwMode = 0;
    if (!GetConsoleMode(output_handle, &dwMode))
    {
      return false;
    }
    dwMode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    if (!SetConsoleMode(output_handle, dwMode))
    {
      return false;
    }
#endif
    return true;
  }

  inline bool isTTY() noexcept
  {
#ifdef _WIN32
    return _isatty(_fileno(stderr));
#else
    return isatty(fileno(stderr));
#endif
  }

  // TODO: Fix this.
  inline bool supportsUnicode() noexcept { return isTTY(); }
}