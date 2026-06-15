#include <lbd/diagnostics/DiagnosticEmitter.hpp>
#include <lbd/diagnostics/DiagnosticInfo.hpp>

namespace lbd::diagnostics
{
    DiagnosticEmitter::DiagnosticEmitter(DiagnosticRenderer& diagnosticRenderer) : diagnosticRenderer(
        diagnosticRenderer) {}

    void DiagnosticEmitter::pushContext(source::Range range, std::string message)
    {
        contextStack.emplace_back(range, std::move(message));
    }

    void DiagnosticEmitter::popContext() { contextStack.pop_back(); }

    void DiagnosticEmitter::emit(const Severity severity, const DiagnosticId id, const source::Range& range,
                                 const fmt::dynamic_format_arg_store<fmt::format_context>& arguments) const
    {
        const std::string message = fmt::vformat(getDiagnosticInfo(id).message, arguments);
        const Diagnostic diagnostic(severity, range, message, contextStack);
        diagnosticRenderer.render(diagnostic);
        if (severity == Severity::ERR) std::exit(EXIT_FAILURE);
    }
}
