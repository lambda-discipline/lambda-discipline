#pragma once

#include <lbd/runtime/builtins.h>

namespace lbd::runtime::builtins
{
    NativeFunction makePrint();

    NativeFunction makeSlurpFile();

    NativeFunction makeLines();

    NativeFunction makeSplit();
}
