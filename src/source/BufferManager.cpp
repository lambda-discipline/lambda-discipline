#include <algorithm>
#include <filesystem>
#include <fstream>
#include <lbd/source/BufferManager.hpp>

namespace lbd::source
{
  BufferId BufferManager::loadFile(const std::string &path) noexcept
  {
    const std::string absolutePath = getAbsolutePath(path);
    if (nameToBufferIdMap.contains(absolutePath)) return nameToBufferIdMap[absolutePath];

    std::ifstream file(absolutePath);
    std::string contents((std::istreambuf_iterator(file)), std::istreambuf_iterator<char>());

    const BufferId id = addBuffer(absolutePath, std::move(contents));
    nameToBufferIdMap[absolutePath] = id;
    return id;
  }

  BufferId BufferManager::createBuffer(std::string name, std::string contents) noexcept
  {
    return addBuffer(std::move(name), std::move(contents));
  }

  const Buffer &BufferManager::getBuffer(const BufferId id) const noexcept { return buffers[id]; }
  std::string_view BufferManager::getBufferName(const BufferId id) const noexcept { return buffers[id].name; }
  std::string_view BufferManager::getBufferContents(const BufferId id) const noexcept { return buffers[id].contents; }
  size_t BufferManager::getBufferSize(const BufferId id) const noexcept { return buffers[id].getSize(); }
  char BufferManager::atOffset(const BufferId id, const Offset offset) const noexcept { return buffers[id][offset]; }

  RowNumber BufferManager::getRow(const BufferId id, const Offset offset) const noexcept
  {
    const auto &lines = buffers[id].lineOffsets;
    const auto it = std::ranges::upper_bound(lines, offset);
    return static_cast<RowNumber>(it - lines.begin());
  }

  ColumnNumber BufferManager::getColumn(const BufferId id, const Offset offset) const noexcept
  {
    const auto &lines = buffers[id].lineOffsets;
    const auto it = std::ranges::upper_bound(lines, offset);
    const RowNumber row = static_cast<RowNumber>(it - lines.begin());
    return offset - lines[row - 1] + 1;
  }

  std::string_view BufferManager::getLine(const BufferId id, const Offset offset) const noexcept
  {
    const auto &buffer = buffers[id];
    const auto &lines = buffer.lineOffsets;

    const auto it = std::ranges::upper_bound(lines, offset);
    const size_t row = it - lines.begin();
    const size_t lineStart = lines[row - 1];
    const size_t lineEnd = row < lines.size() ? lines[row] - 1 : buffer.contents.size();

    return {buffer.contents.data() + lineStart, lineEnd - lineStart};
  }

  bool BufferManager::isBufferLoaded(const std::string &name) const noexcept
  {
    const std::string absolutePath = getAbsolutePath(name);
    return nameToBufferIdMap.contains(absolutePath);
  }

  BufferId BufferManager::addBuffer(std::string name, std::string contents) noexcept
  {
    const auto id = static_cast<BufferId>(buffers.size());
    buffers.push_back(Buffer{id, std::move(name), std::move(contents)});
    buffers.back().computeLineOffsets();
    return id;
  }

  std::string getAbsolutePath(const std::string &path) noexcept { return std::filesystem::absolute(path).string(); }
}