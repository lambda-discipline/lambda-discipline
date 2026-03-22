#pragma once

#include <lbd/runtime/builtins.h>

namespace lbd::runtime::builtins
{
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
