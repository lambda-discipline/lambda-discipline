#include <lbd/intp/builtin-modules/builtin_module_io.h>
#include <lbd/utils/string_escape.h>

namespace intp::interp::builtins {
    // Prints Argument to stdout and returns 0.
    NativeFunction makePrint() {
        const std::string name = "print";
        return {
            -1, name, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                         const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                for (auto &argument: arguments) {
                    const Value &value = argument->force();
                    std::cout << value;
                }
                return std::make_pair(Value{static_cast<double>(0)}, ResultOptions{.sideEffects = true});
            }
        };
    }

    NativeFunction makeAdd() {
        const std::string name = "add";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &arguments,
                            const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = arguments[0]->force();
                const Value &value2 = arguments[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    optionsValue.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                              "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) + std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction makeSub() {
        const std::string name = "sub";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &arguments,
                            const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = arguments[0]->force();
                const Value &value2 = arguments[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    optionsValue.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                              "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) - std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction makeMul() {
        const std::string name = "mul";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &arguments,
                            const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = arguments[0]->force();
                const Value &value2 = arguments[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    optionsValue.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                              "\n", name, " signature: Float -> Float -> Float");
                }
                const double result = std::get<double>(value1) * std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction makeCmp() {
        const std::string name = "cmp";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &arguments,
                            const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = arguments[0]->force();
                const Value &value2 = arguments[1]->force();
                if (!std::holds_alternative<double>(value1) || !std::holds_alternative<double>(value2)) {
                    optionsValue.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                              "\n", name, " signature: Float -> Float -> Float");
                }
                const double num1 = std::get<double>(value1);
                const double num2 = std::get<double>(value2);
                const int result = num1 < num2 ? -1 : num1 > num2 ? 1 : 0;
                return std::make_pair(Value{static_cast<double>(result)}, ResultOptions{});
            }
        };
    }

    NativeFunction makeIfZero() {
        const std::string name = "if_zero";
        return {
            3, name, [name](const std::vector<std::shared_ptr<Thunk> > &arguments,
                            const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &conditionValue = arguments[0]->force();
                if (!std::holds_alternative<double>(conditionValue)) {
                    optionsValue.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                              "\n", name,
                                              " signature: Float -> A -> B -> A|B\n"
                                              "runtime error: expected <double> got ", conditionValue);
                }
                // Lazy branching: only force the chosen clause.
                if (const double condition = std::get<double>(conditionValue); condition == 0.0) {
                    return std::make_pair(Value{arguments[1]->force()}, ResultOptions{});
                }
                return std::make_pair(Value{arguments[2]->force()}, ResultOptions{});
            }
        };
    }

    NativeFunction makeParseFloat() {
        const std::string name = "parse_float";
        return {
            1, name, [name](const std::vector<std::shared_ptr<Thunk> > &arguments,
                            const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &argument0 = arguments[0]->force();
                if (!std::holds_alternative<std::string>(argument0)) {
                    optionsValue.logger.error({},
                                              "runtime error: wrong arguments provided to native function ", name,
                                              "\n", name,
                                              " signature: String -> Float\n"
                                              "runtime error: expected String got ", argument0);
                }
                const std::string &argumentString = std::get<std::string>(argument0);
                try {
                    const double value = std::stod(argumentString);
                    return {Value{value}, ResultOptions{}};
                } catch (const std::invalid_argument &) {
                    optionsValue.logger.error({}, "runtime error: ", name, " could not parse string \"",
                                              escape(argumentString),
                                              "\"");
                } catch (const std::out_of_range &) {
                    optionsValue.logger.error({}, "runtime error: ", name, " out of range for string \"",
                                              escape(argumentString),
                                              "\"");
                }
            }
        };
    }
}
