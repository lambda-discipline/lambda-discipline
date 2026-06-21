#pragma once

#include <functional>
#include <optional>
#include <string>
#include <variant>
#include <lbd/Context.hpp>
#include <lbd/frontend/Program.hpp>
#include <lbd/frontend/ast/AstNode.hpp>
#include <lbd/frontend/ast/expression/Expression.hpp>

namespace lbd::runtime::type
{
  struct FunctionType;
}

namespace lbd::runtime
{
  struct NativeFunction;
  struct Thunk;
  struct Environment;
  struct List;
  struct Value;

  /// Runtime representation of Lambda-Expression.
  struct Closure
  {
    std::string parameter;
    const frontend::ast::expression::Expression *body; /// Non-owning, read-only AST pointer.
    std::shared_ptr<Environment> environment; /// Environment at the time of Lambda-Expression creation.

    [[nodiscard]] std::string toString() const;

    friend std::ostream &operator<<(std::ostream &outputStream, const Closure &closure);
  };

  struct List
  {
    std::vector<Value> elements;

    [[nodiscard]] std::string toString() const;

    friend std::ostream &operator<<(std::ostream &outputStream, const List &list);
  };

  using ValueVariant = std::variant<
    double, // TODO: Create a type alias called Number for this.
    std::string,
    Closure,
    std::shared_ptr<NativeFunction>,
    std::shared_ptr<List>
  >;

  struct Value : ValueVariant
  {
    using ValueVariant::ValueVariant; // Inherit constructors.

    /// Pretty print a runtime value for REPL/diagnostics.
    [[nodiscard]] std::string toString() const;

    friend std::ostream &operator<<(std::ostream &outputStream, const Value &value);
  };

  struct ResultOptions
  {
    bool sideEffects = false;

    void interpolate(const ResultOptions &resultOptions);
  };

  struct NativeFunction
  {
    using Implementation = std::function<std::pair<Value, ResultOptions>
      (const std::vector<std::shared_ptr<Thunk>> &, const std::shared_ptr<Environment> &)>;

    NativeFunction(std::string name, std::shared_ptr<type::FunctionType> signature, Implementation implementation);

    [[nodiscard]] std::string getName() const;

    [[nodiscard]] std::shared_ptr<type::FunctionType> getSignature() const;

    [[nodiscard]] Implementation getImplementation() const;

    [[nodiscard]] int getArity() const;

    [[nodiscard]] std::string toString() const;

    friend std::ostream &operator<<(std::ostream &outputStream, const NativeFunction &nativeFunction);

  private:
    std::string name;
    std::shared_ptr<type::FunctionType> signature;
    Implementation implementation;
  };

  /// Lazy-Thunk (call-by-need).
  struct Thunk : std::enable_shared_from_this<Thunk>
  {
    mutable std::optional<Value> cached;
    const frontend::ast::expression::Expression *expression = nullptr; /// Non-owning, read-only AST pointer.
    frontend::ast::expression::ExpressionPtr owned; /// Owning storage (when needed) (primarily in REPL).
    std::shared_ptr<Environment> environment; /// Environment for evaluating Expression.
    std::optional<source::Range> origin = std::nullopt;

    Thunk() = default;

    Thunk(const frontend::ast::expression::Expression *expression, std::shared_ptr<Environment> environment,
          const std::optional<source::Range> &origin = std::nullopt);

    /// Force computation on Thunk and return a const reference to Value.
    const Value &force(Context &context) const; /// Marked const as cached is mutable.

    /// Sets Thunk's fields after construction.
    /// Allows for recursive reference.
    void set(const frontend::ast::expression::Expression *expression_, std::shared_ptr<Environment> environment_,
             std::optional<source::Range> origin_ = std::nullopt);

    void setOwned(frontend::ast::expression::ExpressionPtr expression_, std::shared_ptr<Environment> environment_,
                  std::optional<source::Range> origin_ = std::nullopt);

    [[nodiscard]] source::Range getRange() const noexcept;
  };

  struct Environment : std::enable_shared_from_this<Environment>
  {
    std::unordered_map<std::string, std::shared_ptr<Thunk>> table;
    std::shared_ptr<Environment> parent;

    explicit Environment(std::shared_ptr<Environment> parent = nullptr);

    std::shared_ptr<Thunk> lookup(const std::string &name) const;

    void bind(const std::string &name, std::shared_ptr<Thunk> thunk);

    std::vector<std::vector<std::string>> toVector(Context &context, bool force) const;
  };

  Value evaluateExpression(const frontend::ast::expression::Expression &expression,
                           const std::shared_ptr<Environment> &environment,
                           Context &context);

  std::vector<Value> forceArguments(const std::vector<std::shared_ptr<Thunk>> &arguments);

  Value applyFunctionApplication(Context &context, const Value &functionName,
                                 const std::vector<std::shared_ptr<Thunk>> &arguments,
                                 const std::shared_ptr<Environment> &callSiteEnvironment,
                                 const std::optional<source::Range> &callRange = std::nullopt);

  /// Program Driver.
  struct Result
  {
    std::shared_ptr<Environment> globalEnvironment;
    Value value;
    ResultOptions options = {};
  };

  Result interpret(frontend::Program &program, Context &context,
                   std::optional<std::shared_ptr<Environment>> globalEnvironment = std::nullopt) noexcept;

  /// Add builtins Native Functions into Environment.
  void installBuiltins(Context &context, const std::shared_ptr<Environment> &environment);
}
