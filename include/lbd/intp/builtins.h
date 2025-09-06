#pragma once

#include <lbd/intp/interpreter.h>
#include <lbd/options.h>

namespace intp::interp::builtins {
    extern options::Options options_v;

    std::vector<NativeFunction> get_builtins(options::Options options_ = {});
}
