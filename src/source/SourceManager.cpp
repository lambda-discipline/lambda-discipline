#include <filesystem>
#include <lbd/source/FileLoader.hpp>
#include <lbd/source/SourceManager.hpp>

namespace lbd::source
{
    static std::string getAbsolutePath(const std::string& path) { return std::filesystem::absolute(path).string(); }

    FileId SourceManager::loadFile(const std::string& path)
    {
        std::string absolutePath = getAbsolutePath(path);
        if (const auto iterator = pathToFileIdMap.find(absolutePath); iterator != pathToFileIdMap.end())
            return iterator->second;
        FileId fileId = addFile(absolutePath, loadFileFromDisk(absolutePath));
        pathToFileIdMap.emplace(absolutePath, fileId);
        return fileId;
    }

    const FileEntry& SourceManager::getFile(const FileId id) const { return fileEntries.at(id); }

    bool SourceManager::isFilePathLoaded(const std::string& path) const
    {
        const std::string absolutePath = getAbsolutePath(path);
        return pathToFileIdMap.contains(absolutePath);
    }

    FileId SourceManager::addFile(const std::string& path, const std::string& contents)
    {
        fileEntries.emplace_back(path, contents);
        return static_cast<FileId>(fileEntries.size() - 1);
    }
}
