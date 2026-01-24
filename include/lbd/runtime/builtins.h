#pragma once

#include <lbd/runtime/interpreter.h>
#include <lbd/options.h>

namespace runtime::builtins {
    extern context::Options optionsValue;

    std::vector<NativeFunction> getBuiltins(context::Options options_ = {});
}
