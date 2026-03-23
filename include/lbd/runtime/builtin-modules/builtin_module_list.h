#pragma once

#include <lbd/runtime/builtins.h>

namespace lbd::runtime::builtins
{
    NativeFunction makeList(Context& context);

    NativeFunction makeListSize(Context& context);

    NativeFunction makeListGet(Context& context);

    NativeFunction makeListRemove(Context& context);

    NativeFunction makeListAppend(Context& context);

    NativeFunction makeMap(Context& context);

    NativeFunction makeTranspose(Context& context);

    NativeFunction makeSort(Context& context);

    NativeFunction makeZip(Context& context);

    NativeFunction makeFoldRight(Context& context);
}
