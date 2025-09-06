#include <lbd/intp/builtin-modules/builtin_module_list.h>

namespace intp::interp::builtins {
    static Value list_get(const std::shared_ptr<List> &list_v, size_t index) {
        if (index >= list_v->elements.size()) {
            options_v.logger.error({}, "runtime error: list index out of range, index is ", index);
        }
        return list_v->elements[index];
    }

    static Value list_remove(const std::shared_ptr<List> &list_v, size_t index) {
        if (index >= list_v->elements.size()) {
            options_v.logger.error({}, "runtime error: list index out of range, index is ", index);
        }
        Value value = list_v->elements[index];
        list_v->elements.erase(list_v->elements.begin() + static_cast<std::vector<Value>::difference_type>(index));
        return value;
    }

    static void list_append(const std::shared_ptr<List> &list_v, Value value) {
        list_v->elements.push_back(std::move(value));
    }

    std::shared_ptr<List> make_list_obj(const std::vector<Value> &elements) {
        return std::make_shared<List>(List{elements});
    }

    NativeFunction make_list() {
        const std::string name = "list";
        return {
            -1, name, [](const std::vector<std::shared_ptr<Thunk> > &args,
                         const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                std::vector<Value> values;
                for (auto &arg: args) {
                    values.push_back(arg->force());
                }
                return std::make_pair(Value{make_list_obj(values)}, ResultOptions{});
            }
        };
    }

