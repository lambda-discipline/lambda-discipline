#include <lbd/source/Range.hpp>

namespace lbd::source
{
    Range::Range(const Location begin, const Location end) noexcept
        : begin(begin.offset <= end.offset ? begin : end), end(begin.offset <= end.offset ? end : begin) {}

    Range::Range(const Location location) noexcept : begin(location), end(location) {}
    Range::Range() noexcept : begin({}), end({}) {}

    std::ostream& operator<<(std::ostream& outputStream, const Range& range)
    {
        return outputStream << range.begin.offset << ".." << range.end.offset;
    }
}
