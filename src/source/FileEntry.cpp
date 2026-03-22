#include <lbd/source/FileEntry.hpp>

namespace lbd::source
{
    FileEntry::FileEntry(std::string path, std::string contents) : path(std::move(path)),
                                                                   contents(std::move(contents)) {}

    std::string_view FileEntry::getPath() const { return path; }

    std::string_view FileEntry::getContents() const { return contents; }
}
