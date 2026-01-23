#pragma once

#include <lbd/interpreter/builtins.h>

namespace interpreter::builtins {
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
