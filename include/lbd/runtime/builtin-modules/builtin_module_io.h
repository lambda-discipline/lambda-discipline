#pragma once

#include <lbd/runtime/builtins.h>

namespace lbd::runtime::builtins
{
    NativeFunction makePrint(Context& context);

    NativeFunction makeSlurpFile(Context& context);

    NativeFunction makeLines(Context& context);

    NativeFunction makeSplit(Context& context);
}
