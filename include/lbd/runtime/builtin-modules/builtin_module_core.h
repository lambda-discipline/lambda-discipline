#pragma once

#include <lbd/runtime/builtins.h>

namespace runtime::builtins {
    NativeFunction makeAdd();

    NativeFunction makeSub();

    NativeFunction makeMul();

    NativeFunction makeCmp();

    NativeFunction makeIfZero();

    NativeFunction makeParseFloat();
}
