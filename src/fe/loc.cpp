#include <lbd/fe/loc.h>

#include <iostream>
#include <utility>

namespace fe::loc {
    Loc::Loc(const size_t row, const size_t col, std::string filepath) : row(row), col(col),
                                                                         filepath(std::move(filepath)) {
    }

    std::ostream &operator<<(std::ostream &stream, const Loc &location) {
        return stream << location.filepath << ":" << location.row << ":" << location.col;
    }
}
