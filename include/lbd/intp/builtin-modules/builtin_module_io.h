#pragma once

#include <lbd/intp/builtins.h>

namespace intp::interp::builtins {
    NativeFunction makeSlurpFile();

    NativeFunction makeLines();

    NativeFunction makeSplit();
}
