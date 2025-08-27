#pragma once

#include <lbd/fe/loc.h>
#include <lbd/intp/types.h>
#include <string>
#include <variant>
#include <vector>

namespace fe::ast {
    struct IdenAstNode {
        std::string value;
        loc::Loc loc;

        friend std::ostream &operator<<(std::ostream &os, const IdenAstNode &node);
    };

    struct StringAstNode {
        std::string value;
        loc::Loc loc;

        friend std::ostream &operator<<(std::ostream &os, const StringAstNode &node);
    };

    struct FloatAstNode {
        double value;
        loc::Loc loc;

        friend std::ostream &operator<<(std::ostream &os, const FloatAstNode &node);
    };

    struct Expression;

    struct LambdaExpression {
        IdenAstNode arg;
        intp::types::Type arg_type;
        std::unique_ptr<Expression> expr;
        loc::Loc loc;
        /// Maybe, this is redundant
        /// TODO: Remove after checking
        intp::types::CompoundType lmd_expr_type; // Lambda Expression Type

        void print(std::ostream &os, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &os, const LambdaExpression &l_expr);
    };

    struct FunctionApplication {
        IdenAstNode fn_name;
        std::vector<std::unique_ptr<Expression> > args;
        loc::Loc loc;

        void print(std::ostream &os, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &os, const FunctionApplication &fn_apl);
    };

    struct Expression {
        using ExpressionVariant = std::variant<
            IdenAstNode,
            StringAstNode,
            FloatAstNode,
            LambdaExpression,
            FunctionApplication
        >;

        ExpressionVariant value;

        explicit Expression(IdenAstNode value);

        explicit Expression(StringAstNode value);

        explicit Expression(FloatAstNode value);

        explicit Expression(LambdaExpression value);

        explicit Expression(FunctionApplication value);

        void print(std::ostream &os, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &os, const Expression &expr);

        [[nodiscard]] loc::Loc get_loc() const;

        Expression(const Expression &) = delete;

        Expression &operator=(const Expression &) = delete;

        Expression(Expression &&) noexcept = default;

        Expression &operator=(Expression &&) noexcept = default;
    };

    struct DefAstNode {
        IdenAstNode def_name;
        intp::types::Type typ;
        Expression expr;
        loc::Loc loc;

        void print(std::ostream &os, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &os, const DefAstNode &node);
    };

    struct AstNode {
        using NodeVariant = std::variant<Expression, DefAstNode>;

        NodeVariant value;

        void print(std::ostream &os, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &os, const AstNode &node);
    };

    struct Program {
        std::vector<AstNode> nodes;

        friend std::ostream &operator<<(std::ostream &os, const Program &p);
    };
}
