#include <sstream>
#include <utility>
#include <lbd/error.h>
#include <lbd/Options.hpp>
#include <lbd/runtime/builtins.h>
#include <lbd/runtime/interpreter.h>
#include <lbd/runtime/type.h>
#include <lbd/utils/string_escape.h>

// TODO: Everywhere make context the first parameter for consistency.
// TODO: Add Context guards.

namespace lbd::runtime
{
    // TODO: Remove this global variable.
    static ResultOptions globalResultOptions;

    [[nodiscard]] std::string Closure::toString() const
    {
        std::ostringstream outputStringStream;
        outputStringStream << "<Closure: " << parameter << ">";
        return outputStringStream.str();
    }

    std::ostream& operator<<(std::ostream& outputStream, const Closure& closure)
    {
        return outputStream << closure.toString();
    }

    [[nodiscard]] std::string List::toString() const
    {
        std::ostringstream outputStringStream;
        outputStringStream << "[";
        for (size_t i = 0; i < elements.size(); ++i)
        {
            outputStringStream << escapeString(elements[i].toString());
            if (i + 1 != elements.size())
            {
                outputStringStream << ", ";
            }
        }
        outputStringStream << "]";
        return outputStringStream.str();
    }

    std::ostream& operator<<(std::ostream& outputStream, const List& list)
    {
        return outputStream << list.toString();
    }

