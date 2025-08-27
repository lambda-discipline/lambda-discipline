#include <lbd/fe/loc.h>

#include <utility>

namespace fe::loc {
    Loc::Loc(const size_t row, const size_t col, std::string filepath) : row(row), col(col),
                                                                         filepath(std::move(filepath)) {
    }

    std::ostream &operator<<(std::ostream &os, const Loc &loc) {
        return os << loc.filepath << ":" << loc.row << ":" << loc.col;
    }
}
