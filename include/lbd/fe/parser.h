#pragma once

#include <lbd/options.h>
#include <lbd/fe/ast.h>
#include <lbd/fe/token.h>
#include <lbd/intp/types.h>
#include <vector>

namespace fe::parser {
    struct Parser {
        ast::Program program;

        explicit Parser(const std::vector<token::Token> &tokens, options::Options options_ = {});

    private:
        // TODO: Add checks for T to be a variant of fe::token::TokenType
        template<typename T>
        static void assertToken(const std::vector<token::Token> &tokens, size_t &index);

        template<typename T>
        static void assertAndConsume(const std::vector<token::Token> &tokens, size_t &index);

        static ast::IdentifierAstNode consumeIdentifier(const std::vector<token::Token> &tokens, size_t &index);

        static intp::types::PrimitiveType consumePrimitiveTypeName(const std::vector<token::Token> &tokens, size_t &index);

        static intp::types::Type parseType(const std::vector<token::Token> &tokens, size_t &index);

        static ast::Expression parseExpression(const std::vector<token::Token> &tokens, size_t &index);

        static ast::LambdaExpression parseLambdaExpression(const std::vector<token::Token> &tokens, size_t &index);

        static ast::FunctionApplication parseFunctionApplication(const std::vector<token::Token> &tokens, size_t &index);

        static ast::DefinitionAstNode parseDefinitionAstNode(const std::vector<token::Token> &tokens, size_t &index);

        static std::vector<ast::AstNode> build(const std::vector<token::Token> &tokens, size_t &index);
    };
}
