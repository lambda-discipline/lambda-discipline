#include <ranges>
#include <lbd/diagnostics/Diagnostic.hpp>
#include <lbd/diagnostics/Severity.hpp>

namespace lbd::diagnostics
{
  Diagnostic::Diagnostic(const Severity severity, const source::Range &range, std::string message,
                         const std::vector<ContextFrame> &contextStack)
    : severity(severity), range(range), message(std::move(message)), contextStack(contextStack) {}

  Severity Diagnostic::getSeverity() const { return severity; }

  source::Range Diagnostic::getRange() const { return range; }

  std::string_view Diagnostic::getMessage() const { return message; }

  const std::vector<ContextFrame> &Diagnostic::getContextStack() const { return contextStack; }
}