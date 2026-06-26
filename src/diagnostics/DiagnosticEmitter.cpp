#include <lbd/Exceptions.hpp>
#include <lbd/diagnostics/DiagnosticEmitter.hpp>
#include <lbd/diagnostics/DiagnosticInfo.hpp>

namespace lbd::diagnostics
{
  DiagnosticEmitter::DiagnosticEmitter(DiagnosticRenderer &diagnosticRenderer, const bool exitOnError)
    : diagnosticRenderer(diagnosticRenderer), m_exitOnError(exitOnError) {}

  void DiagnosticEmitter::pushContext(source::Range range, std::string message)
  {
    contextStack.emplace_back(range, std::move(message));
  }

  void DiagnosticEmitter::popContext() { contextStack.pop_back(); }

  void DiagnosticEmitter::info(const std::optional<source::Range> &range, const std::string_view message) const
  {
    emit(Severity::INFO, range, message);
  }

  void DiagnosticEmitter::warning(const std::optional<source::Range> &range, const std::string_view message) const
  {
    emit(Severity::WARNING, range, message);
  }

  void DiagnosticEmitter::error(const std::optional<source::Range> &range, const std::string_view message) const
  {
    emit(Severity::ERR, range, message);
  }

  void DiagnosticEmitter::emit(const Severity severity, const DiagnosticId id,
                               const std::optional<source::Range> &range,
                               const fmt::dynamic_format_arg_store<fmt::format_context> &arguments) const
  {
    const std::string message = fmt::vformat(getDiagnosticInfo(id).message, arguments);
    emit(severity, range, message);
  }

  void DiagnosticEmitter::emit(const Severity severity, const std::optional<source::Range> &range,
                               const std::string_view message) const
  {
    const Diagnostic diagnostic(severity, range, std::string(message), contextStack);
    diagnosticRenderer.render(diagnostic);

    if (severity == Severity::ERR)
    {
      if (m_exitOnError) std::exit(EXIT_FAILURE);
      throw ControlledExit();
    }
  }
}
