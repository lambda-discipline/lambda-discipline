#pragma once

#include <lbd/frontend/location.h>
#include <lbd/interpreter/type.h>
#include <string>
#include <variant>
#include <vector>

namespace frontend {
    struct IdentifierAstNode {
        std::string value;
        Location location;

        friend std::ostream &operator<<(std::ostream &stream, const IdentifierAstNode &node);
    };

    struct StringAstNode {
        std::string value;
        Location location;

        friend std::ostream &operator<<(std::ostream &stream, const StringAstNode &node);
    };

    struct FloatAstNode {
        double value;
        Location location;

        friend std::ostream &operator<<(std::ostream &stream, const FloatAstNode &node);
    };

    struct Expression;

    struct LambdaExpression {
        IdentifierAstNode argument;
        interpreter::type::Type argumentType;
        std::unique_ptr<Expression> expression;
        Location location;
        /// Maybe, this is redundant.
        // TODO: Remove after checking.
        interpreter::type::CompoundType lambdaExpressionType;

        void print(std::ostream &stream, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &stream, const LambdaExpression &lambdaExpression);
    };

    struct FunctionApplication {
        IdentifierAstNode functionName;
        std::vector<std::unique_ptr<Expression> > arguments;
        Location location;

        void print(std::ostream &stream, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &stream, const FunctionApplication &functionApplication);
    };

    struct Expression {
        using ExpressionVariant = std::variant<
            IdentifierAstNode,
            StringAstNode,
            FloatAstNode,
            LambdaExpression,
            FunctionApplication
        >;

        ExpressionVariant value;

        explicit Expression(IdentifierAstNode value);

        explicit Expression(StringAstNode value);

        explicit Expression(FloatAstNode value);

        explicit Expression(LambdaExpression value);

        explicit Expression(FunctionApplication value);

        void print(std::ostream &stream, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &stream, const Expression &expression);

        [[nodiscard]] Location getLocation() const;

        Expression(const Expression &) = delete;

        Expression &operator=(const Expression &) = delete;

        Expression(Expression &&) noexcept = default;

        Expression &operator=(Expression &&) noexcept = default;
    };

    struct DefinitionAstNode {
        IdentifierAstNode definitionName;
        interpreter::type::Type definitionType;
        Expression expression;
        Location location;

        void print(std::ostream &stream, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &stream, const DefinitionAstNode &node);
    };

    struct AstNode {
        using NodeVariant = std::variant<Expression, DefinitionAstNode>;

        NodeVariant value;

        void print(std::ostream &stream, size_t indent) const;

        friend std::ostream &operator<<(std::ostream &stream, const AstNode &node);
    };

    struct Program {
        std::vector<AstNode> nodes;

        friend std::ostream &operator<<(std::ostream &stream, const Program &program);
    };
}
