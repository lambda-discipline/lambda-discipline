#pragma once

#include <deque>
#include <string>
#include <unordered_map>
#include <lbd/source/Buffer.hpp>

namespace lbd::source
{
  class BufferManager
  {
  public:
    // Create Buffers.
    [[nodiscard]] BufferId loadFile(const std::string &path) noexcept;

    [[nodiscard]] BufferId createBuffer(std::string name, std::string contents) noexcept;

    // Access Buffers.
    [[nodiscard]] const Buffer &getBuffer(const BufferId id) const noexcept { return m_buffers[id]; }

    [[nodiscard]] std::string_view getBufferName(const BufferId id) const noexcept { return m_buffers[id].getName(); }

    [[nodiscard]] size_t getBufferSize(const BufferId id) const noexcept { return m_buffers[id].getSize(); }

    [[nodiscard]] char atOffset(const BufferId id, const Offset offset) const noexcept { return m_buffers[id][offset]; }

    [[nodiscard]] std::string_view getBufferContents(const BufferId id) const noexcept
    {
      return m_buffers[id].getContents();
    }

    // Location utilities.
    [[nodiscard]] RowNumber getRow(BufferId id, Offset offset) const noexcept;

    [[nodiscard]] ColumnNumber getColumn(BufferId id, Offset offset) const noexcept;

    [[nodiscard]] std::string_view getLine(BufferId id, Offset offset) const noexcept;

    // Optional utilities.
    [[nodiscard]] bool isBufferLoaded(const std::string &name) const noexcept;

  private:
    [[nodiscard]] BufferId addBuffer(std::string name, std::string contents) noexcept;

    std::deque<Buffer> m_buffers;
    std::unordered_map<std::string, BufferId> m_bufferToIdMap;
  };

  [[nodiscard]] std::string getAbsolutePath(const std::string &path) noexcept;
}
