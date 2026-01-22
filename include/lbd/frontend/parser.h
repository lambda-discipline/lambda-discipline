#pragma once

#include <lbd/options.h>
#include <lbd/frontend/ast.h>
#include <lbd/frontend/token.h>
#include <lbd/intp/types.h>
#include <vector>

namespace frontend {
    struct Parser {
        Program program;

        explicit Parser(const std::vector<token::Token> &tokens, options::Options options_ = {});

    private:
        // TODO: Add checks for T to be a variant of frontend::TokenType
        template<typename T>
        static void assertToken(const std::vector<token::Token> &tokens, size_t &index);

        template<typename T>
        static void assertAndConsume(const std::vector<token::Token> &tokens, size_t &index);

        static IdentifierAstNode consumeIdentifier(const std::vector<token::Token> &tokens, size_t &index);

        static intp::types::PrimitiveType consumePrimitiveTypeName(const std::vector<token::Token> &tokens, size_t &index);

        static intp::types::Type parseType(const std::vector<token::Token> &tokens, size_t &index);

        static Expression parseExpression(const std::vector<token::Token> &tokens, size_t &index);

        static LambdaExpression parseLambdaExpression(const std::vector<token::Token> &tokens, size_t &index);

        static FunctionApplication parseFunctionApplication(const std::vector<token::Token> &tokens, size_t &index);

        static DefinitionAstNode parseDefinitionAstNode(const std::vector<token::Token> &tokens, size_t &index);

        static std::vector<AstNode> build(const std::vector<token::Token> &tokens, size_t &index);
    };
}
