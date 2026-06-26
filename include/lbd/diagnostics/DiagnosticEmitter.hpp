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
    DiagnosticEmitter(DiagnosticRenderer &diagnosticRenderer, bool exitOnError);

    void pushContext(source::Range range, std::string message);

    void popContext();

    [[nodiscard]] bool shouldExitOnError() const noexcept { return m_exitOnError; }

    void setExitOnError(const bool exitOnError) noexcept { m_exitOnError = exitOnError; }

    // Message only.
    void info(const std::string_view message) const { emit(Severity::INFO, std::nullopt, message); }

    void warning(const std::string_view message) const { emit(Severity::WARNING, std::nullopt, message); }

    [[noreturn]] void error(const std::string_view message) const { emit(Severity::ERR, std::nullopt, message); }

    // Range + message.
    void info(const std::optional<source::Range> &range, std::string_view message) const;

    void warning(const std::optional<source::Range> &range, std::string_view message) const;

    [[noreturn]] void error(const std::optional<source::Range> &range, std::string_view message) const;

    // Range + DiagnosticId + format args.
    template<typename... Arguments>
    void info(const std::optional<source::Range> &range, DiagnosticId id, Arguments &&... args) const
    {
      emitImpl(Severity::INFO, range, id, std::forward<Arguments>(args)...);
    }

    template<typename... Arguments>
    void warning(const std::optional<source::Range> &range, DiagnosticId id, Arguments &&... args) const
    {
      emitImpl(Severity::WARNING, range, id, std::forward<Arguments>(args)...);
    }

    template<typename... Arguments>
    [[noreturn]] void error(const std::optional<source::Range> &range, DiagnosticId id, Arguments &&... args) const
    {
      emitImpl(Severity::ERR, range, id, std::forward<Arguments>(args)...);
    }

  private:
    template<typename... Arguments>
    void emitImpl(const Severity severity, const std::optional<source::Range> &range, const DiagnosticId id,
                  Arguments &&... arguments) const
    {
      fmt::dynamic_format_arg_store<fmt::format_context> store;
      (store.push_back(std::forward<Arguments>(arguments)), ...);

      emit(severity, id, range, store);
    }

    void emit(Severity severity, DiagnosticId id, const std::optional<source::Range> &range,
              const fmt::dynamic_format_arg_store<fmt::format_context> &arguments) const;

    void emit(Severity severity, const std::optional<source::Range> &range, std::string_view message) const;

    DiagnosticRenderer &diagnosticRenderer;
    std::vector<ContextFrame> contextStack;
    bool m_exitOnError;
  };
}
