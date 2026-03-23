#pragma once

#include <vector>
#include <lbd/diagnostics/ContextFrame.hpp>
#include <lbd/diagnostics/Severity.hpp>
#include <lbd/source/Range.hpp>

namespace lbd::diagnostics
{
    class Diagnostic
    {
    public:
        Diagnostic(Severity severity, const source::Range& range, std::string message,
                   const std::vector<ContextFrame>& contextStack);

        [[nodiscard]] Severity getSeverity() const;

        [[nodiscard]] source::Range getRange() const;

        [[nodiscard]] std::string_view getMessage() const;

        [[nodiscard]] const std::vector<ContextFrame>& getContextStack() const;

    private:
        Severity severity;
        source::Range range;
        std::string message;
        std::vector<ContextFrame> contextStack;
    };
}
