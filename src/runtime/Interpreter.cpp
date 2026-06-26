#include <sstream>
#include <utility>
#include <lbd/Error.hpp>
#include <lbd/Options.hpp>
#include <lbd/frontend/ast/expression/FunctionApplicationExpression.hpp>
#include <lbd/frontend/ast/expression/IdentifierExpression.hpp>
#include <lbd/frontend/ast/expression/LambdaExpression.hpp>
#include <lbd/frontend/ast/expression/NumberExpression.hpp>
#include <lbd/frontend/ast/expression/StringExpression.hpp>
#include <lbd/frontend/ast/statement/ExpressionStatement.hpp>
#include <lbd/frontend/ast/statement/SymbolDefinitionStatement.hpp>
#include <lbd/runtime/Builtins.hpp>
#include <lbd/runtime/Interpreter.hpp>
#include <lbd/runtime/Type.hpp>
#include <lbd/utils/StringEscaping.hpp>

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

  std::ostream &operator<<(std::ostream &outputStream, const Closure &closure)
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

  std::ostream &operator<<(std::ostream &outputStream, const List &list)
  {
    return outputStream << list.toString();
  }

  [[nodiscard]] std::string Value::toString() const
  {
    return std::visit([&]<typename T0>(T0 &&arg)
    {
      using T = std::decay_t<T0>;
      if constexpr (std::is_same_v<T, double>)
      {
        return std::to_string(arg);
      } else if constexpr (std::is_same_v<T, std::string>)
      {
        return unescapeString(arg);
      } else if constexpr (std::is_same_v<T, Closure>)
      {
        return arg.toString();
      } else if constexpr (std::is_same_v<T, std::shared_ptr<NativeFunction>> ||
                           std::is_same_v<T, std::shared_ptr<List>>)
      {
        return arg->toString();
      } else
      {
        STATIC_ASSERT_UNREACHABLE_T(T, "unhandled runtime value");
      }
    }, *this);
  }

  std::ostream &operator<<(std::ostream &outputStream, const Value &value)
  {
    return outputStream << value.toString();
  }

  void ResultOptions::interpolate(const ResultOptions &resultOptions)
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
    } else
    {
      outputStringStream << arity;
    }
    outputStringStream << ")>";
    return outputStringStream.str();
  }

  std::ostream &operator<<(std::ostream &outputStream, const NativeFunction &nativeFunction)
  {
    return outputStream << nativeFunction.toString();
  }

  Thunk::Thunk(const frontend::ast::expression::Expression *expression, std::shared_ptr<Environment> environment,
               const std::optional<source::Range> &origin)
    : expression(expression), environment(std::move(environment)), origin(origin) {}

  const Value &Thunk::force(Context &context) const
  {
    if (cached.has_value())
    {
      return cached.value();
    }
    // Expression is not initialized
    if (!expression)
    {
      context.getDiagnosticEmitter().error(
        *origin,
        diagnostics::DiagnosticId::RUNTIME_FORCE_EMPTY_THUNK
      );
    }
    cached = evaluateExpression(*expression, environment, context);
    return cached.value();
  }

  void Thunk::set(const frontend::ast::expression::Expression *expression_, std::shared_ptr<Environment> environment_,
                  std::optional<source::Range> origin_)
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

  void Thunk::setOwned(frontend::ast::expression::ExpressionPtr expression_, std::shared_ptr<Environment> environment_,
                       std::optional<source::Range> origin_)
  {
    owned = std::move(expression_);
    expression = owned.get();
    environment = std::move(environment_);
    if (origin_.has_value())
    {
      origin = origin_.value();
    }
    cached.reset();
  }

  source::Range Thunk::getRange() const noexcept { return *origin; }

  Environment::Environment(std::shared_ptr<Environment> parent) : parent(std::move(parent)) {}

  std::shared_ptr<Thunk> Environment::lookup(const std::string &name) const
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

  void Environment::bind(const std::string &name, std::shared_ptr<Thunk> thunk)
  {
    table[name] = std::move(thunk);
  }

  std::vector<std::vector<std::string>> Environment::toVector(Context &context, const bool force) const
  {
    std::vector<std::vector<std::string>> result;
    result.reserve(table.size());
    for (const auto &[name, thunk]: table)
    {
      std::string valueString = "<Thunk: unevaluated>";
      try
      {
        if (force)
        {
          const Value &value = thunk->force(context);
          valueString = escapeString(value.toString());
        } else if (thunk->cached)
        {
          valueString = escapeString(thunk->cached->toString()); // Already computed.
        }
      } catch (const std::exception &) {}
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

  static Value evaluateIdentifierExpression(const frontend::ast::expression::IdentifierExpression &identifierExpression,
                                            const std::shared_ptr<Environment> &environment, Context &context)
  {
    const auto thunk = environment->lookup(identifierExpression.getName());
    if (!thunk)
    {
      context.getDiagnosticEmitter().error(
        identifierExpression.getRange(),
        diagnostics::DiagnosticId::RUNTIME_UNDEFINED_IDENTIFIER, identifierExpression.getName()
      );
    }
    return thunk->force(context);
  }

  static Value evaluateLambdaExpression(const frontend::ast::expression::LambdaExpression &lambdaExpression,
                                        const std::shared_ptr<Environment> &environment)
  {
    return Value(Closure{
      lambdaExpression.getArgumentIdentifierExpression().getName(),
      &lambdaExpression.getExpression(),
      environment
    });
  }

  static Value evaluateFunctionApplicationExpression(
    const frontend::ast::expression::FunctionApplicationExpression &functionApplicationExpression,
    const std::shared_ptr<Environment> &environment, Context &context)
  {
    // Lookup the callee lazily.
    const std::string &functionName = functionApplicationExpression.getFunctionNameIdentifierExpression().getName();
    const auto calleeThunk = environment->lookup(functionName);
    if (!calleeThunk)
    {
      context.getDiagnosticEmitter().error(
        functionApplicationExpression.getRange(),
        diagnostics::DiagnosticId::RUNTIME_UNDEFINED_FUNCTION, functionName
      );
    }

    const Value functionValue = calleeThunk->force(context);
    std::vector<std::shared_ptr<Thunk>> argumentThunks;
    argumentThunks.reserve(functionApplicationExpression.getArguments().size());
    for (const auto &argument: functionApplicationExpression.getArguments())
    {
      argumentThunks.push_back(std::make_shared<Thunk>(argument.get(), environment));
    }
    source::Range callRange = functionApplicationExpression.getRange();
    return applyFunctionApplication(context, functionValue, argumentThunks, environment, callRange);
  }

  Value evaluateExpression(const frontend::ast::expression::Expression &expression,
                           const std::shared_ptr<Environment> &environment, Context &context)
  {
    if (const auto *identifierExpression = dynamic_cast<const frontend::ast::expression::IdentifierExpression *>
        (&expression))
    {
      return evaluateIdentifierExpression(*identifierExpression, environment, context);
    }

    if (const auto *stringExpression = dynamic_cast<const frontend::ast::expression::StringExpression *>
        (&expression))
    {
      return Value{stringExpression->getValue()};
    }

    if (const auto *numberExpression = dynamic_cast<const frontend::ast::expression::NumberExpression *>
        (&expression))
    {
      return Value{numberExpression->getValue()};
    }

    if (const auto *lambdaExpression = dynamic_cast<const frontend::ast::expression::LambdaExpression *>
        (&expression))
    {
      return evaluateLambdaExpression(*lambdaExpression, environment);
    }

    if (const auto *functionApplicationExpression = dynamic_cast
        <const frontend::ast::expression::FunctionApplicationExpression *>(&expression))
    {
      return evaluateFunctionApplicationExpression(*functionApplicationExpression, environment, context);
    }

    // TODO: Add an assertion unimplemented.
    return Value{0.0};
  }

  static std::shared_ptr<Thunk> valueToThunk(const Value &value)
  {
    auto thunk = std::make_shared<Thunk>();
    thunk->cached = value;
    return thunk;
  }

  Value applyFunctionApplication(Context &context, const Value &functionName,
                                 const std::vector<std::shared_ptr<Thunk>> &arguments,
                                 const std::shared_ptr<Environment> &callSiteEnvironment,
                                 const std::optional<source::Range> &callRange)
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
          source::Range(callRange.value()),
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
          if (const auto &nativeFunction = *std::get<std::shared_ptr<NativeFunction>>(currentFunction);
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
        const auto &argumentThunk = workArguments[index++];
        const auto childEnvironment = std::make_shared<Environment>(env);
        childEnvironment->bind(param, argumentThunk);
        resultantValue = evaluateExpression(*body, childEnvironment, context);
      }
      // Native Function case: Consumes its arity-many Argument Thunks
      else if (std::holds_alternative<std::shared_ptr<NativeFunction>>(currentFunction))
      {
        if (const auto &nativeFunction = *std::get<std::shared_ptr<NativeFunction>>(currentFunction);
          nativeFunction.getArity() != -1)
        {
          const int arity = nativeFunction.getArity();
          const type::FunctionType &signature = *nativeFunction.getSignature();
          if (workArguments.size() - index < arity)
          {
            context.getDiagnosticEmitter().error(
              source::Range(callRange.value()),
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
              *callRange,
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
              *callRange,
              diagnostics::DiagnosticId::INTERNAL_RETURN_TYPE_MISMATCH,
              nativeFunction.getName(), signature.getReturnType().toString(),
              type::typeFromValue(resultantValue_)->toString()
            );
            // TODO: possibly not create an additional type-object just for printing.
          }
          resultantValue = resultantValue_;
          globalResultOptions.interpolate(resultOptions);
          index += arity;
        } else
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
      } else
      {
        // Top frame is not a Function (Closure, NativeFunction) Value but there are still Arguments left
        context.getDiagnosticEmitter().error(
          source::Range(callRange.value()),
          diagnostics::DiagnosticId::RUNTIME_APPLY_NON_FUNCTION_VALUE, frames.back().toString()
        );
      }
      if (!resultantValue)
      {
        context.getDiagnosticEmitter().error(
          source::Range(callRange.value()),
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
            source::Range(callRange.value()),
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
  static void bindSymbolDefinitionStatementLazy(
    frontend::ast::statement::SymbolDefinitionStatement &symbolDefinitionStatement,
    const std::shared_ptr<Environment> &environment, Context &context)
  {
    const auto thunk = std::make_shared<Thunk>();
    environment->bind(symbolDefinitionStatement.getSymbolNameIdentifierExpression().getName(), thunk);
    const auto expressionRange = symbolDefinitionStatement.getExpression().getRange();
    if (context.getOptions().ownExpression)
    {
      thunk->setOwned(symbolDefinitionStatement.releaseExpressionPtr(), environment, expressionRange);
    } else
    {
      thunk->set(&symbolDefinitionStatement.getExpression(), environment, expressionRange);
    }
  }

  Result interpret(frontend::Program &program, Context &context,
                   std::optional<std::shared_ptr<Environment>> globalEnvironment)
  {
    if (!globalEnvironment)
    {
      globalEnvironment = std::make_shared<Environment>();
      installBuiltins(context, *globalEnvironment);
    }

    Value resultantValue;

    for (const auto &astNodePtrs = program.getAstNodePtrs(); auto &astNodePtr: astNodePtrs)
    {
      auto &astNode = *astNodePtr;

      if (const auto *expressionStatement = dynamic_cast<const frontend::ast::statement::ExpressionStatement *>
          (&astNode))
      {
        resultantValue = evaluateExpression(expressionStatement->getExpression(), *globalEnvironment, context);
        continue;
      }

      if (auto *symbolDefinitionStatement = dynamic_cast<frontend::ast::statement::SymbolDefinitionStatement *>
          (&astNode))
      {
        bindSymbolDefinitionStatementLazy(*symbolDefinitionStatement, *globalEnvironment, context);
        resultantValue = symbolDefinitionStatement->getSymbolNameIdentifierExpression().getName();
        continue;
      }

      // TODO: Add assertion for unimplemented type.
    }
    return {*globalEnvironment, resultantValue, globalResultOptions};
  }

  void installBuiltins(Context &context, const std::shared_ptr<Environment> &environment)
  {
    for (auto &nativeFunction: builtins::getBuiltins(context))
    {
      const auto thunk = std::make_shared<Thunk>();
      thunk->cached = Value{std::make_shared<NativeFunction>(nativeFunction)};
      environment->bind(nativeFunction.getName(), thunk);
    }
  }
}
