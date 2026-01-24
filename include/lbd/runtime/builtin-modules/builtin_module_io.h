#pragma once

#include <lbd/runtime/builtins.h>

namespace runtime::builtins {
    NativeFunction makeSlurpFile();

    NativeFunction makeLines();

    NativeFunction makeSplit();
}
