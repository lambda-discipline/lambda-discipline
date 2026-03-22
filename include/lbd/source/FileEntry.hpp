#pragma once

#include <string>

namespace lbd::source
{
    class FileEntry
    {
    public:
        FileEntry(std::string path, std::string contents);

        [[nodiscard]] std::string_view getPath() const;

        [[nodiscard]] std::string_view getContents() const;

    private:
        std::string path;
        std::string contents;
    };
}
