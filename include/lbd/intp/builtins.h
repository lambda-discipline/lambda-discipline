#pragma once

#include <lbd/intp/interpreter.h>

namespace intp::interp {
    std::vector<NativeFunction> get_builtins();
}
