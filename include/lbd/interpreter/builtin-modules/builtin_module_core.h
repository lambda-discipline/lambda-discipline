#pragma once

#include <lbd/interpreter/builtins.h>

namespace interpreter::builtins {
    NativeFunction makePrint();

    NativeFunction makeAdd();

    NativeFunction makeSub();

    NativeFunction makeMul();

    NativeFunction makeCmp();

    NativeFunction makeIfZero();

    NativeFunction makeParseFloat();
}
