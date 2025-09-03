#include <lbd/intp/builtins.h>

namespace intp::interp {
    static options::Options options_v;

    // Prints Argument to stdout and returns 0
    static NativeFunction make_print() {
        const std::string name = "print";
        return {
            -1, name, [](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                for (auto &arg: args) {
                    const Value &value = arg->force();
                    std::cout << value;
                }
                return Value{static_cast<double>(0)};
            }
        };
    }

    static NativeFunction make_add() {
        const std::string name = "add";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                const Value &value1 = args[0]->force();
                const Value &value2 = args[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) + std::get<double>(value2);
                return Value{result};
            }
        };
    }

    static NativeFunction make_sub() {
        const std::string name = "sub";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                const Value &value1 = args[0]->force();
                const Value &value2 = args[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) - std::get<double>(value2);
                return Value{result};
            }
        };
    }

    static NativeFunction make_mul() {
        const std::string name = "mul";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                const Value &value1 = args[0]->force();
                const Value &value2 = args[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) * std::get<double>(value2);
                return Value{result};
            }
        };
    }

    static NativeFunction make_cmp() {
        const std::string name = "cmp";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                const Value &value1 = args[0]->force();
                const Value &value2 = args[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: Float -> Float -> Float");
                }
                const double num1 = std::get<double>(value1);
                const double num2 = std::get<double>(value2);
                const int result = num1 < num2 ? -1 : num1 > num2 ? 1 : 0;
                return Value{static_cast<double>(result)};
            }
        };
    }

    static NativeFunction make_if_zero() {
        const std::string name = "if_zero";
        return {
            3, name, [name](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                const Value &cond_value = args[0]->force();
                if (!std::holds_alternative<double>(cond_value)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: Float -> Any -> Any -> Any""\n"
                                           "runtime error: expected <double> got ", cond_value);
                }
                // Lazy branching: only force the chosen clause
                if (const double cond = std::get<double>(cond_value); cond == 0.0) {
                    return args[1]->force();
                }
                return args[2]->force();
            }
        };
    }

    static std::shared_ptr<List> make_list_obj(const std::vector<Value> &elements) {
        return std::make_shared<List>(List{elements});
    }

    static Value list_get(const std::shared_ptr<List> &list_v, size_t index) {
        if (index >= list_v->elements.size())
            throw std::out_of_range("list index out of range");
        return list_v->elements[index];
    }

    static void list_append(const std::shared_ptr<List> &list_v, Value value) {
        list_v->elements.push_back(std::move(value));
    }

    static NativeFunction make_list() {
        const std::string name = "list";
        return {
            -1, name, [](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                std::vector<Value> values;
                for (auto &arg: args) {
                    values.push_back(arg->force());
                }
                return make_list_obj(values);
            }
        };
    }

    static NativeFunction make_list_append() {
        const std::string name = "list_append";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::shared_ptr<List> >(arg0)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: List -> Any -> List""\n"
                                           "runtime error: expected <List> got ", arg0);
                }
                auto list_v = std::get<std::shared_ptr<List> >(arg0);
                list_append(list_v, args[1]->force());
                return list_v;
            }
        };
    }

    std::vector<NativeFunction> get_builtins(const options::Options options_) {
        options_v = options_;
        return {
            {make_print()},
            {make_add()},
            {make_sub()},
            {make_mul()},
            {make_cmp()},
            {make_if_zero()},
            {make_list()},
            {make_list_append()}
        };
    }
}
