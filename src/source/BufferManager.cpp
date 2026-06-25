#include <algorithm>
#include <filesystem>
#include <fstream>
#include <lbd/source/BufferManager.hpp>

namespace lbd::source
{
  BufferId BufferManager::loadFile(const std::string &path) noexcept
  {
    const std::string absolutePath = getAbsolutePath(path);
    if (const auto it = m_bufferToIdMap.find(absolutePath); it != m_bufferToIdMap.end()) return it->second;

    // TODO: Read file in binary mode, as it may translate the line-endings to LF, making offset incorrect.
    std::ifstream file(absolutePath);
    // TODO: emit diagnostic / return invalid BufferId, upon failure to open a file.
    std::string contents((std::istreambuf_iterator(file)), std::istreambuf_iterator<char>());

    const BufferId id = addBuffer(absolutePath, std::move(contents));
    m_bufferToIdMap.emplace(absolutePath, id);
    return id;
  }

  BufferId BufferManager::createBuffer(std::string name, std::string contents) noexcept
  {
    return addBuffer(std::move(name), std::move(contents));
  }

  RowNumber BufferManager::getRow(const BufferId id, const Offset offset) const noexcept
  {
    const auto &lines = m_buffers[id].getLineOffsets();
    const auto it = std::ranges::upper_bound(lines, offset);
    return static_cast<RowNumber>(it - lines.begin());
  }

  ColumnNumber BufferManager::getColumn(const BufferId id, const Offset offset) const noexcept
  {
    const auto &lines = m_buffers[id].getLineOffsets();
    const auto it = std::ranges::upper_bound(lines, offset);
    const RowNumber row = static_cast<RowNumber>(it - lines.begin());
    return offset - lines[row - 1] + 1;
  }

  std::string_view BufferManager::getLine(const BufferId id, const Offset offset) const noexcept
  {
    const auto &buffer = getBuffer(id);
    const auto &lines = buffer.getLineOffsets();

    const auto it = std::ranges::upper_bound(lines, offset);
    const size_t row = static_cast<size_t>(it - lines.begin());
    const size_t lineStart = lines[row - 1];
    // TODO: Relook whether this works for Windows CRLF (may need to subtract 2 in that case.)
    const size_t lineEnd = row < lines.size() ? lines[row] - 1 : buffer.getSize();

    return buffer.getContents().substr(lineStart, lineEnd - lineStart);
  }

  bool BufferManager::isBufferLoaded(const std::string &name) const noexcept
  {
    const std::string absolutePath = getAbsolutePath(name);
    return m_bufferToIdMap.contains(absolutePath);
  }

  BufferId BufferManager::addBuffer(std::string name, std::string contents) noexcept
  {
    const auto id = static_cast<BufferId>(m_buffers.size());
    m_buffers.emplace_back(id, std::move(name), std::move(contents));
    return id;
  }

  std::string getAbsolutePath(const std::string &path) noexcept
  {
    // TODO: Handle thrown exception.
    return std::filesystem::weakly_canonical(path).string();
  }
}
