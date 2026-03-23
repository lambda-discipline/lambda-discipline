#pragma once

#include <lbd/source/BufferId.hpp>

namespace lbd::source
{
    using Offset = uint32_t;
    using RowNumber = uint32_t;
    using ColumnNumber = uint32_t;

    struct Location
    {
        BufferId bufferId;
        Offset offset;
    };
}
