#pragma once

#include <lbd/intp/builtins.h>

namespace intp::interp::builtins {
    NativeFunction makePrint();

    NativeFunction makeAdd();

    NativeFunction makeSub();

    NativeFunction makeMul();

    NativeFunction makeCmp();

    NativeFunction makeIfZero();

    NativeFunction makeParseFloat();
}
