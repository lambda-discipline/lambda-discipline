#pragma once

#include <string>

namespace frontend {
    struct Location {
        // TODO: Maybe make these members private.
        size_t row;
        size_t col;
        std::string filepath;

        Location(size_t row, size_t col, std::string filepath);

        friend std::ostream &operator<<(std::ostream &stream, const Location &location);
    };
}
