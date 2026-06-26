#pragma once

#include <utility>
#include <vector>
#include <fmt/args.h>
#include <fmt/format.h>
#include <lbd/diagnostics/ContextFrame.hpp>
#include <lbd/diagnostics/Diagnostic.hpp>
#include <lbd/diagnostics/DiagnosticId.hpp>
#include <lbd/diagnostics/DiagnosticRenderer.hpp>
#include <lbd/diagnostics/Severity.hpp>

namespace lbd::diagnostics
{
  class DiagnosticEmitter
  {
  public:
    explicit DiagnosticEmitter(DiagnosticRenderer &diagnosticRenderer);

    void pushContext(source::Range range, std::string message);

    void popContext();

    /// NOTE: Use this only to report errors caused by external systems (e.g. filesystem or I/O), not by language diagnostics.
    [[noreturn]] void error(const source::Range &range, const std::string &message) const noexcept;

    template<typename... Args>
    [[noreturn]] void error(const source::Range &range, const DiagnosticId id, Args &&... args)
    {
      fmt::dynamic_format_arg_store<fmt::format_context> store;
      (store.push_back(std::forward<Args>(args)), ...);
      emit(Severity::ERR, id, range, store);
      unreachable_impl();
    }

    template<typename... Args>
    [[noreturn]] void warning(const source::Range &range, const DiagnosticId id, Args &&... args)
    {
      fmt::dynamic_format_arg_store<fmt::format_context> store;
      (store.push_back(std::forward<Args>(args)), ...);
      emit(Severity::WARNING, id, range, store);
    }

    void emit(Severity severity, DiagnosticId id, const source::Range &range,
              const fmt::dynamic_format_arg_store<fmt::format_context> &arguments) const;

    void emit(Severity severity, const source::Range &range, const std::string &message) const noexcept;

  private:
    DiagnosticRenderer &diagnosticRenderer;
    std::vector<ContextFrame> contextStack;
  };
}
