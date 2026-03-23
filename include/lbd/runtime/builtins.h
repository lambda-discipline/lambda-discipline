#pragma once

#include <lbd/Context.hpp>
#include <lbd/runtime/interpreter.h>
#include <lbd/runtime/type.h>

namespace lbd::runtime::builtins
{
    // Helper functions to make defining native-functions easier.
    using TypePointer = std::shared_ptr<type::Type>;

    TypePointer simpleType(type::TypeTag tag, bool hardCheck = true);

    TypePointer listType();

    std::shared_ptr<type::FunctionType> functionType(const std::vector<TypePointer>& argumentTypes,
                                                     const TypePointer& returnType, bool isVariadic = false);

    /// Type-Checking of native-function's signature must be handled by the user.
    std::vector<NativeFunction> getBuiltins(Context &context);
}
