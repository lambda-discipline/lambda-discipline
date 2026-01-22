#pragma once

#include <string>

namespace fe::loc {
    struct Loc {
        // TODO: Maybe make these members private.
        size_t row;
        size_t col;
        std::string filepath;

        Loc(size_t row, size_t col, std::string filepath);

        friend std::ostream &operator<<(std::ostream &stream, const Loc &location);
    };
}
