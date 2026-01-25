#pragma once

#include <lbd/runtime/interpreter.h>
#include <lbd/runtime/type.h>
#include <lbd/options.h>

namespace runtime::builtins {
    extern context::Options optionsValue;

    /// Helper functions to make defining native-functions easier.
    using TypePointer = std::shared_ptr<type::Type>;

    inline TypePointer simpleType(type::TypeTag tag) {
        return std::make_shared<type::SimpleType>(tag);
    }

    inline TypePointer listType() {
        return std::make_shared<type::ListType>();
    }

    inline std::shared_ptr<type::FunctionType> functionType(const std::vector<TypePointer> &argumentTypes,
                                                            const TypePointer &returnType,
                                                            bool isVariadic = false) {
        return std::make_shared<type::FunctionType>(argumentTypes, returnType, isVariadic);
    }

    // TODO: Move the definitions to source-file.

    // TODO: Generate the SIGNATURE documentation on-demand, rather than editing it manually.

    std::vector<NativeFunction> getBuiltins(context::Options options_ = {});
}
