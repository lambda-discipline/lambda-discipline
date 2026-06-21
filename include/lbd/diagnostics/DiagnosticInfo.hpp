#pragma once

#include <cstddef>
#include <iterator>
#include <lbd/diagnostics/DiagnosticId.hpp>

namespace lbd::diagnostics
{
  struct DiagnosticInfo
  {
    const char *message;
  };

  inline constexpr DiagnosticInfo DIAGNOSTIC_INFOS[] = {
#define DIAGNOSTIC(id, message) {message},

#include <lbd/resources/Diagnostics.def>
#undef DIAGNOSTIC
};

inline const DiagnosticInfo &getDiagnosticInfo(DiagnosticId id)
{
  return DIAGNOSTIC_INFOS[static_cast<std::size_t>(id)];
}

static_assert(
  static_cast<std::size_t>(DiagnosticId::COUNT) == std::size(DIAGNOSTIC_INFOS),
  "Mismatch between DiagnosticId and DiagnosticInfo table."
);
}
