#pragma once

#include <lbd/Context.hpp>
#include <lbd/runtime/Interpreter.hpp>
#include <lbd/types/Core.hpp>

namespace lbd::runtime::builtins
{
  // Helper functions to make defining native-functions easier.
  using TypePointer = types::TypePtr;

  TypePointer simpleType(types::TypeTag tag, bool hardCheck = true);

  TypePointer listType();

  TypePointer functionType(const std::vector<TypePointer> &argumentTypes,
                           const TypePointer &returnType, bool isVariadic = false);

  /// Type-Checking of native-function's signature must be handled by the user.
  std::vector<NativeFunction> getBuiltins(Context &context);
}
