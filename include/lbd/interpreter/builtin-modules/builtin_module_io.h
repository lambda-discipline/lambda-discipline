#pragma once

#include <lbd/interpreter/builtins.h>

namespace interpreter::builtins {
    NativeFunction makeSlurpFile();

    NativeFunction makeLines();

    NativeFunction makeSplit();
}
