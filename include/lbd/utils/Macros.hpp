#pragma once

namespace lbd
{
  [[noreturn]] inline void unreachable_impl()
  {
#if defined(_MSC_VER) && !defined(__clang__)
    __assume(false);
#else
    __builtin_unreachable();
#endif
  }
}