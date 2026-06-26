#include <algorithm>
#include <filesystem>
#include <fstream>
#include <fmt/core.h>
#include <lbd/Error.hpp>
#include <lbd/source/BufferManager.hpp>

namespace lbd::source
{
  BufferId BufferManager::loadFile(const std::string &path)
  {
    const std::string absolutePath = getAbsolutePath(path);
    if (const auto it = m_bufferToIdMap.find(absolutePath); it != m_bufferToIdMap.end()) return it->second;

    std::ifstream file(absolutePath, std::ios::binary);
    if (!file) throw FileSystemError(fmt::format("Failed to open file `{}`.", absolutePath));

    file.seekg(0, std::ios::end);
    const auto size = file.tellg();
    if (size == std::streampos(-1)) throw IOError(fmt::format("Failed to determine size of file '{}'.", absolutePath));
    file.seekg(0);

    std::string contents(size, '\0');
    file.read(contents.data(), size);
    if (!file) throw IOError(fmt::format("Failed to read file `{}`.", absolutePath));

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

    size_t lineEnd = row < lines.size() ? lines[row] - 1 : buffer.getSize();
    const auto &contents = buffer.getContents();
    if (lineEnd > lineStart && contents[lineEnd - 1] == '\r') --lineEnd; // CRLF

    return contents.substr(lineStart, lineEnd - lineStart);
  }

  bool BufferManager::isBufferLoaded(const std::string &name) const
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

  std::string getAbsolutePath(const std::string &path)
  {
    try
    {
      return std::filesystem::weakly_canonical(path).string();
    } catch (const std::filesystem::filesystem_error &e)
    {
      std::throw_with_nested(FileSystemError(fmt::format("Failed to resolve path `{}`: {}", path, e.what())));
    }
  }
}
