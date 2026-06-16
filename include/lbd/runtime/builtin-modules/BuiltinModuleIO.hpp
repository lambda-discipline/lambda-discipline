#pragma once

#include <lbd/runtime/Builtins.hpp>

namespace lbd::runtime::builtins
{
  NativeFunction makePrint(Context & context);

  NativeFunction makeSlurpFile(Context & context);

  NativeFunction makeLines(Context & context);

  NativeFunction makeSplit(Context & context);
}