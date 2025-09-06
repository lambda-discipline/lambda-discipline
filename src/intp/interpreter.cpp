#include <lbd/intp/interpreter.h>
#include <lbd/intp/builtins.h>
#include <lbd/options.h>
#include <lbd/error.h>
#include <sstream>

namespace intp::interp {
    static options::Options options_v;
    static ResultOptions global_result_options;

    [[nodiscard]] std::string Closure::to_string() const {
        std::ostringstream oss;
        oss << "<closure: " << param << ">";
        return oss.str();
    }

    std::ostream &operator<<(std::ostream &os, const Closure &closure) {
        return os << closure.to_string();
    }

    [[nodiscard]] std::string List::to_string() const {
        std::ostringstream oss;
        oss << "[";
        for (size_t i = 0; i < elements.size(); ++i) {
            oss << elements[i];
            if (i + 1 != elements.size()) {
                oss << ", ";
            }
        }
        oss << "]";
        return oss.str();
    }

    [[nodiscard]] std::string Value::to_string() const {
        return std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, double>) {
                return std::to_string(arg);
            } else if constexpr (std::is_same_v<T, std::string>) {
                return arg;
            } else if constexpr (std::is_same_v<T, Closure>) {
                return arg.to_string();
            } else if constexpr (std::is_same_v<T, std::shared_ptr<NativeFunction> > ||
                                 std::is_same_v<T, std::shared_ptr<List> >) {
                return arg->to_string();
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled runtime value");
            }
        }, *this);
    }

    std::ostream &operator<<(std::ostream &os, const Value &value) {
        return os << value.to_string();
    }

    void ResultOptions::interpolate(const ResultOptions &result_options_) {
        side_effects |= result_options_.side_effects;
    }

    std::ostream &operator<<(std::ostream &os, const List &list) {
        return os << list.to_string();
    }

    [[nodiscard]] std::string NativeFunction::to_string() const {
        std::ostringstream oss;
        oss << "<native_fn: " << name << " " << arity << ">";
        return oss.str();
    }

    std::ostream &operator<<(std::ostream &os, const NativeFunction &native_fn) {
        return os << native_fn.to_string();
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
            options_v.logger.error(origin, "runtime error: forcing empty thunk");
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

    std::vector<std::vector<std::string> > Env::to_vector(const bool force) const {
        std::vector<std::vector<std::string> > vec;
        vec.reserve(table.size());
        for (const auto &[name, thunk]: table) {
            std::string val_str = "<thunk: unevaluated>";
            try {
                if (force) {
                    const Value &val = thunk->force();
                    val_str = val.to_string();
                } else if (thunk->cached) {
                    val_str = thunk->cached->to_string(); // already computed
                }
            } catch (const std::exception &) {
            }
            vec.push_back({name, val_str});
        }
        if (parent) {
            auto parent_vec = parent->to_vector(force);
            vec.insert(vec.end(), std::make_move_iterator(parent_vec.begin()),
                       std::make_move_iterator(parent_vec.end()));
        }
        return vec;
    }

    static Value eval_iden_ast_node(const fe::ast::IdenAstNode &iden_ast_node, const std::shared_ptr<Env> &env) {
        const auto thunk = env->lookup(iden_ast_node.value);
        if (!thunk) {
            options_v.logger.error(iden_ast_node.loc, "runtime error: undefined identifier ", iden_ast_node.value);
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
            options_v.logger.error(fn_apl.loc, "runtime error: undefined function ", fn_apl.fn_name.value);
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
                options_v.logger.error(call_loc, "internal error: no function frame to apply");
            }
            // Work on the top-most frame
            Value current_fn = frames.back();
            std::optional<Value> resultant_value;
            // If we've consumed all available Argument Thunks:
            if (idx >= work_args.size()) {
                // If top frame is a function, and we have no more args to feed it,
                // return that function (partial application) or its value as-is.
                if (std::holds_alternative<std::shared_ptr<NativeFunction> >(current_fn)) {
                    // Allow arity==0 and arity==-1 (variadic) to execute with zero args.
                    if (const auto &native_fn = *std::get<std::shared_ptr<NativeFunction> >(current_fn);
                        native_fn.arity == 0 || native_fn.arity == -1) {
                        std::vector<std::shared_ptr<Thunk> > slice; // empty
                        auto [value, result_options] = native_fn.impl(slice, call_site_env);
                        global_result_options.interpolate(result_options);
                        return value;
                    }
                    // Non-zero arity native function but no args left: partial application (return the function)
                    return current_fn;
                }
                if (std::holds_alternative<Closure>(current_fn)) {
                    // Closures require one argument; with none left, this is a partial application (return the closure).
                    return current_fn;
                }
                // It's not a function (shouldn't happen), return it as value.
                return current_fn;
            }
            // Closure case: Closure consumes exactly one Argument (its Param)
            if (std::holds_alternative<Closure>(current_fn)) {
                const auto [param, body, env] = std::get<Closure>(current_fn);
                const auto &arg_thunk = work_args[idx++];
                const auto child_env = std::make_shared<Env>(env);
                child_env->bind(param, arg_thunk);
                resultant_value = eval_expr(*body, child_env);
            }
            // Native Function case: Consumes its arity-many Argument Thunks
            else if (std::holds_alternative<std::shared_ptr<NativeFunction> >(current_fn)) {
                if (const auto &[arity, name, impl] = *std::get<std::shared_ptr<NativeFunction> >(current_fn);
                    arity != -1) {
                    if (work_args.size() - idx < arity) {
                        options_v.logger.error(call_loc, "runtime error: native function ", name, " expects ", arity,
                                               " argument(s), found ", work_args.size() - idx);
                    }
                    std::vector<std::shared_ptr<Thunk> > slice;
                    slice.reserve(arity);
                    for (size_t i = 0; i < arity; ++i) {
                        slice.push_back(work_args[idx + i]);
                    }
                    auto [resultant_value_, result_options] = impl(slice, call_site_env);
                    resultant_value = resultant_value_;
                    global_result_options.interpolate(result_options);
                    idx += arity;
                } else {
                    std::vector<std::shared_ptr<Thunk> > slice;
                    slice.reserve(args.size() - idx);
                    for (size_t i = 0; i < args.size() - idx; ++i) {
                        slice.push_back(work_args[idx + i]);
                    }
                    auto [resultant_value_, result_options] = impl(slice, call_site_env);
                    resultant_value = resultant_value_;
                    global_result_options.interpolate(result_options);
                    idx += args.size() - idx;
                }
            } else {
                // Top frame is not a Function (Closure, NativeFunction) Value but there are still Arguments left
                options_v.logger.error(call_loc, "runtime error: trying to apply non-function value ", frames.back());
            }
            if (!resultant_value) {
                options_v.logger.error(call_loc, "internal error: application produced no result");
            }
            if (std::holds_alternative<Closure>(*resultant_value) ||
                std::holds_alternative<std::shared_ptr<NativeFunction> >(*resultant_value)) {
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
                    options_v.logger.error(call_loc, "runtime error: too many arguments applied to non-function value ",
                                           concrete);
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
                                       const options::Options options) {
        const auto thunk = std::make_shared<Thunk>();
        env->bind(def_ast_node.def_name.value, thunk);
        if (options.own_expr) {
            thunk->set_owned(std::move(def_ast_node.expr), env, def_ast_node.expr.get_loc());
        } else {
            thunk->set(&def_ast_node.expr, env, def_ast_node.expr.get_loc());
        }
    }

    Result interpret(fe::ast::Program &program, std::optional<std::shared_ptr<Env> > global_env,
                     const options::Options options_) {
        options_v = options_;
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
                    bind_def_ast_node_lazy(arg, *global_env, options_v);
                    const fe::ast::DefAstNode &def_ast_node = arg;
                    result_value = def_ast_node.def_name.value;
                } else {
                    STATIC_ASSERT_UNREACHABLE_T(T, "unhandled program node");
                }
            }, value);
        }
        return {*global_env, result_value, global_result_options};
    }

    void install_builtins(const std::shared_ptr<Env> &env) {
        for (auto &native_fn: builtins::get_builtins(options_v)) {
            const auto thunk = std::make_shared<Thunk>();
            thunk->cached = Value{std::make_shared<NativeFunction>(native_fn)};
            env->bind(native_fn.name, thunk);
        }
    }
}
