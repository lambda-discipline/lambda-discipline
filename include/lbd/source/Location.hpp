#pragma once

#include <cstdint>
#include <optional>
#include <lbd/source/FileId.hpp>

namespace lbd::source
{
    using Offset = uint32_t;
    using RowNumber = uint32_t;
    using ColumnNumber = uint32_t;

    class Location
    {
    public:
        Location(std::optional<FileId> fileId, Offset offset, RowNumber row, ColumnNumber column);

        [[nodiscard]] std::optional<FileId> getFileId() const;

        [[nodiscard]] Offset getOffset() const;

        [[nodiscard]] RowNumber getRow() const;

        [[nodiscard]] ColumnNumber getColumn() const;

    private:
        std::optional<FileId> fileId;
        Offset offset;
        RowNumber row;
        ColumnNumber column;
    };
}
