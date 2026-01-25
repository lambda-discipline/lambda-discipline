#include <lbd/runtime/builtins.h>
#include <lbd/runtime/builtin-modules/builtin_module_core.h>
#include <lbd/runtime/builtin-modules/builtin_module_io.h>
#include <lbd/runtime/builtin-modules/builtin_module_list.h>

// TODO: Add module system like use module io. Which dlopen's the module and loads it.

namespace runtime::builtins {
    context::Options optionsValue;

    TypePointer simpleType(type::TypeTag tag, bool hardCheck) {
        return std::make_shared<type::SimpleType>(tag, hardCheck);
    }

    TypePointer listType() {
        return std::make_shared<type::ListType>();
    }

    std::shared_ptr<type::FunctionType> functionType(const std::vector<TypePointer> &argumentTypes,
                                                     const TypePointer &returnType, bool isVariadic) {
        return std::make_shared<type::FunctionType>(argumentTypes, returnType, isVariadic);
    }

    std::vector<NativeFunction> getBuiltins(const context::Options options_) {
        optionsValue = options_;
        return {
            {makeAdd()},
            {makeSub()},
            {makeMul()},
            {makeCmp()},
            {makeIfZero()},
            {makeParseFloat()},
            // List module
            {makeList()},
            {makeListSize()},
            {makeListGet()},
            {makeListRemove()},
            {makeListAppend()},
            {makeMap()},
            {makeTranspose()},
            {makeSort()},
            {makeZip()},
            {makeFoldRight()},
            // IO module
            {makePrint()},
            {makeSlurpFile()},
            {makeLines()},
            {makeSplit()},
        };
    }
}
