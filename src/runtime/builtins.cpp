#include <lbd/runtime/builtins.h>
#include <lbd/runtime/builtin-modules/builtin_module_core.h>
#include <lbd/runtime/builtin-modules/builtin_module_list.h>
#include <lbd/runtime/builtin-modules/builtin_module_io.h>

// TODO: Add module system like use module io. Which dlopen's the module and loads it.

namespace runtime::builtins {
    context::Options optionsValue;

    std::vector<NativeFunction> getBuiltins(const context::Options options_) {
        optionsValue = options_;
        return {
            {makePrint()},
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
            {makeSlurpFile()},
            {makeLines()},
            {makeSplit()},
        };
    }
}
