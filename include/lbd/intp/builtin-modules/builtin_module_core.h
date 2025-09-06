#pragma once

#include <lbd/intp/builtins.h>

namespace intp::interp::builtins {
    NativeFunction make_print();

    NativeFunction make_add();

    NativeFunction make_sub();

    NativeFunction make_mul();

    NativeFunction make_cmp();

    NativeFunction make_if_zero();

    NativeFunction make_parse_float();
}
