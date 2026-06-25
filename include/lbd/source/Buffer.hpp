#pragma once

#include <string>
#include <vector>
#include <lbd/source/BufferId.hpp>
#include <lbd/source/Location.hpp>

namespace lbd::source
{
  class Buffer
  {
  public:
    Buffer(BufferId id, std::string name, std::string contents);

    [[nodiscard]] BufferId getId() const noexcept { return m_id; }

    [[nodiscard]] std::string_view getName() const noexcept { return m_name; }

    [[nodiscard]] std::string_view getContents() const noexcept { return m_contents; }

    [[nodiscard]] const std::vector<Offset> &getLineOffsets() const noexcept { return m_lineOffsets; }

    [[nodiscard]] size_t getSize() const noexcept { return m_contents.size(); }

    [[nodiscard]] char operator[](const size_t index) const noexcept { return m_contents[index]; }

    /**
     *Computes the starting offset of each line.
     *
     * The first entry is always `0`, representing the start of the first line.
     */
    void computeLineOffsets() noexcept;

  private:
    BufferId m_id;
    std::string m_name;
    std::string m_contents;
    std::vector<Offset> m_lineOffsets;
  };
}
