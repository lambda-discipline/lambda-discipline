#include <ranges>
#include <lbd/runtime/builtins.h>
#include <lbd/runtime/type.h>
#include <lbd/runtime/builtin-modules/builtin_module_list.h>

namespace lbd::runtime::builtins
{
    static Value listGet(const std::shared_ptr<List>& list, size_t index)
    {
        if (index >= list->elements.size())
        {
            optionsValue.logger.error({}, "runtime error: list index out of range, index is ", index);
        }
        return list->elements[index];
    }

    static Value listRemove(const std::shared_ptr<List>& list, size_t index)
    {
        if (index >= list->elements.size())
        {
            optionsValue.logger.error({}, "runtime error: list index out of range, index is ", index);
        }
        Value value = list->elements[index];
        list->elements.erase(list->elements.begin() + static_cast<std::vector<Value>::difference_type>(index));
        return value;
    }

    static void listAppend(const std::shared_ptr<List>& list, Value value)
    {
        list->elements.push_back(std::move(value));
    }

    std::shared_ptr<List> makeListObject(const std::vector<Value>& elements)
    {
        return std::make_shared<List>(List{elements});
    }

    NativeFunction makeList()
    {
        const std::string name = "list";
        const auto signature = functionType(
            {simpleType(type::TypeTag::Any)},
            listType(),
            true
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                const std::shared_ptr<Environment>&) -> std::pair<Value, ResultOptions>
            {
                std::vector<Value> values;
                for (auto& argument : arguments)
                {
                    values.push_back(argument->force());
                }
                return std::make_pair(Value{makeListObject(values)}, ResultOptions{});
            }
        };
    }

    NativeFunction makeListSize()
    {
        const std::string name = "listSize";
        const auto signature = functionType(
            {listType()},
            simpleType(type::TypeTag::Float)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                const std::shared_ptr<Environment>&) -> std::pair<Value, ResultOptions>
            {
                const Value& argument0 = arguments[0]->force();
                const auto list = std::get<std::shared_ptr<List>>(argument0);
                return std::make_pair(Value{static_cast<double>(list->elements.size())}, ResultOptions{});
            }
        };
    }

    NativeFunction makeListGet()
    {
        const std::string name = "listGet";
        const auto signature = functionType(
            {listType(), simpleType(type::TypeTag::Float)},
            simpleType(type::TypeTag::Any)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                const std::shared_ptr<Environment>&) -> std::pair<Value, ResultOptions>
            {
                const Value& argument0 = arguments[0]->force();
                const Value& argument1 = arguments[1]->force();
                return std::make_pair(Value{
                                          listGet(std::get<std::shared_ptr<List>>(argument0),
                                                  static_cast<size_t>(std::get<double>(argument1)))
                                      }, ResultOptions{});
            }
        };
    }

    NativeFunction makeListRemove()
    {
        const std::string name = "listRemove";
        const auto signature = functionType(
            {listType(), simpleType(type::TypeTag::Float)},
            simpleType(type::TypeTag::Any)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                const std::shared_ptr<Environment>&) -> std::pair<Value, ResultOptions>
            {
                const Value& argument0 = arguments[0]->force();
                const Value& argument1 = arguments[1]->force();
                return std::make_pair(
                    Value{
                        listRemove(std::get<std::shared_ptr<List>>(argument0),
                                   static_cast<size_t>(std::get<double>(argument1)))
                    },
                    ResultOptions{});
            }
        };
    }

    NativeFunction makeListAppend()
    {
        const std::string name = "listAppend";
        const auto signature = functionType(
            {listType(), simpleType(type::TypeTag::Any)},
            nullptr
        );
        return {
            name, signature, [name](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                    const std::shared_ptr<Environment>&) -> std::pair<Value, ResultOptions>
            {
                const Value& argument0 = arguments[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List>>(argument0))
                {
                    optionsValue.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                              "\n", name,
                                              " signature: List -> Any -> List""\n"
                                              "runtime error: expected <List> got ", argument0);
                }
                auto list = std::get<std::shared_ptr<List>>(argument0);
                listAppend(list, arguments[1]->force());
                return std::make_pair(Value{list}, ResultOptions{});
            }
        };
    }

    NativeFunction makeMap()
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
            name, signature, [](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                const std::shared_ptr<Environment>& callSiteEnvironment) -> std::pair<Value,
            ResultOptions>
            {
                const Value& functionValue = arguments[0]->force();
                const Value& listValue = arguments[1]->force();
                const auto list = std::get<std::shared_ptr<List>>(listValue);
                std::vector<Value> results;
                results.reserve(list->elements.size());
                for (auto& element : list->elements)
                {
                    auto elementThunk = std::make_shared<Thunk>();
                    elementThunk->cached = element;
                    // TODO: Accumulate ResultOptions from applyFunctionApplication
                    auto mappedValue = applyFunctionApplication(functionValue, {elementThunk}, callSiteEnvironment);
                    results.push_back(mappedValue);
                }
                return {Value{std::make_shared<List>(List{std::move(results)})}, ResultOptions{}};
            }
        };
    }

    NativeFunction makeTranspose()
    {
        const std::string name = "transpose";
        const auto signature = functionType(
            {listType()},
            listType()
        );
        return {
            name, signature, [name](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                    const std::shared_ptr<Environment>&) -> std::pair<Value, ResultOptions>
            {
                const Value& argument0 = arguments[0]->force();
                const auto outerList = std::get<std::shared_ptr<List>>(argument0);
                if (outerList->elements.empty()) return {Value{std::make_shared<List>(List{})}, ResultOptions{}};
                // Ensure all elements are lists
                std::vector<std::shared_ptr<List>> rows;
                rows.reserve(outerList->elements.size());
                size_t minSize = SIZE_MAX;
                for (auto& element : outerList->elements)
                {
                    // TODO: Add explicit element type test on list.
                    if (!std::holds_alternative<std::shared_ptr<List>>(element))
                    {
                        optionsValue.logger.error({},
                                                  "runtime error: native function ", name,
                                                  " expects List<List>, but got element ", element);
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
                    for (const auto& row : rows)
                    {
                        column.push_back(row->elements[col]);
                    }
                    transposed.emplace_back(std::make_shared<List>(List{std::move(column)}));
                }
                return {Value{std::make_shared<List>(List{std::move(transposed)})}, ResultOptions{}};
            }
        };
    }

    NativeFunction makeSort()
    {
        const std::string name = "sort";
        const auto signature = functionType(
            {listType()},
            listType()
        );
        return {
            name, signature, [name](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                    const std::shared_ptr<Environment>&) -> std::pair<Value, ResultOptions>
            {
                const Value& argument0 = arguments[0]->force();
                const auto list = std::get<std::shared_ptr<List>>(argument0);
                // Ensure all elements are floats
                std::vector<double> floats;
                floats.reserve(list->elements.size());
                for (auto& element : list->elements)
                {
                    if (!std::holds_alternative<double>(element))
                    {
                        optionsValue.logger.error({},
                                                  "runtime error: native function ", name,
                                                  "expects List of Float, but got element ", element);
                    }
                    floats.push_back(std::get<double>(element));
                }
                std::sort(floats.begin(), floats.end());
                std::vector<Value> sorted;
                sorted.reserve(floats.size());
                for (double f : floats)
                {
                    sorted.emplace_back(f);
                }
                return {Value{std::make_shared<List>(List{std::move(sorted)})}, ResultOptions{}};
            }
        };
    }

    NativeFunction makeZip()
    {
        const std::string name = "zip";
        const auto signature = functionType(
            {listType()},
            listType()
        );
        return {
            name, signature, [name](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                    const std::shared_ptr<Environment>&) -> std::pair<Value, ResultOptions>
            {
                const Value& argument0 = arguments[0]->force();
                const auto outerList = std::get<std::shared_ptr<List>>(argument0);
                if (outerList->elements.empty())
                {
                    return {Value{std::make_shared<List>(List{})}, ResultOptions{}};
                }
                // Ensure all elements are lists
                std::vector<std::shared_ptr<List>> lists;
                lists.reserve(outerList->elements.size());
                size_t minSize = SIZE_MAX;
                for (auto& element : outerList->elements)
                {
                    if (!std::holds_alternative<std::shared_ptr<List>>(element))
                    {
                        optionsValue.logger.error({},
                                                  "runtime error: native function ", name,
                                                  "expects List of List, but got element ", element);
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
                    for (const auto& list : lists) tuple.push_back(list->elements[i]);
                    zipped.emplace_back(std::make_shared<List>(List{std::move(tuple)}));
                }
                return {Value{std::make_shared<List>(List{std::move(zipped)})}, ResultOptions{}};
            }
        };
    }

    NativeFunction makeFoldRight()
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
            name, signature, [](const std::vector<std::shared_ptr<Thunk>>& arguments,
                                const std::shared_ptr<Environment>& callSiteEnvironment) -> std::pair<Value,
            ResultOptions>
            {
                const Value& functionValue = arguments[0]->force();
                const Value& initialValue = arguments[1]->force();
                const Value& listValue = arguments[2]->force();
                const auto list = std::get<std::shared_ptr<List>>(listValue);
                // Start with the initial accumulator value
                Value accumulatedValue = initialValue;
                // Traverse from the last element to the first
                for (auto& element : std::ranges::reverse_view(list->elements))
                {
                    auto elementThunk = std::make_shared<Thunk>();
                    elementThunk->cached = element;
                    auto accumulatedThunk = std::make_shared<Thunk>();
                    accumulatedThunk->cached = accumulatedValue;
                    // fn takes (element, accumulator)
                    accumulatedValue = applyFunctionApplication(functionValue, {elementThunk, accumulatedThunk},
                                                                callSiteEnvironment);
                }
                return {accumulatedValue, ResultOptions{}};
            }
        };
    }
}
