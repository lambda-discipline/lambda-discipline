#include <lbd/intp/builtin-modules/builtin_module_io.h>
#include <lbd/utils/string_escape.h>

namespace intp::interp::builtins {
    // Prints Argument to stdout and returns 0
    NativeFunction make_print() {
        const std::string name = "print";
        return {
            -1, name, [](const std::vector<std::shared_ptr<Thunk> > &args,
                         const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                for (auto &arg: args) {
                    const Value &value = arg->force();
                    std::cout << value;
                }
                return std::make_pair(Value{static_cast<double>(0)}, ResultOptions{.side_effects = true});
            }
        };
    }

    NativeFunction make_add() {
        const std::string name = "add";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = args[0]->force();
                const Value &value2 = args[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) + std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction make_sub() {
        const std::string name = "sub";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = args[0]->force();
                const Value &value2 = args[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) - std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction make_mul() {
        const std::string name = "mul";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = args[0]->force();
                const Value &value2 = args[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) * std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction make_cmp() {
        const std::string name = "cmp";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = args[0]->force();
                const Value &value2 = args[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: Float -> Float -> Float");
                }
                const double num1 = std::get<double>(value1);
                const double num2 = std::get<double>(value2);
                const int result = num1 < num2 ? -1 : num1 > num2 ? 1 : 0;
                return std::make_pair(Value{static_cast<double>(result)}, ResultOptions{});
            }
        };
    }

    NativeFunction make_if_zero() {
        const std::string name = "if_zero";
        return {
            3, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &cond_value = args[0]->force();
                if (!std::holds_alternative<double>(cond_value)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: Float -> A -> B -> A|B\n"
                                           "runtime error: expected <double> got ", cond_value);
                }
                // Lazy branching: only force the chosen clause
                if (const double cond = std::get<double>(cond_value); cond == 0.0) {
                    return std::make_pair(Value{args[1]->force()}, ResultOptions{});
                }
                return std::make_pair(Value{args[2]->force()}, ResultOptions{});
            }
        };
    }

    NativeFunction make_parse_float() {
        const std::string name = "parse_float";
        return {
            1, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::string>(arg0)) {
                    options_v.logger.error({},
                                           "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: String -> Float\n"
                                           "runtime error: expected String got ", arg0);
                }
                const std::string &s = std::get<std::string>(arg0);
                try {
                    const double value = std::stod(s);
                    return {Value{value}, ResultOptions{}};
                } catch (const std::invalid_argument &) {
                    options_v.logger.error({}, "runtime error: ", name, " could not parse string \"", escape(s), "\"");
                } catch (const std::out_of_range &) {
                    options_v.logger.error({}, "runtime error: ", name, " out of range for string \"", escape(s), "\"");
                }
            }
        };
    }
}
