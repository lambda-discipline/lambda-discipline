#pragma once

#include <functional>
#include <optional>
#include <string>
#include <variant>
#include <lbd/fe/ast.h>
#include <lbd/fe/parser.h>

namespace intp::interp {
    struct NativeFunction;
    struct Thunk;
    struct Env;
    struct List;
    struct Value;

    /// Runtime representation of Lambda Expression
    struct Closure {
        std::string param;
        const fe::ast::Expression *body; /// Non-owning, read-only AST pointer
        std::shared_ptr<Env> env; /// Environment at the time of Lambda Expression creation

        [[nodiscard]] std::string to_string() const;

        friend std::ostream &operator<<(std::ostream &os, const Closure &closure);
    };

    struct List {
        std::vector<Value> elements;

        [[nodiscard]] std::string to_string() const;

        friend std::ostream &operator<<(std::ostream &os, const List &list);
    };

    using ValueVariant = std::variant<
        double,
        std::string,
        Closure,
        std::shared_ptr<NativeFunction>,
        std::shared_ptr<List>
    >;

    struct Value : ValueVariant {
        using ValueVariant::ValueVariant; // inherit constructors

        /// Pretty print a runtime value for REPL/diagnostics
        [[nodiscard]] std::string to_string() const;

        friend std::ostream &operator<<(std::ostream &os, const Value &value);
    };

    struct NativeFunction {
        using Impl = std::function<Value(
            const std::vector<std::shared_ptr<Thunk> > &, const std::shared_ptr<Env> &)>;

        std::size_t arity;
        std::string name;
        Impl impl;

        [[nodiscard]] std::string to_string() const;

        friend std::ostream &operator<<(std::ostream &os, const NativeFunction &native_fn);
    };

    /// Lazy Thunk (call-by-need)
    struct Thunk : std::enable_shared_from_this<Thunk> {
        mutable std::optional<Value> cached;
        const fe::ast::Expression *expr = nullptr; /// Non-owning, read-only AST pointer
        std::unique_ptr<fe::ast::Expression> owned; /// Owning storage (when needed) (primarily in REPL)
        std::shared_ptr<Env> env; /// Environment for evaluating Expression
        std::optional<fe::loc::Loc> origin = std::nullopt;

        Thunk() = default;

        Thunk(const fe::ast::Expression *expr, std::shared_ptr<Env> env,
              std::optional<fe::loc::Loc> origin = std::nullopt);

        /// Force computation on Thunk and return a const reference to Value
        const Value &force() const; /// Marked const as cached is mutable

        /// Sets Thunk's fields after construction
        /// Allows for recursive reference
        void set(const fe::ast::Expression *expr_, std::shared_ptr<Env> env_,
                 std::optional<fe::loc::Loc> origin_ = std::nullopt);

        void set_owned(fe::ast::Expression expr_, std::shared_ptr<Env> env_,
                       std::optional<fe::loc::Loc> origin_ = std::nullopt);
    };

    struct Env : std::enable_shared_from_this<Env> {
        std::unordered_map<std::string, std::shared_ptr<Thunk> > table;
        std::shared_ptr<Env> parent;

        explicit Env(std::shared_ptr<Env> parent = nullptr);

        std::shared_ptr<Thunk> lookup(const std::string &name) const;

        void bind(const std::string &name, std::shared_ptr<Thunk> thunk);

        std::vector<std::vector<std::string> > to_vector(bool force) const;
    };

    Value eval_expr(const fe::ast::Expression &expr, std::shared_ptr<Env> env);

    Value apply_fn_apl(Value fn_value, const std::vector<std::shared_ptr<Thunk> > &args,
                       const std::shared_ptr<Env> &call_site_env,
                       const std::optional<fe::loc::Loc> &call_loc = std::nullopt);

    /// Program Driver
    struct Result {
        std::shared_ptr<Env> global_env;
        Value value;
    };

    struct Options {
        bool own_expr = false;
    };

    Result interpret(fe::ast::Program &program, std::optional<std::shared_ptr<Env> > global_env = std::nullopt,
                     Options options = {});

    /// Add builtins Native Functions into Environment
    void install_builtins(const std::shared_ptr<Env> &env);
}
