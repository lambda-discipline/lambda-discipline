#pragma once

#include <lbd/intp/builtins.h>

namespace intp::interp::builtins {
    NativeFunction make_list();

    NativeFunction make_list_size();

    NativeFunction make_list_get();

    NativeFunction make_list_remove();

    NativeFunction make_list_append();
}
