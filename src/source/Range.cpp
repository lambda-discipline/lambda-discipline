#include <lbd/source/Range.hpp>

namespace lbd::source
{
    Range::Range(const Location& begin, const Location& end) : begin(begin), end(end) {}

    bool Range::isValid() const { return begin.getOffset() < end.getOffset(); }

    Location Range::getBegin() const { return begin; }

    Location Range::getEnd() const { return end; }

    std::ostream& operator<<(std::ostream& outputStream, const Range& range)
    {
        // TODO: Improve this.
        outputStream << "  --> offset " << range.getBegin().getOffset() << ".." << range.getEnd().getOffset();
        return outputStream;
    }
}
