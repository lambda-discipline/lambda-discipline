#include <lbd/frontend/location.h>
#include <iostream>
#include <utility>

namespace frontend {
    Location::Location(const size_t row, const size_t col, std::string filepath) : row(row), col(col),
        filepath(std::move(filepath)) {
    }

    std::ostream &operator<<(std::ostream &stream, const Location &location) {
        return stream << location.filepath << ":" << location.row << ":" << location.col;
    }
}
