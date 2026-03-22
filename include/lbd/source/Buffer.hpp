#pragma once

#include <optional>
#include <string>
#include <string_view>
#include <lbd/source/FileId.hpp>
#include <lbd/source/SourceManager.hpp>

namespace lbd::source
{
    class Buffer
    {
    public:
        explicit Buffer(FileId fileId, const SourceManager& sourceManager);

        explicit Buffer(std::string_view contents);

        [[nodiscard]] std::optional<FileId> getFileId() const;

        [[nodiscard]] std::string_view getText() const;

        [[nodiscard]] size_t getSize() const;

        [[nodiscard]] char operator[](size_t index) const;

    private:
        std::optional<FileId> fileId;
        std::string text;
    };
}
