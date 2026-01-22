#pragma once

#include <functional>
#include <optional>
#include <string>
#include <variant>
#include <lbd/fe/ast.h>
#include <lbd/fe/parser.h>
#include <lbd/options.h>

namespace intp::interp {
    struct NativeFunction;
    struct Thunk;
    struct Environment;
    struct List;
    struct Value;

    /// Runtime representation of Lambda-Expression.
    struct Closure {
        std::string parameter;
        const fe::ast::Expression *body; /// Non-owning, read-only AST pointer.
        std::shared_ptr<Environment> environment; /// Environment at the time of Lambda-Expression creation.

        [[nodiscard]] std::string toString() const;

        friend std::ostream &operator<<(std::ostream &stream, const Closure &closure);
    };

    struct List {
        std::vector<Value> elements;

        [[nodiscard]] std::string toString() const;

        friend std::ostream &operator<<(std::ostream &stream, const List &list);
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

        friend std::ostream &operator<<(std::ostream &stream, const Value &value);
    };

    struct ResultOptions {
        bool sideEffects = false;

        void interpolate(const ResultOptions &resultOptions_);
    };

    struct NativeFunction {
        using Implementation = std::function<std::pair<Value, ResultOptions>(
            const std::vector<std::shared_ptr<Thunk> > &, const std::shared_ptr<Environment> &)>;

        int arity;
        std::string name;
        Implementation implementation;

        [[nodiscard]] std::string toString() const;

        friend std::ostream &operator<<(std::ostream &stream, const NativeFunction &nativeFunction);
    };

    /// Lazy-Thunk (call-by-need).
    struct Thunk : std::enable_shared_from_this<Thunk> {
        mutable std::optional<Value> cached;
        const fe::ast::Expression *expression = nullptr; /// Non-owning, read-only AST pointer.
        std::unique_ptr<fe::ast::Expression> owned; /// Owning storage (when needed) (primarily in REPL).
        std::shared_ptr<Environment> environment; /// Environment for evaluating Expression.
        std::optional<fe::loc::Loc> origin = std::nullopt;

        Thunk() = default;

        Thunk(const fe::ast::Expression *expression, std::shared_ptr<Environment> environment,
              std::optional<fe::loc::Loc> origin = std::nullopt);

        /// Force computation on Thunk and return a const reference to Value.
        const Value &force() const; /// Marked const as cached is mutable.

        /// Sets Thunk's fields after construction.
        /// Allows for recursive reference.
        void set(const fe::ast::Expression *expression_, std::shared_ptr<Environment> environment_,
                 std::optional<fe::loc::Loc> origin_ = std::nullopt);

        void set_owned(fe::ast::Expression expression_, std::shared_ptr<Environment> environment_,
                       std::optional<fe::loc::Loc> origin_ = std::nullopt);
    };

    struct Environment : std::enable_shared_from_this<Environment> {
        std::unordered_map<std::string, std::shared_ptr<Thunk> > table;
        std::shared_ptr<Environment> parent;

        explicit Environment(std::shared_ptr<Environment> parent = nullptr);

        std::shared_ptr<Thunk> lookup(const std::string &name) const;

        void bind(const std::string &name, std::shared_ptr<Thunk> thunk);

        std::vector<std::vector<std::string> > toVector(bool force) const;
    };

    Value evalExpression(const fe::ast::Expression &expression, std::shared_ptr<Environment> environment);

    Value applyFunctionApplication(Value functionName, const std::vector<std::shared_ptr<Thunk> > &arguments,
                                   const std::shared_ptr<Environment> &callSiteEnvironment,
                                   const std::optional<fe::loc::Loc> &callLocation = std::nullopt);

    /// Program Driver.
    struct Result {
        std::shared_ptr<Environment> globalEnvironment;
        Value value;
        ResultOptions options = {};
    };

    Result interpret(fe::ast::Program &program, std::optional<std::shared_ptr<Environment> > globalEnvironment = std::nullopt,
                     options::Options options_ = {});

    /// Add builtins Native Functions into Environment.
    void installBuiltins(const std::shared_ptr<Environment> &environment);
}
