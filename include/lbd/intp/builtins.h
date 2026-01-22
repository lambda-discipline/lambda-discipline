#pragma once

#include <lbd/intp/interpreter.h>
#include <lbd/options.h>

namespace intp::interp::builtins {
    extern options::Options optionsValue;

    std::vector<NativeFunction> getBuiltins(options::Options options_ = {});
}
