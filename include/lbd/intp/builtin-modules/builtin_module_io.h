#pragma once

#include <lbd/intp/builtins.h>

namespace intp::interp::builtins {
    NativeFunction make_slurp_file();

    NativeFunction make_lines();

    NativeFunction make_split();
}
