#include <lbd/intp/interpreter.h>
#include <lbd/intp/builtins.h>
#include <lbd/error.h>

namespace intp::interp {
    std::ostream &operator<<(std::ostream &os, const Closure &closure) {
        return os << "<closure: " << closure.param << ">";
    }

    std::ostream &operator<<(std::ostream &os, const NativeFunction &native_fn) {
        return os << "<native_fn: " << native_fn.name << " " << native_fn.arity << ">";
    }

    static std::ostream &print_loc_prefix(std::ostream &os, const std::optional<fe::loc::Loc> &loc) {
        if (loc.has_value()) {
            return os << loc.value() << ": ";
        }
        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Value &value) {
        std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, std::string> ||
                          std::is_same_v<T, double> ||
                          std::is_same_v<T, Closure> ||
                          std::is_same_v<T, NativeFunction>) {
                os << arg;
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled runtime value");
            }
        }, value);
        return os;
    }

    Thunk::Thunk(const fe::ast::Expression *expr, std::shared_ptr<Env> env,
                 std::optional<fe::loc::Loc> origin) : expr(expr), env(std::move(env)), origin(std::move(origin)) {
    }

    const Value &Thunk::force() const {
        if (cached.has_value()) {
            return cached.value();
        }
        // Expression is not initialized
        if (!expr) {
            print_loc_prefix(std::cerr, origin)
                    << "runtime error: forcing empty thunk";
            exit(EXIT_FAILURE);
        }
        cached = eval_expr(*expr, env);
        return cached.value();
    }

    void Thunk::set(const fe::ast::Expression *expr_, std::shared_ptr<Env> env_,
                    std::optional<fe::loc::Loc> origin_) {
        expr = expr_;
        owned.reset();
        env = std::move(env_);
        if (origin_.has_value()) {
            origin = std::move(origin_.value());
        }
        cached.reset();
    }

    void Thunk::set_owned(fe::ast::Expression expr_, std::shared_ptr<Env> env_,
                          std::optional<fe::loc::Loc> origin_) {
        owned = std::make_unique<fe::ast::Expression>(std::move(expr_));
        expr = owned.get();
        env = std::move(env_);
        if (origin_.has_value()) {
            origin = std::move(origin_.value());
        }
        cached.reset();
    }

    Env::Env(std::shared_ptr<Env> parent) : parent(std::move(parent)) {
    }

    std::shared_ptr<Thunk> Env::lookup(const std::string &name) const {
        // Precedence: Local Environment > Global Environment
        if (const auto it = table.find(name); it != table.end()) {
            return it->second;
        }
        if (parent) {
            return parent->lookup(name);
        }
        return nullptr;
    }

    void Env::bind(const std::string &name, std::shared_ptr<Thunk> thunk) {
        table[name] = std::move(thunk);
    }

    void Env::dump(std::ostream &os, const bool force) const {
        for (const auto &[name, thunk]: table) {
            os << "  " << name << " = ";
            try {
                if (force) {
                    const Value &val = thunk->force();
                    os << val;
                } else {
                    if (thunk->cached) {
                        os << *(thunk->cached); // already computed
                    } else {
                        os << "<thunk: unevaluated>";
                    }
                }
            } catch (const std::exception &ex) {
                os << "<thunk: unevaluated>";
            }
            os << std::endl;
        }
        if (parent) {
            os << "info: parent environment" << std::endl;
            parent->dump(os, force);
        }
    }

    static Value eval_iden_ast_node(const fe::ast::IdenAstNode &iden_ast_node, const std::shared_ptr<Env> &env) {
        const auto thunk = env->lookup(iden_ast_node.value);
        if (!thunk) {
            print_loc_prefix(std::cerr, iden_ast_node.loc)
                    << "runtime error: undefined identifier "
                    << iden_ast_node.value
                    << std::endl;
            exit(EXIT_FAILURE);
        }
        return thunk->force();
    }

    static Value eval_lambda_expr(const fe::ast::LambdaExpression &l_expr, const std::shared_ptr<Env> &env) {
        return Value(Closure{l_expr.arg.value, l_expr.expr.get(), env});
    }

    static Value eval_fn_apl(const fe::ast::FunctionApplication &fn_apl, const std::shared_ptr<Env> &env) {
        // Lookup the callee lazily
        const auto callee_thunk = env->lookup(fn_apl.fn_name.value);
        if (!callee_thunk) {
            print_loc_prefix(std::cerr, fn_apl.loc)
                    << "runtime error: undefined function "
                    << fn_apl.fn_name.value
                    << std::endl;
            exit(EXIT_FAILURE);
        }
        const Value fn_value = callee_thunk->force();
        std::vector<std::shared_ptr<Thunk> > arg_thunks;
        arg_thunks.reserve(fn_apl.args.size());
        for (const auto &arg: fn_apl.args) {
            arg_thunks.push_back(std::make_shared<Thunk>(arg.get(), env));
        }
        return apply_fn_apl(fn_value, arg_thunks, env, fn_apl.loc);
    }

    Value eval_expr(const fe::ast::Expression &expr, std::shared_ptr<Env> env) {
        return std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, fe::ast::IdenAstNode>) {
                return eval_iden_ast_node(arg, env);
            } else if constexpr (std::is_same_v<T, fe::ast::StringAstNode> || std::is_same_v<T,
                                     fe::ast::FloatAstNode>) {
                return Value(arg.value);
            } else if constexpr (std::is_same_v<T, fe::ast::LambdaExpression>) {
                return eval_lambda_expr(arg, env);
            } else if constexpr (std::is_same_v<T, fe::ast::FunctionApplication>) {
                return eval_fn_apl(arg, env);
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled expression");
            }
        }, expr.value);
    }

    static std::shared_ptr<Thunk> value_to_thunk(const Value &v) {
        auto t = std::make_shared<Thunk>();
        t->cached = v;
        return t;
    }

    Value apply_fn_apl(Value fn_value, const std::vector<std::shared_ptr<Thunk> > &args,
                       const std::shared_ptr<Env> &call_site_env, const std::optional<fe::loc::Loc> &call_loc) {
        // Local mutable copy of the args, for inserting evaluated values as Thunks when needed.
        std::vector<std::shared_ptr<Thunk> > work_args = args;
        // Frame Stack: Functions to which we are currently applying Arguments.
        // Start with the initial Function.
        std::vector<Value> frames;
        frames.push_back(std::move(fn_value));
        size_t idx = 0; // Index of next Thunk to consume from work_args
        while (true) {
            // If there are no frames left, that's unexpected (shouldn't happen), bail.
            if (frames.empty()) {
                print_loc_prefix(std::cerr, call_loc)
                        << "internal error: no function frame to apply" << std::endl;
                std::exit(EXIT_FAILURE);
            }
            // If we've consumed all available Argument Thunks:
            if (idx >= work_args.size()) {
                // If top frame is a function, and we have no more args to feed it,
                // return that function (partial application) or its value as-is.
                // If it's not a function (shouldn't happen), return it as value.
                return frames.back();
            }
            // Work on the top-most frame
            Value current_fn = frames.back();
            std::optional<Value> resultant_value;
            // Closure case: Closure consumes exactly one Argument (its Param)
            if (std::holds_alternative<Closure>(current_fn)) {
                const auto [param, body, env] = std::get<Closure>(current_fn);
                const auto &arg_thunk = work_args[idx++];
                const auto child_env = std::make_shared<Env>(env);
                child_env->bind(param, arg_thunk);
                resultant_value = eval_expr(*body, child_env);
            }
            // Native Function case: Consumes its arity-many Argument Thunks
            else if (std::holds_alternative<NativeFunction>(current_fn)) {
                const auto &[arity, name, impl] = std::get<NativeFunction>(current_fn);
                if (work_args.size() - idx < arity) {
                    print_loc_prefix(std::cerr, call_loc)
                            << "runtime error: native function " << name
                            << " expects " << arity << " argument(s), found "
                            << (work_args.size() - idx) << std::endl;
                    std::exit(EXIT_FAILURE);
                }
                std::vector<std::shared_ptr<Thunk> > slice;
                slice.reserve(arity);
                for (size_t i = 0; i < arity; ++i) {
                    slice.push_back(work_args[idx + i]);
                }
                resultant_value = impl(slice, call_site_env);
                idx += arity;
            } else {
                // Top frame is not a Function (Closure, NativeFunction) Value but there are still Arguments left
                print_loc_prefix(std::cerr, call_loc)
                        << "runtime error: trying to apply non-function value "
                        << frames.back() << std::endl;
                std::exit(EXIT_FAILURE);
            }
            if (!resultant_value) {
                print_loc_prefix(std::cerr, call_loc)
                        << "internal error: application produced no result" << std::endl;
                std::exit(EXIT_FAILURE);
            }
            if (std::holds_alternative<Closure>(*resultant_value) ||
                std::holds_alternative<NativeFunction>(*resultant_value)) {
                // Replace the top Frame with the returned Function (curry)
                frames.back() = std::move(*resultant_value);
                continue;
            }
            // Resultant is a concrete Value (different from Function Value i.e. double, string).
            // Pop the frame that produced it, and insert this Value as a thunk at the current idx
            // so the previous Frame (if any) will consume it.
            Value concrete = std::move(*resultant_value);
            frames.pop_back();
            // If there are no more Frames after popping, then this Value is the final result of the
            // whole Function Application, but only valid if there are no further Argument Thunks remaining.
            if (frames.empty()) {
                if (idx < work_args.size()) {
                    print_loc_prefix(std::cerr, call_loc)
                            << "runtime error: too many arguments applied to non-function value "
                            << concrete << std::endl;
                    std::exit(EXIT_FAILURE);
                }
                // No Thunks remaining then concrete Value is the result
                return concrete;
            }
            // There is a previous Frame; insert the concrete Value as a Thunk at the
            // current index so the previous Frame will consume it on the next iteration.
            auto inj_th = value_to_thunk(concrete);
            work_args.insert(work_args.begin() + static_cast<int>(idx), inj_th);
            // Do NOT advance idx: the inserted thunk is at position idx and will be consumed
            // by the previous frame on the next iteration.
            // Loop will continue with frames.back() being the previous frame.
        }
    }

    // Creates placeholder Thunk then set body so recursion can refer to it during lazy evaluation
    static void bind_def_ast_node_lazy(fe::ast::DefAstNode &def_ast_node, const std::shared_ptr<Env> &env,
                                       Options options) {
        const auto thunk = std::make_shared<Thunk>();
        env->bind(def_ast_node.def_name.value, thunk);
        if (options.own_expr) {
            thunk->set_owned(std::move(def_ast_node.expr), env, def_ast_node.expr.get_loc());
        } else {
            thunk->set(&def_ast_node.expr, env, def_ast_node.expr.get_loc());
        }
    }

    Result interpret(fe::ast::Program &program, std::optional<std::shared_ptr<Env> > global_env,
                     Options options) {
        if (!global_env) {
            global_env = std::make_shared<Env>();
            install_builtins(*global_env);
        }
        Value result_value;
        for (auto &[value]: program.nodes) {
            std::visit([&]<typename T0>(T0 &&arg) {
                using T = std::decay_t<T0>;
                if constexpr (std::is_same_v<T, fe::ast::Expression>) {
                    result_value = eval_expr(arg, *global_env);
                } else if constexpr (std::is_same_v<T, fe::ast::DefAstNode>) {
                    bind_def_ast_node_lazy(arg, *global_env, options);
                    const fe::ast::DefAstNode &def_ast_node = arg;
                    result_value = def_ast_node.def_name.value;
                } else {
                    STATIC_ASSERT_UNREACHABLE_T(T, "unhandled program node");
                }
            }, value);
        }
        return {*global_env, result_value};
    }

    void install_builtins(const std::shared_ptr<Env> &env) {
        for (auto &native_fn: get_builtins()) {
            const auto thunk = std::make_shared<Thunk>();
            thunk->cached = Value{native_fn};
            env->bind(native_fn.name, thunk);
        }
    }
}
