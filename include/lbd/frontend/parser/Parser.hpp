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
        [[nodiscard]] explicit Parser(Context& context, lexer::Lexer& lexer) noexcept;

        [[nodiscard]] std::vector<std::unique_ptr<ast::AstNode>> parse() const noexcept;

    private:
        void assertToken(token::TokenKind expectedKind) const noexcept;
        void assertAndAdvance(token::TokenKind expectedKind) const noexcept;

        [[nodiscard]] std::vector<std::unique_ptr<ast::AstNode>> parseFile(const std::string& path,
                                                                           const source::Range& range) const noexcept;

        [[nodiscard]] ast::DefinitionAstNode parseDefinitionAstNode() const noexcept;
        [[nodiscard]] ast::IdentifierAstNode parseIdentifierAstNode() const noexcept;
        [[nodiscard]] ast::StringAstNode parseStringAstNode() const noexcept;
        [[nodiscard]] ast::FloatAstNode parseNumberAstNode() const noexcept;
        [[nodiscard]] interpreter::type::PrimitiveType parsePrimitiveTypeName() const noexcept;
        [[nodiscard]] interpreter::type::Type parseType() const noexcept;
        [[nodiscard]] ast::Expression parseExpression() const noexcept;
        [[nodiscard]] ast::LambdaExpression parseLambdaExpression() const noexcept;
        [[nodiscard]] ast::FunctionApplication parseFunctionApplication() const noexcept;

        Context& context;
        lexer::Lexer& lexer;
    };
}
