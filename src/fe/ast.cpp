#include <lbd/error.h>
#include <lbd/fe/ast.h>

#include "lbd/string_escape.h"

namespace fe::ast {
    static void print_indent(std::ostream &os, const size_t indent) {
        for (size_t i = 0; i < indent; ++i) {
            os << "    ";
        }
    }

    std::ostream &operator<<(std::ostream &os, const IdenAstNode &node) {
        return os << node.value;
    }

    std::ostream &operator<<(std::ostream &os, const StringAstNode &node) {\
        return os << "\"" << escape(node.value) << "\"";
    }

    std::ostream &operator<<(std::ostream &os, const FloatAstNode &node) {
        return os << node.value;
    }

    void LambdaExpression::print(std::ostream &os, const size_t indent) const {
        print_indent(os, indent);
        os << "\\" << arg << ": " << arg_type << "." << std::endl;
        expr->print(os, indent + 1);
    }

    std::ostream &operator<<(std::ostream &os, const LambdaExpression &l_expr) {
        l_expr.print(os, 0);
        return os;
    }

    void FunctionApplication::print(std::ostream &os, const size_t indent) const {
        print_indent(os, indent);
        os << "(" << fn_name;
        for (auto &arg: args) {
            os << " " << *arg;
        }
        os << ")";
    }

    std::ostream &operator<<(std::ostream &os, const FunctionApplication &fn_apl) {
        fn_apl.print(os, 0);
        return os;
    }

    Expression::Expression(IdenAstNode value) : value(std::move(value)) {
    }

    Expression::Expression(StringAstNode value) : value(std::move(value)) {
    }

    Expression::Expression(FloatAstNode value) : value(std::move(value)) {
    }

    Expression::Expression(LambdaExpression value) : value(std::move(value)) {
    }

    Expression::Expression(FunctionApplication value) : value(std::move(value)) {
    }

    void Expression::print(std::ostream &os, size_t indent) const {
        std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, IdenAstNode>
                          || std::is_same_v<T, StringAstNode>
                          || std::is_same_v<T, FloatAstNode>) {
                print_indent(os, indent);
                os << arg;
            } else if constexpr (std::is_same_v<T, LambdaExpression> || std::is_same_v<T, FunctionApplication>) {
                arg.print(os, indent);
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled expression");
            }
        }, value);
    }

    std::ostream &operator<<(std::ostream &os, const Expression &expr) {
        expr.print(os, 0);
        return os;
    }

    loc::Loc Expression::get_loc() const {
        return std::visit([&](auto &&arg) {
            return arg.loc;
        }, value);
    }

    void DefAstNode::print(std::ostream &os, size_t indent) const {
        print_indent(os, indent);
        os << "def " << def_name << ": " << typ << " = " << expr;
    }

    std::ostream &operator<<(std::ostream &os, const DefAstNode &node) {
        node.print(os, 0);
        return os;
    }

    void AstNode::print(std::ostream &os, size_t indent) const {
        std::visit([&](auto &&arg) {
            arg.print(os, indent);
        }, value);
        os << std::endl;
    }

    std::ostream &operator<<(std::ostream &os, const AstNode &node) {
        node.print(os, 0);
        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Program &p) {
        for (const ast::AstNode &node: p.nodes) {
            os << node;
        }
        return os;
    }
}
