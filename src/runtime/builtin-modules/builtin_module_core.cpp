#include <lbd/runtime/builtin-modules/builtin_module_io.h>
#include <lbd/utils/string_escape.h>

namespace runtime::builtins {
    NativeFunction makeAdd() {
        const std::string name = "add";
        const auto signature = functionType(
            {simpleType(type::TypeTag::Float), simpleType(type::TypeTag::Float)},
            simpleType(type::TypeTag::Float)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = arguments[0]->force();
                const Value &value2 = arguments[1]->force();
                const double result = std::get<double>(value1) + std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction makeSub() {
        const std::string name = "sub";
        const auto signature = functionType(
            {simpleType(type::TypeTag::Float), simpleType(type::TypeTag::Float)},
            simpleType(type::TypeTag::Float)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = arguments[0]->force();
                const Value &value2 = arguments[1]->force();
                const double result = std::get<double>(value1) - std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction makeMul() {
        const std::string name = "mul";
        const auto signature = functionType(
            {simpleType(type::TypeTag::Float), simpleType(type::TypeTag::Float)},
            simpleType(type::TypeTag::Float)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = arguments[0]->force();
                const Value &value2 = arguments[1]->force();
                const double result = std::get<double>(value1) * std::get<double>(value2);
                return std::make_pair(Value{result}, ResultOptions{});
            }
        };
    }

    NativeFunction makeCmp() {
        const std::string name = "cmp";
        const auto signature = functionType(
            {simpleType(type::TypeTag::Float), simpleType(type::TypeTag::Float)},
            simpleType(type::TypeTag::Float)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &value1 = arguments[0]->force();
                const Value &value2 = arguments[1]->force();
                const double num1 = std::get<double>(value1);
                const double num2 = std::get<double>(value2);
                const int result = num1 < num2 ? -1 : num1 > num2 ? 1 : 0;
                return std::make_pair(Value{static_cast<double>(result)}, ResultOptions{});
            }
        };
    }

    NativeFunction makeIfZero() {
        const std::string name = "if_zero";
        // TODO: The return-type should be either argument-1 or argument-2.
        const auto signature = functionType(
            {
                simpleType(type::TypeTag::Float),
                simpleType(type::TypeTag::Any, false),
                simpleType(type::TypeTag::Any, false)
            },
            simpleType(type::TypeTag::Any)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &conditionValue = arguments[0]->force();
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
        const auto signature = functionType(
            {simpleType(type::TypeTag::String)},
            simpleType(type::TypeTag::Float)
        );
        return {
            name, signature, [name](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                    const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &argument0 = arguments[0]->force();
                const auto &argumentString = std::get<std::string>(argument0);
                try {
                    const double value = std::stod(argumentString);
                    return {Value{value}, ResultOptions{}};
                } catch (const std::invalid_argument &) {
                    optionsValue.logger.error({}, "runtime error: ", name, " could not parse string \"",
                                              escapeString(argumentString),
                                              "\"");
                } catch (const std::out_of_range &) {
                    optionsValue.logger.error({}, "runtime error: ", name, " out of range for string \"",
                                              escapeString(argumentString),
                                              "\"");
                }
            }
        };
    }
}
