#pragma once

#include <lbd/interpreter/interpreter.h>
#include <lbd/options.h>

namespace interpreter::builtins {
    extern context::Options optionsValue;

    std::vector<NativeFunction> getBuiltins(context::Options options_ = {});
}
