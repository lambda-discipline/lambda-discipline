#include <lbd/error.h>
#include <lbd/fe/ast.h>

#include "../../include/lbd/utils/string_escape.h"

namespace fe::ast {
    static void printIndent(std::ostream &stream, const size_t indent) {
        for (size_t i = 0; i < indent; ++i) {
            stream << "    ";
        }
    }

    std::ostream &operator<<(std::ostream &stream, const IdentifierAstNode &node) {
        return stream << node.value;
    }

    std::ostream &operator<<(std::ostream &stream, const StringAstNode &node) {\
        return stream << "\"" << escape(node.value) << "\"";
    }

    std::ostream &operator<<(std::ostream &stream, const FloatAstNode &node) {
        return stream << node.value;
    }

    void LambdaExpression::print(std::ostream &stream, const size_t indent) const {
        printIndent(stream, indent);
        stream << "\\" << arg << ": " << argumentType << "." << std::endl;
        expression->print(stream, indent + 1);
    }

    std::ostream &operator<<(std::ostream &stream, const LambdaExpression &lambdaExpression) {
        lambdaExpression.print(stream, 0);
        return stream;
    }

    void FunctionApplication::print(std::ostream &stream, const size_t indent) const {
        printIndent(stream, indent);
        stream << "(" << functionName;
        for (auto &argument: arguments) {
            stream << " " << *argument;
        }
        stream << ")";
    }

    std::ostream &operator<<(std::ostream &stream, const FunctionApplication &functionApplication) {
        functionApplication.print(stream, 0);
        return stream;
    }

    Expression::Expression(IdentifierAstNode value) : value(std::move(value)) {
    }

    Expression::Expression(StringAstNode value) : value(std::move(value)) {
    }

    Expression::Expression(FloatAstNode value) : value(std::move(value)) {
    }

    Expression::Expression(LambdaExpression value) : value(std::move(value)) {
    }

    Expression::Expression(FunctionApplication value) : value(std::move(value)) {
    }

    void Expression::print(std::ostream &stream, size_t indent) const {
        std::visit([&]<typename T0>(T0 &&argument) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, IdentifierAstNode>
                          || std::is_same_v<T, StringAstNode>
                          || std::is_same_v<T, FloatAstNode>) {
                printIndent(stream, indent);
                stream << argument;
            } else if constexpr (std::is_same_v<T, LambdaExpression> || std::is_same_v<T, FunctionApplication>) {
                argument.print(stream, indent);
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled expression");
            }
        }, value);
    }

    std::ostream &operator<<(std::ostream &stream, const Expression &expression) {
        expression.print(stream, 0);
        return stream;
    }

    loc::Loc Expression::getLocation() const {
        return std::visit([&](auto &&arg) {
            return arg.location;
        }, value);
    }

    void DefinitionAstNode::print(std::ostream &stream, const size_t indent) const {
        printIndent(stream, indent);
        stream << "def " << definitionName << ": " << definitionType << " = " << expression;
    }

    std::ostream &operator<<(std::ostream &stream, const DefinitionAstNode &node) {
        node.print(stream, 0);
        return stream;
    }

    void AstNode::print(std::ostream &stream, size_t indent) const {
        std::visit([&](auto &&arg) {
            arg.print(stream, indent);
        }, value);
        stream << std::endl;
    }

    std::ostream &operator<<(std::ostream &stream, const AstNode &node) {
        node.print(stream, 0);
        return stream;
    }

    std::ostream &operator<<(std::ostream &stream, const Program &program) {
        for (const AstNode &node: program.nodes) {
            stream << node;
        }
        return stream;
    }
}
