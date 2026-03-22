#pragma once

#include <iostream>
#include <lbd/source/Location.hpp>

namespace lbd::source
{
    class Range
    {
    public:
        Range(const Location& begin, const Location& end);

        [[nodiscard]] bool isValid() const;

        [[nodiscard]] Location getBegin() const;

        [[nodiscard]] Location getEnd() const;

        friend std::ostream& operator<<(std::ostream& outputStream, const Range& range);

    private:
        Location begin;
        Location end;
    };
}
