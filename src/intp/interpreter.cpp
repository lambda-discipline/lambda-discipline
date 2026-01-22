#include <lbd/intp/interpreter.h>
#include <lbd/intp/builtins.h>
#include <lbd/options.h>
#include <lbd/error.h>
#include <sstream>

#include "lbd/utils/string_escape.h"

namespace intp::interp {
    static options::Options optionsValue;
    static ResultOptions globalResultOptions;

    [[nodiscard]] std::string Closure::toString() const {
        std::ostringstream oss;
        oss << "<closure: " << parameter << ">";
        return oss.str();
    }

    std::ostream &operator<<(std::ostream &stream, const Closure &closure) {
        return stream << closure.toString();
    }

    [[nodiscard]] std::string List::toString() const {
        std::ostringstream stringStream;
        stringStream << "[";
        for (size_t i = 0; i < elements.size(); ++i) {
            stringStream << escapeString(elements[i].toString());
            if (i + 1 != elements.size()) {
                stringStream << ", ";
            }
        }
        stringStream << "]";
        return stringStream.str();
    }

    [[nodiscard]] std::string Value::toString() const {
        return std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, double>) {
                return std::to_string(arg);
            } else if constexpr (std::is_same_v<T, std::string>) {
                return arg;
            } else if constexpr (std::is_same_v<T, Closure>) {
                return arg.toString();
            } else if constexpr (std::is_same_v<T, std::shared_ptr<NativeFunction> > ||
                                 std::is_same_v<T, std::shared_ptr<List> >) {
                return arg->toString();
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled runtime value");
            }
        }, *this);
    }

    std::ostream &operator<<(std::ostream &stream, const Value &value) {
        return stream << value.toString();
    }

    void ResultOptions::interpolate(const ResultOptions &resultOptions_) {
        sideEffects |= resultOptions_.sideEffects;
    }

    std::ostream &operator<<(std::ostream &stream, const List &list) {
        return stream << list.toString();
    }

    [[nodiscard]] std::string NativeFunction::toString() const {
        std::ostringstream stringStream;
        // TODO: Update the type name.
        stringStream << "<native_fn: " << name << " " << arity << ">";
        return stringStream.str();
    }

    std::ostream &operator<<(std::ostream &stream, const NativeFunction &nativeFunction) {
        return stream << nativeFunction.toString();
    }

    Thunk::Thunk(const frontend::Expression *expression, std::shared_ptr<Environment> environment,
                 std::optional<frontend::Location> origin) : expression(expression), environment(std::move(environment)),
                                                       origin(std::move(origin)) {
    }

    const Value &Thunk::force() const {
        if (cached.has_value()) {
            return cached.value();
        }
        // Expression is not initialized
        if (!expression) {
            optionsValue.logger.error(origin, "runtime error: forcing empty thunk");
        }
        cached = evalExpression(*expression, environment);
        return cached.value();
    }

    void Thunk::set(const frontend::Expression *expression_, std::shared_ptr<Environment> environment_,
                    std::optional<frontend::Location> origin_) {
        expression = expression_;
        owned.reset();
        environment = std::move(environment_);
        if (origin_.has_value()) {
            origin = std::move(origin_.value());
        }
        cached.reset();
    }

    void Thunk::set_owned(frontend::Expression expression_, std::shared_ptr<Environment> environment_,
                          std::optional<frontend::Location> origin_) {
        owned = std::make_unique<frontend::Expression>(std::move(expression_));
        expression = owned.get();
        environment = std::move(environment_);
        if (origin_.has_value()) {
            origin = std::move(origin_.value());
        }
        cached.reset();
    }

    Environment::Environment(std::shared_ptr<Environment> parent) : parent(std::move(parent)) {
    }

    std::shared_ptr<Thunk> Environment::lookup(const std::string &name) const {
        // Precedence: Local Environment > Global Environment.
        if (const auto it = table.find(name); it != table.end()) {
            return it->second;
        }
        if (parent) {
            return parent->lookup(name);
        }
        return nullptr;
    }

    void Environment::bind(const std::string &name, std::shared_ptr<Thunk> thunk) {
        table[name] = std::move(thunk);
    }

    std::vector<std::vector<std::string> > Environment::toVector(const bool force) const {
        std::vector<std::vector<std::string> > result;
        result.reserve(table.size());
        for (const auto &[name, thunk]: table) {
            std::string valueString = "<thunk: unevaluated>";
            try {
                if (force) {
                    const Value &value = thunk->force();
                    valueString = escapeString(value.toString());
                } else if (thunk->cached) {
                    valueString = escapeString(thunk->cached->toString()); // Already computed.
                }
            } catch (const std::exception &) {
            }
            result.push_back({name, valueString});
        }
        if (parent) {
            auto parentVector = parent->toVector(force);
            result.insert(result.end(), std::make_move_iterator(parentVector.begin()),
                          std::make_move_iterator(parentVector.end()));
        }
        return result;
    }

    static Value evaluateIdentifierAstNode(const frontend::IdentifierAstNode &identifierAstNode,
                                           const std::shared_ptr<Environment> &environment) {
        const auto thunk = environment->lookup(identifierAstNode.value);
        if (!thunk) {
            optionsValue.logger.error(identifierAstNode.location, "runtime error: undefined identifier ",
                                      identifierAstNode.value);
        }
        return thunk->force();
    }

    static Value evaluateLambdaExpression(const frontend::LambdaExpression &lambdaExpression,
                                          const std::shared_ptr<Environment> &environment) {
        return Value(Closure{lambdaExpression.argument.value, lambdaExpression.expression.get(), environment});
    }

    static Value evaluateFunctionApplication(const frontend::FunctionApplication &functionApplication,
                                             const std::shared_ptr<Environment> &environment) {
        // Lookup the callee lazily.
        const auto calleeThunk = environment->lookup(functionApplication.functionName.value);
        if (!calleeThunk) {
            optionsValue.logger.error(functionApplication.location, "runtime error: undefined function ",
                                      functionApplication.functionName.value);
        }
        const Value functionValue = calleeThunk->force();
        std::vector<std::shared_ptr<Thunk> > argumentThunks;
        argumentThunks.reserve(functionApplication.arguments.size());
        for (const auto &argument: functionApplication.arguments) {
            argumentThunks.push_back(std::make_shared<Thunk>(argument.get(), environment));
        }
        return applyFunctionApplication(functionValue, argumentThunks, environment, functionApplication.location);
    }

    Value evalExpression(const frontend::Expression &expression, std::shared_ptr<Environment> environment) {
        return std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, frontend::IdentifierAstNode>) {
                return evaluateIdentifierAstNode(arg, environment);
            } else if constexpr (std::is_same_v<T, frontend::StringAstNode> || std::is_same_v<T,
                                     frontend::FloatAstNode>) {
                return Value(arg.value);
            } else if constexpr (std::is_same_v<T, frontend::LambdaExpression>) {
                return evaluateLambdaExpression(arg, environment);
            } else if constexpr (std::is_same_v<T, frontend::FunctionApplication>) {
                return evaluateFunctionApplication(arg, environment);
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled expression");
            }
        }, expression.value);
    }

    static std::shared_ptr<Thunk> valueToThunk(const Value &value) {
        auto thunk = std::make_shared<Thunk>();
        thunk->cached = value;
        return thunk;
    }

    Value applyFunctionApplication(Value functionName, const std::vector<std::shared_ptr<Thunk> > &arguments,
                                   const std::shared_ptr<Environment> &callSiteEnvironment,
                                   const std::optional<frontend::Location> &callLocation) {
        // Local mutable copy of the args, for inserting evaluated values as Thunks when needed.
        std::vector<std::shared_ptr<Thunk> > workArguments = arguments;
        // Frame Stack: Functions to which we are currently applying Arguments.
        // Start with the initial Function.
        std::vector<Value> frames;
        frames.push_back(std::move(functionName));
        size_t index = 0; // Index of next Thunk to consume from work_args
        while (true) {
            // If there are no frames left, that's unexpected (shouldn't happen), bail.
            if (frames.empty()) {
                optionsValue.logger.error(callLocation, "internal error: no function frame to apply");
            }
            // Work on the top-most frame
            Value currentFunction = frames.back();
            std::optional<Value> resultantValue;
            // If we've consumed all available Argument Thunks:
            if (index >= workArguments.size()) {
                // If top frame is a function, and we have no more args to feed it,
                // return that function (partial application) or its value as-is.
                if (std::holds_alternative<std::shared_ptr<NativeFunction> >(currentFunction)) {
                    // Allow arity==0 and arity==-1 (variadic) to execute with zero args.
                    if (const auto &nativeFunction = *std::get<std::shared_ptr<NativeFunction> >(currentFunction);
                        nativeFunction.arity == 0 || nativeFunction.arity == -1) {
                        std::vector<std::shared_ptr<Thunk> > slice; // empty
                        auto [value, result_options] = nativeFunction.implementation(slice, callSiteEnvironment);
                        globalResultOptions.interpolate(result_options);
                        return value;
                    }
                    // Non-zero arity native function but no args left: partial application (return the function)
                    return currentFunction;
                }
                if (std::holds_alternative<Closure>(currentFunction)) {
                    // Closures require one argument; with none left, this is a partial application (return the closure).
                    return currentFunction;
                }
                // It's not a function (shouldn't happen), return it as value.
                return currentFunction;
            }
            // Closure case: Closure consumes exactly one Argument (its Param)
            if (std::holds_alternative<Closure>(currentFunction)) {
                const auto [param, body, env] = std::get<Closure>(currentFunction);
                const auto &argumentThunk = workArguments[index++];
                const auto childEnvironment = std::make_shared<Environment>(env);
                childEnvironment->bind(param, argumentThunk);
                resultantValue = evalExpression(*body, childEnvironment);
            }
            // Native Function case: Consumes its arity-many Argument Thunks
            else if (std::holds_alternative<std::shared_ptr<NativeFunction> >(currentFunction)) {
                if (const auto &[arity, name, implementation] = *std::get<std::shared_ptr<NativeFunction> >(
                        currentFunction);
                    arity != -1) {
                    if (workArguments.size() - index < arity) {
                        optionsValue.logger.error(callLocation, "runtime error: native function ", name, " expects ",
                                                  arity, " argument(s), found ", workArguments.size() - index);
                    }
                    std::vector<std::shared_ptr<Thunk> > slice;
                    slice.reserve(arity);
                    for (size_t i = 0; i < arity; ++i) {
                        slice.push_back(workArguments[index + i]);
                    }
                    auto [resultantValue_, resultOptions] = implementation(slice, callSiteEnvironment);
                    resultantValue = resultantValue_;
                    globalResultOptions.interpolate(resultOptions);
                    index += arity;
                } else {
                    std::vector<std::shared_ptr<Thunk> > slice;
                    slice.reserve(arguments.size() - index);
                    for (size_t i = 0; i < arguments.size() - index; ++i) {
                        slice.push_back(workArguments[index + i]);
                    }
                    auto [resultantValue_, resultOptions] = implementation(slice, callSiteEnvironment);
                    resultantValue = resultantValue_;
                    globalResultOptions.interpolate(resultOptions);
                    index += arguments.size() - index;
                }
            } else {
                // Top frame is not a Function (Closure, NativeFunction) Value but there are still Arguments left
                optionsValue.logger.error(callLocation, "runtime error: trying to apply non-function value ",
                                          frames.back());
            }
            if (!resultantValue) {
                optionsValue.logger.error(callLocation, "internal error: application produced no result");
            }
            if (std::holds_alternative<Closure>(*resultantValue) ||
                std::holds_alternative<std::shared_ptr<NativeFunction> >(*resultantValue)) {
                // Replace the top Frame with the returned Function (curry)
                frames.back() = std::move(*resultantValue);
                continue;
            }
            // Resultant is a concrete Value (different from Function Value i.e. double, string).
            // Pop the frame that produced it, and insert this Value as a thunk at the current index
            // so the previous Frame (if any) will consume it.
            Value concrete = std::move(*resultantValue);
            frames.pop_back();
            // If there are no more Frames after popping, then this Value is the final result of the
            // whole Function Application, but only valid if there are no further Argument Thunks remaining.
            if (frames.empty()) {
                if (index < workArguments.size()) {
                    optionsValue.logger.error(callLocation,
                                              "runtime error: too many arguments applied to non-function value ",
                                              concrete);
                }
                // No Thunks remaining then concrete Value is the result
                return concrete;
            }
            // There is a previous Frame; insert the concrete Value as a Thunk at the
            // current index so the previous Frame will consume it on the next iteration.
            // TODO: Why is it named injThunk
            auto injThunk = valueToThunk(concrete);
            workArguments.insert(workArguments.begin() + static_cast<int>(index), injThunk);
            // Do NOT advance index: the inserted thunk is at position index and will be consumed
            // by the previous frame on the next iteration.
            // Loop will continue with frames.back() being the previous frame.
        }
    }

    // Creates placeholder Thunk then set body so recursion can refer to it during lazy evaluation
    static void bindDefinitionAstNodeLazy(frontend::DefinitionAstNode &definitionAstNode,
                                          const std::shared_ptr<Environment> &environment,
                                          const options::Options options) {
        const auto thunk = std::make_shared<Thunk>();
        environment->bind(definitionAstNode.definitionName.value, thunk);
        if (options.ownExpression) {
            thunk->set_owned(std::move(definitionAstNode.expression), environment,
                             definitionAstNode.expression.getLocation());
        } else {
            thunk->set(&definitionAstNode.expression, environment, definitionAstNode.expression.getLocation());
        }
    }

    Result interpret(frontend::Program &program, std::optional<std::shared_ptr<Environment> > globalEnvironment,
                     const options::Options options_) {
        optionsValue = options_;
        if (!globalEnvironment) {
            globalEnvironment = std::make_shared<Environment>();
            installBuiltins(*globalEnvironment);
        }
        Value resultantValue;
        for (auto &[value]: program.nodes) {
            std::visit([&]<typename T0>(T0 &&argument) {
                using T = std::decay_t<T0>;
                if constexpr (std::is_same_v<T, frontend::Expression>) {
                    resultantValue = evalExpression(argument, *globalEnvironment);
                } else if constexpr (std::is_same_v<T, frontend::DefinitionAstNode>) {
                    bindDefinitionAstNodeLazy(argument, *globalEnvironment, optionsValue);
                    const frontend::DefinitionAstNode &definitionAstNode = argument;
                    resultantValue = definitionAstNode.definitionName.value;
                } else {
                    STATIC_ASSERT_UNREACHABLE_T(T, "unhandled program node");
                }
            }, value);
        }
        return {*globalEnvironment, resultantValue, globalResultOptions};
    }

    void installBuiltins(const std::shared_ptr<Environment> &environment) {
        for (auto &nativeFunction: builtins::getBuiltins(optionsValue)) {
            const auto thunk = std::make_shared<Thunk>();
            thunk->cached = Value{std::make_shared<NativeFunction>(nativeFunction)};
            environment->bind(nativeFunction.name, thunk);
        }
    }
}
