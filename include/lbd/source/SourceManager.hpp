#pragma once

#include <string>
#include <unordered_map>
#include <vector>
#include <lbd/source/FileEntry.hpp>
#include <lbd/source/FileId.hpp>

namespace lbd::source
{
    class SourceManager
    {
    public:
        [[nodiscard]] FileId loadFile(const std::string& path);

        [[nodiscard]] const FileEntry& getFile(FileId id) const;

        [[nodiscard]] bool isFilePathLoaded(const std::string& path) const;

    private:
        [[nodiscard]] FileId addFile(const std::string& path, const std::string& contents);

        std::vector<FileEntry> fileEntries;
        std::unordered_map<std::string, FileId> pathToFileIdMap;
    };
}