    [[nodiscard]] std::string Value::toString() const
    {
        return std::visit([&]<typename T0>(T0&& arg)
        {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, double>)
            {
                return std::to_string(arg);
            }
            else if constexpr (std::is_same_v<T, std::string>)
            {
                return unescapeString(arg);
            }
            else if constexpr (std::is_same_v<T, Closure>)
            {
                return arg.toString();
            }
            else if constexpr (std::is_same_v<T, std::shared_ptr<NativeFunction>> ||
                std::is_same_v<T, std::shared_ptr<List>>)
            {
                return arg->toString();
            }
            else
            {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled runtime value");
            }
        }, *this);
    }

    std::ostream& operator<<(std::ostream& outputStream, const Value& value)
    {
        return outputStream << value.toString();
    }

    void ResultOptions::interpolate(const ResultOptions& resultOptions)
    {
        sideEffects |= resultOptions.sideEffects;
    }

    NativeFunction::NativeFunction(std::string name, std::shared_ptr<type::FunctionType> signature,
                                   Implementation implementation) : name(std::move(name)),
                                                                    signature(std::move(signature)),
                                                                    implementation(std::move(implementation)) {}

    std::string NativeFunction::getName() const
    {
        return name;
    }

    std::shared_ptr<type::FunctionType> NativeFunction::getSignature() const
    {
        return signature;
    }

    NativeFunction::Implementation NativeFunction::getImplementation() const
    {
        return implementation;
    }

    int NativeFunction::getArity() const
    {
        return signature->arity();
    }

    std::string NativeFunction::toString() const
    {
        std::ostringstream outputStringStream;
        outputStringStream << "<NativeFunction: " << name << "(";
        if (const int arity = getArity(); arity == -1)
        {
            // Variadic-Function.
            outputStringStream << "...";
        }
        else
        {
            outputStringStream << arity;
        }
        outputStringStream << ")>";
        return outputStringStream.str();
    }

    std::ostream& operator<<(std::ostream& outputStream, const NativeFunction& nativeFunction)
    {
        return outputStream << nativeFunction.toString();
    }

    Thunk::Thunk(const frontend::ast::Expression* expression, std::shared_ptr<Environment> environment,
                 const std::optional<source::Location>& origin) : expression(expression),
                                                                  environment(std::move(environment)),
                                                                  origin(origin) {}

    const Value& Thunk::force(Context& context) const
    {
        if (cached.has_value())
        {
            return cached.value();
        }
        // Expression is not initialized
        if (!expression)
        {
            context.getDiagnosticEmitter().error(
                {origin.value(), origin.value()},
                diagnostics::DiagnosticId::RUNTIME_FORCE_EMPTY_THUNK
            );
        }
        cached = evalExpression(*expression, environment, context);
        return cached.value();
    }

    void Thunk::set(const frontend::ast::Expression* expression_, std::shared_ptr<Environment> environment_,
                    std::optional<source::Location> origin_)
    {
        expression = expression_;
        owned.reset();
        environment = std::move(environment_);
        if (origin_.has_value())
        {
            origin = origin_.value();
        }
        cached.reset();
    }

    void Thunk::setOwned(frontend::ast::Expression expression_, std::shared_ptr<Environment> environment_,
                         std::optional<source::Location> origin_)
    {
        owned = std::make_unique<frontend::ast::Expression>(std::move(expression_));
        expression = owned.get();
        environment = std::move(environment_);
        if (origin_.has_value())
        {
            origin = origin_.value();
        }
        cached.reset();
    }

    [[nodiscard]] source::Range Thunk::getRange() noexcept
    {
        // TODO: Return the range when we starts to store it instead of Location.
        return {};
    }

    Environment::Environment(std::shared_ptr<Environment> parent) : parent(std::move(parent)) {}

    std::shared_ptr<Thunk> Environment::lookup(const std::string& name) const
    {
        // Precedence: Local Environment > Global Environment.
        if (const auto it = table.find(name); it != table.end())
        {
            return it->second;
        }
        if (parent)
        {
            return parent->lookup(name);
        }
        return nullptr;
    }

    void Environment::bind(const std::string& name, std::shared_ptr<Thunk> thunk)
    {
        table[name] = std::move(thunk);
    }

    std::vector<std::vector<std::string>> Environment::toVector(Context& context, const bool force) const
    {
        std::vector<std::vector<std::string>> result;
        result.reserve(table.size());
        for (const auto& [name, thunk] : table)
        {
            std::string valueString = "<Thunk: unevaluated>";
            try
            {
                if (force)
                {
                    const Value& value = thunk->force(context);
                    valueString = escapeString(value.toString());
                }
                else if (thunk->cached)
                {
                    valueString = escapeString(thunk->cached->toString()); // Already computed.
                }
            }
            catch (const std::exception&) {}
            result.push_back({name, valueString});
        }
        if (parent)
        {
            auto parentVector = parent->toVector(context, force);
            result.insert(result.end(), std::make_move_iterator(parentVector.begin()),
                          std::make_move_iterator(parentVector.end()));
        }
        return result;
    }

    static Value evaluateIdentifierAstNode(const frontend::ast::IdentifierAstNode& identifierAstNode,
                                           const std::shared_ptr<Environment>& environment, Context& context)
    {
        const auto thunk = environment->lookup(identifierAstNode.value);
        if (!thunk)
        {
            context.getDiagnosticEmitter().error(
                source::Range(identifierAstNode.location),
                diagnostics::DiagnosticId::RUNTIME_UNDEFINED_IDENTIFIER, identifierAstNode.value
            );
        }
        return thunk->force(context);
    }

    static Value evaluateLambdaExpression(const frontend::ast::LambdaExpression& lambdaExpression,
                                          const std::shared_ptr<Environment>& environment)
    {
        return Value(Closure{lambdaExpression.argument.value, lambdaExpression.expression.get(), environment});
    }

    static Value evaluateFunctionApplication(const frontend::ast::FunctionApplication& functionApplication,
                                             const std::shared_ptr<Environment>& environment, Context& context)
    {
        // Lookup the callee lazily.
        const auto calleeThunk = environment->lookup(functionApplication.functionName.value);
        if (!calleeThunk)
        {
            context.getDiagnosticEmitter().error(
                source::Range(functionApplication.location),
                diagnostics::DiagnosticId::RUNTIME_UNDEFINED_FUNCTION, functionApplication.functionName.value
            );
        }
        const Value functionValue = calleeThunk->force(context);
        std::vector<std::shared_ptr<Thunk>> argumentThunks;
        argumentThunks.reserve(functionApplication.arguments.size());
        for (const auto& argument : functionApplication.arguments)
        {
            argumentThunks.push_back(std::make_shared<Thunk>(argument.get(), environment));
        }
        source::Location newLocation = functionApplication.location;
        return applyFunctionApplication(context, functionValue, argumentThunks, environment, newLocation);
    }

    Value evalExpression(const frontend::ast::Expression& expression, std::shared_ptr<Environment> environment,
                         Context& context)
    {
        return std::visit([&]<typename T0>(T0&& arg)
        {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, frontend::ast::IdentifierAstNode>)
            {
                return evaluateIdentifierAstNode(arg, environment, context);
            }
            else if constexpr (std::is_same_v<T, frontend::ast::StringAstNode> || std::is_same_v<T,
                frontend::ast::FloatAstNode>)
            {
                return Value(arg.value);
            }
            else if constexpr (std::is_same_v<T, frontend::ast::LambdaExpression>)
            {
                return evaluateLambdaExpression(arg, environment);
            }
            else if constexpr (std::is_same_v<T, frontend::ast::FunctionApplication>)
            {
                return evaluateFunctionApplication(arg, environment, context);
            }
            else
            {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled expression");
            }
        }, expression.value);
    }

    static std::shared_ptr<Thunk> valueToThunk(const Value& value)
    {
        auto thunk = std::make_shared<Thunk>();
        thunk->cached = value;
        return thunk;
    }

    Value applyFunctionApplication(Context& context, const Value& functionName,
                                   const std::vector<std::shared_ptr<Thunk>>& arguments,
                                   const std::shared_ptr<Environment>& callSiteEnvironment,
                                   const std::optional<source::Location>& callLocation)
    {
        // Local mutable copy of the args, for inserting evaluated values as Thunks when needed.
        std::vector<std::shared_ptr<Thunk>> workArguments = arguments;
        // Frame Stack: Functions to which we are currently applying Arguments.
        // Start with the initial Function.
        std::vector<Value> frames;
        frames.push_back(functionName);
        size_t index = 0; // Index of next Thunk to consume from work_args
        while (true)
        {
            // If there are no frames left, that's unexpected (shouldn't happen), bail.
            if (frames.empty())
            {
                context.getDiagnosticEmitter().error(
                    source::Range(callLocation.value()),
                    diagnostics::DiagnosticId::RUNTIME_EMPTY_CALL_STACK
                );
            }
            // Work on the top-most frame
            Value currentFunction = frames.back();
            std::optional<Value> resultantValue;
            // If we've consumed all available Argument Thunks:
            if (index >= workArguments.size())
            {
                // If top frame is a function, and we have no more args to feed it,
                // return that function (partial application) or its value as-is.
                if (std::holds_alternative<std::shared_ptr<NativeFunction>>(currentFunction))
                {
                    // Allow arity==0 and arity==-1 (variadic) to execute with zero args.
                    if (const auto& nativeFunction = *std::get<std::shared_ptr<NativeFunction>>(currentFunction);
                        nativeFunction.getArity() == 0 || nativeFunction.getArity() == -1)
                    {
                        std::vector<std::shared_ptr<Thunk>> slice; // empty
                        auto [value, result_options] = nativeFunction.getImplementation()(slice, callSiteEnvironment);
                        globalResultOptions.interpolate(result_options);
                        return value;
                    }
                    // Non-zero arity native function but no args left: partial application (return the function)
                    return currentFunction;
                }
                if (std::holds_alternative<Closure>(currentFunction))
                {
                    // Closures require one argument; with none left, this is a partial application (return the closure).
                    return currentFunction;
                }
                // It's not a function (shouldn't happen), return it as value.
                return currentFunction;
            }
            // Closure case: Closure consumes exactly one Argument (its Param)
            if (std::holds_alternative<Closure>(currentFunction))
            {
                const auto [param, body, env] = std::get<Closure>(currentFunction);
                const auto& argumentThunk = workArguments[index++];
                const auto childEnvironment = std::make_shared<Environment>(env);
                childEnvironment->bind(param, argumentThunk);
                resultantValue = evalExpression(*body, childEnvironment, context);
            }
            // Native Function case: Consumes its arity-many Argument Thunks
            else if (std::holds_alternative<std::shared_ptr<NativeFunction>>(currentFunction))
            {
                if (const auto& nativeFunction = *std::get<std::shared_ptr<NativeFunction>>(currentFunction);
                    nativeFunction.getArity() != -1)
                {
                    const int arity = nativeFunction.getArity();
                    const type::FunctionType& signature = *nativeFunction.getSignature();
                    if (workArguments.size() - index < arity)
                    {
                        context.getDiagnosticEmitter().error(
                            source::Range(callLocation.value()),
                            diagnostics::DiagnosticId::RUNTIME_NATIVE_FUNCTION_SIGNATURE_MISMATCH,
                            nativeFunction.getName(), arity, workArguments.size() - index,
                            nativeFunction.getName(), signature.toString()
                        );
                    }
                    std::vector<std::shared_ptr<Thunk>> slice;
                    slice.reserve(arity);
                    for (size_t i = 0; i < arity; ++i)
                    {
                        slice.push_back(workArguments[index + i]);
                    }
                    // Type-Check the function-arguments.
                    if (!signature.matchesArgumentTypes(context, slice))
                    {
                        context.getDiagnosticEmitter().error(
                            source::Range(callLocation.value()),
                            diagnostics::DiagnosticId::RUNTIME_INVALID_INPUTS_TO_NATIVE_FUNCTION,
                            nativeFunction.getName(),
                            nativeFunction.getName(), signature.toString()
                        );
                        // TODO: Also print exactly which argument caused the error.
                        //       Print the signature got, with the incorrect argument colored differently.
                    }
                    auto [resultantValue_, resultOptions] = nativeFunction.getImplementation()(
                        slice, callSiteEnvironment);
                    if (!signature.matchesReturnType(resultantValue_))
                    {
                        context.getDiagnosticEmitter().error(
                            source::Range(callLocation.value()),
                            diagnostics::DiagnosticId::INTERNAL_RETURN_TYPE_MISMATCH,
                            nativeFunction.getName(), signature.getReturnType().toString(),
                            type::typeFromValue(resultantValue_)->toString()
                        );
                        // TODO: possibly not create an additional type-object just for printing.
                    }
                    resultantValue = resultantValue_;
                    globalResultOptions.interpolate(resultOptions);
                    index += arity;
                }
                else
                {
                    std::vector<std::shared_ptr<Thunk>> slice;
                    slice.reserve(arguments.size() - index);
                    for (size_t i = 0; i < arguments.size() - index; ++i)
                    {
                        slice.push_back(workArguments[index + i]);
                    }
                    auto [resultantValue_, resultOptions] = nativeFunction.getImplementation()(
                        slice, callSiteEnvironment);
                    resultantValue = resultantValue_;
                    globalResultOptions.interpolate(resultOptions);
                    index += arguments.size() - index;
                }
            }
            else
            {
                // Top frame is not a Function (Closure, NativeFunction) Value but there are still Arguments left
                context.getDiagnosticEmitter().error(
                    source::Range(callLocation.value()),
                    diagnostics::DiagnosticId::RUNTIME_APPLY_NON_FUNCTION_VALUE, frames.back().toString()
                );
            }
            if (!resultantValue)
            {
                context.getDiagnosticEmitter().error(
                    source::Range(callLocation.value()),
                    diagnostics::DiagnosticId::INTERNAL_NO_RESULT
                );
            }
            if (std::holds_alternative<Closure>(*resultantValue) ||
                std::holds_alternative<std::shared_ptr<NativeFunction>>(*resultantValue))
            {
                // Replace the top Frame with the returned Function (curry)
                frames.back() = std::move(*resultantValue);
                continue;
            }
            // Result is a concrete Value (different from Function Value i.e. double, string).
            // Pop the frame that produced it, and insert this Value as a thunk at the current index
            // so the previous Frame (if any) will consume it.
            Value concrete = std::move(*resultantValue);
            frames.pop_back();
            // If there are no more Frames after popping, then this Value is the final result of the
            // whole Function Application, but only valid if there are no further Argument Thunks remaining.
            if (frames.empty())
            {
                if (index < workArguments.size())
                {
                    context.getDiagnosticEmitter().error(
                        source::Range(callLocation.value()),
                        diagnostics::DiagnosticId::RUNTIME_TOO_MANY_ARGUMENTS_TO_FUNCTION, concrete.toString()
                    );
                }
                // No Thunks remaining then concrete Value is the result
                return concrete;
            }
            // There is a previous Frame; insert the concrete Value as a Thunk at the
            // current index so the previous Frame will consume it on the next iteration.
            auto thunk = valueToThunk(concrete);
            workArguments.insert(workArguments.begin() + static_cast<int>(index), thunk);
            // Do NOT advance index: the inserted thunk is at position index and will be consumed
            // by the previous frame on the next iteration.
            // Loop will continue with frames.back() being the previous frame.
        }
    }

    // Creates placeholder Thunk then set body so recursion can refer to it during lazy evaluation
    static void bindDefinitionAstNodeLazy(frontend::ast::DefinitionAstNode& definitionAstNode,
                                          const std::shared_ptr<Environment>& environment,
                                          Context& context)
    {
        const auto thunk = std::make_shared<Thunk>();
        environment->bind(definitionAstNode.definitionName.value, thunk);
        if (context.getOptions().ownExpression)
        {
            thunk->setOwned(std::move(definitionAstNode.expression), environment,
                            definitionAstNode.expression.getLocation());
        }
        else
        {
            thunk->set(&definitionAstNode.expression, environment, definitionAstNode.expression.getLocation());
        }
    }

    Result interpret(const frontend::Program& program, Context& context,
                     std::optional<std::shared_ptr<Environment>> globalEnvironment) noexcept
    {
        if (!globalEnvironment)
        {
            globalEnvironment = std::make_shared<Environment>();
            installBuiltins(context, *globalEnvironment);
        }
        Value resultantValue;
        for (const auto& node : program.astNodes)
        {
            std::visit([&]<typename T0>(T0&& argument)
            {
                using T = std::decay_t<T0>;
                if constexpr (std::is_same_v<T, frontend::ast::Expression>)
                {
                    resultantValue = evalExpression(argument, *globalEnvironment, context);
                }
                else if constexpr (std::is_same_v<T, frontend::ast::DefinitionAstNode>)
                {
                    bindDefinitionAstNodeLazy(argument, *globalEnvironment, context);
                    const frontend::ast::DefinitionAstNode& definitionAstNode = argument;
                    resultantValue = definitionAstNode.definitionName.value;
                }
                else
                {
                    STATIC_ASSERT_UNREACHABLE_T(T, "unhandled program node");
                }
            }, node->value);
        }
        return {*globalEnvironment, resultantValue, globalResultOptions};
    }

    void installBuiltins(Context& context, const std::shared_ptr<Environment>& environment)
    {
        for (auto& nativeFunction : builtins::getBuiltins(context))
        {
            const auto thunk = std::make_shared<Thunk>();
            thunk->cached = Value{std::make_shared<NativeFunction>(nativeFunction)};
            environment->bind(nativeFunction.getName(), thunk);
        }
    }
}
