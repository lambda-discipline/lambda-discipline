#include <lbd/source/Buffer.hpp>

namespace lbd::source
{
  Buffer::Buffer(const BufferId id, std::string name, std::string contents)
    : m_id(id), m_name(std::move(name)), m_contents(std::move(contents))
  {
    computeLineOffsets();
  }

  void Buffer::computeLineOffsets() noexcept
  {
    m_lineOffsets.clear();
    m_lineOffsets.push_back(0);

    for (Offset i = 0; i < m_contents.size(); ++i)
    {
      if (m_contents[i] == '\r')
      {
        // CRLF
        if (i + 1 < m_contents.size() && m_contents[i + 1] == '\n') ++i;
        // else CR
        m_lineOffsets.push_back(i + 1);
      } else if (m_contents[i] == '\n')
      {
        // LF
        m_lineOffsets.push_back(i + 1);
      }
    }
  }
}
