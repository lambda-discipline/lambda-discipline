#pragma once

#include <string>

namespace frontend {
    struct Location {
        Location(size_t row, size_t col, std::string filepath);

        friend std::ostream &operator<<(std::ostream &stream, const Location &location);

    private:
        size_t row;
        size_t col;
        std::string filepath;
    };
}
