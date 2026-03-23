#pragma once

#include <deque>
#include <string>
#include <unordered_map>
#include <lbd/source/Buffer.hpp>

namespace lbd::source
{
    class BufferManager
    {
    public:
        // Create Buffers.
        [[nodiscard]] BufferId loadFile(const std::string& path) noexcept;
        [[nodiscard]] BufferId createBuffer(std::string name, std::string contents) noexcept;

        // Access Buffers.
        [[nodiscard]] const Buffer& getBuffer(BufferId id) const noexcept;
        [[nodiscard]] std::string_view getBufferName(BufferId id) const noexcept;
        [[nodiscard]] std::string_view getBufferContents(BufferId id) const noexcept;
        [[nodiscard]] size_t getBufferSize(BufferId id) const noexcept;
        [[nodiscard]] char atOffset(BufferId id, Offset offset) const noexcept;

        // Location utilities.
        [[nodiscard]] RowNumber getRow(BufferId id, Offset offset) const noexcept;
        [[nodiscard]] ColumnNumber getColumn(BufferId id, Offset offset) const noexcept;
        [[nodiscard]] std::string_view getLine(BufferId id, Offset offset) const noexcept;

        // Optional utilities.
        [[nodiscard]] bool isBufferLoaded(const std::string& name) const noexcept;

    private:
        [[nodiscard]] BufferId addBuffer(std::string name, std::string contents) noexcept;

        std::deque<Buffer> buffers;
        std::unordered_map<std::string, BufferId> nameToBufferIdMap;
    };

    std::string getAbsolutePath(const std::string& path) noexcept;
}