    NativeFunction make_list_size() {
        const std::string name = "list_size";
        return {
            1, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(arg0)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List -> Float""\n"
                                           "runtime error: expected <List> got ", arg0);
                }
                const auto list_v = std::get<std::shared_ptr<List> >(arg0);
                return std::make_pair(Value{static_cast<double>(list_v->elements.size())}, ResultOptions{});
            }
        };
    }

    NativeFunction make_list_get() {
        const std::string name = "list_get";
        return {
            // TODO: Add type checker to replace this manual approach
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(arg0)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List -> Float -> List""\n"
                                           "runtime error: expected <List> got ", arg0);
                }
                const Value &arg1 = args[1]->force();
                if (!std::holds_alternative<double>(arg1)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List -> Float -> List""\n"
                                           "runtime error: expected <Float> got ", arg1);
                }
                return std::make_pair(Value{
                                          list_get(std::get<std::shared_ptr<List> >(arg0),
                                                   static_cast<size_t>(std::get<double>(arg1)))
                                      },
                                      ResultOptions{});
            }
        };
    }

    NativeFunction make_list_remove() {
        const std::string name = "list_remove";
        return {
            // TODO: Add type checker to replace this manual approach
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(arg0)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List -> Float -> List""\n"
                                           "runtime error: expected <List> got ", arg0);
                }
                const Value &arg1 = args[1]->force();
                if (!std::holds_alternative<double>(arg1)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List -> Float -> List""\n"
                                           "runtime error: expected <Float> got ", arg1);
                }
                return std::make_pair(
                    Value{
                        list_remove(std::get<std::shared_ptr<List> >(arg0), static_cast<size_t>(std::get<double>(arg1)))
                    },
                    ResultOptions{});
            }
        };
    }

    NativeFunction make_list_append() {
        const std::string name = "list_append";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(arg0)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List -> Any -> List""\n"
                                           "runtime error: expected <List> got ", arg0);
                }
                auto list_v = std::get<std::shared_ptr<List> >(arg0);
                list_append(list_v, args[1]->force());
                return std::make_pair(Value{list_v}, ResultOptions{});
            }
        };
    }

    NativeFunction make_map() {
        const std::string name = "map";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &call_site_env) -> std::pair<Value, ResultOptions> {
                const Value &fn_val = args[0]->force();
                const Value &list_val = args[1]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(list_val)) {
                    options_v.logger.error({},
                                           "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: (A -> B) -> List<A> -> List<B>\n"
                                           "runtime error: expected List<A> got ", list_val);
                }
                const auto list_v = std::get<std::shared_ptr<List> >(list_val);
                std::vector<Value> results;
                results.reserve(list_v->elements.size());
                for (auto &elem: list_v->elements) {
                    auto elem_thunk = std::make_shared<Thunk>();
                    elem_thunk->cached = elem;
                    // TODO: Accumulate ResultOptions from apply_fn_apl
                    auto mapped_val = apply_fn_apl(fn_val, {elem_thunk}, call_site_env);
                    results.push_back(mapped_val);
                }
                return {Value{std::make_shared<List>(List{std::move(results)})}, ResultOptions{}};
            }
        };
    }

    NativeFunction make_transpose() {
        const std::string name = "transpose";
        return {
            1, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(arg0)) {
                    options_v.logger.error({},
                                           "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List<List> -> List<List>\n"
                                           "runtime error: expected List got ", arg0);
                }
                const auto outer_list = std::get<std::shared_ptr<List> >(arg0);
                if (outer_list->elements.empty()) return {Value{std::make_shared<List>(List{})}, ResultOptions{}};
                // Ensure all elements are lists
                std::vector<std::shared_ptr<List> > rows;
                rows.reserve(outer_list->elements.size());
                size_t min_size = SIZE_MAX;
                for (auto &elem: outer_list->elements) {
                    if (!std::holds_alternative<std::shared_ptr<List> >(elem)) {
                        options_v.logger.error({},
                                               "runtime error: native function ", name,
                                               " expects List<List>, but got element ", elem);
                    }
                    auto row = std::get<std::shared_ptr<List> >(elem);
                    rows.push_back(row);
                    min_size = std::min(min_size, row->elements.size());
                }
                // Build columns
                std::vector<Value> transposed;
                transposed.reserve(min_size);
                for (size_t col = 0; col < min_size; ++col) {
                    std::vector<Value> column;
                    column.reserve(rows.size());
                    for (const auto &row: rows) {
                        column.push_back(row->elements[col]);
                    }
                    transposed.emplace_back(std::make_shared<List>(List{std::move(column)}));
                }
                return {Value{std::make_shared<List>(List{std::move(transposed)})}, ResultOptions{}};
            }
        };
    }

    NativeFunction make_sort() {
        const std::string name = "sort";
        return {
            1, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(arg0)) {
                    options_v.logger.error({},
                                           "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List<Float> -> List<Float>\n"
                                           "runtime error: expected List<Float> got ", arg0);
                }
                const auto list_v = std::get<std::shared_ptr<List> >(arg0);
                // Ensure all elements are floats
                std::vector<double> floats;
                floats.reserve(list_v->elements.size());
                for (auto &elem: list_v->elements) {
                    if (!std::holds_alternative<double>(elem)) {
                        options_v.logger.error({},
                                               "runtime error: native function ", name,
                                               "expects List of Float, but got element ", elem);
                    }
                    floats.push_back(std::get<double>(elem));
                }
                std::sort(floats.begin(), floats.end());
                std::vector<Value> sorted;
                sorted.reserve(floats.size());
                for (double f: floats) {
                    sorted.emplace_back(f);
                }
                return {Value{std::make_shared<List>(List{std::move(sorted)})}, ResultOptions{}};
            }
        };
    }

    NativeFunction make_zip() {
        const std::string name = "zip";
        return {
            1, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(arg0)) {
                    options_v.logger.error({},
                                           "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List<List> -> List<List>\n"
                                           "runtime error: expected List<List> got ", arg0);
                }
                const auto outer_list = std::get<std::shared_ptr<List> >(arg0);
                if (outer_list->elements.empty()) {
                    return {Value{std::make_shared<List>(List{})}, ResultOptions{}};
                }
                // Ensure all elements are lists
                std::vector<std::shared_ptr<List> > lists;
                lists.reserve(outer_list->elements.size());
                size_t min_size = SIZE_MAX;
                for (auto &elem: outer_list->elements) {
                    if (!std::holds_alternative<std::shared_ptr<List> >(elem)) {
                        options_v.logger.error({},
                                               "runtime error: native function ", name,
                                               "expects List of List, but got element ", elem);
                    }
                    auto list = std::get<std::shared_ptr<List> >(elem);
                    lists.push_back(list);
                    min_size = std::min(min_size, list->elements.size());
                }
                // Build zipped result
                std::vector<Value> zipped;
                zipped.reserve(min_size);
                for (size_t i = 0; i < min_size; ++i) {
                    std::vector<Value> tuple;
                    tuple.reserve(lists.size());
                    for (const auto &list: lists) tuple.push_back(list->elements[i]);
                    zipped.emplace_back(std::make_shared<List>(List{std::move(tuple)}));
                }
                return {Value{std::make_shared<List>(List{std::move(zipped)})}, ResultOptions{}};
            }
        };
    }

    NativeFunction make_foldr() {
        const std::string name = "foldr";
        return {
            3, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &call_site_env) -> std::pair<Value, ResultOptions> {
                const Value &fn_val = args[0]->force();
                const Value &init_val = args[1]->force();
                const Value &list_val = args[2]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(list_val)) {
                    options_v.logger.error({},
                                           "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: (A -> B -> B) -> List<A> -> B -> B\n"
                                           "runtime error: expected List<A> got ", list_val);
                }
                const auto list_v = std::get<std::shared_ptr<List> >(list_val);
                // Start with the initial accumulator value
                Value acc = init_val;
                // Traverse from the last element to the first
                for (auto it = list_v->elements.rbegin(); it != list_v->elements.rend(); ++it) {
                    auto elem_thunk = std::make_shared<Thunk>();
                    elem_thunk->cached = *it;
                    auto acc_thunk = std::make_shared<Thunk>();
                    acc_thunk->cached = acc;
                    // fn takes (element, accumulator)
                    acc = apply_fn_apl(fn_val, {elem_thunk, acc_thunk}, call_site_env);
                }
                return {acc, ResultOptions{}};
            }
        };
    }
}
