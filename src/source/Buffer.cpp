#include <lbd/source/Buffer.hpp>

namespace lbd::source
{
    Buffer::Buffer(FileId fileId, const SourceManager& sourceManager)
        : fileId(fileId), text(sourceManager.getFile(fileId).getContents()) {}

    Buffer::Buffer(const std::string_view contents) : fileId(std::nullopt), text(contents) {}

    std::optional<FileId> Buffer::getFileId() const { return fileId; }

    std::string_view Buffer::getText() const { return text; }

    size_t Buffer::getSize() const { return text.size(); }

    char Buffer::operator[](const size_t index) const { return text[index]; }
}
