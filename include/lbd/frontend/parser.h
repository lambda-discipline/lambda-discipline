#pragma once

#include <lbd/options.h>
#include <lbd/frontend/ast.h>
#include <lbd/frontend/token.h>
#include <lbd/frontend/program.h>
#include <lbd/frontend/type.h>
#include <vector>

namespace frontend {
    struct Parser {
        Program parse();

        explicit Parser(const std::vector<token::Token> &tokens, context::Options options = {});

    private:
        const context::Options options;
        const std::vector<token::Token> &tokens;
        size_t index;

        // TODO: Add checks for T to be a variant of frontend::TokenType
        template<typename T>
        void assertToken();

        template<typename T>
        void assertAndConsume();

        IdentifierAstNode consumeIdentifier();

        interpreter::type::PrimitiveType consumePrimitiveTypeName();

        interpreter::type::Type parseType();

        Expression parseExpression();

        LambdaExpression parseLambdaExpression();

        FunctionApplication parseFunctionApplication();

        DefinitionAstNode parseDefinitionAstNode();

        std::vector<AstNode> build();
    };
}
