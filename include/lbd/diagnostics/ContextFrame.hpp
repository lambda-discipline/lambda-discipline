#pragma once

#include <string>
#include <lbd/source/Range.hpp>

namespace lbd::diagnostics
{
    class ContextFrame
    {
    public:
        ContextFrame(const source::Range& range, std::string message);

        [[nodiscard]] source::Range getRange() const;

        [[nodiscard]] std::string getMessage() const;

    private:
        source::Range range;
        std::string message;
    };
}
