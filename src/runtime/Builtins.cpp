#include <lbd/Context.hpp>
#include <lbd/runtime/Builtins.hpp>
#include <lbd/runtime/builtin-modules/BuiltinModuleCore.hpp>
#include <lbd/runtime/builtin-modules/BuiltinModuleIO.hpp>
#include <lbd/runtime/builtin-modules/BuiltinModuleList.hpp>

// TODO: Add module system like use module io. Which dlopen the module and loads it.

namespace lbd::runtime::builtins
{
  TypePointer simpleType(const types::TypeTag tag, const bool hardCheck)
  {
    auto type = types::Type::named(types::typeTagToString(tag));
    type->setAllowsHardCheck(hardCheck);
    return type;
  }

  TypePointer listType() { return types::Type::named("List"); }

  TypePointer functionType(const std::vector<TypePointer> &argumentTypes,
                           const TypePointer &returnType, const bool isVariadic)
  {
    TypePointer result;
    if (argumentTypes.size() != 1)
    {
      // For multi-argument functions, nest them
      result = returnType;
      for (auto it = argumentTypes.rbegin(); it != argumentTypes.rend(); ++it)
      {
        result = types::Type::function(*it, result);
      }
    } else
    {
      result = types::Type::function(argumentTypes[0], returnType);
    }

    if (isVariadic)
    {
      result->setIsVariadic(true);
    }

    return result;
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
