#pragma once

#include <lbd/options.h>
#include <lbd/frontend/ast.h>
#include <lbd/frontend/token.h>
#include <lbd/frontend/program.h>
#include <lbd/interpreter/type.h>
#include <vector>

namespace frontend {
    struct Parser {
        // TODO: Make this private
        Program program;

        explicit Parser(const std::vector<token::Token> &tokens, context::Options options_ = {});

    private:
        // TODO: Add checks for T to be a variant of frontend::TokenType
        template<typename T>
        static void assertToken(const std::vector<token::Token> &tokens, size_t &index);

        template<typename T>
        static void assertAndConsume(const std::vector<token::Token> &tokens, size_t &index);

        static IdentifierAstNode consumeIdentifier(const std::vector<token::Token> &tokens, size_t &index);

        static interpreter::type::PrimitiveType consumePrimitiveTypeName(
            const std::vector<token::Token> &tokens, size_t &index);

        static interpreter::type::Type parseType(const std::vector<token::Token> &tokens, size_t &index);

        static Expression parseExpression(const std::vector<token::Token> &tokens, size_t &index);

        static LambdaExpression parseLambdaExpression(const std::vector<token::Token> &tokens, size_t &index);

        static FunctionApplication parseFunctionApplication(const std::vector<token::Token> &tokens, size_t &index);

        static DefinitionAstNode parseDefinitionAstNode(const std::vector<token::Token> &tokens, size_t &index);

        static std::vector<AstNode> build(const std::vector<token::Token> &tokens, size_t &index);
    };
}
