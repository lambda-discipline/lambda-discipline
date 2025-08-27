#pragma once

#include <iostream>
#include <string>

namespace fe::loc {
    struct Loc {
        size_t row;
        size_t col;
        std::string filepath;

        Loc(size_t row, size_t col, std::string filepath);

        friend std::ostream &operator<<(std::ostream &os, const Loc &loc);
    };
}
