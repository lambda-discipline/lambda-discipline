#include <lbd/Context.hpp>
#include <lbd/runtime/Builtins.hpp>
#include <lbd/runtime/builtin-modules/BuiltinModuleCore.hpp>
#include <lbd/runtime/builtin-modules/BuiltinModuleIO.hpp>
#include <lbd/runtime/builtin-modules/BuiltinModuleList.hpp>

// TODO: Add module system like use module io. Which dlopen the module and loads it.

namespace lbd::runtime::builtins
{
  TypePointer simpleType(type::TypeTag tag, bool hardCheck)
  {
    return std::make_shared<type::SimpleType>(tag, hardCheck);
  }

  TypePointer listType() { return std::make_shared<type::ListType>(); }

  std::shared_ptr<type::FunctionType> functionType(const std::vector<TypePointer> &argumentTypes,
                                                   const TypePointer &returnType, bool isVariadic)
  {
    return std::make_shared<type::FunctionType>(argumentTypes, returnType, isVariadic);
  }

  std::vector<NativeFunction> getBuiltins(Context &context)
  {
    return {
      {makeAdd(context)},
      {makeSub(context)},
      {makeMul(context)},
      {makeCmp(context)},
      {makeNull(context)},
      {makeParseFloat(context)},
      // List module
      {makeList(context)},
      {makeListSize(context)},
      {makeListGet(context)},
      {makeListRemove(context)},
      {makeListAppend(context)},
      {makeMap(context)},
      {makeTranspose(context)},
      {makeSort(context)},
      {makeZip(context)},
      {makeFoldRight(context)},
      // IO module
      {makePrint(context)},
      {makeSlurpFile(context)},
      {makeLines(context)},
      {makeSplit(context)},
    };
  }
}