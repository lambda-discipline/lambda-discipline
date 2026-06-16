#pragma once

#include <string>
#include <string_view>
#include <vector>
#include <lbd/source/BufferId.hpp>
#include <lbd/source/Location.hpp>

namespace lbd::source
{
  struct Buffer
  {
    BufferId id;
    std::string name;
    std::string contents;
    std::vector<Offset> lineOffsets;

    [[nodiscard]] size_t getSize() const;

    [[nodiscard]] char operator[](size_t index) const;

    void computeLineOffsets() noexcept;
  };
}