#pragma once

#include <functional>
#include <optional>
#include <string>
#include <variant>
#include <lbd/frontend/ast.h>
#include <lbd/frontend/parser.h>
#include <lbd/options.h>

namespace interpreter {
    struct NativeFunction;
    struct Thunk;
    struct Environment;
    struct List;
    struct Value;

    /// Runtime representation of Lambda-Expression.
    struct Closure {
        std::string parameter;
        const frontend::Expression *body; /// Non-owning, read-only AST pointer.
        std::shared_ptr<Environment> environment; /// Environment at the time of Lambda-Expression creation.

        [[nodiscard]] std::string toString() const;

        friend std::ostream &operator<<(std::ostream &outputStream, const Closure &closure);
    };

    struct List {
        std::vector<Value> elements;

        [[nodiscard]] std::string toString() const;

        friend std::ostream &operator<<(std::ostream &outputStream, const List &list);
    };

    using ValueVariant = std::variant<
        double,
        std::string,
        Closure,
        std::shared_ptr<NativeFunction>,
        std::shared_ptr<List>
    >;

    struct Value : ValueVariant {
        using ValueVariant::ValueVariant; // Inherit constructors.

        /// Pretty print a runtime value for REPL/diagnostics.
        [[nodiscard]] std::string toString() const;

        friend std::ostream &operator<<(std::ostream &outputStream, const Value &value);
    };

    struct ResultOptions {
        bool sideEffects = false;

        // TODO: _ may not be needed here
        void interpolate(const ResultOptions &resultOptions);
    };

    struct NativeFunction {
        using Implementation = std::function<std::pair<Value, ResultOptions>(
            const std::vector<std::shared_ptr<Thunk> > &, const std::shared_ptr<Environment> &)>;

        int arity;
        std::string name;
        Implementation implementation;

        [[nodiscard]] std::string toString() const;

        friend std::ostream &operator<<(std::ostream &outputStream, const NativeFunction &nativeFunction);
    };

    /// Lazy-Thunk (call-by-need).
    struct Thunk : std::enable_shared_from_this<Thunk> {
        mutable std::optional<Value> cached;
        const frontend::Expression *expression = nullptr; /// Non-owning, read-only AST pointer.
        std::unique_ptr<frontend::Expression> owned; /// Owning storage (when needed) (primarily in REPL).
        std::shared_ptr<Environment> environment; /// Environment for evaluating Expression.
        std::optional<frontend::Location> origin = std::nullopt;

        Thunk() = default;

        Thunk(const frontend::Expression *expression, std::shared_ptr<Environment> environment,
              std::optional<frontend::Location> origin = std::nullopt);

        /// Force computation on Thunk and return a const reference to Value.
        const Value &force() const; /// Marked const as cached is mutable.

        /// Sets Thunk's fields after construction.
        /// Allows for recursive reference.
        void set(const frontend::Expression *expression_, std::shared_ptr<Environment> environment_,
                 std::optional<frontend::Location> origin_ = std::nullopt);

        void setOwned(frontend::Expression expression_, std::shared_ptr<Environment> environment_,
                      std::optional<frontend::Location> origin_ = std::nullopt);
    };

    struct Environment : std::enable_shared_from_this<Environment> {
        std::unordered_map<std::string, std::shared_ptr<Thunk> > table;
        std::shared_ptr<Environment> parent;

        explicit Environment(std::shared_ptr<Environment> parent = nullptr);

        std::shared_ptr<Thunk> lookup(const std::string &name) const;

        void bind(const std::string &name, std::shared_ptr<Thunk> thunk);

        std::vector<std::vector<std::string> > toVector(bool force) const;
    };

    Value evalExpression(const frontend::Expression &expression, std::shared_ptr<Environment> environment);

    Value applyFunctionApplication(Value functionName, const std::vector<std::shared_ptr<Thunk> > &arguments,
                                   const std::shared_ptr<Environment> &callSiteEnvironment,
                                   const std::optional<frontend::Location> &callLocation = std::nullopt);

    /// Program Driver.
    struct Result {
        std::shared_ptr<Environment> globalEnvironment;
        Value value;
        ResultOptions options = {};
    };

    Result interpret(frontend::Program &program,
                     std::optional<std::shared_ptr<Environment> > globalEnvironment = std::nullopt,
                     context::Options options_ = {});

    /// Add builtins Native Functions into Environment.
    void installBuiltins(const std::shared_ptr<Environment> &environment);
}
