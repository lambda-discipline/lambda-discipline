#include <ranges>
#include <lbd/diagnostics/Diagnostic.hpp>
#include <lbd/diagnostics/Severity.hpp>

namespace lbd::diagnostics
{
  Diagnostic::Diagnostic(const Severity severity, const std::optional<source::Range> &range, std::string message,
                         const std::vector<ContextFrame> &contextStack)
    : m_severity(severity), m_range(range), m_message(std::move(message)), m_contextStack(contextStack) {}
}
