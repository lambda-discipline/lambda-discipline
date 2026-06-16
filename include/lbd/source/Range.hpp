#pragma once

#include <iostream>
#include <lbd/source/Location.hpp>

namespace lbd::source
{
  struct Range
  {
    Location begin;
    Location end;

    [[nodiscard]] Range(Location begin, Location end) noexcept;

    // For debugging.
    [[nodiscard]] explicit Range(Location location) noexcept;

    [[nodiscard]] Range() noexcept;

    friend std::ostream &operator<<(std::ostream &outputStream, const Range &range);
  };
}