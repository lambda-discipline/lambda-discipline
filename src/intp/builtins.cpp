#include <lbd/intp/builtins.h>

namespace intp::interp {
    // TODO: If Arity is -1 that means it can consume all the arguments
    // Prints Argument to stdout and returns the same Argument as Value
    static NativeFunction make_print() {
        const std::string name = "print";
        return {
            1, name, [](const std::vector<std::shared_ptr<Thunk> > &args, const std::shared_ptr<Env> &) -> Value {
                const Value &value = args[0]->force();
                std::cout << value << std::endl;
                return value;
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
                    std::cerr << "runtime error: wrong arguments provided to native function " << name << std::endl
                            << name << " signature: Float -> Float -> Float" << std::endl;
                    exit(EXIT_FAILURE);
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
                    std::cerr << "runtime error: wrong arguments provided to native function " << name << std::endl
                            << name << " signature: Float -> Float -> Float" << std::endl;
                    exit(EXIT_FAILURE);
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
                    std::cerr << "runtime error: wrong arguments provided to native function " << name << std::endl
                            << name << " signature: Float -> Float -> Float" << std::endl;
                    exit(EXIT_FAILURE);
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
                    std::cerr << "runtime error: wrong arguments provided to native function " << name << std::endl
                            << name << " signature: Float -> Float -> Float" << std::endl;
                    exit(EXIT_FAILURE);
                }
                const double num1 = std::get<double>(value1);
                const double num2 = std::get<double>(value2);
                const int result = (num1 < num2) ? -1 : num1 > num2 ? 1 : 0;
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
                    std::cerr << "runtime error: wrong argument type to native function " << name << "\n"
                            << name << " signature: Float -> Any -> Any -> Any\n"
                            << "runtime error: expected <double> got " << cond_value << std::endl;
                    exit(EXIT_FAILURE);
                }
                // Lazy branching: only force the chosen clause
                if (const double cond = std::get<double>(cond_value); cond == 0.0) {
                    return args[1]->force();
                }
                return args[2]->force();
            }
        };
    }


    std::vector<NativeFunction> get_builtins() {
        return {
            {make_print()},
            {make_add()},
            {make_sub()},
            {make_mul()},
            {make_cmp()},
            {make_if_zero()}
        };
    }
}
