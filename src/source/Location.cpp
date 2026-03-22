#include <lbd/source/Location.hpp>

namespace lbd::source
{
    Location::Location(const std::optional<FileId> fileId, const Offset offset, const RowNumber row,
                       const ColumnNumber column) : fileId(fileId), offset(offset), row(row), column(column) {}

    std::optional<FileId> Location::getFileId() const { return fileId; }

    Offset Location::getOffset() const { return offset; }

    RowNumber Location::getRow() const { return row; }

    ColumnNumber Location::getColumn() const { return column; }
}
