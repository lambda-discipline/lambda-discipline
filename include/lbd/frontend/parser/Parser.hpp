#pragma once

#include <vector>
#include <lbd/Context.hpp>
#include <lbd/frontend/type.h>
#include <lbd/frontend/ast/Ast.hpp>
#include <lbd/frontend/lexer/Lexer.hpp>

namespace lbd::frontend::parser
{
    class Parser
    {
    public:
        explicit Parser(lexer::Lexer& lexer, Context& context);

        [[nodiscard]] std::vector<std::unique_ptr<ast::AstNode>> parse() const;

    private:
        void assertToken(token::TokenKind expectedKind) const;

        void assertAndAdvance(token::TokenKind expectedKind) const;

        [[nodiscard]] std::vector<std::unique_ptr<ast::AstNode>> parseFile(const std::string& path) const;

        [[nodiscard]] ast::DefinitionAstNode parseDefinitionAstNode() const;

        [[nodiscard]] ast::IdentifierAstNode parseIdentifierAstNode() const;

        [[nodiscard]] ast::StringAstNode parseStringAstNode() const;

        [[nodiscard]] ast::FloatAstNode parseNumberAstNode() const;

        [[nodiscard]] interpreter::type::PrimitiveType parsePrimitiveTypeName() const;

        [[nodiscard]] interpreter::type::Type parseType() const;

        [[nodiscard]] ast::Expression parseExpression() const;

        [[nodiscard]] ast::LambdaExpression parseLambdaExpression() const;

        [[nodiscard]] ast::FunctionApplication parseFunctionApplication() const;

        Context& context;
        lexer::Lexer& lexer;
    };
}
