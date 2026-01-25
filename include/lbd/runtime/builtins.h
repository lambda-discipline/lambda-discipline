#pragma once

#include <lbd/options.h>
#include <lbd/runtime/interpreter.h>
#include <lbd/runtime/type.h>

namespace runtime::builtins {
    extern context::Options optionsValue;

    /// Helper functions to make defining native-functions easier.
    using TypePointer = std::shared_ptr<type::Type>;

    TypePointer simpleType(type::TypeTag tag, bool hardCheck = true);

    TypePointer listType();

    std::shared_ptr<type::FunctionType> functionType(const std::vector<TypePointer> &argumentTypes,
                                                     const TypePointer &returnType, bool isVariadic = false);

    // TODO: Generate the SIGNATURE documentation on-demand, rather than editing it manually.

    std::vector<NativeFunction> getBuiltins(context::Options options_ = {});
}
