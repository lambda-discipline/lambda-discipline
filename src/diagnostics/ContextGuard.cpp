#include <lbd/diagnostics/ContextGuard.hpp>

namespace lbd::diagnostics
{
    ContextGuard::ContextGuard(DiagnosticEmitter& diagnosticEmitter, const source::Range& range, std::string message)
        : diagnosticEmitter(diagnosticEmitter)
    {
        diagnosticEmitter.pushContext(range, std::move(message));
    }

    ContextGuard::~ContextGuard() { diagnosticEmitter.popContext(); }
}
