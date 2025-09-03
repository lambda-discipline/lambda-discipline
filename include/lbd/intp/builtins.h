#pragma once

#include <lbd/intp/interpreter.h>
#include <lbd/options.h>

namespace intp::interp {
    std::vector<NativeFunction> get_builtins(options::Options options_ = {});
}
