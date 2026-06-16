#include <algorithm>
#include <ranges>
#include <lbd/runtime/Builtins.hpp>
#include <lbd/runtime/Type.hpp>
#include <lbd/runtime/builtin-modules/BuiltinModuleList.hpp>

namespace lbd::runtime::builtins
{
  NativeFunction makeList(Context &context)
  {
    const std::string name = "list";
    const auto signature = functionType(
      {simpleType(type::TypeTag::Any)},
      listType(),
      true
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        std::vector<Value> values;
        for (auto &argument: arguments)
        {
          values.push_back(argument->force(context));
        }
        return std::make_pair(Value{std::make_shared<List>(List{values})}, ResultOptions{});
      }
    };
  }

  NativeFunction makeListSize(Context &context)
  {
    const std::string name = "listSize";
    const auto signature = functionType(
      {listType()},
      simpleType(type::TypeTag::Float)
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        const auto list = std::get<std::shared_ptr<List>>(argument0);
        return std::make_pair(Value{static_cast<double>(list->elements.size())}, ResultOptions{});
      }
    };
  }

  NativeFunction makeListGet(Context &context)
  {
    const std::string name = "listGet";
    const auto signature = functionType(
      {listType(), simpleType(type::TypeTag::Float)},
      simpleType(type::TypeTag::Any)
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        const Value &argument1 = arguments[1]->force(context);
        const std::shared_ptr<List> list = std::get<std::shared_ptr<List>>(argument0);
        const auto index = static_cast<size_t>(std::get<double>(argument1));
        if (index >= list->elements.size())
        {
          context.getDiagnosticEmitter().error(
            arguments[1]->getRange(),
            diagnostics::DiagnosticId::KEY_OUT_OF_BOUNDS, index
          );
        }
        return std::make_pair(Value{list->elements[index]}, ResultOptions{});
      }
    };
  }

  NativeFunction makeListRemove(Context &context)
  {
    const std::string name = "listRemove";
    const auto signature = functionType(
      {listType(), simpleType(type::TypeTag::Float)},
      simpleType(type::TypeTag::Any)
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        const Value &argument1 = arguments[1]->force(context);
        const std::shared_ptr<List> list = std::get<std::shared_ptr<List>>(argument0);
        const auto index = static_cast<size_t>(std::get<double>(argument1));
        if (index >= list->elements.size())
        {
          context.getDiagnosticEmitter().error(
            arguments[1]->getRange(),
            diagnostics::DiagnosticId::KEY_OUT_OF_BOUNDS, index
          );
        }
        const Value value = list->elements[index];
        list->elements.erase(list->elements.begin() + static_cast<std::vector<Value>::difference_type>(index));
        return std::make_pair(Value{value}, ResultOptions{});
      }
    };
  }

  NativeFunction makeListAppend(Context &context)
  {
    const std::string name = "listAppend";
    const auto signature = functionType(
      {listType(), simpleType(type::TypeTag::Any)},
      nullptr
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        // TODO: Test whether this validation even needed here.
        if (!std::holds_alternative<std::shared_ptr<List>>(argument0))
        {
          context.getDiagnosticEmitter().error(
            arguments[0]->getRange(),
            diagnostics::DiagnosticId::TYPE_MISMATCH, "List"
          );
        }
        auto list = std::get<std::shared_ptr<List>>(argument0);
        list->elements.push_back(arguments[1]->force(context));
        return std::make_pair(Value{list}, ResultOptions{});
      }
    };
  }

  NativeFunction makeMap(Context &context)
  {
    const std::string name = "map";
    // TODO: The type should ideally be (Any1 -> Any2) -> [Any1] -> [Any2],
    //       instead of (Any1 -> Any2) -> [Any3] -> [Any4].
    const auto signature = functionType(
      {
        functionType({simpleType(type::TypeTag::Any)}, simpleType(type::TypeTag::Any)),
        listType()
      },
      listType()
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &callSiteEnvironment) -> std::pair<Value,
    ResultOptions>
      {
        const Value &functionValue = arguments[0]->force(context);
        const Value &listValue = arguments[1]->force(context);
        const auto list = std::get<std::shared_ptr<List>>(listValue);
        std::vector<Value> results;
        results.reserve(list->elements.size());
        for (auto &element: list->elements)
        {
          auto elementThunk = std::make_shared<Thunk>();
          elementThunk->cached = element;
          // TODO: Accumulate ResultOptions from applyFunctionApplication
          auto mappedValue = applyFunctionApplication(context, functionValue, {elementThunk},
                                                      callSiteEnvironment);
          results.push_back(mappedValue);
        }
        return {Value{std::make_shared<List>(List{std::move(results)})}, ResultOptions{}};
      }
    };
  }

  NativeFunction makeTranspose(Context &context)
  {
    const std::string name = "transpose";
    const auto signature = functionType(
      {listType()},
      listType()
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        const auto outerList = std::get<std::shared_ptr<List>>(argument0);
        if (outerList->elements.empty()) return {Value{std::make_shared<List>(List{})}, ResultOptions{}};
        // Ensure all elements are lists
        std::vector<std::shared_ptr<List>> rows;
        rows.reserve(outerList->elements.size());
        size_t minSize = SIZE_MAX;
        for (auto &element: outerList->elements)
        {
          // TODO: Add explicit element type test on list.
          if (!std::holds_alternative<std::shared_ptr<List>>(element))
          {
            context.getDiagnosticEmitter().error(
              arguments[0]->getRange(),
              diagnostics::DiagnosticId::TYPE_MISMATCH, "List<List>"
            );
          }
          auto row = std::get<std::shared_ptr<List>>(element);
          rows.push_back(row);
          minSize = std::min(minSize, row->elements.size());
        }
        // Build columns
        std::vector<Value> transposed;
        transposed.reserve(minSize);
        for (size_t col = 0; col < minSize; ++col)
        {
          std::vector<Value> column;
          column.reserve(rows.size());
          for (const auto &row: rows)
          {
            column.push_back(row->elements[col]);
          }
          transposed.emplace_back(std::make_shared<List>(List{std::move(column)}));
        }
        return {Value{std::make_shared<List>(List{std::move(transposed)})}, ResultOptions{}};
      }
    };
  }

  NativeFunction makeSort(Context &context)
  {
    const std::string name = "sort";
    const auto signature = functionType(
      {listType()},
      listType()
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        const auto list = std::get<std::shared_ptr<List>>(argument0);
        // Ensure all elements are floats
        std::vector<double> floats;
        floats.reserve(list->elements.size());
        for (auto &element: list->elements)
        {
          if (!std::holds_alternative<double>(element))
          {
            context.getDiagnosticEmitter().error(
              arguments[0]->getRange(),
              diagnostics::DiagnosticId::TYPE_MISMATCH, "List<Number>"
            );
          }
          floats.push_back(std::get<double>(element));
        }
        std::sort(floats.begin(), floats.end());
        std::vector<Value> sorted;
        sorted.reserve(floats.size());
        for (double f: floats)
        {
          sorted.emplace_back(f);
        }
        return {Value{std::make_shared<List>(List{std::move(sorted)})}, ResultOptions{}};
      }
    };
  }

  NativeFunction makeZip(Context &context)
  {
    const std::string name = "zip";
    const auto signature = functionType(
      {listType()},
      listType()
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        const auto outerList = std::get<std::shared_ptr<List>>(argument0);
        if (outerList->elements.empty())
        {
          return {Value{std::make_shared<List>(List{})}, ResultOptions{}};
        }
        // Ensure all elements are lists
        std::vector<std::shared_ptr<List>> lists;
        lists.reserve(outerList->elements.size());
        size_t minSize = SIZE_MAX;
        for (auto &element: outerList->elements)
        {
          if (!std::holds_alternative<std::shared_ptr<List>>(element))
          {
            context.getDiagnosticEmitter().error(
              arguments[0]->getRange(),
              diagnostics::DiagnosticId::TYPE_MISMATCH, "List<List>"
            );
          }
          auto list = std::get<std::shared_ptr<List>>(element);
          lists.push_back(list);
          minSize = std::min(minSize, list->elements.size());
        }
        // Build zipped result
        std::vector<Value> zipped;
        zipped.reserve(minSize);
        for (size_t i = 0; i < minSize; ++i)
        {
          std::vector<Value> tuple;
          tuple.reserve(lists.size());
          for (const auto &list: lists) tuple.push_back(list->elements[i]);
          zipped.emplace_back(std::make_shared<List>(List{std::move(tuple)}));
        }
        return {Value{std::make_shared<List>(List{std::move(zipped)})}, ResultOptions{}};
      }
    };
  }

  NativeFunction makeFoldRight(Context &context)
  {
    const std::string name = "foldRight";
    // TODO: Implement indexed any-type, as the type should ideally be:
    //       (Any1 -> Any2 -> Any2) -> Any2 -> [Any1] -> Any2
    const auto signature = functionType(
      {
        functionType({simpleType(type::TypeTag::Any), simpleType(type::TypeTag::Any)},
                     simpleType(type::TypeTag::Any)),
        simpleType(type::TypeTag::Any),
        listType()
      },
      simpleType(type::TypeTag::Any)
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &callSiteEnvironment) -> std::pair<Value,
    ResultOptions>
      {
        const Value &functionValue = arguments[0]->force(context);
        const Value &initialValue = arguments[1]->force(context);
        const Value &listValue = arguments[2]->force(context);
        const auto list = std::get<std::shared_ptr<List>>(listValue);
        // Start with the initial accumulator value
        Value accumulatedValue = initialValue;
        // Traverse from the last element to the first
        for (auto &element: std::ranges::reverse_view(list->elements))
        {
          auto elementThunk = std::make_shared<Thunk>();
          elementThunk->cached = element;
          auto accumulatedThunk = std::make_shared<Thunk>();
          accumulatedThunk->cached = accumulatedValue;
          // fn takes (element, accumulator)
          accumulatedValue = applyFunctionApplication(context, functionValue,
                                                      {elementThunk, accumulatedThunk}, callSiteEnvironment);
        }
        return {accumulatedValue, ResultOptions{}};
      }
    };
  }
}