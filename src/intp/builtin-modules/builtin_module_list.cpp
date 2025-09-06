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
}
