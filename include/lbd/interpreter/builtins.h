#pragma once

#include <lbd/interpreter/interpreter.h>
#include <lbd/options.h>

namespace interpreter::builtins {
    extern global::Options optionsValue;

    std::vector<NativeFunction> getBuiltins(global::Options options_ = {});
}
