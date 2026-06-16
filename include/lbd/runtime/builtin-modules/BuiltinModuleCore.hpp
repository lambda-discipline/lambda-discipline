#pragma once

#include <lbd/runtime/Builtins.hpp>

namespace lbd::runtime::builtins
{
  NativeFunction makeAdd(Context & context);

  NativeFunction makeSub(Context & context);

  NativeFunction makeMul(Context & context);

  NativeFunction makeCmp(Context & context);

  NativeFunction makeNull(Context & context);

  NativeFunction makeParseFloat(Context & context);
}