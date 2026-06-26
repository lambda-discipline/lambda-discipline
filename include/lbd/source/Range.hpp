#pragma once

#include <ostream>
#include <lbd/source/Location.hpp>

namespace lbd::source
{
  class Range
  {
  public:
    [[nodiscard]] Range(Location begin, Location end) noexcept;

    // TODO: Refactor diagnostics to support an optional source range, allowing this constructor to be removed.
    [[deprecated("Temporary constructor. Diagnostics will be refactored to support an optional source range.")]]
    [[nodiscard]] Range() noexcept;

    [[nodiscard]] Location getBegin() const noexcept { return m_begin; }

    [[nodiscard]] Location getEnd() const noexcept { return m_end; }

    friend std::ostream &operator<<(std::ostream &outputStream, const Range &range);

  private:
    Location m_begin;
    Location m_end;
  };
}
