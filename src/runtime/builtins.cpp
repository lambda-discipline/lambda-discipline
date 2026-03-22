#include <lbd/runtime/builtins.h>
#include <lbd/runtime/builtin-modules/builtin_module_core.h>
#include <lbd/runtime/builtin-modules/builtin_module_io.h>
#include <lbd/runtime/builtin-modules/builtin_module_list.h>

// TODO: Add module system like use module io. Which dlopen the module and loads it.

namespace lbd::runtime::builtins
{
    Options optionsValue;

    TypePointer simpleType(type::TypeTag tag, bool hardCheck)
    {
        return std::make_shared<type::SimpleType>(tag, hardCheck);
    }

    TypePointer listType()
    {
        return std::make_shared<type::ListType>();
    }

    std::shared_ptr<type::FunctionType> functionType(const std::vector<TypePointer>& argumentTypes,
                                                     const TypePointer& returnType, bool isVariadic)
    {
        return std::make_shared<type::FunctionType>(argumentTypes, returnType, isVariadic);
    }

    std::vector<NativeFunction> getBuiltins(const Options options_)
    {
        optionsValue = options_;
        return {
            {makeAdd()},
            {makeSub()},
            {makeMul()},
            {makeCmp()},
            {makeNull()},
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
