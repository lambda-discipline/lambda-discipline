#pragma once

#include <lbd/intp/builtins.h>

namespace intp::interp::builtins {
    NativeFunction makeList();

    NativeFunction makeListSize();

    NativeFunction makeListGet();

    NativeFunction makeListRemove();

    NativeFunction makeListAppend();

    NativeFunction makeMap();

    NativeFunction makeTranspose();

    NativeFunction makeSort();

    NativeFunction makeZip();

    NativeFunction makeFoldRight();
}
