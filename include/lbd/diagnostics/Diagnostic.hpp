#pragma once

#include <optional>
#include <vector>
#include <lbd/diagnostics/ContextFrame.hpp>
#include <lbd/diagnostics/Severity.hpp>
#include <lbd/source/Range.hpp>

namespace lbd::diagnostics
{
  class Diagnostic
  {
  public:
    Diagnostic(Severity severity, const std::optional<source::Range> &range, std::string message,
               const std::vector<ContextFrame> &contextStack);

    [[nodiscard]] Severity getSeverity() const noexcept { return m_severity; }

    [[nodiscard]] const std::optional<source::Range> &getRange() const noexcept { return m_range; }

    [[nodiscard]] std::string_view getMessage() const noexcept { return m_message; }

    [[nodiscard]] const std::vector<ContextFrame> &getContextStack() const noexcept { return m_contextStack; }

  private:
    Severity m_severity;
    std::optional<source::Range> m_range;
    std::string m_message;
    std::vector<ContextFrame> m_contextStack;
  };
}
