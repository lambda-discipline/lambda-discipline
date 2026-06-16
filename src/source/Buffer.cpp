#include <lbd/source/Buffer.hpp>

namespace lbd::source
{
  size_t Buffer::getSize() const { return contents.size(); }
  char Buffer::operator[](const size_t index) const { return contents[index]; }

  void Buffer::computeLineOffsets() noexcept
  {
    lineOffsets.clear();
    lineOffsets.push_back(0);
    for (uint32_t i = 0; i < contents.size(); ++i)
    {
      if (contents[i] == '\n') lineOffsets.push_back(i + 1);
    }
  }
}