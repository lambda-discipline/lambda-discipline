#pragma once

#include <string>
#include <lbd/diagnostics/DiagnosticEmitter.hpp>
#include <lbd/source/Range.hpp>

namespace lbd::diagnostics
{
  class [[nodiscard]] ContextGuard
  {
  public:
    ContextGuard(DiagnosticEmitter &diagnosticEmitter, const source::Range &range, std::string message);

    ~ContextGuard();

  private:
    DiagnosticEmitter &diagnosticEmitter;
  };
}