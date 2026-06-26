#include <lbd/source/Range.hpp>

namespace lbd::source
{
  Range::Range(const Location begin, const Location end) noexcept
    : m_begin(begin.offset <= end.offset ? begin : end), m_end(begin.offset <= end.offset ? end : begin) {}

  Range::Range() noexcept : m_begin({}), m_end({}) {}

  std::ostream &operator<<(std::ostream &outputStream, const Range &range)
  {
    return outputStream << range.m_begin.offset << ".." << range.m_end.offset;
  }
}
